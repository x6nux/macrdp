// Performance benchmarks for macrdp video pipeline at 4K@120fps
//
// Target: 8.33ms per frame (1/120s) for 120fps feasibility.
// Run with: cargo bench -p macrdp-encode

use criterion::{criterion_group, criterion_main, Criterion};
use macrdp_encode::color_convert::VImageConverter;
use macrdp_encode::{align16, OpenH264Encoder, VideoEncoder};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a gradient test pattern that simulates real screen content.
/// Uniform data compresses trivially and does not represent real workloads.
fn generate_test_pattern(width: u32, height: u32, stride: usize) -> Vec<u8> {
    let mut bgra = vec![0u8; stride * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * stride + x * 4;
            bgra[offset] = (x % 256) as u8; // B
            bgra[offset + 1] = (y % 256) as u8; // G
            bgra[offset + 2] = ((x + y) % 256) as u8; // R
            bgra[offset + 3] = 255; // A
        }
    }
    bgra
}

/// Scalar reference BGRA->YUV420 for comparison against vImage SIMD path.
/// Intentionally simple — matches the kind of loop the old code used.
fn scalar_bgra_to_yuv420(bgra: &[u8], width: u32, height: u32, stride: usize, yuv: &mut [u8]) {
    let w = width as usize;
    let h = height as usize;
    let y_size = w * h;
    let uv_w = w / 2;

    // Y plane
    for row in 0..h {
        for col in 0..w {
            let px = row * stride + col * 4;
            let b = bgra[px] as i32;
            let g = bgra[px + 1] as i32;
            let r = bgra[px + 2] as i32;
            yuv[row * w + col] = ((77 * r + 150 * g + 29 * b) >> 8).clamp(0, 255) as u8;
        }
    }

    // U and V planes (subsampled 2x2)
    for row in (0..h).step_by(2) {
        for col in (0..w).step_by(2) {
            let px = row * stride + col * 4;
            let b = bgra[px] as i32;
            let g = bgra[px + 1] as i32;
            let r = bgra[px + 2] as i32;
            let u_idx = y_size + (row / 2) * uv_w + col / 2;
            let v_idx = u_idx + uv_w * (h / 2);
            yuv[u_idx] = (((-43 * r - 85 * g + 128 * b) >> 8) + 128).clamp(0, 255) as u8;
            yuv[v_idx] = (((128 * r - 107 * g - 21 * b) >> 8) + 128).clamp(0, 255) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Color conversion benchmarks
// ---------------------------------------------------------------------------

fn bench_color_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_conversion");

    // -- 4K resolution: 3840 x 2160 ------------------------------------------
    let width = 3840u32;
    let height = 2160u32;
    let stride = width as usize * 4;
    let bgra = generate_test_pattern(width, height, stride);

    let converter = VImageConverter::new().expect("VImageConverter::new failed");

    // vImage BGRA -> I420 (4K)
    let mut yuv_i420 = vec![0u8; (width * height * 3 / 2) as usize];
    group.bench_function("vimage_bgra_to_i420_4k", |b| {
        b.iter(|| {
            converter
                .bgra_to_i420(&bgra, width, height, stride, &mut yuv_i420)
                .unwrap();
        })
    });

    // vImage BGRA -> NV12 (4K)
    let mut y_buf = vec![0u8; (width * height) as usize];
    let mut uv_buf = vec![0u8; (width * height / 2) as usize];
    group.bench_function("vimage_bgra_to_nv12_4k", |b| {
        b.iter(|| {
            converter
                .bgra_to_nv12(&bgra, width, height, stride, &mut y_buf, &mut uv_buf)
                .unwrap();
        })
    });

    // Scalar reference BGRA -> YUV420 (4K)
    let mut yuv_scalar = vec![0u8; (width * height * 3 / 2) as usize];
    group.bench_function("scalar_bgra_to_yuv420_4k", |b| {
        b.iter(|| {
            scalar_bgra_to_yuv420(&bgra, width, height, stride, &mut yuv_scalar);
        })
    });

    // -- 1080p for comparison -------------------------------------------------
    let w1080 = 1920u32;
    let h1080 = 1080u32;
    let s1080 = w1080 as usize * 4;
    let bgra_1080 = generate_test_pattern(w1080, h1080, s1080);

    let mut yuv_1080 = vec![0u8; (w1080 * h1080 * 3 / 2) as usize];
    group.bench_function("vimage_bgra_to_i420_1080p", |b| {
        b.iter(|| {
            converter
                .bgra_to_i420(&bgra_1080, w1080, h1080, s1080, &mut yuv_1080)
                .unwrap();
        })
    });

    let mut yuv_scalar_1080 = vec![0u8; (w1080 * h1080 * 3 / 2) as usize];
    group.bench_function("scalar_bgra_to_yuv420_1080p", |b| {
        b.iter(|| {
            scalar_bgra_to_yuv420(&bgra_1080, w1080, h1080, s1080, &mut yuv_scalar_1080);
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// OpenH264 encode benchmarks
// ---------------------------------------------------------------------------

fn bench_openh264_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("openh264_encode");
    group.sample_size(20); // fewer samples — encode is slow at 4K

    // 4K encode
    {
        let width = 3840u32;
        let height = 2160u32;
        let stride = width as usize * 4;
        let bgra = generate_test_pattern(width, height, stride);
        let mut encoder = OpenH264Encoder::new(
            align16(width),
            align16(height),
            120.0,
            50_000_000,
            false,
        )
        .expect("Failed to create 4K OpenH264 encoder");

        group.bench_function("openh264_4k_120fps", |b| {
            b.iter(|| {
                encoder.encode_bgra(&bgra, width, height, stride).unwrap();
            })
        });
    }

    // 1080p encode
    {
        let width = 1920u32;
        let height = 1080u32;
        let stride = width as usize * 4;
        let bgra = generate_test_pattern(width, height, stride);
        let mut encoder = OpenH264Encoder::new(
            align16(width),
            align16(height),
            120.0,
            30_000_000,
            false,
        )
        .expect("Failed to create 1080p OpenH264 encoder");

        group.bench_function("openh264_1080p_120fps", |b| {
            b.iter(|| {
                encoder.encode_bgra(&bgra, width, height, stride).unwrap();
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_color_conversion, bench_openh264_encode);
criterion_main!(benches);
