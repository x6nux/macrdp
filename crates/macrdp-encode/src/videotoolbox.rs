//! VideoToolbox H.264 hardware encoder (macOS)
//!
//! Uses Apple's VideoToolbox framework for GPU-accelerated H.264 encoding.
//! Accepts BGRA pixel data and outputs Annex B H.264 NAL units.

use anyhow::Result;
use bytes::Bytes;
use std::ffi::c_void;
use std::sync::Arc;

use crate::{Avc444EncodedFrame, EncodedFrame, VideoEncoder};

// --- FFI declarations ---

type CVPixelBufferRef = *mut c_void;
type VTCompressionSessionRef = *mut c_void;
type CMSampleBufferRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;
type CFAllocatorRef = *const c_void;
type OSStatus = i32;
type VTEncodeInfoFlags = u32;

#[repr(C)]
#[derive(Copy, Clone)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

impl CMTime {
    fn make(value: i64, timescale: i32) -> Self {
        Self { value, timescale, flags: 1, epoch: 0 } // flags=1 = valid
    }
    #[allow(dead_code)]
    fn invalid() -> Self {
        Self { value: 0, timescale: 0, flags: 0, epoch: 0 }
    }
}

type VTCompressionOutputCallback = extern "C" fn(
    output_callback_ref_con: *mut c_void,
    source_frame_ref_con: *mut c_void,
    status: OSStatus,
    info_flags: VTEncodeInfoFlags,
    sample_buffer: CMSampleBufferRef,
);

#[link(name = "VideoToolbox", kind = "framework")]
#[link(name = "CoreMedia", kind = "framework")]
#[link(name = "CoreVideo", kind = "framework")]
#[allow(dead_code)]
extern "C" {
    fn VTCompressionSessionCreate(
        allocator: CFAllocatorRef, width: i32, height: i32, codec_type: u32,
        encoder_specification: CFDictionaryRef,
        source_image_buffer_attributes: CFDictionaryRef,
        compressed_data_allocator: CFAllocatorRef,
        output_callback: Option<VTCompressionOutputCallback>,
        output_callback_ref_con: *mut c_void,
        compression_session_out: *mut VTCompressionSessionRef,
    ) -> OSStatus;
    fn VTSessionSetProperty(session: VTCompressionSessionRef, key: CFStringRef, value: CFTypeRef) -> OSStatus;
    fn VTCompressionSessionPrepareToEncodeFrames(session: VTCompressionSessionRef) -> OSStatus;
    fn VTCompressionSessionEncodeFrame(
        session: VTCompressionSessionRef, image_buffer: CVPixelBufferRef,
        pts: CMTime, duration: CMTime, frame_properties: CFDictionaryRef,
        source_frame_refcon: *mut c_void, info_flags_out: *mut VTEncodeInfoFlags,
    ) -> OSStatus;
    fn VTCompressionSessionCompleteFrames(
        session: VTCompressionSessionRef, complete_until_pts: CMTime,
    ) -> OSStatus;
    fn VTCompressionSessionInvalidate(session: VTCompressionSessionRef);

    fn CMSampleBufferGetDataBuffer(sbuf: CMSampleBufferRef) -> *mut c_void;
    fn CMBlockBufferGetDataPointer(
        buf: *mut c_void, offset: usize, length_at_offset_out: *mut usize,
        total_length_out: *mut usize, data_pointer_out: *mut *mut u8,
    ) -> OSStatus;
    fn CMBlockBufferGetDataLength(buf: *mut c_void) -> usize;
    fn CMSampleBufferGetFormatDescription(sbuf: CMSampleBufferRef) -> *const c_void;
    fn CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
        video_desc: *const c_void, index: usize,
        parameter_set_pointer_out: *mut *const u8, parameter_set_size_out: *mut usize,
        parameter_set_count_out: *mut usize, nal_unit_header_length_out: *mut i32,
    ) -> OSStatus;

    fn CVPixelBufferCreateWithBytes(
        allocator: CFAllocatorRef, width: usize, height: usize,
        pixel_format: u32, base_address: *mut c_void, bytes_per_row: usize,
        release_callback: *const c_void, release_ref_con: *mut c_void,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> OSStatus;
    fn CVPixelBufferCreateWithPlanarBytes(
        allocator: CFAllocatorRef, width: usize, height: usize,
        pixel_format_type: u32,
        data_ptr: *mut c_void,        // top-level data pointer (NULL for biplanar)
        data_size: usize,             // total data size
        number_of_planes: usize,
        plane_base_address: *const *mut c_void,
        plane_width: *const usize,
        plane_height: *const usize,
        plane_bytes_per_row: *const usize,
        release_callback: *const c_void,
        release_ref_con: *mut c_void,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> OSStatus;
    fn CVPixelBufferCreate(
        allocator: CFAllocatorRef, width: usize, height: usize,
        pixel_format_type: u32, pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> OSStatus;
    fn CVPixelBufferLockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
    fn CVPixelBufferUnlockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
    fn CVPixelBufferGetBaseAddress(pixel_buffer: CVPixelBufferRef) -> *mut c_void;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: CVPixelBufferRef) -> usize;
    fn CVPixelBufferRelease(pixel_buffer: CVPixelBufferRef);
    fn CVPixelBufferGetBaseAddressOfPlane(pixel_buffer: CVPixelBufferRef, plane_idx: usize) -> *mut c_void;
    fn CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer: CVPixelBufferRef, plane_idx: usize) -> usize;
    fn VTCompressionSessionGetPixelBufferPool(session: VTCompressionSessionRef) -> *mut c_void; // CVPixelBufferPoolRef
    fn CVPixelBufferPoolCreatePixelBuffer(
        allocator: CFAllocatorRef,
        pool: *mut c_void, // CVPixelBufferPoolRef
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> OSStatus;

    static kCVPixelBufferPixelFormatTypeKey: CFStringRef;
    static kCVPixelBufferWidthKey: CFStringRef;
    static kCVPixelBufferHeightKey: CFStringRef;

    static kCVPixelBufferIOSurfacePropertiesKey: CFStringRef;
    static kVTCompressionPropertyKey_RealTime: CFStringRef;
    static kVTCompressionPropertyKey_ProfileLevel: CFStringRef;
    static kVTCompressionPropertyKey_AllowFrameReordering: CFStringRef;
    static kVTCompressionPropertyKey_MaxKeyFrameInterval: CFStringRef;
    static kVTCompressionPropertyKey_ExpectedFrameRate: CFStringRef;
    static kVTCompressionPropertyKey_MaxFrameDelayCount: CFStringRef;
    static kVTCompressionPropertyKey_AverageBitRate: CFStringRef;
    static kVTCompressionPropertyKey_H264EntropyMode: CFStringRef;
    static kVTCompressionPropertyKey_AllowOpenGOP: CFStringRef;
    static kVTCompressionPropertyKey_AllowTemporalCompression: CFStringRef;
    static kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder: CFStringRef;
    static kVTVideoEncoderSpecification_EnableLowLatencyRateControl: CFStringRef;
    static kVTProfileLevel_H264_High_AutoLevel: CFStringRef;
    static kVTProfileLevel_H264_Baseline_AutoLevel: CFStringRef;
    static kVTProfileLevel_H264_ConstrainedBaseline_AutoLevel: CFStringRef;
    static kVTH264EntropyMode_CABAC: CFStringRef;
    static kVTH264EntropyMode_CAVLC: CFStringRef;

    static kCFBooleanTrue: CFTypeRef;
    static kCFBooleanFalse: CFTypeRef;

    fn CFNumberCreate(allocator: CFAllocatorRef, the_type: i64, value: *const c_void) -> CFTypeRef;
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef, keys: *const CFTypeRef, values: *const CFTypeRef,
        num_values: isize, key_callbacks: *const c_void, value_callbacks: *const c_void,
    ) -> CFDictionaryRef;
    fn CFArrayCreate(
        allocator: CFAllocatorRef, values: *const *const c_void,
        num_values: isize, callbacks: *const c_void,
    ) -> CFTypeRef;
    fn CFRelease(cf: *const c_void);

    static kCFTypeArrayCallBacks: c_void;
    static kVTCompressionPropertyKey_DataRateLimits: CFStringRef;
    static kVTCompressionPropertyKey_ColorPrimaries: CFStringRef;
    static kVTCompressionPropertyKey_TransferFunction: CFStringRef;
    static kVTCompressionPropertyKey_YCbCrMatrix: CFStringRef;
    static kCMFormatDescriptionExtension_FullRangeVideo: CFStringRef;
}

