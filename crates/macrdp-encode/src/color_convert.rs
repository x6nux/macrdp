//! Apple vImage (Accelerate framework) SIMD-accelerated BGRA → NV12/I420 color conversion.
//!
//! Uses `vImageConvert_ARGB8888To420Yp8_Cb8_Cr8` (I420) and
//! `vImageConvert_ARGB8888To420Yp8_CbCr8` (NV12) with a BGRA→ARGB permute map
//! so no separate channel-swizzle pass is needed.

use std::ffi::c_void;

// ---------------------------------------------------------------------------
// FFI types — matching Apple's vImage_Types.h definitions exactly
// ---------------------------------------------------------------------------

/// `vImage_Buffer` — describes a raster plane for vImage operations.
/// Fields: data, height (vImagePixelCount = unsigned long), width, rowBytes (size_t).
#[repr(C)]
struct VImageBuffer {
    data: *mut c_void,
    height: usize,   // vImagePixelCount
    width: usize,    // vImagePixelCount
    row_bytes: usize, // size_t
}

/// Opaque conversion info populated by `vImageConvert_ARGBToYpCbCr_GenerateConversion`.
/// From vImage_Types.h: `uint8_t __attribute__((aligned(16))) opaque[128]`.
#[repr(C)]
#[repr(align(16))]
#[derive(Clone)]
struct VImageARGBToYpCbCr {
    _opaque: [u8; 128],
}

/// Pixel-range descriptor for Y′CbCr ↔ ARGB conversion.
/// From vImage_Types.h — all fields are `int32_t`.
#[repr(C)]
#[derive(Clone)]
struct VImageYpCbCrPixelRange {
    yp_bias: i32,
    cb_cr_bias: i32,
    yp_range_max: i32,
    cb_cr_range_max: i32,
    yp_max: i32,
    yp_min: i32,
    cb_cr_max: i32,
    cb_cr_min: i32,
}

/// Opaque matrix type for ARGB→YpCbCr. We only reference it by pointer.
#[repr(C)]
struct VImageARGBToYpCbCrMatrix {
    _opaque: [u8; 1],
}

// Format constants from vImage_Types.h enum values.
const KV_IMAGE_ARGB8888: u32 = 0;
const KV_IMAGE_420YP8_CB8_CR8: u32 = 3; // I420 (planar Cb + Cr)  — 'y420'/'f420'
const KV_IMAGE_420YP8_CBCR8: u32 = 4;   // NV12 (interleaved CbCr) — '420v'/'420f'

// vImage processing flags
const KV_IMAGE_NO_FLAGS: u32 = 0;

// ---------------------------------------------------------------------------
// FFI bindings — linked via Accelerate.framework
// ---------------------------------------------------------------------------

#[link(name = "Accelerate", kind = "framework")]
extern "C" {
    /// Pre-computed BT.601 matrix constant provided by Apple.
    /// Declared as: `extern const vImage_ARGBToYpCbCrMatrix *kvImage_ARGBToYpCbCrMatrix_ITU_R_601_4`
    /// i.e. this symbol IS the pointer itself.
    #[link_name = "kvImage_ARGBToYpCbCrMatrix_ITU_R_601_4"]
    static MATRIX_BT601_PTR: *const VImageARGBToYpCbCrMatrix;

    /// Build conversion info from a colour matrix + pixel range + format pair.
    fn vImageConvert_ARGBToYpCbCr_GenerateConversion(
        matrix: *const VImageARGBToYpCbCrMatrix,
        pixel_range: *const VImageYpCbCrPixelRange,
        info: *mut VImageARGBToYpCbCr,
        argb_format: u32,
        ypcbcr_format: u32,
        flags: u32,
    ) -> isize;

    /// ARGB8888 → planar Y′ + Cb + Cr (I420).
    fn vImageConvert_ARGB8888To420Yp8_Cb8_Cr8(
        src: *const VImageBuffer,
        dest_yp: *const VImageBuffer,
        dest_cb: *const VImageBuffer,
        dest_cr: *const VImageBuffer,
        info: *const VImageARGBToYpCbCr,
        permute_map: *const u8,
        flags: u32,
    ) -> isize;

    /// ARGB8888 → Y′ plane + interleaved CbCr plane (NV12).
    fn vImageConvert_ARGB8888To420Yp8_CbCr8(
        src: *const VImageBuffer,
        dest_yp: *const VImageBuffer,
        dest_cbcr: *const VImageBuffer,
        info: *const VImageARGBToYpCbCr,
        permute_map: *const u8,
        flags: u32,
    ) -> isize;
}

