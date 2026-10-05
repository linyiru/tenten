//! A software rasterizer for one thing: dots. Float RGB canvas, analytic
//! anti-aliasing from a rounded-square distance field, a two-level bloom, and
//! a finish pass (shoulder, vignette, grain) down to 8-bit RGB.

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 3]>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, bg: [f32; 3]) -> Canvas {
        Canvas { w, h, px: vec![bg; w * h] }
    }

    /// A dot centred at (cx, cy) px with half-size `half` px. `rond` 0..100
    /// rounds the corners from square to circle, like the font's ROND axis.
    pub fn dot(&mut self, cx: f32, cy: f32, half: f32, rond: f32, col: [f32; 3], alpha: f32) {
        if half < 0.15 || alpha <= 0.0 {
            return;
        }
        let rr = half * (rond / 100.0).clamp(0.0, 1.0);
        let inner = half - rr;
        let x0 = ((cx - half - 1.0).floor().max(0.0)) as usize;
        let y0 = ((cy - half - 1.0).floor().max(0.0)) as usize;
        let x1 = ((cx + half + 1.0).ceil().min(self.w as f32)) as usize;
        let y1 = ((cy + half + 1.0).ceil().min(self.h as f32)) as usize;
        // Sub-pixel dots fade by area instead of vanishing.
        let small = (half * 2.0).min(1.0);
        for y in y0..y1 {
            let qy = (y as f32 + 0.5 - cy).abs() - inner;
            let row = y * self.w;
            for x in x0..x1 {
                let qx = (x as f32 + 0.5 - cx).abs() - inner;
                let (ox, oy) = (qx.max(0.0), qy.max(0.0));
                let d = (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - rr;
                let cov = (0.5 - d).clamp(0.0, 1.0) * alpha * small;
                if cov > 0.0 {
                    let p = &mut self.px[row + x];
                    for c in 0..3 {
                        p[c] += (col[c] - p[c]) * cov;
                    }
                }
            }
        }
    }

    /// Adds `other` scaled by `k` (used to average motion-blur subframes).
    pub fn accumulate(&mut self, other: &Canvas, k: f32) {
        for (a, b) in self.px.iter_mut().zip(&other.px) {
            for c in 0..3 {
                a[c] += b[c] * k;
            }
        }
    }

    pub fn scale(&mut self, k: f32) {
        self.px.iter_mut().for_each(|p| p.iter_mut().for_each(|v| *v *= k));
    }

    /// Glow from everything brighter than `thr`, at 1/4 and 1/16 resolution.
    pub fn bloom(&mut self, thr: f32, k: f32) {
        let (w4, h4) = (self.w.div_ceil(4), self.h.div_ceil(4));
        let mut q = vec![[0f32; 3]; w4 * h4];
        for y in 0..self.h {
            for x in 0..self.w {
                let p = self.px[y * self.w + x];
                let d = &mut q[(y / 4) * w4 + x / 4];
                for c in 0..3 {
                    d[c] += (p[c] - thr).max(0.0) / 16.0;
                }
            }
        }
        let (w16, h16) = (w4.div_ceil(4), h4.div_ceil(4));
        let mut s = vec![[0f32; 3]; w16 * h16];
        for y in 0..h4 {
            for x in 0..w4 {
                let p = q[y * w4 + x];
                let d = &mut s[(y / 4) * w16 + x / 4];
                for c in 0..3 {
                    d[c] += p[c] / 16.0;
                }
            }
        }
        blur(&mut q, w4, h4, 3);
        blur(&mut s, w16, h16, 3);
        for y in 0..self.h {
            for x in 0..self.w {
                let a = sample(&q, w4, h4, (x as f32 + 0.5) / 4.0 - 0.5, (y as f32 + 0.5) / 4.0 - 0.5);
                let b = sample(&s, w16, h16, (x as f32 + 0.5) / 16.0 - 0.5, (y as f32 + 0.5) / 16.0 - 0.5);
                let p = &mut self.px[y * self.w + x];
                for c in 0..3 {
                    p[c] += k * (a[c] * 0.9 + b[c] * 1.4);
                }
            }
        }
    }

    /// Display-ready RGB: soft shoulder above 0.8, vignette, ±1 LSB grain.
    pub fn finish(&self, frame: u64, fade: f32, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.w * self.h * 3);
        let (hw, hh) = (self.w as f32 / 2.0, self.h as f32 / 2.0);
        for y in 0..self.h {
            for x in 0..self.w {
                let (dx, dy) = ((x as f32 - hw) / hw, (y as f32 - hh) / hw);
                let vig = 1.0 - 0.22 * (dx * dx + dy * dy);
                let g = (hash(x as i32, y as i32, frame as i32) + hash(y as i32, x as i32, frame as i32 + 7) - 1.0) / 255.0;
                for c in 0..3 {
                    let v = self.px[y * self.w + x][c] * vig * fade;
                    let v = if v < 0.8 { v } else { 0.8 + 0.2 * (1.0 - (-(v - 0.8) / 0.2).exp()) };
                    out.push(((v + g).clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
                }
            }
        }
    }
}

/// Three box passes each way approximate a gaussian.
fn blur(b: &mut [[f32; 3]], w: usize, h: usize, r: usize) {
    let mut t = vec![[0f32; 3]; w.max(h)];
    for _ in 0..3 {
        for y in 0..h {
            box1(b, y * w, 1, w, r, &mut t);
        }
        for x in 0..w {
            box1(b, x, w, h, r, &mut t);
        }
    }
}

fn box1(b: &mut [[f32; 3]], start: usize, stride: usize, n: usize, r: usize, t: &mut [[f32; 3]]) {
    let k = 1.0 / (2 * r + 1) as f32;
    let at = |b: &[[f32; 3]], i: isize| if i < 0 || i >= n as isize { [0.0; 3] } else { b[start + i as usize * stride] };
    let mut acc = [0f32; 3];
    for i in -(r as isize)..=(r as isize) {
        let v = at(b, i);
        (0..3).for_each(|c| acc[c] += v[c]);
    }
    for (i, ti) in t.iter_mut().enumerate().take(n) {
        *ti = [acc[0] * k, acc[1] * k, acc[2] * k];
        let (add, sub) = (at(b, (i + r + 1) as isize), at(b, i as isize - r as isize));
        (0..3).for_each(|c| acc[c] += add[c] - sub[c]);
    }
    for i in 0..n {
        b[start + i * stride] = t[i];
    }
}

fn sample(b: &[[f32; 3]], w: usize, h: usize, x: f32, y: f32) -> [f32; 3] {
    let (x, y) = (x.clamp(0.0, (w - 1) as f32), y.clamp(0.0, (h - 1) as f32));
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let mut o = [0f32; 3];
    for c in 0..3 {
        let a = b[y0 * w + x0][c] + (b[y0 * w + x1][c] - b[y0 * w + x0][c]) * fx;
        let d = b[y1 * w + x0][c] + (b[y1 * w + x1][c] - b[y1 * w + x0][c]) * fx;
        o[c] = a + (d - a) * fy;
    }
    o
}

/// A seeded hash in [0, 1).
pub fn hash(i: i32, j: i32, k: i32) -> f32 {
    let mut h = (i.wrapping_mul(374761393)) ^ (j.wrapping_mul(668265263)) ^ (k.wrapping_mul(-2048144777));
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1274126177);
    ((h ^ ((h as u32) >> 16) as i32) as u32) as f32 / 4294967296.0
}

/// An uncompressed PNG (stored deflate blocks), enough for review stills.
pub fn png(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for row in rgb.chunks(w * 3) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push((i + 1 == blocks.len()) as u8);
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &v in &raw {
        a = (a + v as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&(b << 16 | a).to_be_bytes());
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    for (tag, data) in [(b"IHDR", ihdr), (b"IDAT", z), (b"IEND", Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(tag);
        out.extend_from_slice(&data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    out
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}
