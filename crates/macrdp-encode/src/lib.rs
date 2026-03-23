//! Video encoding abstraction (H.264 via VideoToolbox / OpenH264)

mod openh264_enc;
pub mod yuv444_split;
#[cfg(target_os = "macos")]
mod videotoolbox;
#[cfg(target_os = "macos")]

use anyhow::Result;
use bytes::Bytes;

pub use openh264_enc::OpenH264Encoder;
#[cfg(target_os = "macos")]
pub use videotoolbox::VtEncoder;

/// Encoded frame output
pub struct EncodedFrame {
    /// H.264 NAL units (Annex B format)
    pub data: Bytes,
    /// Whether this is a key frame (IDR)
    pub is_keyframe: bool,
    pub width: u32,
    pub height: u32,
}

/// AVC444 dual-stream encoded result
pub struct Avc444EncodedFrame {
    /// Stream1: Main YUV420 H.264 (luma + downsampled chroma)
    pub main_view: EncodedFrame,
    /// Stream2: Auxiliary YUV420 H.264 (chroma compensation)
    pub aux_view: EncodedFrame,
}

/// Quality preset
#[derive(Debug, Clone, Copy)]
pub enum Quality {
    LowLatency,
    Balanced,
    HighQuality,
}

/// Video encoder trait
pub trait VideoEncoder: Send {
    /// AVC420 encode (existing)
    fn encode_bgra(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<EncodedFrame>;

    /// AVC444 dual-stream encode.
    /// Internally performs BGRA -> YUV444 -> B-area split -> dual session encode.
    fn encode_bgra_444(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<Avc444EncodedFrame>;

    fn set_bitrate(&mut self, bitrate_bps: u32);
    fn force_keyframe(&mut self);

    /// Whether this encoder supports AVC444 dual-stream encoding
    fn supports_444(&self) -> bool;
}

/// Align a dimension up to the nearest multiple of 16 (H.264 macroblock size)
pub fn align16(v: u32) -> u32 {
    (v + 15) & !15
}

/// Calculate optimal bitrate for screen content
pub fn screen_bitrate(width: u32, height: u32, fps: f32, quality: Quality) -> u32 {
    let pixels = width as f64 * height as f64;
    let base_bpp = match quality {
        Quality::LowLatency => 8.0,
        Quality::Balanced => 16.0,
        Quality::HighQuality => 24.0,
    };
    let fps_factor = (fps as f64 / 30.0).max(1.0);
    (pixels * base_bpp * fps_factor) as u32
}

/// Encoder preference
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncoderPreference {
    /// OpenH264 CPU encoder — full P-frame support, higher latency (~40ms)
    Software,
    /// VideoToolbox GPU encoder — IDR-only (P-frames incompatible with RDP AVC420), low latency (~6ms)
    Hardware,
    /// Same as Software (recommended default)
    Auto,
}

impl EncoderPreference {
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s.map(|s| s.to_lowercase()).as_deref() {
            Some("hardware") | Some("gpu") | Some("videotoolbox") | Some("vt") => Self::Hardware,
            Some("software") | Some("cpu") | Some("openh264") | Some("oh264") => Self::Software,
            _ => Self::Auto,
        }
    }
}

/// Create an H.264 encoder based on preference.
/// When `mode_444` is true, the encoder will initialize dual sessions for AVC444 support.
pub fn create_encoder(width: u32, height: u32, fps: f32, quality: Quality, preference: EncoderPreference, mode_444: bool, bitrate: u32) -> Result<Box<dyn VideoEncoder>> {
    let enc_w = align16(width);
    let enc_h = align16(height);

    // Hardware: VideoToolbox GPU encoder
    #[cfg(target_os = "macos")]
    if preference == EncoderPreference::Hardware {
        match VtEncoder::new(enc_w, enc_h, fps, bitrate, mode_444) {
            Ok(encoder) => {
                tracing::info!(enc_w, enc_h, mode_444, "Using VideoToolbox hardware encoder (GPU)");
                return Ok(Box::new(encoder));
            }
            Err(e) => {
                tracing::warn!("VideoToolbox unavailable: {e}, falling back to OpenH264");
            }
        }
    }

    // Software / Auto: OpenH264 CPU encoder (full P-frame support)
    tracing::info!(enc_w, enc_h, mode_444, "Using OpenH264 software encoder (CPU)");
    let encoder = OpenH264Encoder::new(enc_w, enc_h, fps, bitrate, mode_444)?;
    Ok(Box::new(encoder))
}