// ---------------------------------------------------------------------------
// Public converter
// ---------------------------------------------------------------------------

/// SIMD-accelerated BGRA → YUV colour converter backed by Apple vImage.
///
/// Holds two pre-baked conversion info blobs (one per output format) so the
/// costly matrix generation happens only once at construction time.
pub struct VImageConverter {
    /// Pre-computed info for BGRA → I420.
    i420_info: VImageARGBToYpCbCr,
    /// Pre-computed info for BGRA → NV12.
    nv12_info: VImageARGBToYpCbCr,
}

impl VImageConverter {
    /// Create a new converter using BT.601 full-range (0-255 for Y/Cb/Cr).
    pub fn new() -> Result<Self, String> {
        // Full-range: Yp_bias=0, CbCr_bias=128, all max=255, all min=0
        let pixel_range = VImageYpCbCrPixelRange {
            yp_bias: 0,
            cb_cr_bias: 128,
            yp_range_max: 255,
            cb_cr_range_max: 255,
            yp_max: 255,
            yp_min: 0,
            cb_cr_max: 255,
            cb_cr_min: 0,
        };

        let mut i420_info = VImageARGBToYpCbCr {
            _opaque: [0u8; 128],
        };
        let mut nv12_info = VImageARGBToYpCbCr {
            _opaque: [0u8; 128],
        };

        // Generate I420 conversion info
        let err = unsafe {
            vImageConvert_ARGBToYpCbCr_GenerateConversion(
                MATRIX_BT601_PTR,
                &pixel_range as *const _,
                &mut i420_info as *mut _,
                KV_IMAGE_ARGB8888,
                KV_IMAGE_420YP8_CB8_CR8,
                KV_IMAGE_NO_FLAGS,
            )
        };
        if err != 0 {
            return Err(format!(
                "vImageConvert_ARGBToYpCbCr_GenerateConversion (I420) failed: {err}"
            ));
        }

        // Generate NV12 conversion info
        let err = unsafe {
            vImageConvert_ARGBToYpCbCr_GenerateConversion(
                MATRIX_BT601_PTR,
                &pixel_range as *const _,
                &mut nv12_info as *mut _,
                KV_IMAGE_ARGB8888,
                KV_IMAGE_420YP8_CBCR8,
                KV_IMAGE_NO_FLAGS,
            )
        };
        if err != 0 {
            return Err(format!(
                "vImageConvert_ARGBToYpCbCr_GenerateConversion (NV12) failed: {err}"
            ));
        }

        Ok(Self {
            i420_info,
            nv12_info,
        })
    }