// CFNumber types
const K_CF_NUMBER_SINT32_TYPE: i64 = 3;
const K_CF_NUMBER_FLOAT64_TYPE: i64 = 13;

// Pixel format: BGRA
const K_CV_PIXEL_FORMAT_32BGRA: u32 = 0x42475241; // 'BGRA'

// Pixel format: NV12 (420f — YUV 4:2:0 biplanar, full range)
// Full range (Y: 0-255) avoids the washed-out look of video range (Y: 16-235)
const K_CV_PIXEL_FORMAT_420F: u32 = 0x34323066; // '420f'

// H.264 codec type
const K_CM_VIDEO_CODEC_TYPE_H264: u32 = 0x61766331; // 'avc1'

fn cf_i32(v: i32) -> CFTypeRef {
    unsafe { CFNumberCreate(std::ptr::null(), K_CF_NUMBER_SINT32_TYPE, &v as *const _ as *const c_void) }
}
fn cf_f64(v: f64) -> CFTypeRef {
    unsafe { CFNumberCreate(std::ptr::null(), K_CF_NUMBER_FLOAT64_TYPE, &v as *const _ as *const c_void) }
}

// --- Callback context (shared between encoder thread and VT callback thread) ---

struct CallbackCtx {
    output: std::sync::Mutex<(Vec<u8>, bool)>, // (nal_data, is_keyframe)
    ready: std::sync::Condvar,
    has_data: std::sync::atomic::AtomicBool,
}

