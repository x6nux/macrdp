//! RDPGFX (Graphics Pipeline) DVC handler

use std::sync::{Arc, Mutex};

use ironrdp_core::{encode_vec, impl_as_any, Encode, WriteCursor, EncodeResult};
use ironrdp_dvc::{DvcEncode, DvcProcessor, DvcServerProcessor};
use ironrdp_pdu::geometry::InclusiveRectangle;
use ironrdp_pdu::rdp::vc::dvc::gfx::{
    Avc420BitmapStream, Avc444BitmapStream, CapabilitiesConfirmPdu, CapabilitiesV10Flags,
    CapabilitiesV103Flags, CapabilitiesV104Flags, CapabilitiesV107Flags, CapabilitiesV81Flags,
    CapabilitySet, ClientPdu, Codec1Type, CreateSurfacePdu, EndFramePdu, Encoding,
    MapSurfaceToOutputPdu, PixelFormat as GfxPixelFormat, QuantQuality, ResetGraphicsPdu,
    ServerPdu, StartFramePdu, Timestamp, WireToSurface1Pdu,
};
use ironrdp_pdu::gcc::{Monitor, MonitorFlags};
use ironrdp_pdu::{decode, PduResult};

type DvcMessage = ironrdp_dvc::DvcMessage;
use tracing::{debug, info};

use crate::display::GfxFrameUpdate;

/// GFX channel name as defined by RDP spec
pub const GFX_CHANNEL_NAME: &str = "Microsoft::Windows::RDS::Graphics";

/// Wrapper to make raw PDU bytes usable as DvcMessage
pub struct RawGfxPdu(pub Vec<u8>);

impl Encode for RawGfxPdu {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        dst.write_slice(&self.0);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "GfxServerPdu"
    }

    fn size(&self) -> usize {
        self.0.len()
    }
}

impl DvcEncode for RawGfxPdu {}

// SAFETY: RawGfxPdu only contains Vec<u8> which is Send
unsafe impl Send for RawGfxPdu {}

/// Wrap raw GFX PDU bytes in RDP_SEGMENTED_DATA (ZGFX) format.
/// MS-RDPEGFX Section 2.2.5: ALL GFX PDUs must be ZGFX-wrapped before DVC transport.
/// Using uncompressed mode (descriptor 0xE0/0xE1, compression type 0x04).
fn wrap_zgfx(data: &[u8]) -> Vec<u8> {
    const SINGLE: u8 = 0xE0;
    const MULTIPART: u8 = 0xE1;
    const UNCOMPRESSED: u8 = 0x04;
    // Max data per segment = 65534 bytes. The segmentSize field includes the
    // 1-byte compression type, so segmentSize = data_len + 1 ≤ 65535 (0xFFFF).
    const MAX_SEG_DATA: usize = 65534;

    if data.len() <= MAX_SEG_DATA {
        // Single segment: descriptor(1) + compression_type(1) + data
        let mut out = Vec::with_capacity(2 + data.len());
        out.push(SINGLE);
        out.push(UNCOMPRESSED);
        out.extend_from_slice(data);
        out
    } else {
        // Multipart: descriptor(1) + seg_count(2) + uncompressed_size(4) + segments
        let seg_count = data.len().div_ceil(MAX_SEG_DATA);
        let mut out = Vec::with_capacity(7 + data.len() + seg_count * 5);
        out.push(MULTIPART);
        out.extend_from_slice(&(seg_count as u16).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        for chunk in data.chunks(MAX_SEG_DATA) {
            // segmentSize = compression_type(1) + chunk data
            let seg_size = (chunk.len() + 1) as u32;
            out.extend_from_slice(&seg_size.to_le_bytes());
            out.push(UNCOMPRESSED);
            out.extend_from_slice(chunk);
        }
        out
    }
}

/// Unwrap ZGFX segmented data from client. Returns None if not ZGFX-wrapped.
fn unwrap_zgfx(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }
    match data[0] {
        0xE0 => {
            // Single segment: skip descriptor(1) + compression_type(1)
            if data.len() > 2 && data[1] == 0x04 {
                Some(data[2..].to_vec())
            } else {
                None
            }
        }
        0xE1 => {
            // Multipart: descriptor(1) + segment_count(2) + uncompressed_size(4)
            if data.len() < 7 {
                return None;
            }
            let seg_count = u16::from_le_bytes([data[1], data[2]]) as usize;
            let mut offset = 7;
            let mut result = Vec::new();
            for _ in 0..seg_count {
                if offset + 4 > data.len() {
                    break;
                }
                let seg_size = u32::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                ]) as usize;
                offset += 4;
                if offset < data.len() && data[offset] == 0x04 {
                    // Uncompressed: skip compression_type byte
                    offset += 1;
                    let end = (offset + seg_size - 1).min(data.len());
                    result.extend_from_slice(&data[offset..end]);
                    offset = end;
                } else {
                    break;
                }
            }
            Some(result)
        }
        _ => None, // Not ZGFX-wrapped
    }
}

