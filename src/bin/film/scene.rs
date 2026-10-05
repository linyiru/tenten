//! The director. `frame(t)` is a pure function of time: no state is carried
//! between frames, so any frame renders on its own and in any order. `events`
//! walks the same timing functions to tell the score where every sound goes,
//! so every dot that sounds is a dot on screen.
//!
//! World unit = one lattice cell. 120 bpm: a beat is 0.5 s, a bar 2 s.

use std::collections::HashMap;

use tenten::Font;

use crate::raster::hash;

pub const LEN: f32 = 20.0;

// ── timeline (seconds) ───────────────────────────────────────────
pub const S1_T0: f32 = 0.5; // 01 one light: the dots of 點 light one by one
pub const S1_SPAN: f32 = 3.3;
pub const S2_T0: f32 = 4.0; // 02 score: a playhead writes the rest of the line
pub const S2_STEP: f32 = 0.0625;
pub const LINE: &str = "點點 TENTEN";
pub const WHIP_T0: f32 = 8.0; // whip down to the board
pub const WHIP_T1: f32 = 8.6;
pub const FLAP_A: f32 = 8.5; // 03 board: split-flap
pub const BOARD_J: i32 = 18;
pub const BOARD_A: [&str; 2] = ["點陣字體", "一點一點"];
pub const AXES_T0: f32 = 12.0; // 04 axes: one weight per beat, then a ROND wipe
pub const WGHT_STEPS: [(f32, f32); 5] = [(12.0, 100.0), (12.5, 300.0), (13.0, 500.0), (13.5, 700.0), (14.0, 900.0)];
pub const WIPE_T0: f32 = 14.0;
pub const WIPE_T1: f32 = 15.5;
pub const END_T0: f32 = 16.0; // 05 end card, then everything goes out but one dot
pub const BOARD_B: [&str; 2] = ["　點點　", "　　　　"];
pub const OUT_T0: f32 = 17.5;
pub const LAST_T: f32 = 18.6;

const DRUM: &str = "一二三四五六七八九十天地玄黃宇宙洪荒日月盈昃辰宿列張寒來暑往點陣字體";
const FLAP_TICK: f32 = 0.05;

// ── easing ───────────────────────────────────────────────────────
pub fn seg(t: f32, a: f32, b: f32) -> f32 {
    ((t - a) / (b - a)).clamp(0.0, 1.0)
}
fn lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}
fn out_expo(u: f32) -> f32 {
    if u >= 1.0 { 1.0 } else { 1.0 - 2f32.powf(-10.0 * u) }
}
fn in_out_expo(u: f32) -> f32 {
    if u <= 0.0 {
        0.0
    } else if u >= 1.0 {
        1.0
    } else if u < 0.5 {
        2f32.powf(20.0 * u - 10.0) / 2.0
    } else {
        (2.0 - 2f32.powf(-20.0 * u + 10.0)) / 2.0
    }
}
fn in_out_cubic(u: f32) -> f32 {
    if u < 0.5 { 4.0 * u * u * u } else { 1.0 - (-2.0 * u + 2.0).powi(3) / 2.0 }
}
fn out_cubic(u: f32) -> f32 {
    1.0 - (1.0 - u).powi(3)
}
fn smooth(u: f32) -> f32 {
    u * u * (3.0 - 2.0 * u)
}

// ── glyphs ───────────────────────────────────────────────────────
pub struct Glyphs(HashMap<char, (i32, Vec<(i32, i32)>)>);

impl Glyphs {
    pub fn new(f: &Font) -> Glyphs {
        Glyphs(f.glyphs.iter().filter_map(|g| char::from_u32(g.cp as u32).map(|c| (c, (g.adv_cells(), g.cells())))).collect())
    }
    fn adv(&self, c: char) -> i32 {
        self.0.get(&c).map_or(12, |g| g.0)
    }
    pub fn cells(&self, c: char) -> &[(i32, i32)] {
        self.0.get(&c).map_or(&[], |g| &g.1)
    }
    /// (char, first column) for each character of a line.
    pub fn layout(&self, s: &str) -> Vec<(char, i32)> {
        let mut x = 0;
        s.chars()
            .map(|c| {
                let at = x;
                x += self.adv(c);
                (c, at)
            })
            .collect()
    }
    pub fn width(&self, s: &str) -> i32 {
        s.chars().map(|c| self.adv(c)).sum()
    }
}

