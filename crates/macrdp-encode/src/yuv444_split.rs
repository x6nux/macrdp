//! YUV444 preprocessing for AVC444 dual-stream encoding.
//!
//! Implements BGRA → YUV444 conversion (BT.601) and the B-area pixel mapping
//! defined in MS-RDPEGFX §3.3.8.3.2 for splitting a YUV444 frame into two
//! standard YUV420 frames (Main View + Auxiliary View).

/// A standard YUV420 frame with separate Y, U, V planes.
pub struct Yuv420Frame {
    /// Luma plane, W × H bytes
    pub y: Vec<u8>,
    /// Chroma-blue plane, W/2 × H/2 bytes
    pub u: Vec<u8>,
    /// Chroma-red plane, W/2 × H/2 bytes
    pub v: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Yuv420Frame {
    /// Create a new zeroed frame with the given dimensions.
    /// Width and height should already be even (caller is responsible for alignment).
    pub fn new(width: u32, height: u32) -> Self {
        let y_size = (width * height) as usize;
        let uv_size = (width / 2 * height / 2) as usize;
        Self {
            y: vec![0u8; y_size],
            u: vec![128u8; uv_size],
            v: vec![128u8; uv_size],
            width,
            height,
        }
    }

    /// Ensure the frame is sized for the given dimensions, reallocating if needed.
    /// Resets all planes to neutral values (Y=0, U=V=128).
    pub fn ensure_size(&mut self, width: u32, height: u32) {
        let y_size = (width * height) as usize;
        let uv_size = (width / 2 * height / 2) as usize;

        if self.y.len() != y_size {
            self.y.resize(y_size, 0);
        }
        if self.u.len() != uv_size {
            self.u.resize(uv_size, 128);
            self.v.resize(uv_size, 128);
        }

        self.y.fill(0);
        self.u.fill(128);
        self.v.fill(128);
        self.width = width;
        self.height = height;
    }
}

/// Convert a BGRA frame to full-resolution YUV444 planes using BT.601 **full range**.
///
/// The output planes `y444`, `u444`, `v444` must each be at least `width * height` bytes.
/// BGRA byte order: B[0], G[1], R[2], A[3].
///
/// BT.601 full range (Y: 0-255, UV: 0-255) — matches NV12 '420f' format:
/// ```text
/// Y =  (77*R + 150*G + 29*B) >> 8
/// U = (-43*R -  85*G + 128*B) >> 8 + 128
/// V = (128*R - 107*G -  21*B) >> 8 + 128
/// ```
pub fn bgra_to_yuv444(
    bgra: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    y444: &mut [u8],
    u444: &mut [u8],
    v444: &mut [u8],
) {
    let w = width as usize;
    let h = height as usize;
    debug_assert!(y444.len() >= w * h);
    debug_assert!(u444.len() >= w * h);
    debug_assert!(v444.len() >= w * h);

    for row in 0..h {
        let bgra_row = row * stride;
        let yuv_row = row * w;
        for col in 0..w {
            let px = bgra_row + col * 4;
            let b = bgra[px] as i32;
            let g = bgra[px + 1] as i32;
            let r = bgra[px + 2] as i32;

            // BT.601 full range (no +16 offset on Y, full 0-255 range)
            let y = (77 * r + 150 * g + 29 * b) >> 8;
            let u = ((-43 * r - 85 * g + 128 * b) >> 8) + 128;
            let v = ((128 * r - 107 * g - 21 * b) >> 8) + 128;

            let idx = yuv_row + col;
            y444[idx] = y.clamp(0, 255) as u8;
            u444[idx] = u.clamp(0, 255) as u8;
            v444[idx] = v.clamp(0, 255) as u8;
        }
    }
}

/// Split YUV444 planes into Main View (standard YUV420) and Auxiliary View
/// (chroma compensation YUV420) per MS-RDPEGFX §3.3.8.3.2 B-area mapping.
///
/// Both `main_view` and `aux_view` must be pre-allocated to at least `width × height`.
/// Width and height must be even.
///
/// ## B-area mapping
///
/// **Main View:**
/// - B1: `main.y[row][col] = y444[row][col]` (identity copy)
/// - B2: `main.u[r][c] = avg(u444[2r][2c], u444[2r][2c+1], u444[2r+1][2c], u444[2r+1][2c+1])`
/// - B3: `main.v[r][c]` = same 2×2 average for V
///
/// **Auxiliary View:**
/// - B4-B5: `aux.y` — U444/V444 odd rows, interleaved in 16-row blocks (8 U + 8 V)
/// - B6: `aux.u[r][c] = u444[2r][2c+1]` (even rows, odd columns)
/// - B7: `aux.v[r][c] = v444[2r][2c+1]` (even rows, odd columns)
pub fn yuv444_split_to_yuv420(
    y444: &[u8],
    u444: &[u8],
    v444: &[u8],
    width: u32,
    height: u32,
    main_view: &mut Yuv420Frame,
    aux_view: &mut Yuv420Frame,
) {
    let w = width as usize;
    let h = height as usize;
    let half_w = w / 2;
    let half_h = h / 2;

    debug_assert!(w % 2 == 0 && h % 2 == 0, "width and height must be even");
    debug_assert!(main_view.y.len() >= w * h);
    debug_assert!(main_view.u.len() >= half_w * half_h);
    debug_assert!(aux_view.y.len() >= w * h);
    debug_assert!(aux_view.u.len() >= half_w * half_h);

    // --- B1: Main Y plane — direct copy from Y444 ---
    main_view.y[..w * h].copy_from_slice(&y444[..w * h]);

    // --- B2 + B3: Main U/V planes — 2×2 anti-alias average ---
    for r in 0..half_h {
        let row_top = 2 * r;
        let row_bot = row_top + 1;
        for c in 0..half_w {
            let col_left = 2 * c;
            let col_right = col_left + 1;

            let tl = row_top * w + col_left;
            let tr = row_top * w + col_right;
            let bl = row_bot * w + col_left;
            let br = row_bot * w + col_right;

            // B2: U average
            let u_avg = (u444[tl] as u16 + u444[tr] as u16 + u444[bl] as u16 + u444[br] as u16) / 4;
            main_view.u[r * half_w + c] = u_avg as u8;

            // B3: V average
            let v_avg = (v444[tl] as u16 + v444[tr] as u16 + v444[bl] as u16 + v444[br] as u16) / 4;
            main_view.v[r * half_w + c] = v_avg as u8;
        }
    }

    // --- B4-B5: Aux Y plane — U444/V444 odd rows, interleaved in 16-row blocks ---
    // Zero-fill aux Y first (unused rows stay 0)
    aux_view.y[..w * h].fill(0);

    // Iterate over odd source rows: 1, 3, 5, ...
    let mut src_row = 1usize;
    while src_row < h {
        let half_row = src_row / 2; // 0, 1, 2, ...
        let block = half_row / 8;
        let offset = half_row % 8;
        let u_dst_row = block * 16 + offset;       // U data in first 8 rows of block
        let v_dst_row = block * 16 + offset + 8;   // V data in next 8 rows

        let src_offset = src_row * w;
        // B4: Copy U444 odd row into aux Y
        if u_dst_row < h {
            let u_dst_offset = u_dst_row * w;
            aux_view.y[u_dst_offset..u_dst_offset + w]
                .copy_from_slice(&u444[src_offset..src_offset + w]);
        }
        // B5: Copy V444 odd row into aux Y
        if v_dst_row < h {
            let v_dst_offset = v_dst_row * w;
            aux_view.y[v_dst_offset..v_dst_offset + w]
                .copy_from_slice(&v444[src_offset..src_offset + w]);
        }

        src_row += 2;
    }

    // --- B6 + B7: Aux U/V planes — even rows, odd columns ---
    for r in 0..half_h {
        let src_row = 2 * r; // even row
        for c in 0..half_w {
            let src_col = 2 * c + 1; // odd column
            let src_idx = src_row * w + src_col;
            let dst_idx = r * half_w + c;

            // B6: Aux U
            aux_view.u[dst_idx] = u444[src_idx];
            // B7: Aux V
            aux_view.v[dst_idx] = v444[src_idx];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a simple BGRA buffer with a known color
    fn make_bgra(width: usize, height: usize, b: u8, g: u8, r: u8) -> Vec<u8> {
        let mut buf = vec![0u8; width * height * 4];
        for i in 0..width * height {
            buf[i * 4] = b;
            buf[i * 4 + 1] = g;
            buf[i * 4 + 2] = r;
            buf[i * 4 + 3] = 255; // alpha
        }
        buf
    }

    /// Reference BT.601 full-range conversion for a single pixel
    fn ref_bt601(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let ri = r as i32;
        let gi = g as i32;
        let bi = b as i32;
        let y = ((77 * ri + 150 * gi + 29 * bi) >> 8).clamp(0, 255) as u8;
        let u = (((-43 * ri - 85 * gi + 128 * bi) >> 8) + 128).clamp(0, 255) as u8;
        let v = (((128 * ri - 107 * gi - 21 * bi) >> 8) + 128).clamp(0, 255) as u8;
        (y, u, v)
    }

    #[test]
    fn test_bgra_to_yuv444_uniform_color() {
        let (w, h) = (16u32, 16u32);
        let bgra = make_bgra(w as usize, h as usize, 50, 100, 200);
        let size = (w * h) as usize;
        let mut y444 = vec![0u8; size];
        let mut u444 = vec![0u8; size];
        let mut v444 = vec![0u8; size];

        bgra_to_yuv444(&bgra, w, h, (w * 4) as usize, &mut y444, &mut u444, &mut v444);

        let (exp_y, exp_u, exp_v) = ref_bt601(200, 100, 50);
        for i in 0..size {
            assert_eq!(y444[i], exp_y, "Y mismatch at pixel {i}");
            assert_eq!(u444[i], exp_u, "U mismatch at pixel {i}");
            assert_eq!(v444[i], exp_v, "V mismatch at pixel {i}");
        }
    }

    #[test]
    fn test_bgra_to_yuv444_stride() {
        // Test with stride > width*4 (extra padding bytes per row)
        let (w, h) = (4u32, 4u32);
        let stride = (w * 4 + 16) as usize; // 16 bytes padding per row
        let mut bgra = vec![0u8; stride * h as usize];
        for row in 0..h as usize {
            for col in 0..w as usize {
                let px = row * stride + col * 4;
                bgra[px] = 30;      // B
                bgra[px + 1] = 60;  // G
                bgra[px + 2] = 90;  // R
                bgra[px + 3] = 255; // A
            }
        }

        let size = (w * h) as usize;
        let mut y444 = vec![0u8; size];
        let mut u444 = vec![0u8; size];
        let mut v444 = vec![0u8; size];

        bgra_to_yuv444(&bgra, w, h, stride, &mut y444, &mut u444, &mut v444);

        let (exp_y, exp_u, exp_v) = ref_bt601(90, 60, 30);
        for i in 0..size {
            assert_eq!(y444[i], exp_y, "Y mismatch at pixel {i}");
            assert_eq!(u444[i], exp_u, "U mismatch at pixel {i}");
            assert_eq!(v444[i], exp_v, "V mismatch at pixel {i}");
        }
    }

    #[test]
    fn test_split_b1_identity_copy() {
        // B1: main.y should be an exact copy of y444
        let (w, h) = (16u32, 16u32);
        let size = (w * h) as usize;

        // Fill y444 with recognizable pattern
        let y444: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let u444 = vec![128u8; size];
        let v444 = vec![128u8; size];

        let mut main_view = Yuv420Frame::new(w, h);
        let mut aux_view = Yuv420Frame::new(w, h);

        yuv444_split_to_yuv420(&y444, &u444, &v444, w, h, &mut main_view, &mut aux_view);

        assert_eq!(&main_view.y[..size], &y444[..size], "B1: main Y must be identity copy of Y444");
    }

    #[test]
    fn test_split_b2_b3_average() {
        // B2/B3: main U/V should be 2×2 average, not simple subsampling
        let (w, h) = (4u32, 4u32);
        let size = (w * h) as usize;

        let y444 = vec![128u8; size];

        // Construct U444 with known 2×2 blocks so we can verify the average
        // Block at (row 0-1, col 0-1): values 10, 20, 30, 40 → avg = 25
        // Block at (row 0-1, col 2-3): values 100, 110, 120, 130 → avg = 115
        // Block at (row 2-3, col 0-1): values 50, 60, 70, 80 → avg = 65
        // Block at (row 2-3, col 2-3): values 200, 210, 220, 230 → avg = 215
        let mut u444 = vec![0u8; size];
        // Row 0: [10, 20, 100, 110]
        u444[0] = 10; u444[1] = 20; u444[2] = 100; u444[3] = 110;
        // Row 1: [30, 40, 120, 130]
        u444[4] = 30; u444[5] = 40; u444[6] = 120; u444[7] = 130;
        // Row 2: [50, 60, 200, 210]
        u444[8] = 50; u444[9] = 60; u444[10] = 200; u444[11] = 210;
        // Row 3: [70, 80, 220, 230]
        u444[12] = 70; u444[13] = 80; u444[14] = 220; u444[15] = 230;

        // Use the same pattern for V444 but shifted by +5
        let v444: Vec<u8> = u444.iter().map(|&x| x.saturating_add(5)).collect();

        let mut main_view = Yuv420Frame::new(w, h);
        let mut aux_view = Yuv420Frame::new(w, h);

        yuv444_split_to_yuv420(&y444, &u444, &v444, w, h, &mut main_view, &mut aux_view);

        // Main U: 2×2 averages
        // (10+20+30+40)/4 = 25
        assert_eq!(main_view.u[0], 25, "B2: U avg of block (0,0)");
        // (100+110+120+130)/4 = 115
        assert_eq!(main_view.u[1], 115, "B2: U avg of block (0,1)");
        // (50+60+70+80)/4 = 65
        assert_eq!(main_view.u[2], 65, "B2: U avg of block (1,0)");
        // (200+210+220+230)/4 = 215
        assert_eq!(main_view.u[3], 215, "B2: U avg of block (1,1)");

        // Main V: same averages but shifted by +5
        assert_eq!(main_view.v[0], 30, "B3: V avg of block (0,0)");
        assert_eq!(main_view.v[1], 120, "B3: V avg of block (0,1)");
        assert_eq!(main_view.v[2], 70, "B3: V avg of block (1,0)");
        assert_eq!(main_view.v[3], 220, "B3: V avg of block (1,1)");
    }

    #[test]
    fn test_split_b4_b5_interleaving() {
        // B4-B5: aux.y should contain U444/V444 odd rows interleaved in 16-row blocks
        let (w, h) = (32u32, 32u32);
        let size = (w * h) as usize;
        let wi = w as usize;

        let y444 = vec![0u8; size];

        // Fill U444 and V444 with distinct patterns per row so we can identify them
        let mut u444 = vec![0u8; size];
        let mut v444 = vec![0u8; size];
        for row in 0..h as usize {
            for col in 0..wi {
                u444[row * wi + col] = (row + 1) as u8;        // U: row number + 1
                v444[row * wi + col] = (row + 1 + 100) as u8;  // V: row number + 101
            }
        }

        let mut main_view = Yuv420Frame::new(w, h);
        let mut aux_view = Yuv420Frame::new(w, h);

        yuv444_split_to_yuv420(&y444, &u444, &v444, w, h, &mut main_view, &mut aux_view);

        // Verify the interleaving pattern for odd source rows
        // src_row=1 → half_row=0 → block=0, offset=0 → u_dst=0, v_dst=8
        // src_row=3 → half_row=1 → block=0, offset=1 → u_dst=1, v_dst=9
        // src_row=5 → half_row=2 → block=0, offset=2 → u_dst=2, v_dst=10
        // ...
        // src_row=15 → half_row=7 → block=0, offset=7 → u_dst=7, v_dst=15
        // src_row=17 → half_row=8 → block=1, offset=0 → u_dst=16, v_dst=24
        // src_row=19 → half_row=9 → block=1, offset=1 → u_dst=17, v_dst=25

        let test_cases: Vec<(usize, usize, usize)> = vec![
            // (src_row, expected_u_dst_row, expected_v_dst_row)
            (1,  0,  8),
            (3,  1,  9),
            (5,  2,  10),
            (7,  3,  11),
            (9,  4,  12),
            (11, 5,  13),
            (13, 6,  14),
            (15, 7,  15),
            (17, 16, 24),
            (19, 17, 25),
            (21, 18, 26),
            (23, 19, 27),
            (25, 20, 28),
            (27, 21, 29),
            (29, 22, 30),
            (31, 23, 31),
        ];

        for (src_row, u_dst_row, v_dst_row) in test_cases {
            let expected_u_val = (src_row + 1) as u8;
            let expected_v_val = (src_row + 1 + 100) as u8;

            // Check first pixel of each row as representative
            assert_eq!(
                aux_view.y[u_dst_row * wi], expected_u_val,
                "B4: aux.y row {u_dst_row} should contain U444 row {src_row} (val={expected_u_val})"
            );
            assert_eq!(
                aux_view.y[v_dst_row * wi], expected_v_val,
                "B5: aux.y row {v_dst_row} should contain V444 row {src_row} (val={expected_v_val})"
            );
        }
    }

    #[test]
    fn test_split_b6_b7_odd_columns() {
        // B6/B7: aux U/V should pick from even rows, odd columns
        let (w, h) = (8u32, 8u32);
        let size = (w * h) as usize;
        let wi = w as usize;

        let y444 = vec![128u8; size];

        // Fill U444 and V444 with col index as value so we can verify column selection
        let mut u444 = vec![0u8; size];
        let mut v444 = vec![0u8; size];
        for row in 0..h as usize {
            for col in 0..wi {
                u444[row * wi + col] = (col * 10 + row) as u8;
                v444[row * wi + col] = (col * 10 + row + 50) as u8;
            }
        }

        let mut main_view = Yuv420Frame::new(w, h);
        let mut aux_view = Yuv420Frame::new(w, h);

        yuv444_split_to_yuv420(&y444, &u444, &v444, w, h, &mut main_view, &mut aux_view);

        let half_w = wi / 2;
        let half_h = (h / 2) as usize;

        // B6: aux.u[r][c] = u444[2r][2c+1]
        // B7: aux.v[r][c] = v444[2r][2c+1]
        for r in 0..half_h {
            let src_row = 2 * r; // even row
            for c in 0..half_w {
                let src_col = 2 * c + 1; // odd column
                let expected_u = (src_col * 10 + src_row) as u8;
                let expected_v = (src_col * 10 + src_row + 50) as u8;

                assert_eq!(
                    aux_view.u[r * half_w + c], expected_u,
                    "B6: aux.u[{r}][{c}] should be u444[{src_row}][{src_col}] = {expected_u}"
                );
                assert_eq!(
                    aux_view.v[r * half_w + c], expected_v,
                    "B7: aux.v[{r}][{c}] should be v444[{src_row}][{src_col}] = {expected_v}"
                );
            }
        }
    }

    #[test]
    fn test_yuv420_frame_ensure_size() {
        let mut frame = Yuv420Frame::new(8, 8);
        assert_eq!(frame.y.len(), 64);
        assert_eq!(frame.u.len(), 16);

        // Resize to larger
        frame.ensure_size(16, 16);
        assert_eq!(frame.y.len(), 256);
        assert_eq!(frame.u.len(), 64);
        assert_eq!(frame.v.len(), 64);
        assert_eq!(frame.width, 16);
        assert_eq!(frame.height, 16);

        // All values should be reset
        assert!(frame.y.iter().all(|&v| v == 0), "Y should be zeroed after ensure_size");
        assert!(frame.u.iter().all(|&v| v == 128), "U should be 128 after ensure_size");
        assert!(frame.v.iter().all(|&v| v == 128), "V should be 128 after ensure_size");
    }

    #[test]
    fn test_roundtrip_uniform_color() {
        // End-to-end: BGRA → YUV444 → split → verify uniform color consistency
        let (w, h) = (16u32, 16u32);
        let bgra = make_bgra(w as usize, h as usize, 80, 160, 240);
        let size = (w * h) as usize;

        let mut y444 = vec![0u8; size];
        let mut u444 = vec![0u8; size];
        let mut v444 = vec![0u8; size];
        bgra_to_yuv444(&bgra, w, h, (w * 4) as usize, &mut y444, &mut u444, &mut v444);

        let mut main_view = Yuv420Frame::new(w, h);
        let mut aux_view = Yuv420Frame::new(w, h);
        yuv444_split_to_yuv420(&y444, &u444, &v444, w, h, &mut main_view, &mut aux_view);

        // For a uniform color, all YUV444 values are the same pixel repeated
        let (exp_y, exp_u, exp_v) = ref_bt601(240, 160, 80);

        // B1: main Y should all be exp_y
        assert!(main_view.y.iter().all(|&v| v == exp_y), "uniform B1 check");

        // B2/B3: average of identical values = the value itself
        assert!(main_view.u.iter().all(|&v| v == exp_u), "uniform B2 check");
        assert!(main_view.v.iter().all(|&v| v == exp_v), "uniform B3 check");

        // B6/B7: picking any column from uniform data = same value
        assert!(aux_view.u.iter().all(|&v| v == exp_u), "uniform B6 check");
        assert!(aux_view.v.iter().all(|&v| v == exp_v), "uniform B7 check");
    }
}