extern "C" fn encode_callback(
    ref_con: *mut c_void, _source: *mut c_void, status: OSStatus,
    _info_flags: VTEncodeInfoFlags, sample_buffer: CMSampleBufferRef,
) {
    let ctx = unsafe { &*(ref_con as *const CallbackCtx) };

    if status != 0 || sample_buffer.is_null() {
        // Signal has_data so encoder doesn't timeout waiting — it will get empty data
        tracing::warn!(status, null_buf = sample_buffer.is_null(), "VT encode callback error");
        ctx.has_data.store(true, std::sync::atomic::Ordering::Release);
        ctx.ready.notify_one();
        return;
    }

    let mut annex_b = Vec::new();
    let mut is_keyframe = false;

    unsafe {
        let format_desc = CMSampleBufferGetFormatDescription(sample_buffer);
        let block_buf = CMSampleBufferGetDataBuffer(sample_buffer);
        if block_buf.is_null() { return; }

        let total_len = CMBlockBufferGetDataLength(block_buf);
        let mut data_ptr: *mut u8 = std::ptr::null_mut();
        let mut length: usize = 0;
        if CMBlockBufferGetDataPointer(block_buf, 0, &mut length, std::ptr::null_mut(), &mut data_ptr) != 0 {
            return;
        }
        let data = std::slice::from_raw_parts(data_ptr, total_len);

        // First pass: check if any NAL is IDR (type 5) to determine keyframe
        let mut nal_header_len: i32 = 4;
        {
            let mut param_count: usize = 0;
            let _ = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                format_desc, 0, std::ptr::null_mut(), std::ptr::null_mut(),
                &mut param_count, &mut nal_header_len,
            );
        }
        let nal_len_size = if nal_header_len > 0 { nal_header_len as usize } else { 4 };

        // Scan for IDR NAL to determine if this is a keyframe
        {
            let mut scan_offset = 0;
            while scan_offset + nal_len_size <= total_len {
                let nal_len = u32::from_be_bytes([
                    data[scan_offset], data[scan_offset+1],
                    data[scan_offset+2], data[scan_offset+3],
                ]) as usize;
                scan_offset += nal_len_size;
                if scan_offset + nal_len > total_len { break; }
                if nal_len > 0 {
                    let nal_type = data[scan_offset] & 0x1F;
                    if nal_type == 5 { is_keyframe = true; break; }
                }
                scan_offset += nal_len;
            }
        }

        // SPS/PPS: only prepend for IDR frames (start of new coded video sequence).
        // Sending SPS/PPS before P-frames can cause decoders to reset their
        // reference picture buffer, breaking temporal prediction.
        if is_keyframe {
            let mut param_count: usize = 0;
            if CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                format_desc, 0, std::ptr::null_mut(), std::ptr::null_mut(),
                &mut param_count, std::ptr::null_mut(),
            ) == 0 {
                for i in 0..param_count {
                    let mut ptr: *const u8 = std::ptr::null();
                    let mut size: usize = 0;
                    if CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                        format_desc, i, &mut ptr, &mut size,
                        std::ptr::null_mut(), std::ptr::null_mut(),
                    ) == 0 {
                        annex_b.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                        let param_data = std::slice::from_raw_parts(ptr, size);
                        annex_b.extend_from_slice(param_data);
                    }
                }
            }
        }

        // AVCC → Annex B conversion
        // Only keep VCL NALs (1=P-slice, 5=IDR) and parameter sets (7=SPS, 8=PPS).
        // Strip ALL other NAL types (SEI, AUD, filler, etc.) — VT's SEI may contain
        // Recovery Point info that causes Windows DXVA decoder to reset reference buffers.
        let mut offset = 0;
        while offset + nal_len_size <= total_len {
            let nal_len = u32::from_be_bytes([
                data[offset], data[offset+1], data[offset+2], data[offset+3],
            ]) as usize;
            offset += nal_len_size;
            if offset + nal_len > total_len { break; }
            if nal_len > 0 {
                let nal_type = data[offset] & 0x1F;
                if matches!(nal_type, 1 | 5 | 7 | 8) {
                    annex_b.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    annex_b.extend_from_slice(&data[offset..offset + nal_len]);
                }
            }
            offset += nal_len;
        }
    }

    // Signal data ready
    {
        let mut guard = ctx.output.lock().unwrap();
        *guard = (annex_b, is_keyframe);
    }
    ctx.has_data.store(true, std::sync::atomic::Ordering::Release);
    ctx.ready.notify_one();
}

// --- YUV444 reusable buffers for AVC444 mode ---

struct Yuv444Buffers {
    y444: Vec<u8>,
    u444: Vec<u8>,
    v444: Vec<u8>,
    main_view: crate::yuv444_split::Yuv420Frame,
    aux_view: crate::yuv444_split::Yuv420Frame,
}

impl Yuv444Buffers {
    fn new(width: u32, height: u32) -> Self {
        let full = (width * height) as usize;
        Self {
            y444: vec![0u8; full],
            u444: vec![0u8; full],
            v444: vec![0u8; full],
            main_view: crate::yuv444_split::Yuv420Frame::new(width, height),
            aux_view: crate::yuv444_split::Yuv420Frame::new(width, height),
        }
    }
}

/// Send-safe pointer wrappers for multi-threaded NV12 conversion.
/// Safety: callers ensure the pointed-to memory is valid and disjoint per thread.
#[derive(Clone, Copy)]
struct SendPtr(usize);
#[derive(Clone, Copy)]
struct SendPtrMut(usize);
unsafe impl Send for SendPtr {}
unsafe impl Send for SendPtrMut {}
unsafe impl Sync for SendPtr {}
unsafe impl Sync for SendPtrMut {}
impl SendPtr {
    fn from(p: *const u8) -> Self { Self(p as usize) }
    unsafe fn add(self, off: usize) -> *const u8 { (self.0 + off) as *const u8 }
}
impl SendPtrMut {
    fn from(p: *mut u8) -> Self { Self(p as usize) }
    unsafe fn add(self, off: usize) -> *mut u8 { (self.0 + off) as *mut u8 }
}

// --- Encoder ---

pub struct VtEncoder {
    session: VTCompressionSessionRef,
    session_aux: Option<VTCompressionSessionRef>,
    callback_ctx: Arc<CallbackCtx>,
    callback_ctx_aux: Option<Arc<CallbackCtx>>,
    width: u32,
    height: u32,
    frame_count: u64,
    fps: f32,
    mode_444: bool,
    yuv444_buf: Option<Yuv444Buffers>,
}

// VTCompressionSession is thread-safe per Apple docs
unsafe impl Send for VtEncoder {}

impl VtEncoder {
    pub fn new(width: u32, height: u32, fps: f32, bitrate: u32, mode_444: bool) -> Result<Self> {

        let callback_ctx = Arc::new(CallbackCtx {
            output: std::sync::Mutex::new((Vec::new(), false)),
            ready: std::sync::Condvar::new(),
            has_data: std::sync::atomic::AtomicBool::new(false),
        });

        // NV12 full-range input for both AVC420 and AVC444.
        // BGRA input produces video range (16-235) causing washed-out colors.
        let session = Self::create_session(width, height, fps, bitrate, &callback_ctx, K_CV_PIXEL_FORMAT_420F)?;

        // AVC444: single session, no aux session. Both streams use the same encoder
        // per MS-RDPEGFX requirement: "MUST be encoded using the same encoder".
        let (session_aux, callback_ctx_aux) = (None::<VTCompressionSessionRef>, None::<Arc<CallbackCtx>>);

        let yuv444_buf = if mode_444 {
            Some(Yuv444Buffers::new(width, height))
        } else {
            None
        };

        tracing::info!(
            width, height, fps, mode_444,
            bitrate_mbps = bitrate as f64 / 1_000_000.0,
            "VideoToolbox hardware encoder created"
        );

        Ok(Self {
            session,
            session_aux,
            callback_ctx,
            callback_ctx_aux,
            width,
            height,
            frame_count: 0,
            fps,
            mode_444,
            yuv444_buf,
        })
    }