    /// Convert BGRA to planar I420 (Y + Cb + Cr contiguous in `yuv_out`).
    ///
    /// `yuv_out` must be at least `width * height * 3 / 2` bytes.
    /// Width and height must both be even.
    pub fn bgra_to_i420(
        &self,
        bgra: &[u8],
        width: u32,
        height: u32,
        stride: usize,
        yuv_out: &mut [u8],
    ) -> Result<(), String> {
        let w = width as usize;
        let h = height as usize;
        let y_size = w * h;
        let uv_w = w / 2;
        let uv_h = h / 2;
        let uv_size = uv_w * uv_h;
        let required = y_size + uv_size * 2;

        if w % 2 != 0 || h % 2 != 0 {
            return Err(format!("width ({w}) and height ({h}) must both be even"));
        }
        if bgra.len() < (h - 1) * stride + w * 4 {
            return Err(format!(
                "BGRA buffer too small: need at least {} bytes, got {}",
                (h - 1) * stride + w * 4,
                bgra.len()
            ));
        }
        if yuv_out.len() < required {
            return Err(format!(
                "I420 output buffer too small: need {required}, got {}",
                yuv_out.len()
            ));
        }

        // BGRA memory layout: [B, G, R, A]  (indices 0,1,2,3)
        // vImage expects ARGB:  [A, R, G, B] (indices 0,1,2,3)
        // permuteMap[dest_channel] = src_channel  →  [3, 2, 1, 0]
        let permute_map: [u8; 4] = [3, 2, 1, 0];

        let src_buf = VImageBuffer {
            data: bgra.as_ptr() as *mut c_void,
            height: h,
            width: w,
            row_bytes: stride,
        };

        // Split yuv_out into Y / Cb / Cr regions
        let (y_plane, uv_planes) = yuv_out.split_at_mut(y_size);
        let (cb_plane, cr_plane) = uv_planes.split_at_mut(uv_size);

        let y_buf = VImageBuffer {
            data: y_plane.as_mut_ptr() as *mut c_void,
            height: h,
            width: w,
            row_bytes: w,
        };
        let cb_buf = VImageBuffer {
            data: cb_plane.as_mut_ptr() as *mut c_void,
            height: uv_h,
            width: uv_w,
            row_bytes: uv_w,
        };
        let cr_buf = VImageBuffer {
            data: cr_plane.as_mut_ptr() as *mut c_void,
            height: uv_h,
            width: uv_w,
            row_bytes: uv_w,
        };

        let err = unsafe {
            vImageConvert_ARGB8888To420Yp8_Cb8_Cr8(
                &src_buf,
                &y_buf,
                &cb_buf,
                &cr_buf,
                &self.i420_info,
                permute_map.as_ptr(),
                KV_IMAGE_NO_FLAGS,
            )
        };
        if err != 0 {
            return Err(format!(
                "vImageConvert_ARGB8888To420Yp8_Cb8_Cr8 failed: {err}"
            ));
        }

        Ok(())
    }

