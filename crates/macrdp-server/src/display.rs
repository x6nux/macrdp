use anyhow::Result;
use bytes::Bytes;
use ironrdp_server::{
    BitmapUpdate, DesktopSize, DisplayUpdate, GfxFrameUpdate, PixelFormat as RdpPixelFormat,
    RdpServerDisplay, RdpServerDisplayUpdates,
    gfx::GfxState,
};
use macrdp_capture::{CaptureConfig, CapturedFrame, CgFallbackCapturer, ScreenCapturer};
use macrdp_encode::{self, Quality, VideoEncoder};
use std::num::{NonZeroU16, NonZeroUsize};
use std::sync::{Arc, Mutex};

/// Maximum tile size for bitmap updates
const TILE_SIZE: u16 = 64;

/// Convert a captured frame into tiled BitmapUpdate chunks
pub fn frame_to_bitmap_updates(frame: &CapturedFrame, tile_size: u16) -> Vec<BitmapUpdate> {
    let mut updates = Vec::new();
    let bpp: usize = 4;

    let cols = (frame.width as u16 + tile_size - 1) / tile_size;
    let rows = (frame.height as u16 + tile_size - 1) / tile_size;

    for row in 0..rows {
        for col in 0..cols {
            let x = col * tile_size;
            let y = row * tile_size;
            let w = (frame.width as u16 - x).min(tile_size);
            let h = (frame.height as u16 - y).min(tile_size);

            let Some(width) = NonZeroU16::new(w) else { continue };
            let Some(height) = NonZeroU16::new(h) else { continue };

            let mut tile_data = Vec::with_capacity(w as usize * h as usize * bpp);
            for dy in 0..h {
                let src_y = (y + dy) as usize;
                let src_x_start = x as usize * bpp;
                let src_x_end = src_x_start + w as usize * bpp;
                let row_start = src_y * frame.stride;
                if row_start + src_x_end <= frame.data.len() {
                    tile_data.extend_from_slice(&frame.data[row_start + src_x_start..row_start + src_x_end]);
                }
            }

            let stride = w as usize * bpp;
            let Some(stride) = NonZeroUsize::new(stride) else { continue };

            updates.push(BitmapUpdate {
                x,
                y,
                width,
                height,
                format: RdpPixelFormat::BgrA32,
                data: Bytes::from(tile_data),
                stride,
            });
        }
    }

    updates
}

/// Display adapter that bridges ScreenCapturer to ironrdp-server
pub struct MacDisplay {
    width: u16,
    height: u16,
    /// Maximum resolution (auto-detected or configured). Client cannot exceed this.
    max_width: u16,
    max_height: u16,
    /// Whether resolution is fixed by config (true) or follows client (false)
    fixed_resolution: bool,
    frame_rate: u32,
    quality: Quality,
    encoder_pref: macrdp_encode::EncoderPreference,
    /// Whether AVC444 mode is requested by config
    mode_444: bool,
    base_bitrate: u32,
    gfx_state: Arc<Mutex<GfxState>>,
}

impl MacDisplay {
    pub fn new(
        width: u16, height: u16,
        fixed_resolution: bool,
        frame_rate: u32, quality: Quality,
        encoder_pref: macrdp_encode::EncoderPreference,
        mode_444: bool,
        bitrate_override: Option<u32>,
        gfx_state: Arc<Mutex<GfxState>>,
    ) -> Self {
        let base_bitrate = bitrate_override
            .unwrap_or_else(|| macrdp_encode::screen_bitrate(width as u32, height as u32, frame_rate as f32, quality));
        tracing::info!(base_bitrate_mbps = base_bitrate as f64 / 1_000_000.0, "Base bitrate");
        Self {
            width, height,
            max_width: width, max_height: height,
            fixed_resolution,
            frame_rate, quality, encoder_pref, mode_444, base_bitrate, gfx_state,
        }
    }
}