    /// Create a VT compression session with the given parameters.
    /// `pixel_format` controls the expected input pixel format (BGRA or NV12).
    fn create_session(
        width: u32, height: u32, fps: f32, bitrate: u32,
        callback_ctx: &Arc<CallbackCtx>,
        pixel_format: u32,
    ) -> Result<VTCompressionSessionRef> {
        let mut session: VTCompressionSessionRef = std::ptr::null_mut();

        unsafe {
            // Hardware acceleration + low-latency rate control.
            // Low-latency RC produces clean first keyframes and adapts QP per-frame.
            // AverageBitRate serves as a target hint for its internal algorithm.
            let spec_keys = [
                kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder,
                kVTVideoEncoderSpecification_EnableLowLatencyRateControl,
            ];
            let spec_values = [kCFBooleanTrue, kCFBooleanTrue];
            let encoder_spec = CFDictionaryCreate(
                std::ptr::null(), spec_keys.as_ptr(), spec_values.as_ptr(),
                2, std::ptr::null(), std::ptr::null(),
            );

            // Source image buffer attributes — tell VT what pixel format to expect.
            // This allows VT to create a compatible pixel buffer pool.
            let src_keys: [CFTypeRef; 3] = [
                kCVPixelBufferPixelFormatTypeKey as CFTypeRef,
                kCVPixelBufferWidthKey as CFTypeRef,
                kCVPixelBufferHeightKey as CFTypeRef,
            ];
            let fmt_num = cf_i32(pixel_format as i32);
            let w_num = cf_i32(width as i32);
            let h_num = cf_i32(height as i32);
            let src_values: [CFTypeRef; 3] = [fmt_num, w_num, h_num];
            let src_attrs = CFDictionaryCreate(
                std::ptr::null(), src_keys.as_ptr(), src_values.as_ptr(),
                3, std::ptr::null(), std::ptr::null(),
            );

            let status = VTCompressionSessionCreate(
                std::ptr::null(), width as i32, height as i32,
                K_CM_VIDEO_CODEC_TYPE_H264,
                encoder_spec,
                src_attrs,  // source image buffer attributes
                std::ptr::null(),
                Some(encode_callback),
                Arc::as_ptr(callback_ctx) as *mut c_void,
                &mut session,
            );

            CFRelease(encoder_spec as *const _);
            CFRelease(src_attrs as *const _);
            CFRelease(fmt_num);
            CFRelease(w_num);
            CFRelease(h_num);

            if status != 0 || session.is_null() {
                anyhow::bail!("VTCompressionSessionCreate failed: {status}");
            }

            // Constrained Baseline Profile — compatible with Apple Silicon hardware encoder
            // in low-latency mode. High Profile causes null sample_buffer (frame drops) on
            // Apple Silicon with RequireHardwareAccelerated + EnableLowLatencyRateControl.
            VTSessionSetProperty(session, kVTCompressionPropertyKey_ProfileLevel,
                kVTProfileLevel_H264_ConstrainedBaseline_AutoLevel);
            // Explicit CAVLC entropy mode (required for Constrained Baseline)
            VTSessionSetProperty(session, kVTCompressionPropertyKey_H264EntropyMode,
                kVTH264EntropyMode_CAVLC);
            // Low-latency: no frame reordering, no B-frames, zero delay
            VTSessionSetProperty(session, kVTCompressionPropertyKey_RealTime, kCFBooleanTrue);
            VTSessionSetProperty(session, kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanFalse);
            VTSessionSetProperty(session, kVTCompressionPropertyKey_AllowOpenGOP, kCFBooleanFalse);
            VTSessionSetProperty(session, kVTCompressionPropertyKey_MaxFrameDelayCount, cf_i32(0));
            // Temporal compression (P-frames) — do NOT set ReferenceBufferCount
            // (on Apple Silicon, setting it to 1 forces all-IDR)
            VTSessionSetProperty(session, kVTCompressionPropertyKey_AllowTemporalCompression, kCFBooleanTrue);
            // Force full-range video output (Y: 0-255) to avoid washed-out colors.
            // Without this, VT defaults to video range (Y: 16-235) which looks gray.
            VTSessionSetProperty(session, kCMFormatDescriptionExtension_FullRangeVideo, kCFBooleanTrue);

            // Rate control: AverageBitRate only (soft target).
            // VT will aim for this average but allow bursts for keyframes.
            // No DataRateLimits — hard ceiling starves first keyframe causing blur.
            // No EnableLowLatencyRateControl — it ignores AverageBitRate entirely.
            VTSessionSetProperty(session, kVTCompressionPropertyKey_ExpectedFrameRate, cf_f64(fps as f64));
            VTSessionSetProperty(session, kVTCompressionPropertyKey_AverageBitRate, cf_i32(bitrate as i32));
            tracing::info!(bitrate_mbps = bitrate as f64 / 1_000_000.0, fps, "VT session bitrate set");
            // IDR every 2 seconds for error recovery
            VTSessionSetProperty(session, kVTCompressionPropertyKey_MaxKeyFrameInterval, cf_i32(fps as i32 * 2));

            VTCompressionSessionPrepareToEncodeFrames(session);
        }

        Ok(session)
    }