// ── shared timing ────────────────────────────────────────────────
/// 01: the dots of the first glyph, centre first, then a seeded order that
/// accelerates into the downbeat. (time, col, row)
pub fn s1_order(g: &Glyphs) -> Vec<(f32, i32, i32)> {
    let first = LINE.chars().next().unwrap();
    let mut cells = g.cells(first).to_vec();
    let d = |&(c, r): &(i32, i32)| (c as f32 - 5.5).powi(2) + (r as f32 - 5.5).powi(2);
    let centre = cells.iter().cloned().min_by(|a, b| d(a).total_cmp(&d(b))).unwrap();
    cells.retain(|&c| c != centre);
    cells.sort_by(|a, b| hash(a.0, a.1, 35).total_cmp(&hash(b.0, b.1, 35)));
    cells.insert(0, centre);
    let n = (cells.len() - 1).max(1) as f32;
    cells.iter().enumerate().map(|(k, &(c, r))| (S1_T0 + S1_SPAN * (k as f32 / n).powf(0.55), c, r)).collect()
}

/// 02: when the playhead reaches column `c` of the line.
pub fn s2_col_time(c: i32) -> f32 {
    S2_T0 + (c - 12) as f32 * S2_STEP
}

/// 03/05: a board slot's flap. Returns (start, steps). On the end card the
/// second line clears first and fast, so the camera can leave it.
fn flap_plan(t0: f32, line: usize, slot: usize) -> (f32, usize) {
    if t0 == END_T0 && line == 1 {
        return (t0 + slot as f32 * 0.04, 3 + slot % 3);
    }
    let lag = if t0 == END_T0 { 0.2 } else { line as f32 * 0.14 };
    (t0 + slot as f32 * 0.06 + lag, 6 + (slot * 7 + line * 3) % 9)
}

fn drum() -> Vec<char> {
    DRUM.chars().collect()
}

/// The character a slot shows at t, and how fresh its last flip is (0..1).
fn flap_char(from: char, to: char, t: f32, start: f32, steps: usize) -> (char, f32) {
    if t < start {
        return (from, 0.0);
    }
    let k = ((t - start) / FLAP_TICK) as usize;
    let fresh = (-(t - start - k.min(steps) as f32 * FLAP_TICK) * 14.0).exp();
    if k >= steps {
        return (to, fresh);
    }
    let d = drum();
    let e = d.iter().position(|&c| c == to).unwrap_or((to as usize * 7) % d.len());
    (d[(e + d.len() * 4 - steps + k) % d.len()], fresh)
}

fn board_slot(t: f32, line: usize, slot: usize) -> (char, f32) {
    let a = BOARD_A[line].chars().nth(slot).unwrap();
    let b = BOARD_B[line].chars().nth(slot).unwrap();
    if t < END_T0 {
        let (s, n) = flap_plan(FLAP_A, line, slot);
        flap_char('　', a, t, s, n)
    } else {
        let (s, n) = flap_plan(END_T0, line, slot);
        flap_char(a, b, t, s, n)
    }
}

pub fn wght(t: f32) -> f32 {
    if t < AXES_T0 {
        return 400.0;
    }
    let mut w = 400.0;
    for &(at, to) in &WGHT_STEPS {
        w = lerp(w, to, out_cubic(seg(t, at, at + 0.16)));
    }
    lerp(w, 400.0, in_out_cubic(seg(t, 15.5, 16.2)))
}

/// Where the ROUND→PIXEL wipe stands (world x), and how far ROND has
/// come back to round after it (0..1).
pub fn wipe(t: f32) -> (f32, f32) {
    let x = if t < WIPE_T0 { -1e9 } else { lerp(-6.0, 54.0, in_out_cubic(seg(t, WIPE_T0, WIPE_T1))) };
    (x, in_out_cubic(seg(t, END_T0, END_T0 + 0.6)))
}

pub fn rond_at(t: f32, x: f32) -> f32 {
    let (wx, back) = wipe(t);
    let sq = smooth(((wx - x) / 3.0 + 0.5).clamp(0.0, 1.0));
    100.0 * (1.0 - sq * (1.0 - back))
}

/// The share of the board that is square, for the score's timbre.
pub fn squareness(t: f32) -> f32 {
    let n = 48;
    (0..n).map(|i| 1.0 - rond_at(t, i as f32 + 0.5) / 100.0).sum::<f32>() / n as f32
}