#[async_trait::async_trait]
impl RdpServerDisplay for MacDisplay {
    async fn size(&mut self) -> DesktopSize {
        DesktopSize { width: self.width, height: self.height }
    }

    fn request_resize(&mut self, width: u16, height: u16) {
        if self.fixed_resolution {
            tracing::debug!("Ignoring resize request — resolution is fixed by config");
            return;
        }
        let w = width.min(self.max_width);
        let h = height.min(self.max_height);
        if w > 0 && h > 0 && (w != self.width || h != self.height) {
            tracing::info!(
                old_w = self.width, old_h = self.height,
                new_w = w, new_h = h,
                "Adopting client-requested resolution"
            );
            self.width = w;
            self.height = h;
            self.base_bitrate = macrdp_encode::screen_bitrate(
                w as u32, h as u32, self.frame_rate as f32, self.quality,
            );
        }
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        let capture_config = CaptureConfig {
            width: self.width as u32,
            height: self.height as u32,
            frame_rate: self.frame_rate,
        };
        let capturer = ScreenCapturer::new(capture_config.clone()).await?;

        // Create H.264 encoder with configured quality and encoder preference
        let encoder = macrdp_encode::create_encoder(
            self.width as u32,
            self.height as u32,
            self.frame_rate as f32,
            self.quality,
            self.encoder_pref,
            self.mode_444,
            self.base_bitrate,
        ).ok();

        if encoder.is_some() {
            tracing::info!("H.264 encoder available — will use GFX path when client supports it");
        }

        Ok(Box::new(MacDisplayUpdates {
            capturer,
            capture_config,
            encoder,
            gfx_state: Arc::clone(&self.gfx_state),
            base_bitrate: self.base_bitrate,
            mode_444: self.mode_444,
            display_frame_count: 0,
        }))
    }
}

struct MacDisplayUpdates {
    capturer: ScreenCapturer,
    capture_config: CaptureConfig,
    encoder: Option<Box<dyn VideoEncoder>>,
    gfx_state: Arc<Mutex<GfxState>>,
    base_bitrate: u32,
    mode_444: bool,
    display_frame_count: u64,
}

#[async_trait::async_trait]
impl RdpServerDisplayUpdates for MacDisplayUpdates {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        // Drain stale frames — always use the latest available frame.
        // If SCK capturer stops (e.g. screen locked), fall back to CGDisplayCreateImage
        // which works at the display level (including lock screen).
        let frame = loop {
            let frame = match self.capturer.next_frame().await {
                Some(f) => f,
                None => {
                    // SCK stopped — fall back to CoreGraphics capture (works on lock screen)
                    tracing::warn!("SCStream stopped — switching to CoreGraphics fallback (lock screen?)");
                    let fallback = CgFallbackCapturer::new(&self.capture_config);
                    loop {
                        // Try to restore SCK (faster, has dirty rects)
                        match ScreenCapturer::new(self.capture_config.clone()).await {
                            Ok(new_capturer) => {
                                tracing::info!("SCStream recovered — switching back from CoreGraphics");
                                self.capturer = new_capturer;
                                break;
                            }
                            Err(_) => {
                                // SCK still unavailable — use CGDisplayCreateImage
                                if let Some(cg_frame) = fallback.capture_frame() {
                                    // Send this fallback frame through the normal encoding path
                                    return self.encode_and_send(cg_frame);
                                }
                                tokio::time::sleep(fallback.frame_interval()).await;
                            }
                        }
                    }
                    continue; // retry next_frame with restored SCK capturer
                }
            };
            // If another frame is already buffered, skip this one and grab the newer one
            // This prevents frame queuing which adds latency
            match self.capturer.try_next_frame() {
                Some(_newer) => continue, // drop older frame, grab newer
                None => break frame,
            }
        };

        self.encode_and_send(frame)
    }
}