    /// Fast BGRA→NV12 full-range conversion via session pool buffer.
    /// Optimized: unsafe pointer math, no bounds checks, auto-vectorizable loops.
    fn create_nv12_from_bgra_fast(
        session: VTCompressionSessionRef,
        enc_w: u32, enc_h: u32,
        data: &[u8], src_w: u32, src_h: u32, stride: usize,
    ) -> Result<CVPixelBufferRef> {
        let mut pb: CVPixelBufferRef = std::ptr::null_mut();
        let w = enc_w as usize;
        let h = enc_h as usize;
        let sw = src_w.min(enc_w) as usize;
        let sh = src_h.min(enc_h) as usize;

        unsafe {
            let pool = VTCompressionSessionGetPixelBufferPool(session);
            if pool.is_null() { anyhow::bail!("VT pixel buffer pool is null"); }
            let status = CVPixelBufferPoolCreatePixelBuffer(std::ptr::null(), pool, &mut pb);
            if status != 0 || pb.is_null() { anyhow::bail!("Pool alloc failed: {status}"); }

            CVPixelBufferLockBaseAddress(pb, 0);
            let y_base = CVPixelBufferGetBaseAddressOfPlane(pb, 0) as *mut u8;
            let y_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 0);
            let uv_base = CVPixelBufferGetBaseAddressOfPlane(pb, 1) as *mut u8;
            let uv_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 1);