fn make_dvc_message(pdu: &ServerPdu) -> PduResult<DvcMessage> {
    let data = encode_vec(pdu).map_err(|e| ironrdp_pdu::custom_err!("GfxEncode", e))?;
    let wrapped = wrap_zgfx(&data);
    Ok(Box::new(RawGfxPdu(wrapped)))
}

/// Shared state between GfxHandler and the server
#[derive(Debug)]
pub struct GfxState {
    pub channel_id: Option<u32>,
    pub surface_created: bool,
    pub caps_confirmed: bool,
    pub frame_id: u32,
    pub avc420_supported: bool,
    /// Whether the client supports AVC444 (V10+ with AVC not disabled)
    pub avc444_supported: bool,
    /// Whether AVC444 is enabled by server config (chroma_mode = "avc444")
    pub avc444_enabled: bool,
    pub confirmed_cap: Option<CapabilitySet>,
    pub width: u16,
    pub height: u16,
    /// Frames sent but not yet acknowledged
    pub pending_acks: u32,
    /// Last acknowledged frame ID
    pub last_ack_frame: u32,
    /// Network quality estimate (0.0 = congested, 1.0 = excellent)
    pub network_quality: f32,
    /// Send timestamps for RTT calculation (frame_id → Instant)
    pub frame_send_times: std::collections::HashMap<u32, std::time::Instant>,
    /// Exponentially weighted moving average RTT in ms
    pub rtt_ewma_ms: f64,
    /// Last frame encode time in ms
    pub last_encode_ms: f64,
    /// Last frame size in bytes
    pub last_frame_bytes: u32,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Time of first frame sent
    pub start_time: Option<std::time::Instant>,
    /// Instantaneous bitrate tracking (per-frame Mbps samples since last log)
    bitrate_samples: Vec<f64>,
    bitrate_max: f64,
    bitrate_min: f64,
    last_frame_time: Option<std::time::Instant>,
}

impl GfxState {
    pub fn new(width: u16, height: u16, avc444_enabled: bool) -> Self {
        Self {
            channel_id: None,
            surface_created: false,
            caps_confirmed: false,
            frame_id: 0,
            avc420_supported: false,
            avc444_supported: false,
            avc444_enabled,
            confirmed_cap: None,
            width,
            height,
            pending_acks: 0,
            last_ack_frame: 0,
            network_quality: 1.0,
            frame_send_times: std::collections::HashMap::new(),
            rtt_ewma_ms: 0.0,
            last_encode_ms: 0.0,
            last_frame_bytes: 0,
            total_bytes_sent: 0,
            start_time: None,
            bitrate_samples: Vec::new(),
            bitrate_max: 0.0,
            bitrate_min: f64::MAX,
            last_frame_time: None,
        }
    }

    pub fn next_frame_id(&mut self) -> u32 {
        self.frame_id += 1;
        self.pending_acks += 1;
        self.total_bytes_sent += self.last_frame_bytes as u64;
        if self.start_time.is_none() {
            self.start_time = Some(std::time::Instant::now());
        }
        // Track instantaneous bitrate per frame
        if let Some(prev) = self.last_frame_time {
            let dt = prev.elapsed().as_secs_f64().max(0.0001);
            let mbps = self.last_frame_bytes as f64 * 8.0 / dt / 1_000_000.0;
            self.bitrate_samples.push(mbps);
            if mbps > self.bitrate_max { self.bitrate_max = mbps; }
            if mbps < self.bitrate_min { self.bitrate_min = mbps; }
        }
        self.last_frame_time = Some(std::time::Instant::now());
        self.frame_send_times.insert(self.frame_id, std::time::Instant::now());
        // Limit map size to prevent unbounded growth
        if self.frame_send_times.len() > 120 {
            let cutoff = self.frame_id.saturating_sub(120);
            self.frame_send_times.retain(|&id, _| id > cutoff);
        }
        self.frame_id
    }