/// Camera: centre (cells) and pitch (px per cell at 1920 wide).
pub fn camera(t: f32) -> [f32; 3] {
    let line_c = 33.0;
    let board = [24.0, BOARD_J as f32 + 12.0];
    let end = [24.0, BOARD_J as f32 + 6.0];
    let log = |a: f32, b: f32, u: f32| a * (b / a).powf(u);
    if t < S2_T0 {
        return [6.0, 6.0, 62.0 + 6.0 * seg(t, 0.0, S2_T0)];
    }
    if t < WHIP_T0 {
        let u = out_expo(seg(t, S2_T0, S2_T0 + 1.2));
        let p = log(68.0, 26.0, u) * (1.0 - 0.06 * seg(t, S2_T0 + 1.2, WHIP_T0));
        return [lerp(6.0, line_c, u), 6.0, p];
    }
    if t < END_T0 {
        let u = in_out_expo(seg(t, WHIP_T0, WHIP_T1));
        let p0 = 26.0 * 0.94;
        let p = log(p0, 30.0, u) * (1.0 + 0.1 * seg(t, WHIP_T1, END_T0));
        return [lerp(line_c, board[0], u), lerp(6.0, board[1], u), p];
    }
    let u = in_out_cubic(seg(t, END_T0 + 0.3, END_T0 + 1.5));
    let p = log(33.0, 44.0, u) * (1.0 + 0.05 * seg(t, END_T0 + 1.5, LEN));
    [board[0], lerp(board[1], end[1], u), p]
}

// ── the frame ────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct Lit {
    /// 1 is on; above 1 is a flash that blooms.
    pub b: f32,
    /// Size multiplier on the font's dot.
    pub s: f32,
}

pub struct Hud {
    pub text: String,
    pub x: f32,
    pub y: f32,
    /// 0 = left-aligned at x, 1 = right-aligned.
    pub align: f32,
}

pub struct Frame {
    pub t: f32,
    pub cam: [f32; 3],
    pub wght: f32,
    pub lit: HashMap<(i32, i32), Lit>,
    /// The playhead column and its strength.
    pub hot: Option<(i32, f32)>,
    pub hud: Vec<Hud>,
    pub hud_alpha: f32,
    pub fade: f32,
    last: (i32, i32),
}

fn pop(dt: f32) -> Lit {
    Lit { b: 1.0 + 1.8 * (-dt * 7.0).exp(), s: 1.0 + 0.45 * (-dt * 12.0).exp() }
}

impl Frame {
    fn light(&mut self, c: i32, r: i32, l: Lit) {
        let e = self.lit.entry((c, r)).or_insert(Lit { b: 0.0, s: 1.0 });
        e.b = e.b.max(l.b);
        e.s = e.s.max(l.s);
    }

    pub fn rond(&self, i: i32) -> f32 {
        rond_at(self.t, i as f32 + 0.5)
    }

    /// How present cell (i, j) still is during the closing ripple (0..1).
    pub fn presence(&self, i: i32, j: i32) -> f32 {
        if self.t < OUT_T0 || (i, j) == self.last {
            return 1.0;
        }
        let end = [24.0, BOARD_J as f32 + 6.0];
        let d = ((i as f32 + 0.5 - end[0]).powi(2) + (j as f32 + 0.5 - end[1]).powi(2)).sqrt();
        let t0 = OUT_T0 + d * 0.02;
        1.0 - smooth(seg(self.t, t0, t0 + 0.3))
    }
}

/// The lit cell of the end card nearest its centre: the one that stays.
pub fn last_dot(g: &Glyphs) -> (i32, i32) {
    let mut best = ((0, 0), f32::MAX);
    for (l, line) in BOARD_B.iter().enumerate() {
        for (s, ch) in line.chars().enumerate() {
            for &(c, r) in g.cells(ch) {
                let (i, j) = (12 * s as i32 + c, BOARD_J + 12 * l as i32 + r);
                let d = (i as f32 + 0.5 - 24.0).powi(2) + (j as f32 + 0.5 - BOARD_J as f32 - 6.0).powi(2);
                if d < best.1 {
                    best = ((i, j), d);
                }
            }
        }
    }
    best.0
}

const SECTIONS: [(f32, &str); 5] =
    [(0.0, "01 ONE LIGHT"), (S2_T0, "02 SCORE"), (WHIP_T0, "03 BOARD"), (AXES_T0, "04 AXES"), (END_T0, "05 點點")];