            if y_base.is_null() || uv_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("NV12 plane is null");
            }

            // Single-pass BGRA→NV12: process row pairs (Y for 2 rows + UV for 1 row).
            // Single-threaded: thread spawn/join overhead per frame exceeds savings.
            let src = data.as_ptr();
            let uv_w = sw / 2;
            for pr in 0..(sh / 2) {
                let r0 = pr * 2;
                let r1 = r0 + 1;
                let src_r0 = src.add(r0 * stride);
                let src_r1 = src.add(r1 * stride);
                let y_dst0 = y_base.add(r0 * y_bpr);
                let y_dst1 = y_base.add(r1 * y_bpr);
                let uv_dst = uv_base.add(pr * uv_bpr);

                for col in 0..uv_w {
                    let c0 = col * 2;
                    let c1 = c0 + 1;
                    let p00 = src_r0.add(c0 * 4);
                    let p01 = src_r0.add(c1 * 4);
                    let p10 = src_r1.add(c0 * 4);
                    let p11 = src_r1.add(c1 * 4);

                    let (b00, g00, r00) = (*p00 as i32, *p00.add(1) as i32, *p00.add(2) as i32);
                    let (b01, g01, r01) = (*p01 as i32, *p01.add(1) as i32, *p01.add(2) as i32);
                    let (b10, g10, r10) = (*p10 as i32, *p10.add(1) as i32, *p10.add(2) as i32);
                    let (b11, g11, r11) = (*p11 as i32, *p11.add(1) as i32, *p11.add(2) as i32);

                    *y_dst0.add(c0) = ((77 * r00 + 150 * g00 + 29 * b00) >> 8) as u8;
                    *y_dst0.add(c1) = ((77 * r01 + 150 * g01 + 29 * b01) >> 8) as u8;
                    *y_dst1.add(c0) = ((77 * r10 + 150 * g10 + 29 * b10) >> 8) as u8;
                    *y_dst1.add(c1) = ((77 * r11 + 150 * g11 + 29 * b11) >> 8) as u8;

                    let rb = (r00 + r01 + r10 + r11) >> 2;
                    let gb = (g00 + g01 + g10 + g11) >> 2;
                    let bb = (b00 + b01 + b10 + b11) >> 2;
                    *uv_dst.add(col * 2) = (((-43 * rb - 85 * gb + 128 * bb) >> 8) + 128).clamp(0, 255) as u8;
                    *uv_dst.add(col * 2 + 1) = (((128 * rb - 107 * gb - 21 * bb) >> 8) + 128).clamp(0, 255) as u8;
                }
            }

            CVPixelBufferUnlockBaseAddress(pb, 0);
        }
        Ok(pb)
    }

    /// Create a BGRA CVPixelBuffer with VT-managed memory and copy frame data into it.
    fn create_bgra_pixelbuffer(
        enc_w: u32, enc_h: u32,
        data: &[u8], src_w: u32, src_h: u32, stride: usize,
    ) -> Result<CVPixelBufferRef> {
        let mut pb: CVPixelBufferRef = std::ptr::null_mut();
        unsafe {
            let status = CVPixelBufferCreate(
                std::ptr::null(),
                enc_w as usize, enc_h as usize,
                K_CV_PIXEL_FORMAT_32BGRA,
                std::ptr::null(),
                &mut pb,
            );
            if status != 0 || pb.is_null() {
                anyhow::bail!("CVPixelBufferCreate BGRA failed: {status}");
            }

            CVPixelBufferLockBaseAddress(pb, 0);
            let base = CVPixelBufferGetBaseAddress(pb) as *mut u8;
            let bpr = CVPixelBufferGetBytesPerRow(pb);
            let copy_rows = (src_h as usize).min(enc_h as usize);
            let copy_cols_bytes = (src_w as usize * 4).min(bpr);
            for row in 0..copy_rows {
                let src_start = row * stride;
                let dst_start = row * bpr;
                if src_start + copy_cols_bytes <= data.len() {
                    std::ptr::copy_nonoverlapping(
                        data.as_ptr().add(src_start),
                        base.add(dst_start),
                        copy_cols_bytes,
                    );
                }
            }
            CVPixelBufferUnlockBaseAddress(pb, 0);
        }
        Ok(pb)
    }

    /// Convert I420 (YUV420) frame to BGRA using BT.601 reverse transform.
    fn yuv420_to_bgra(frame: &crate::yuv444_split::Yuv420Frame) -> Vec<u8> {
        let w = frame.width as usize;
        let h = frame.height as usize;
        let uv_w = w / 2;
        let mut bgra = vec![0u8; w * h * 4];

        for row in 0..h {
            for col in 0..w {
                let y_idx = row * w + col;
                let uv_idx = (row / 2) * uv_w + (col / 2);

                let y = frame.y.get(y_idx).copied().unwrap_or(16) as i32;
                let u = frame.u.get(uv_idx).copied().unwrap_or(128) as i32;
                let v = frame.v.get(uv_idx).copied().unwrap_or(128) as i32;

                let c = y - 16;
                let d = u - 128;
                let e = v - 128;
                let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
                let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
                let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;

                let px = (row * w + col) * 4;
                bgra[px] = b;
                bgra[px + 1] = g;
                bgra[px + 2] = r;
                bgra[px + 3] = 255;
            }
        }
        bgra
    }

    /// Convert BGRA frame to NV12 full-range via session pool buffer (single pass).
    /// BT.601 full range: Y=0-255, UV=0-255.
    fn create_nv12_from_bgra(
        session: VTCompressionSessionRef,
        enc_w: u32, enc_h: u32,
        data: &[u8], src_w: u32, src_h: u32, stride: usize,
    ) -> Result<CVPixelBufferRef> {
        let mut pb: CVPixelBufferRef = std::ptr::null_mut();
        let w = enc_w as usize;
        let h = enc_h as usize;
        let sw = src_w as usize;
        let sh = src_h as usize;

        unsafe {
            let pool = VTCompressionSessionGetPixelBufferPool(session);
            if pool.is_null() {
                anyhow::bail!("VT pixel buffer pool is null");
            }
            let status = CVPixelBufferPoolCreatePixelBuffer(std::ptr::null(), pool, &mut pb);
            if status != 0 || pb.is_null() {
                anyhow::bail!("CVPixelBufferPoolCreatePixelBuffer failed: {status}");
            }

            CVPixelBufferLockBaseAddress(pb, 0);

            let y_base = CVPixelBufferGetBaseAddressOfPlane(pb, 0) as *mut u8;
            let y_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 0);
            let uv_base = CVPixelBufferGetBaseAddressOfPlane(pb, 1) as *mut u8;
            let uv_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 1);

            if y_base.is_null() || uv_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("NV12 plane address is null");
            }

            let rows = sh.min(h);
            let cols = sw.min(w);

            // Y plane: full resolution, BT.601 full range
            for row in 0..rows {
                let bgra_row = row * stride;
                let y_row = row * y_bpr;
                for col in 0..cols {
                    let px = bgra_row + col * 4;
                    let b = data[px] as i32;
                    let g = data[px + 1] as i32;
                    let r = data[px + 2] as i32;
                    *y_base.add(y_row + col) = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
                }
            }

            // UV plane: half resolution, 2x2 averaged, BT.601 full range
            let uv_rows = rows / 2;
            let uv_cols = cols / 2;
            for row in 0..uv_rows {
                let r0 = row * 2;
                let r1 = r0 + 1;
                let uv_row_off = row * uv_bpr;
                for col in 0..uv_cols {
                    let c0 = col * 2;
                    let c1 = c0 + 1;
                    // 2x2 block averaging
                    let mut rb = 0i32; let mut gb = 0i32; let mut bb = 0i32;
                    for &sr in &[r0, r1] {
                        for &sc in &[c0, c1] {
                            let px = sr * stride + sc * 4;
                            bb += data[px] as i32;
                            gb += data[px + 1] as i32;
                            rb += data[px + 2] as i32;
                        }
                    }
                    rb >>= 2; gb >>= 2; bb >>= 2; // /4
                    let u = (((-43 * rb - 85 * gb + 128 * bb) >> 8) + 128).clamp(0, 255) as u8;
                    let v = (((128 * rb - 107 * gb - 21 * bb) >> 8) + 128).clamp(0, 255) as u8;
                    let off = uv_row_off + col * 2;
                    *uv_base.add(off) = u;
                    *uv_base.add(off + 1) = v;
                }
            }

            CVPixelBufferUnlockBaseAddress(pb, 0);
        }
        Ok(pb)
    }

    /// Allocate NV12 pixel buffer from VT session's pool (IOSurface-backed, hardware compatible)
    /// and fill with I420 plane data.
    fn create_nv12_from_session_pool(
        session: VTCompressionSessionRef,
        width: u32, height: u32,
        y_plane: &[u8], u_plane: &[u8], v_plane: &[u8],
    ) -> Result<CVPixelBufferRef> {
        let mut pb: CVPixelBufferRef = std::ptr::null_mut();
        let w = width as usize;
        let h = height as usize;

        unsafe {
            let pool = VTCompressionSessionGetPixelBufferPool(session);
            if pool.is_null() {
                anyhow::bail!("VTCompressionSessionGetPixelBufferPool returned null");
            }

            let status = CVPixelBufferPoolCreatePixelBuffer(std::ptr::null(), pool, &mut pb);
            if status != 0 || pb.is_null() {
                anyhow::bail!("CVPixelBufferPoolCreatePixelBuffer failed: {status}");
            }

            CVPixelBufferLockBaseAddress(pb, 0);

            // Plane 0: Y
            let y_base = CVPixelBufferGetBaseAddressOfPlane(pb, 0) as *mut u8;
            let y_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 0);
            if y_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("Pool NV12 Y plane is null");
            }
            for row in 0..h {
                let src_off = row * w;
                let dst_off = row * y_bpr;
                if src_off + w <= y_plane.len() {
                    std::ptr::copy_nonoverlapping(
                        y_plane.as_ptr().add(src_off),
                        y_base.add(dst_off),
                        w.min(y_bpr),
                    );
                }
            }

            // Plane 1: interleaved UV (NV12)
            let uv_base = CVPixelBufferGetBaseAddressOfPlane(pb, 1) as *mut u8;
            let uv_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 1);
            if uv_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("Pool NV12 UV plane is null");
            }
            let uv_w = w / 2;
            let uv_h = h / 2;
            for row in 0..uv_h {
                for col in 0..uv_w {
                    let src_idx = row * uv_w + col;
                    let dst_off = row * uv_bpr + col * 2;
                    if src_idx < u_plane.len() && src_idx < v_plane.len() {
                        *uv_base.add(dst_off) = u_plane[src_idx];
                        *uv_base.add(dst_off + 1) = v_plane[src_idx];
                    }
                }
            }

            CVPixelBufferUnlockBaseAddress(pb, 0);
        }

        Ok(pb)
    }

    /// Create an IOSurface-backed NV12 CVPixelBuffer from I420 planes.
    /// Uses CVPixelBufferCreate + memcpy into planes (IOSurface-backed = VT hardware compatible).
    #[allow(dead_code)]
    fn create_nv12_pixelbuffer(
        width: u32, height: u32,
        y_plane: &[u8], u_plane: &[u8], v_plane: &[u8],
    ) -> Result<CVPixelBufferRef> {
        let mut pb: CVPixelBufferRef = std::ptr::null_mut();
        let w = width as usize;
        let h = height as usize;

        unsafe {
            // Create NV12 pixel buffer without IOSurface properties.
            // VT will handle the memory backing internally.
            let status = CVPixelBufferCreate(
                std::ptr::null(),
                w, h,
                K_CV_PIXEL_FORMAT_420F,
                std::ptr::null(), // no attributes — avoids CVPixelBufferCreate+IOSurface+NV12 crash
                &mut pb,
            );

            if status != 0 || pb.is_null() {
                anyhow::bail!("CVPixelBufferCreate NV12 failed: status={status}");
            }

            CVPixelBufferLockBaseAddress(pb, 0);

            // Plane 0: Y (full resolution)
            let y_base = CVPixelBufferGetBaseAddressOfPlane(pb, 0) as *mut u8;
            let y_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 0);
            if y_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("NV12 Y plane is null");
            }
            for row in 0..h {
                let src_start = row * w;
                let dst_start = row * y_bpr;
                if src_start + w <= y_plane.len() {
                    std::ptr::copy_nonoverlapping(
                        y_plane.as_ptr().add(src_start),
                        y_base.add(dst_start),
                        w.min(y_bpr),
                    );
                }
            }

            // Plane 1: interleaved UV (NV12 CbCr)
            let uv_base = CVPixelBufferGetBaseAddressOfPlane(pb, 1) as *mut u8;
            let uv_bpr = CVPixelBufferGetBytesPerRowOfPlane(pb, 1);
            if uv_base.is_null() {
                CVPixelBufferUnlockBaseAddress(pb, 0);
                CVPixelBufferRelease(pb);
                anyhow::bail!("NV12 UV plane is null");
            }
            let uv_w = w / 2;
            let uv_h = h / 2;
            for row in 0..uv_h {
                for col in 0..uv_w {
                    let src_idx = row * uv_w + col;
                    let dst_offset = row * uv_bpr + col * 2;
                    if src_idx < u_plane.len() && src_idx < v_plane.len() {
                        *uv_base.add(dst_offset) = u_plane[src_idx];
                        *uv_base.add(dst_offset + 1) = v_plane[src_idx];
                    }
                }
            }

            CVPixelBufferUnlockBaseAddress(pb, 0);
        }

        Ok(pb)
    }

    /// Encode a single frame through a VT session and wait for the callback
    fn encode_session_frame(
        session: VTCompressionSessionRef,
        ctx: &Arc<CallbackCtx>,
        pixel_buffer: CVPixelBufferRef,
        pts: CMTime,
        duration: CMTime,
        frame_count: u64,
    ) -> Result<(Vec<u8>, bool)> {
        // Reset callback state
        {
            let mut guard = ctx.output.lock().unwrap();
            guard.0.clear();
            guard.1 = false;
            ctx.has_data.store(false, std::sync::atomic::Ordering::Release);
        }

        unsafe {
            let enc_status = VTCompressionSessionEncodeFrame(
                session, pixel_buffer, pts, duration,
                std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(),
            );
            if enc_status != 0 {
                anyhow::bail!("VTCompressionSessionEncodeFrame failed: {enc_status}");
            }
        }

        // Wait for callback
        let timed_out;
        {
            let guard = ctx.output.lock().unwrap();
            let (guard2, wait_result) = ctx.ready.wait_timeout_while(
                guard,
                std::time::Duration::from_millis(100),
                |_| !ctx.has_data.load(std::sync::atomic::Ordering::Acquire),
            ).unwrap();
            timed_out = wait_result.timed_out();
            drop(guard2);
        }

        if timed_out {
            tracing::warn!(frame = frame_count, "VT callback timeout — forcing CompleteFrames");
            unsafe {
                VTCompressionSessionCompleteFrames(session, pts);
            }
            let guard = ctx.output.lock().unwrap();
            let (guard2, _) = ctx.ready.wait_timeout_while(
                guard,
                std::time::Duration::from_millis(50),
                |_| !ctx.has_data.load(std::sync::atomic::Ordering::Acquire),
            ).unwrap();
            drop(guard2);
        }

        let result = {
            let guard = ctx.output.lock().unwrap();
            (guard.0.clone(), guard.1)
        };

        Ok(result)
    }
}

