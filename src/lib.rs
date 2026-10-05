//! Shared by the font builder and the film: the BDF reader and the lattice.
//! Standard library only.

/// One lattice cell, in font units.
pub const CELL: i64 = 100;
/// The BDF's DEFAULT_CHAR, which becomes .notdef.
pub const NOTDEF: i64 = 65534;
/// Rows above the baseline (ascender 1000 / CELL).
pub const ASC_ROWS: i64 = 10;

#[derive(Default, Clone)]
pub struct Glyph {
    pub cp: i64,
    pub adv: i64,
    /// Dot centres in font units, y up.
    pub dots: Vec<(i64, i64)>,
}

impl Glyph {
    /// Lit cells as (column, row), row 0 at the top of the 12-row line.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        self.dots.iter().map(|&(x, y)| (x.div_euclid(CELL) as i32, (ASC_ROWS - 1 - y.div_euclid(CELL)) as i32)).collect()
    }

    pub fn adv_cells(&self) -> i32 {
        (self.adv / CELL) as i32
    }
}

#[derive(Default)]
pub struct Font {
    pub glyphs: Vec<Glyph>,
    pub notdef: Glyph,
}

fn int(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

/// Reads a BDF. `keep`, indexed by code point, limits which glyphs are kept;
/// .notdef is always kept.
pub fn parse(data: &str, keep: Option<&[bool]>) -> Font {
    let mut f = Font::default();
    let (mut cp, mut adv, mut w, mut h, mut xo, mut yo) = (-1i64, 0i64, 0i64, 0i64, 0i64, 0i64);
    let mut lines = data.split('\n');
    while let Some(line) = lines.next() {
        if let Some(r) = line.strip_prefix("ENCODING ") {
            cp = int(r.split_whitespace().next().unwrap_or(""));
        } else if let Some(r) = line.strip_prefix("DWIDTH ") {
            adv = int(r.split_whitespace().next().unwrap_or("")) * CELL;
        } else if let Some(r) = line.strip_prefix("BBX ") {
            let v: Vec<i64> = r.split_whitespace().map(int).collect();
            (w, h, xo, yo) = (v[0], v[1], v[2], v[3]);
        } else if line.starts_with("BITMAP") {
            let mut wanted = cp == NOTDEF;
            if (0..0x110000).contains(&cp) && cp != NOTDEF {
                wanted = keep.is_none_or(|k| k[cp as usize]);
            }
            let mut dots = Vec::new();
            for row in 0..h {
                let Some(l) = lines.next() else { break };
                let l = l.trim();
                let v = u64::from_str_radix(l, 16).unwrap_or(0);
                let bits = l.len() as i64 * 4;
                for col in 0..w.min(bits) {
                    if (v >> (bits - 1 - col)) & 1 == 1 {
                        dots.push(((xo + col) * CELL + CELL / 2, (yo + h - 1 - row) * CELL + CELL / 2));
                    }
                }
            }
            if wanted {
                let g = Glyph { cp, adv, dots };
                if cp == NOTDEF {
                    f.notdef = g;
                } else {
                    f.glyphs.push(g);
                }
            }
        }
    }
    f
}

/// A keep-list for `parse` from the characters of a UTF-8 text.
pub fn charset(data: &str) -> Vec<bool> {
    let mut keep = vec![false; 0x110000];
    for c in data.chars() {
        if c != '\n' && c != '\r' {
            keep[c as usize] = true;
        }
    }
    keep
}

/// Dot half-size as a fraction of the cell, for any wght and ROND. These are
/// the font's own masters (main.rs `radius`), interpolated the way gvar does:
/// piecewise linear in wght around the default, linear in ROND.
pub fn dot_half(wght: f32, rond: f32) -> f32 {
    fn pw(w: f32, a: f32, b: f32, c: f32) -> f32 {
        if w <= 400.0 { a + (b - a) * ((w - 100.0) / 300.0).clamp(0.0, 1.0) } else { b + (c - b) * ((w - 400.0) / 500.0).clamp(0.0, 1.0) }
    }
    let sq = pw(wght, 0.11, 0.27, 0.50);
    let rd = pw(wght, 0.12, 0.30, 0.50);
    sq + (rd - sq) * (rond / 100.0).clamp(0.0, 1.0)
}