    /// Update network quality and RTT based on frame acknowledgment
    pub fn ack_frame(&mut self, ack_frame_id: u32) {
        self.last_ack_frame = ack_frame_id;
        if self.pending_acks > 0 {
            self.pending_acks -= 1;
        }

        // Calculate RTT from send timestamp
        if let Some(send_time) = self.frame_send_times.remove(&ack_frame_id) {
            let rtt_ms = send_time.elapsed().as_secs_f64() * 1000.0;
            // EWMA with alpha=0.2 for smooth averaging
            if self.rtt_ewma_ms == 0.0 {
                self.rtt_ewma_ms = rtt_ms;
            } else {
                self.rtt_ewma_ms = self.rtt_ewma_ms * 0.8 + rtt_ms * 0.2;
            }
        }

        // Network quality based on RTT and ack backlog
        self.network_quality = if self.rtt_ewma_ms < 15.0 && self.pending_acks <= 2 {
            1.0 // Excellent: max bitrate
        } else if self.rtt_ewma_ms < 30.0 && self.pending_acks <= 5 {
            0.8 // Good
        } else if self.rtt_ewma_ms < 60.0 && self.pending_acks <= 10 {
            0.5 // Fair
        } else if self.rtt_ewma_ms < 100.0 {
            0.3 // Poor
        } else {
            0.15 // Congested
        };
    }

    /// Get recommended bitrate based on network quality and base bitrate
    pub fn adaptive_bitrate(&self, base_bitrate: u32) -> u32 {
        (base_bitrate as f32 * self.network_quality) as u32
    }

    pub fn is_ready(&self) -> bool {
        self.channel_id.is_some() && self.avc420_supported && self.caps_confirmed
    }
}

/// GFX DVC processor
pub struct GfxHandler {
    pub state: Arc<Mutex<GfxState>>,
}

impl_as_any!(GfxHandler);

impl GfxHandler {
    pub fn new(state: Arc<Mutex<GfxState>>) -> Self {
        Self { state }
    }

    /// Create a single ZGFX-wrapped buffer containing all GFX PDUs for an H.264 frame.
    /// Per MS-RDPEGFX Section 2.2.5: "The server SHOULD combine multiple RDPGFX commands
    /// into a single RDP_SEGMENTED_DATA structure."
    /// First call also includes surface setup PDUs (ResetGraphics + CreateSurface + MapSurfaceToOutput).
    pub fn create_frame_pdu(state: &mut GfxState, frame: &GfxFrameUpdate) -> Vec<u8> {
        // Concatenate all raw GFX PDUs, then ZGFX-wrap once
        let mut raw_pdus = Vec::new();

        let enc_w = frame.enc_width;
        let enc_h = frame.enc_height;

        // First frame: surface setup (CapConfirm already sent by DVC handler)
        if !state.surface_created {
            if let Ok(data) = encode_vec(&ServerPdu::ResetGraphics(ResetGraphicsPdu {
                width: state.width as u32,
                height: state.height as u32,
                monitors: vec![Monitor {
                    left: 0,
                    top: 0,
                    right: state.width as i32 - 1,
                    bottom: state.height as i32 - 1,
                    flags: MonitorFlags::PRIMARY,
                }],
            })) {
                raw_pdus.extend_from_slice(&data);
            }

            if let Ok(data) = encode_vec(&ServerPdu::CreateSurface(CreateSurfacePdu {
                surface_id: 0,
                width: enc_w,
                height: enc_h,
                pixel_format: GfxPixelFormat::XRgb,
            })) {
                raw_pdus.extend_from_slice(&data);
            }

            if let Ok(data) = encode_vec(&ServerPdu::MapSurfaceToOutput(MapSurfaceToOutputPdu {
                surface_id: 0,
                output_origin_x: 0,
                output_origin_y: 0,
            })) {
                raw_pdus.extend_from_slice(&data);
            }

            state.surface_created = true;
            info!("GFX surface created: {}x{} (enc: {}x{})", state.width, state.height, enc_w, enc_h);
        }

        let frame_id = state.next_frame_id();

        // StartFrame
        if let Ok(data) = encode_vec(&ServerPdu::StartFrame(StartFramePdu {
            timestamp: Timestamp {
                milliseconds: 0,
                seconds: 0,
                minutes: 0,
                hours: 0,
            },
            frame_id,
        })) {
            raw_pdus.extend_from_slice(&data);
        }

        let make_rect = || InclusiveRectangle {
            left: 0,
            top: 0,
            right: frame.width,   // RDPGFX_RECT16 exclusive bound = visible crop
            bottom: frame.height, // RDPGFX_RECT16 exclusive bound = visible crop
        };

        let make_dest_rect = || InclusiveRectangle {
            left: 0,
            top: 0,
            right: enc_w,   // RDPGFX_RECT16 exclusive bound = encoder-aligned
            bottom: enc_h,  // RDPGFX_RECT16 exclusive bound = encoder-aligned
        };

        let make_qq = || QuantQuality {
            quantization_parameter: 22,
            progressive: false,
            quality: 100,
        };

        // Choose AVC444 or AVC420 path based on available data and negotiated caps
        let use_avc444 = frame.h264_aux.is_some()
            && state.avc444_supported
            && state.avc444_enabled;

        if use_avc444 {
            // AVC444 path: WireToSurface1 + Avc444BitmapStream
            let aux_data = frame.h264_aux.as_ref().unwrap();
            let avc444_stream = Avc444BitmapStream {
                encoding: Encoding::LUMA_AND_CHROMA,
                stream1: Avc420BitmapStream {
                    rectangles: vec![make_rect()],
                    quant_qual_vals: vec![make_qq()],
                    data: &frame.h264_data,
                },
                stream2: Some(Avc420BitmapStream {
                    rectangles: vec![make_rect()],
                    quant_qual_vals: vec![make_qq()],
                    data: aux_data,
                }),
            };

            if let Ok(avc444_data) = encode_vec(&avc444_stream) {
                if let Ok(data) = encode_vec(&ServerPdu::WireToSurface1(WireToSurface1Pdu {
                    surface_id: 0,
                    codec_id: Codec1Type::Avc444,
                    pixel_format: GfxPixelFormat::XRgb,
                    destination_rectangle: make_dest_rect(),
                    bitmap_data: avc444_data,
                })) {
                    raw_pdus.extend_from_slice(&data);
                }
            }
        } else {
            // AVC420 path: WireToSurface1 + Avc420BitmapStream
            let avc_stream = Avc420BitmapStream {
                rectangles: vec![make_rect()],
                quant_qual_vals: vec![make_qq()],
                data: &frame.h264_data,
            };

            if let Ok(avc_data) = encode_vec(&avc_stream) {
                if let Ok(data) = encode_vec(&ServerPdu::WireToSurface1(WireToSurface1Pdu {
                    surface_id: 0,
                    codec_id: Codec1Type::Avc420,
                    pixel_format: GfxPixelFormat::XRgb,
                    destination_rectangle: make_dest_rect(),
                    bitmap_data: avc_data,
                })) {
                    raw_pdus.extend_from_slice(&data);
                }
            }
        }

        // EndFrame
        if let Ok(data) = encode_vec(&ServerPdu::EndFrame(EndFramePdu { frame_id })) {
            raw_pdus.extend_from_slice(&data);
        }

        debug!(
            frame_id,
            raw_bytes = raw_pdus.len(),
            h264_bytes = frame.h264_data.len(),
            "GFX frame PDU created",
        );

        // Single ZGFX wrap for all concatenated PDUs
        wrap_zgfx(&raw_pdus)
    }
}

