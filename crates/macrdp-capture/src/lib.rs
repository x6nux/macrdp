//! macOS screen capture via ScreenCaptureKit

use anyhow::{Context, Result};
use bytes::Bytes;
use core_graphics::access::ScreenCaptureAccess;
use screencapturekit::cv::{CVPixelBuffer, CVPixelBufferLockFlags};
use screencapturekit::prelude::*;
use tokio::sync::mpsc;

/// Check if Screen Recording permission is granted (no prompt)
pub fn check_screen_recording_permission() -> bool {
    ScreenCaptureAccess::default().preflight()
}

/// Request Screen Recording permission (triggers system dialog if not granted)
/// Returns true if already granted. Note: even after granting, the app
/// may need to be restarted for the permission to take effect.
pub fn request_screen_recording_permission() -> bool {
    ScreenCaptureAccess::default().request()
}

/// Open System Settings to Privacy & Security page
pub fn open_screen_recording_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn();
}

/// A rectangle region
#[derive(Clone, Debug)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A captured screen frame in raw BGRA pixel format
#[derive(Clone, Debug)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub data: Bytes,
    /// Bytes per row
    pub stride: usize,
    pub timestamp_us: u64,
    /// Regions that changed since the last frame. Empty = full frame changed.
    pub dirty_rects: Vec<Rect>,
}

/// Configuration for screen capture
#[derive(Clone)]
pub struct CaptureConfig {
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
}

/// Screen capturer using ScreenCaptureKit
pub struct ScreenCapturer {
    _stream: SCStream,
    frame_rx: mpsc::Receiver<CapturedFrame>,
}

struct OutputHandler {
    frame_tx: mpsc::Sender<CapturedFrame>,
}

impl SCStreamOutputTrait for OutputHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        if of_type != SCStreamOutputType::Screen {
            return;
        }

        let frame = match extract_frame(&sample) {
            Some(f) => f,
            None => return,
        };

        // Non-blocking send — drop frame if channel is full
        let _ = self.frame_tx.try_send(frame);
    }
}

fn extract_frame(sample: &CMSampleBuffer) -> Option<CapturedFrame> {
    use screencapturekit::cm::SCFrameStatus;

    // Skip non-complete frames (idle, blank, suspended, etc.)
    match sample.frame_status() {
        Some(SCFrameStatus::Idle) | Some(SCFrameStatus::Blank)
        | Some(SCFrameStatus::Suspended) | Some(SCFrameStatus::Stopped) => {
            return None;
        }
        _ => {}
    }

    let pixel_buffer: CVPixelBuffer = sample.image_buffer()?;

    let guard = pixel_buffer.lock(CVPixelBufferLockFlags::READ_ONLY).ok()?;

    let width = guard.width() as u32;
    let height = guard.height() as u32;
    let stride = guard.bytes_per_row();
    let pixels = guard.as_slice();

    if width == 0 || height == 0 || pixels.is_empty() {
        return None;
    }

    // Extract dirty rects from the sample buffer
    // Extract dirty rects — screencapturekit's CGRect has x/y/width/height fields
    let dirty_rects = sample
        .dirty_rects()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.width > 0.0 && r.height > 0.0)
        .map(|r| Rect {
            x: r.x.max(0.0) as u32,
            y: r.y.max(0.0) as u32,
            width: r.width as u32,
            height: r.height as u32,
        })
        .collect::<Vec<_>>();

    let data = Bytes::copy_from_slice(pixels);

    let t = sample.presentation_timestamp();
    let timestamp_us = if t.timescale > 0 {
        ((t.value as u128 * 1_000_000) / t.timescale as u128) as u64
    } else {
        0
    };

    Some(CapturedFrame {
        width,
        height,
        data,
        stride,
        timestamp_us,
        dirty_rects,
    })
}

/// Query the main display's resolution
pub fn detect_display_size() -> Result<(u32, u32)> {
    let content = SCShareableContent::get()
        .context("Failed to get shareable content")?;
    let display = content
        .displays()
        .into_iter()
        .next()
        .context("No display found")?;
    Ok((display.width() as u32, display.height() as u32))
}

impl ScreenCapturer {
    /// Create a new screen capturer for the main display
    pub async fn new(config: CaptureConfig) -> Result<Self> {
        // SCShareableContent::get() is synchronous, run in blocking task
        let content = tokio::task::spawn_blocking(|| SCShareableContent::get())
            .await?
            .context("Failed to get shareable content (Screen Recording permission needed)")?;

        let display = content
            .displays()
            .into_iter()
            .next()
            .context("No display found")?;

        let actual_width = if config.width == 0 {
            display.width() as u32
        } else {
            config.width
        };
        let actual_height = if config.height == 0 {
            display.height() as u32
        } else {
            config.height
        };

        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        let frame_interval = CMTime::new(1, config.frame_rate as i32);

        let stream_config = SCStreamConfiguration::new()
            .with_width(actual_width)
            .with_height(actual_height)
            .with_minimum_frame_interval(&frame_interval)
            .with_pixel_format(PixelFormat::BGRA)
            .with_shows_cursor(true);

        // Channel for frames: buffer 2 frames to allow for jitter
        let (frame_tx, frame_rx) = mpsc::channel(2);

        let handler = OutputHandler { frame_tx };

        let mut stream = SCStream::new(&filter, &stream_config);
        stream.add_output_handler(handler, SCStreamOutputType::Screen);

        stream.start_capture().context("Failed to start capture")?;

        tracing::info!(
            width = actual_width,
            height = actual_height,
            fps = config.frame_rate,
            "Screen capture started"
        );

        Ok(Self {
            _stream: stream,
            frame_rx,
        })
    }

    /// Receive the next captured frame (async, cancellation safe)
    pub async fn next_frame(&mut self) -> Option<CapturedFrame> {
        self.frame_rx.recv().await
    }

    /// Try to get a buffered frame without waiting. Returns None if no frame ready.
    pub fn try_next_frame(&mut self) -> Option<CapturedFrame> {
        self.frame_rx.try_recv().ok()
    }
}

/// Fallback capturer using CGDisplayCreateImage (CoreGraphics).
/// Works during lock screen because it captures at the display level,
/// below the window server / ScreenCaptureKit layer.
pub struct CgFallbackCapturer {
    display_id: u32,
    width: u32,
    height: u32,
    frame_interval: std::time::Duration,
}

impl CgFallbackCapturer {
    /// Create a fallback capturer for the main display
    pub fn new(config: &CaptureConfig) -> Self {
        let display_id = core_graphics::display::CGDisplay::main().id;
        let fps = if config.frame_rate > 0 { config.frame_rate } else { 30 };
        Self {
            display_id,
            width: config.width,
            height: config.height,
            frame_interval: std::time::Duration::from_micros(1_000_000 / fps as u64),
        }
    }

    /// Capture a single frame using CGDisplayCreateImage
    pub fn capture_frame(&self) -> Option<CapturedFrame> {
        let display = core_graphics::display::CGDisplay::new(self.display_id);
        let image = display.image()?;

        let w = image.width() as u32;
        let h = image.height() as u32;
        let bpr = image.bytes_per_row();
        let data = image.data();
        let raw = data.bytes().to_vec();

        Some(CapturedFrame {
            width: if self.width > 0 { self.width } else { w },
            height: if self.height > 0 { self.height } else { h },
            data: Bytes::from(raw),
            stride: bpr,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            dirty_rects: vec![],
        })
    }

    /// Frame interval for pacing
    pub fn frame_interval(&self) -> std::time::Duration {
        self.frame_interval
    }
}