impl MacDisplayUpdates {
    fn encode_and_send(&mut self, frame: CapturedFrame) -> Result<Option<DisplayUpdate>> {
        // Check GFX state and AVC444 negotiation
        let (gfx_ready, use_444) = {
            let state = self.gfx_state.lock().unwrap();
            let ready = state.is_ready() && self.encoder.is_some();
            let use_444 = self.mode_444
                && state.avc444_supported
                && state.avc444_enabled;
            (ready, use_444)
        };

        if gfx_ready {
            // GFX H.264 path — always send at capture rate, never block on acks
            if let Some(encoder) = &mut self.encoder {
                self.display_frame_count += 1;
                let t0 = std::time::Instant::now();

                // AVC444 dual-stream path
                if use_444 && encoder.supports_444() {
                    match encoder.encode_bgra_444(&frame.data, frame.width, frame.height, frame.stride) {
                        Ok(encoded) if !encoded.main_view.data.is_empty() => {
                            let encode_ms = t0.elapsed().as_secs_f64() * 1000.0;
                            let total_bytes = encoded.main_view.data.len() + encoded.aux_view.data.len();
                            tracing::debug!(
                                display_frame = self.display_frame_count,
                                main_bytes = encoded.main_view.data.len(),
                                aux_bytes = encoded.aux_view.data.len(),
                                is_keyframe = encoded.main_view.is_keyframe,
                                encode_ms = format!("{:.1}", encode_ms),
                                "Display: sending AVC444 GFX frame"
                            );
                            {
                                let mut st = self.gfx_state.lock().unwrap();
                                st.last_encode_ms = encode_ms;
                                st.last_frame_bytes = total_bytes as u32;
                            }
                            return Ok(Some(DisplayUpdate::GfxFrame(GfxFrameUpdate {
                                h264_data: encoded.main_view.data,
                                width: frame.width as u16,
                                height: frame.height as u16,
                                enc_width: encoded.main_view.width as u16,
                                enc_height: encoded.main_view.height as u16,
                                is_keyframe: encoded.main_view.is_keyframe,
                                h264_aux: Some(encoded.aux_view.data),
                            })));
                        }
                        Ok(_) => {
                            tracing::warn!(
                                display_frame = self.display_frame_count,
                                "AVC444 encode returned EMPTY data — frame dropped!"
                            );
                            return Ok(Some(DisplayUpdate::DefaultPointer));
                        }
                        Err(e) => {
                            tracing::warn!(display_frame = self.display_frame_count, "AVC444 encode failed: {e}, falling back to AVC420");
                            // Fall through to AVC420 path below
                        }
                    }
                }

                // AVC420 path (default or fallback from AVC444 failure)
                match encoder.encode_bgra(&frame.data, frame.width, frame.height, frame.stride) {
                    Ok(encoded) if !encoded.data.is_empty() => {
                        let encode_ms = t0.elapsed().as_secs_f64() * 1000.0;
                        tracing::debug!(
                            display_frame = self.display_frame_count,
                            h264_bytes = encoded.data.len(),
                            is_keyframe = encoded.is_keyframe,
                            encode_ms = format!("{:.1}", encode_ms),
                            "Display: sending GFX frame"
                        );
                        {
                            let mut st = self.gfx_state.lock().unwrap();
                            st.last_encode_ms = encode_ms;
                            st.last_frame_bytes = encoded.data.len() as u32;
                        }
                        return Ok(Some(DisplayUpdate::GfxFrame(GfxFrameUpdate {
                            h264_data: encoded.data,
                            width: frame.width as u16,
                            height: frame.height as u16,
                            enc_width: encoded.width as u16,
                            enc_height: encoded.height as u16,
                            is_keyframe: encoded.is_keyframe,
                            h264_aux: None,
                        })));
                    }
                    Ok(_) => {
                        tracing::warn!(
                            display_frame = self.display_frame_count,
                            "H.264 encode returned EMPTY data — frame dropped!"
                        );
                        return Ok(Some(DisplayUpdate::DefaultPointer));
                    }
                    Err(e) => {
                        tracing::warn!(display_frame = self.display_frame_count, "H.264 encode failed: {e}");
                    }
                }
            }
        } else if self.encoder.is_some() {
            // H.264 encoder exists — never send bitmaps, wait for GFX to become ready.
            // Mixing bitmap and GFX causes 0xd06 DECOMPRESSION_FAILED on reconnect.
            return Ok(Some(DisplayUpdate::DefaultPointer));
        }

        // Bitmap path (only when GFX is not available at all)
        if !frame.dirty_rects.is_empty() {
            // Find bounding box of all dirty rects to send a single update
            let mut min_x = frame.width;
            let mut min_y = frame.height;
            let mut max_x = 0u32;
            let mut max_y = 0u32;

            for r in &frame.dirty_rects {
                min_x = min_x.min(r.x);
                min_y = min_y.min(r.y);
                max_x = max_x.max(r.x + r.width);
                max_y = max_y.max(r.y + r.height);
            }

            // Clamp to frame bounds
            max_x = max_x.min(frame.width);
            max_y = max_y.min(frame.height);

            if max_x > min_x && max_y > min_y {
                let w = max_x - min_x;
                let h = max_y - min_y;
                let Some(width) = NonZeroU16::new(w as u16) else { return Ok(None) };
                let Some(height) = NonZeroU16::new(h as u16) else { return Ok(None) };

                // Extract only the dirty region from the full frame buffer
                let bpp = 4usize;
                let dirty_stride = w as usize * bpp;
                let mut dirty_data = Vec::with_capacity(dirty_stride * h as usize);
                for row in min_y..max_y {
                    let src_offset = row as usize * frame.stride + min_x as usize * bpp;
                    let src_end = src_offset + dirty_stride;
                    if src_end <= frame.data.len() {
                        dirty_data.extend_from_slice(&frame.data[src_offset..src_end]);
                    }
                }

                let Some(stride) = NonZeroUsize::new(dirty_stride) else { return Ok(None) };

                let update = BitmapUpdate {
                    x: min_x as u16,
                    y: min_y as u16,
                    width,
                    height,
                    format: RdpPixelFormat::BgrA32,
                    data: Bytes::from(dirty_data),
                    stride,
                };

                return Ok(Some(DisplayUpdate::Bitmap(update)));
            }
        }

        // No dirty rects available — send full frame (first frame or fallback)
        let Some(width) = NonZeroU16::new(frame.width as u16) else { return Ok(None) };
        let Some(height) = NonZeroU16::new(frame.height as u16) else { return Ok(None) };
        let Some(stride) = NonZeroUsize::new(frame.stride) else { return Ok(None) };

        let update = BitmapUpdate {
            x: 0,
            y: 0,
            width,
            height,
            format: RdpPixelFormat::BgrA32,
            data: frame.data,
            stride,
        };

        Ok(Some(DisplayUpdate::Bitmap(update)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_to_bitmap_updates() {
        let frame = CapturedFrame {
            width: 100,
            height: 50,
            data: Bytes::from(vec![0u8; 100 * 50 * 4]),
            stride: 400,
            timestamp_us: 0,
            dirty_rects: vec![],
        };

        let updates = frame_to_bitmap_updates(&frame, 64);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].x, 0);
        assert_eq!(updates[0].width.get(), 64);
        assert_eq!(updates[1].x, 64);
        assert_eq!(updates[1].width.get(), 36);
    }

    #[test]
    fn test_frame_to_bitmap_updates_exact_tile() {
        let frame = CapturedFrame {
            width: 128,
            height: 64,
            data: Bytes::from(vec![0u8; 128 * 64 * 4]),
            stride: 512,
            timestamp_us: 0,
            dirty_rects: vec![],
        };

        let updates = frame_to_bitmap_updates(&frame, 64);
        assert_eq!(updates.len(), 2);
    }
}