impl DvcProcessor for GfxHandler {
    fn channel_name(&self) -> &str {
        GFX_CHANNEL_NAME
    }

    fn start(&mut self, channel_id: u32) -> PduResult<Vec<DvcMessage>> {
        info!(channel_id, "GFX channel opened");
        let mut state = self.state.lock().unwrap();
        state.channel_id = Some(channel_id);
        Ok(Vec::new())
    }

    fn process(&mut self, _channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        // Client GFX data is also ZGFX-wrapped. Unwrap the ZGFX layer first.
        let raw_data = unwrap_zgfx(payload);
        let data = raw_data.as_deref().unwrap_or(payload);

        let client_pdu: ClientPdu = match decode(data) {
            Ok(pdu) => pdu,
            Err(e) => {
                // Unknown PDU type (e.g., QoE FrameAcknowledge 0x16, CacheImportOffer 0x10)
                // Log and ignore rather than crashing the connection
                debug!(
                    payload_len = data.len(),
                    first_bytes = ?&data[..data.len().min(8)],
                    "GFX: ignoring unknown client PDU: {e}"
                );
                return Ok(Vec::new());
            }
        };

        match client_pdu {
            ClientPdu::CapabilitiesAdvertise(caps) => {
                let cap_sets = &caps.0;
                info!("GFX client capabilities: {} sets", cap_sets.len());

                let mut state = self.state.lock().unwrap();

                // If already confirmed, ignore duplicate CapabilitiesAdvertise
                if state.caps_confirmed {
                    info!("GFX caps already confirmed, ignoring duplicate CapabilitiesAdvertise");
                    return Ok(Vec::new());
                }

                let mut best_cap = None;

                for cap in cap_sets {
                    match cap {
                        CapabilitySet::V10_7 { flags } if !flags.contains(CapabilitiesV107Flags::AVC_DISABLED) => {
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            best_cap = Some(cap.clone());
                            break;
                        }
                        CapabilitySet::V10_6 { flags } | CapabilitySet::V10_6Err { flags }
                            if !flags.contains(CapabilitiesV104Flags::AVC_DISABLED) =>
                        {
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            best_cap = Some(cap.clone());
                        }
                        CapabilitySet::V10_5 { flags } | CapabilitySet::V10_4 { flags }
                            if !flags.contains(CapabilitiesV104Flags::AVC_DISABLED) =>
                        {
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                        CapabilitySet::V10_3 { flags }
                            if !flags.contains(CapabilitiesV103Flags::AVC_DISABLED) =>
                        {
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                        CapabilitySet::V10_2 { flags } | CapabilitySet::V10 { flags }
                            if !flags.contains(CapabilitiesV10Flags::AVC_DISABLED) =>
                        {
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                        CapabilitySet::V10_1 => {
                            // V10_1 has no AVC_DISABLED flag — always supports AVC
                            state.avc420_supported = true;
                            state.avc444_supported = true;
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                        CapabilitySet::V8_1 { flags }
                            if flags.contains(CapabilitiesV81Flags::AVC420_ENABLED) =>
                        {
                            state.avc420_supported = true;
                            // V8.1 only supports AVC420, not AVC444
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                        _ => {
                            if best_cap.is_none() { best_cap = Some(cap.clone()); }
                        }
                    }
                }

                let confirmed = best_cap.unwrap_or(CapabilitySet::V8 {
                    flags: ironrdp_pdu::rdp::vc::dvc::gfx::CapabilitiesV8Flags::empty(),
                });

                info!(
                    avc420 = state.avc420_supported,
                    avc444_client = state.avc444_supported,
                    avc444_enabled = state.avc444_enabled,
                    "GFX capabilities negotiated"
                );

                // Send CapabilitiesConfirm from the DVC handler so it goes through
                // DrdynvcServer's proper encoding path. Bitmaps are suppressed once
                // GFX channel is open, so no bitmap/GFX mixing will occur.
                state.confirmed_cap = Some(confirmed.clone());
                state.caps_confirmed = true;

                let confirm_pdu = ServerPdu::CapabilitiesConfirm(CapabilitiesConfirmPdu(confirmed));
                let msg = make_dvc_message(&confirm_pdu)?;
                info!("GFX CapabilitiesConfirm sent via DVC handler");
                Ok(vec![msg])
            }

            ClientPdu::FrameAcknowledge(ack) => {
                let mut state = self.state.lock().unwrap();
                state.ack_frame(ack.frame_id);

                // Log stats every 60 acked frames (~1 second at 60fps)
                if ack.frame_id % 60 == 0 {
                    let net_ms = (state.rtt_ewma_ms - state.last_encode_ms).max(0.0);

                    // Compute instant bitrate stats from samples since last log
                    let n = state.bitrate_samples.len() as f64;
                    let (inst_avg, inst_std) = if n > 0.0 {
                        let sum: f64 = state.bitrate_samples.iter().sum();
                        let avg = sum / n;
                        let var: f64 = state.bitrate_samples.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / n;
                        (avg, var.sqrt())
                    } else {
                        (0.0, 0.0)
                    };
                    let inst_max = if state.bitrate_max > 0.0 { state.bitrate_max } else { 0.0 };
                    let inst_min = if state.bitrate_min < f64::MAX { state.bitrate_min } else { 0.0 };

                    info!(
                        "RTT {:.1}ms | encode {:.1}ms | net {:.1}ms | instant {:.1}/{:.1}/{:.1} Mbps (avg/max/min) | std {:.1} | {}KB/f | {} pending",
                        state.rtt_ewma_ms,
                        state.last_encode_ms,
                        net_ms,
                        inst_avg,
                        inst_max,
                        inst_min,
                        inst_std,
                        state.last_frame_bytes / 1024,
                        state.pending_acks,
                    );

                    // Reset for next window
                    state.bitrate_samples.clear();
                    state.bitrate_max = 0.0;
                    state.bitrate_min = f64::MAX;
                }
                Ok(Vec::new())
            }
        }
    }

    fn close(&mut self, channel_id: u32) {
        info!(channel_id, "GFX channel closed");
        let mut state = self.state.lock().unwrap();
        state.channel_id = None;
        state.surface_created = false;
        state.caps_confirmed = false;
        state.avc420_supported = false;
        state.avc444_supported = false;
        state.confirmed_cap = None;
    }
}

impl DvcServerProcessor for GfxHandler {}