pub fn frame(g: &Glyphs, t: f32) -> Frame {
    let mut f = Frame {
        t,
        cam: camera(t),
        wght: wght(t),
        lit: HashMap::new(),
        hot: None,
        hud: Vec::new(),
        hud_alpha: seg(t, 0.4, 1.2) * (1.0 - seg(t, OUT_T0, OUT_T0 + 0.4)),
        fade: seg(t, 0.0, 0.3),
        last: last_dot(g),
    };

    // 01 + 02: the line, first glyph dot by dot, the rest by the playhead.
    let down = if t >= S2_T0 { 0.7 * (-(t - S2_T0) * 5.0).exp() } else { 0.0 };
    for (tk, c, r) in s1_order(g) {
        if t >= tk {
            let mut l = pop(t - tk);
            l.b += down;
            f.light(c, r, l);
        }
    }
    let layout = g.layout(LINE);
    for &(ch, c0) in &layout[1..] {
        for &(dc, r) in g.cells(ch) {
            let tc = s2_col_time(c0 + dc);
            if t >= tc {
                f.light(c0 + dc, r, pop(t - tc));
            }
        }
    }
    let end_col = g.width(LINE);
    if t >= S2_T0 && t < s2_col_time(end_col) + 0.3 {
        let c = 12 + ((t - S2_T0) / S2_STEP) as i32;
        f.hot = Some((c.min(end_col), 1.0 - seg(t, s2_col_time(end_col), s2_col_time(end_col) + 0.3)));
    }

    // 03 + 05: the board.
    if t >= FLAP_A {
        for line in 0..2 {
            for slot in 0..4 {
                let (ch, fresh) = board_slot(t, line, slot);
                for &(c, r) in g.cells(ch) {
                    f.light(12 * slot as i32 + c, BOARD_J + 12 * line as i32 + r, Lit { b: 1.0 + 0.9 * fresh, s: 1.0 });
                }
            }
        }
    }

    // 04: every weight lands on a beat with a pulse.
    let pulse = WGHT_STEPS.iter().filter(|s| t >= s.0).map(|s| (-(t - s.0) * 9.0).exp()).fold(0.0, f32::max);
    if pulse > 0.0 {
        for l in f.lit.values_mut() {
            l.b += 0.5 * pulse;
        }
    }

    // 05: the last dot shrinks and goes out.
    if let Some(l) = f.lit.get_mut(&f.last) {
        let u = seg(t, LAST_T, LAST_T + 0.6);
        if t >= OUT_T0 {
            l.b = 1.0 + 1.2 * (-(t - OUT_T0) * 3.0).exp() + 1.5 * (-(t - LAST_T).max(0.0) * 6.0).exp() * (t >= LAST_T) as i32 as f32;
        }
        l.s *= 1.0 - smooth(u);
    }

    // HUD
    let sec = SECTIONS.iter().rev().find(|s| t >= s.0).unwrap().1;
    f.hud.push(Hud { text: sec.to_string(), x: 72.0, y: 64.0, align: 0.0 });
    f.hud.push(Hud { text: format!("00:{:05.2}", t), x: 1848.0, y: 64.0, align: 1.0 });
    f.hud.push(Hud { text: "TENTEN 點點 12×12".to_string(), x: 72.0, y: 1016.0 - 48.0, align: 0.0 });
    let r = rond_at(t, f.cam[0]).round();
    f.hud.push(Hud { text: format!("WGHT {:3.0} ROND {:3.0}", f.wght, r), x: 1848.0, y: 1016.0 - 48.0, align: 1.0 });
    f
}

// ── events for the score ─────────────────────────────────────────
pub enum Ev {
    /// 01: one dot lights. k of n.
    Ping { t: f32, row: i32, col: i32, k: usize, n: usize },
    /// 02: the playhead plays a lit dot; `of` dots share the column.
    Note { t: f32, row: i32, x: f32, of: usize },
    /// 03/05: a flap ticks; `land` on the final character.
    Clack { t: f32, slot: usize, land: bool },
    /// 04: a weight lands.
    Hit { t: f32, wght: f32 },
    /// 05: the last dot goes out.
    Last { t: f32 },
}

pub fn events(g: &Glyphs) -> Vec<Ev> {
    let mut ev = Vec::new();
    let s1 = s1_order(g);
    for (k, &(t, col, row)) in s1.iter().enumerate() {
        ev.push(Ev::Ping { t, row, col, k, n: s1.len() });
    }
    let w = g.width(LINE) as f32;
    let mut cols: HashMap<i32, Vec<i32>> = HashMap::new();
    for &(ch, c0) in &g.layout(LINE)[1..] {
        for &(dc, r) in g.cells(ch) {
            cols.entry(c0 + dc).or_default().push(r);
        }
    }
    for (c, rows) in &cols {
        for &row in rows {
            ev.push(Ev::Note { t: s2_col_time(*c), row, x: *c as f32 / w, of: rows.len() });
        }
    }
    for t0 in [FLAP_A, END_T0] {
        for line in 0..2 {
            for slot in 0..4 {
                let (a, b) = (BOARD_A[line].chars().nth(slot).unwrap(), BOARD_B[line].chars().nth(slot).unwrap());
                let (from, to) = if t0 == FLAP_A { ('　', a) } else { (a, b) };
                if from == to {
                    continue;
                }
                let (s, n) = flap_plan(t0, line, slot);
                for k in 0..=n {
                    ev.push(Ev::Clack { t: s + k as f32 * FLAP_TICK, slot, land: k == n });
                }
            }
        }
    }
    for &(t, w) in &WGHT_STEPS {
        ev.push(Ev::Hit { t, wght: w });
    }
    ev.push(Ev::Last { t: LAST_T });
    ev
}