    /// Convert BGRA to NV12 (separate Y plane + interleaved CbCr plane).
    ///
    /// `y_out` must be at least `width * height` bytes.
    /// `uv_out` must be at least `width * height / 2` bytes (interleaved CbCr pairs).
    /// Width and height must both be even.
    pub fn bgra_to_nv12(
        &self,
        bgra: &[u8],
        width: u32,
        height: u32,
        stride: usize,
        y_out: &mut [u8],
        uv_out: &mut [u8],
    ) -> Result<(), String> {
        let w = width as usize;
        let h = height as usize;
        let y_size = w * h;
        let uv_size = w * (h / 2); // interleaved CbCr: (w/2) samples * 2 bytes = w bytes per row, h/2 rows

        if w % 2 != 0 || h % 2 != 0 {
            return Err(format!("width ({w}) and height ({h}) must both be even"));
        }
        if bgra.len() < (h - 1) * stride + w * 4 {
            return Err(format!(
                "BGRA buffer too small: need at least {} bytes, got {}",
                (h - 1) * stride + w * 4,
                bgra.len()
            ));
        }
        if y_out.len() < y_size {
            return Err(format!(
                "Y output buffer too small: need {y_size}, got {}",
                y_out.len()
            ));
        }
        if uv_out.len() < uv_size {
            return Err(format!(
                "UV output buffer too small: need {uv_size}, got {}",
                uv_out.len()
            ));
        }

        let permute_map: [u8; 4] = [3, 2, 1, 0];

        let src_buf = VImageBuffer {
            data: bgra.as_ptr() as *mut c_void,
            height: h,
            width: w,
            row_bytes: stride,
        };

        let y_buf = VImageBuffer {
            data: y_out.as_mut_ptr() as *mut c_void,
            height: h,
            width: w,
            row_bytes: w,
        };
        let cbcr_buf = VImageBuffer {
            data: uv_out.as_mut_ptr() as *mut c_void,
            height: h / 2,
            width: w / 2,
            row_bytes: w, // interleaved CbCr: 2 bytes per sample, w/2 samples = w bytes per row
        };

        let err = unsafe {
            vImageConvert_ARGB8888To420Yp8_CbCr8(
                &src_buf,
                &y_buf,
                &cbcr_buf,
                &self.nv12_info,
                permute_map.as_ptr(),
                KV_IMAGE_NO_FLAGS,
            )
        };
        if err != 0 {
            return Err(format!(
                "vImageConvert_ARGB8888To420Yp8_CbCr8 failed: {err}"
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vimage_bgra_to_i420_gradient() {
        let converter = VImageConverter::new().expect("VImageConverter::new failed");

        let (w, h): (u32, u32) = (64, 64);
        let stride = w as usize * 4;
        let mut bgra = vec![0u8; stride * h as usize];

        // Create a diagonal gradient: R and G increase with col/row
        for row in 0..h as usize {
            for col in 0..w as usize {
                let px = row * stride + col * 4;
                bgra[px] = (col * 4).min(255) as u8;       // B
                bgra[px + 1] = (row * 4).min(255) as u8;   // G
                bgra[px + 2] = ((row + col) * 2).min(255) as u8; // R
                bgra[px + 3] = 255;                         // A
            }
        }

        let y_size = (w * h) as usize;
        let uv_size = (w / 2 * h / 2) as usize;
        let mut yuv = vec![0u8; y_size + uv_size * 2];

        converter
            .bgra_to_i420(&bgra, w, h, stride, &mut yuv)
            .expect("bgra_to_i420 failed");

        let y_plane = &yuv[..y_size];
        let cb_plane = &yuv[y_size..y_size + uv_size];
        let cr_plane = &yuv[y_size + uv_size..];

        // Y values should not be uniform (gradient input)
        let y_min = *y_plane.iter().min().unwrap();
        let y_max = *y_plane.iter().max().unwrap();
        assert!(
            y_max > y_min + 20,
            "Y plane should have variation for gradient input: min={y_min}, max={y_max}"
        );

        // UV values should have some variation too
        let cb_min = *cb_plane.iter().min().unwrap();
        let cb_max = *cb_plane.iter().max().unwrap();
        assert!(
            cb_max > cb_min,
            "Cb plane should have variation: min={cb_min}, max={cb_max}"
        );

        let cr_min = *cr_plane.iter().min().unwrap();
        let cr_max = *cr_plane.iter().max().unwrap();
        assert!(
            cr_max > cr_min,
            "Cr plane should have variation: min={cr_min}, max={cr_max}"
        );
    }

    #[test]
    fn test_vimage_bgra_to_nv12_uniform_gray() {
        let converter = VImageConverter::new().expect("VImageConverter::new failed");

        let (w, h): (u32, u32) = (64, 64);
        let stride = w as usize * 4;

        // Uniform gray: BGRA = (128, 128, 128, 255)
        let mut bgra = vec![0u8; stride * h as usize];
        for i in 0..(w * h) as usize {
            bgra[i * 4] = 128;     // B
            bgra[i * 4 + 1] = 128; // G
            bgra[i * 4 + 2] = 128; // R
            bgra[i * 4 + 3] = 255; // A
        }

        let y_size = (w * h) as usize;
        let uv_size = w as usize * (h as usize / 2); // interleaved CbCr
        let mut y_out = vec![0u8; y_size];
        let mut uv_out = vec![0u8; uv_size];

        converter
            .bgra_to_nv12(&bgra, w, h, stride, &mut y_out, &mut uv_out)
            .expect("bgra_to_nv12 failed");

        // For uniform gray (128,128,128), BT.601 full-range Y should be ~128
        // Allow some tolerance for rounding differences
        for (i, &y) in y_out.iter().enumerate() {
            assert!(
                (y as i32 - 128).unsigned_abs() <= 2,
                "Y[{i}] = {y}, expected ~128 for uniform gray"
            );
        }

        // UV should be ~128 (neutral chroma) for gray
        for (i, pair) in uv_out.chunks(2).enumerate() {
            let cb = pair[0];
            let cr = pair[1];
            assert!(
                (cb as i32 - 128).unsigned_abs() <= 2,
                "Cb[{i}] = {cb}, expected ~128 for gray"
            );
            assert!(
                (cr as i32 - 128).unsigned_abs() <= 2,
                "Cr[{i}] = {cr}, expected ~128 for gray"
            );
        }
    }

    #[test]
    fn test_vimage_bgra_to_i420_size_validation() {
        let converter = VImageConverter::new().expect("VImageConverter::new failed");

        let (w, h): (u32, u32) = (64, 64);
        let stride = w as usize * 4;
        let bgra = vec![128u8; stride * h as usize];

        // Provide a too-small output buffer
        let mut yuv_too_small = vec![0u8; 100]; // way too small for 64x64

        let result = converter.bgra_to_i420(&bgra, w, h, stride, &mut yuv_too_small);
        assert!(
            result.is_err(),
            "Should fail with too-small output buffer"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("too small"),
            "Error should mention 'too small', got: {err_msg}"
        );
    }
}