impl VideoEncoder for VtEncoder {
    fn encode_bgra(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<EncodedFrame> {
        let frame_duration = (600.0 / self.fps as f64) as i64;
        let pts = CMTime::make(self.frame_count as i64 * frame_duration, 600);
        let duration = CMTime::make(frame_duration, 600);

        // Convert BGRA → NV12 full-range via session pool buffer.
        let pixel_buffer = Self::create_nv12_from_bgra_fast(
            self.session, self.width, self.height, data, width, height, stride,
        )?;

        let (nal_data, is_keyframe) = Self::encode_session_frame(
            self.session, &self.callback_ctx, pixel_buffer, pts, duration, self.frame_count,
        )?;
        unsafe { CVPixelBufferRelease(pixel_buffer); }

        self.frame_count += 1;

        // NAL type diagnostic for first 10 frames
        if self.frame_count <= 10 {
            let mut nal_types = Vec::new();
            let mut profile_info = String::new();
            let mut scan = 0usize;
            while scan + 4 < nal_data.len() {
                if nal_data[scan] == 0 && nal_data[scan+1] == 0 && nal_data[scan+2] == 0 && nal_data[scan+3] == 1 {
                    scan += 4;
                    if scan < nal_data.len() {
                        let nal_type = nal_data[scan] & 0x1F;
                        let nal_name = match nal_type {
                            1 => "P-slice",
                            5 => "IDR",
                            6 => "SEI",
                            7 => "SPS",
                            8 => "PPS",
                            9 => "AUD",
                            _ => "other",
                        };
                        nal_types.push(format!("{}({})", nal_name, nal_type));
                        // Extract SPS profile info
                        if nal_type == 7 && scan + 3 < nal_data.len() {
                            let profile_idc = nal_data[scan + 1];
                            let constraint = nal_data[scan + 2];
                            let level_idc = nal_data[scan + 3];
                            let name = match profile_idc {
                                66 => "Baseline",
                                77 => "Main",
                                100 => "High",
                                _ => "Unknown",
                            };
                            profile_info = format!("{}(idc={},constraint=0x{:02X},level={})",
                                name, profile_idc, constraint, level_idc);
                        }
                    }
                } else {
                    scan += 1;
                }
            }
            tracing::debug!(
                frame = self.frame_count,
                output_bytes = nal_data.len(),
                is_keyframe,
                nal_units = nal_types.join(", "),
                profile = profile_info,
                "VideoToolbox NAL diagnostic"
            );
        }

        if self.frame_count % 300 == 0 {
            tracing::debug!(
                frame = self.frame_count,
                output_bytes = nal_data.len(),
                is_keyframe,
                "VideoToolbox encode result"
            );
        }

        Ok(EncodedFrame {
            data: Bytes::from(nal_data),
            is_keyframe,
            width: self.width,
            height: self.height,
        })
    }

    fn encode_bgra_444(&mut self, data: &[u8], width: u32, height: u32, stride: usize) -> Result<Avc444EncodedFrame> {
        let bufs = self.yuv444_buf.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AVC444 not enabled: no YUV444 buffers"))?;

        let w = self.width;
        let h = self.height;

        // Step 1: BGRA → YUV444 → B-area split (only needed for aux view)
        crate::yuv444_split::bgra_to_yuv444(
            data, width, height, stride,
            &mut bufs.y444, &mut bufs.u444, &mut bufs.v444,
        );
        // Main view = standard YUV420, aux view = chroma compensation
        crate::yuv444_split::yuv444_split_to_yuv420(
            &bufs.y444, &bufs.u444, &bufs.v444,
            w, h,
            &mut bufs.main_view, &mut bufs.aux_view,
        );

        // Single encoder session, sequential: main (frame 2N) then aux (frame 2N+1).
        let frame_duration = (600.0 / self.fps as f64) as i64;
        let duration = CMTime::make(frame_duration, 600);

        // Step 2: Encode main view — standard YUV420 from B-area split
        let main_pts = CMTime::make(self.frame_count as i64 * frame_duration, 600);
        let main_pb = Self::create_nv12_from_session_pool(
            self.session, w, h,
            &bufs.main_view.y, &bufs.main_view.u, &bufs.main_view.v,
        )?;
        let (main_nal, main_keyframe) = Self::encode_session_frame(
            self.session, &self.callback_ctx, main_pb, main_pts, duration, self.frame_count,
        )?;
        unsafe { CVPixelBufferRelease(main_pb); }

        // Step 3: Encode aux view — chroma compensation, same encoder (coherent refs)
        self.frame_count += 1;
        let aux_pts = CMTime::make(self.frame_count as i64 * frame_duration, 600);
        let aux_pb = Self::create_nv12_from_session_pool(
            self.session, w, h,
            &bufs.aux_view.y, &bufs.aux_view.u, &bufs.aux_view.v,
        )?;
        let (aux_nal, aux_keyframe) = Self::encode_session_frame(
            self.session, &self.callback_ctx, aux_pb, aux_pts, duration, self.frame_count,
        )?;
        unsafe { CVPixelBufferRelease(aux_pb); }

        self.frame_count += 1;

        tracing::debug!(
            frame = self.frame_count,
            main_bytes = main_nal.len(),
            aux_bytes = aux_nal.len(),
            "AVC444 dual-stream encode"
        );

        Ok(Avc444EncodedFrame {
            main_view: EncodedFrame {
                data: Bytes::from(main_nal),
                is_keyframe: main_keyframe,
                width: w, height: h,
            },
            aux_view: EncodedFrame {
                data: Bytes::from(aux_nal),
                is_keyframe: aux_keyframe,
                width: w, height: h,
            },
        })
    }

    fn set_bitrate(&mut self, bitrate_bps: u32) {
        unsafe {
            VTSessionSetProperty(self.session, kVTCompressionPropertyKey_AverageBitRate, cf_i32(bitrate_bps as i32));
        }
        tracing::debug!(bitrate_mbps = bitrate_bps as f64 / 1_000_000.0, "VideoToolbox bitrate updated");
    }

    fn force_keyframe(&mut self) {
        // Will be applied on next encode call via frame properties
    }

    fn supports_444(&self) -> bool {
        self.mode_444
    }
}

impl Drop for VtEncoder {
    fn drop(&mut self) {
        unsafe {
            VTCompressionSessionInvalidate(self.session);
            if let Some(aux) = self.session_aux {
                VTCompressionSessionInvalidate(aux);
            }
        }
    }
}
