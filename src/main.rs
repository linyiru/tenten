//! tenten: builds Tenten, a variable font whose glyphs are dots on a 12 x 12
//! lattice, from a pixel-font BDF. Every glyph is a composite of one shared
//! dot glyph, and the dot alone varies: wght is its size, ROND its shape.
//! Standard library only.

use std::{env, fs, process};

const CELL: i64 = 100;
const NOTDEF: i64 = 65534;
const UPM: i64 = 1200;
const ASC: i64 = 1000;
const DESC: i64 = -200;
const XH: i64 = 600;
const CAP: i64 = 800;
const DOT_GID: usize = 1;
const FIRST_GID: usize = 2;
const DEFAULT_R: i64 = 30;
const MAX_R: i64 = 50;
const SEC: i64 = 1082392;

#[derive(Default, Clone)]
struct Glyph {
    cp: i64,
    adv: i64,
    dots: Vec<(i64, i64)>,
}

#[derive(Default)]
struct Font {
    glyphs: Vec<Glyph>,
    notdef: Glyph,
}

fn int(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

fn parse(data: &str, keep: Option<&[bool]>) -> Font {
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

fn charset(data: &str) -> Vec<bool> {
    let mut keep = vec![false; 0x110000];
    for c in data.chars() {
        if c != '\n' && c != '\r' {
            keep[c as usize] = true;
        }
    }
    keep
}

/// A big-endian writer.
#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u8(&mut self, v: i64) {
        self.0.push(v as u8);
    }
    fn u16(&mut self, v: i64) {
        self.0.extend_from_slice(&(v as u16).to_be_bytes());
    }
    fn u32(&mut self, v: i64) {
        self.0.extend_from_slice(&(v as u32).to_be_bytes());
    }
    fn u16s(&mut self, vs: &[i64]) {
        vs.iter().for_each(|&v| self.u16(v));
    }
    fn tag(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
    fn pad(&mut self, a: usize) {
        while !self.0.len().is_multiple_of(a) {
            self.0.push(0);
        }
    }
    fn len(&self) -> i64 {
        self.0.len() as i64
    }
}

fn checksum(p: &[u8]) -> u32 {
    p.chunks(4).fold(0u32, |sum, c| {
        let mut word = [0u8; 4];
        word[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn log2(n: i64) -> i64 {
    let mut k = 0;
    while 1 << (k + 1) <= n {
        k += 1;
    }
    k
}

fn round_div(a: i64, b: i64) -> i64 {
    if a < 0 { -((-a * 2 + b) / (b * 2)) } else { (a * 2 + b) / (b * 2) }
}

const COS: [i64; 9] = [1000000, 923880, 707107, 382683, 0, -382683, -707107, -923880, -1000000];

fn cos(k: i64) -> i64 {
    let mut m = k.rem_euclid(16);
    if m > 8 {
        m = 16 - m;
    }
    COS[m as usize]
}

fn ux(i: i64) -> i64 {
    cos(i)
}

fn uy(i: i64) -> i64 {
    -cos(i - 4)
}

fn dot_coord(i: i64, r: i64, rond: bool, u: i64) -> i64 {
    if rond {
        let rad = if i % 2 == 1 { r * SEC } else { r * 1000000 };
        return round_div(u * rad, 1000000 * 1000000);
    }
    round_div(u * r, ux(i).abs().max(uy(i).abs()))
}

fn dot_x(i: i64, r: i64, rond: bool) -> i64 {
    dot_coord(i, r, rond, ux(i))
}

fn dot_y(i: i64, r: i64, rond: bool) -> i64 {
    dot_coord(i, r, rond, uy(i))
}

fn radius(wght: i64, rond: bool) -> i64 {
    match (rond, wght) {
        (true, 100) => 12,
        (true, 900) => 50,
        (true, _) => 30,
        (false, 100) => 11,
        (false, 900) => 50,
        _ => 27,
    }
}

fn dot_glyph(b: &mut Buf) -> i64 {
    let (lo, hi) = (0..16).fold((0, 0), |(lo, hi), i| {
        let x = dot_x(i, DEFAULT_R, true);
        (lo.min(x), hi.max(x))
    });
    b.u16s(&[1, lo, lo, hi, hi, 15, 0]);
    for i in 0..16 {
        b.u8(1 - i % 2);
    }
    for f in [dot_x, dot_y] {
        let mut prev = 0;
        for i in 0..16 {
            b.u16(f(i, DEFAULT_R, true) - prev);
            prev = f(i, DEFAULT_R, true);
        }
    }
    lo
}

fn delta(i: i64, wght: i64, rond: bool, sub1: i64, sub2: i64, x: bool) -> i64 {
    let mut d = if x {
        dot_x(i, radius(wght, rond), rond) - dot_x(i, DEFAULT_R, true)
    } else {
        dot_y(i, radius(wght, rond), rond) - dot_y(i, DEFAULT_R, true)
    };
    for s in [sub1, sub2] {
        d -= match s {
            1 => delta(i, 100, true, 0, 0, x),
            2 => delta(i, 900, true, 0, 0, x),
            3 => delta(i, 400, false, 0, 0, x),
            _ => 0,
        };
    }
    d
}

fn deltas(b: &mut Buf, wght: i64, rond: bool, sub1: i64, sub2: i64) {
    for axis in 0..2 {
        b.u8(0x40 | 19);
        for i in 0..20 {
            b.u16(if i < 16 { delta(i, wght, rond, sub1, sub2, axis == 0) } else { 0 });
        }
    }
}

fn dot_variations(b: &mut Buf) {
    const NEG: i64 = 0xC000;
    const POS: i64 = 0x4000;
    b.u16s(&[0x8000 | 5, 4 + 5 * 8]);
    for (w, r) in [(NEG, 0), (POS, 0), (0, NEG), (NEG, NEG), (POS, NEG)] {
        b.u16s(&[82, 0x8000, w, r]);
    }
    b.u8(0);
    deltas(b, 100, true, 0, 0);
    deltas(b, 900, true, 0, 0);
    deltas(b, 400, false, 0, 0);
    deltas(b, 100, false, 1, 3);
    deltas(b, 900, false, 2, 3);
    b.pad(2);
}

struct Stats {
    glyphs: usize,
    adv: Vec<i64>,
    lsb: Vec<i64>,
    xmin: i64,
    ymin: i64,
    xmax: i64,
    ymax: i64,
    advmax: i64,
    minlsb: i64,
    minrsb: i64,
    extent: i64,
    maxdots: i64,
    advsum: i64,
    firstcp: i64,
    lastcp: i64,
}

fn composite(b: &mut Buf, g: &Glyph, st: &mut Stats, gid: usize) {
    st.adv[gid] = g.adv;
    st.advsum += g.adv;
    st.advmax = st.advmax.max(g.adv);
    if g.dots.is_empty() {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (1 << 30, 1 << 30, -(1 << 30), -(1 << 30));
    for &(x, y) in &g.dots {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    (x0, y0, x1, y1) = (x0 - MAX_R, y0 - MAX_R, x1 + MAX_R, y1 + MAX_R);
    b.u16s(&[-1, x0, y0, x1, y1]);
    for (k, &(x, y)) in g.dots.iter().enumerate() {
        let more = if k + 1 < g.dots.len() { 0x0020 } else { 0 };
        b.u16s(&[0x0003 | more, DOT_GID as i64, x, y]);
    }
    st.lsb[gid] = x0;
    st.xmin = st.xmin.min(x0);
    st.ymin = st.ymin.min(y0);
    st.xmax = st.xmax.max(x1);
    st.ymax = st.ymax.max(y1);
    st.minlsb = st.minlsb.min(x0);
    st.minrsb = st.minrsb.min(g.adv - x1);
    st.extent = st.extent.max(x1);
    st.maxdots = st.maxdots.max(g.dots.len() as i64);
}

fn glyf(f: &Font, st: &mut Stats, glyf: &mut Buf, loca: &mut Buf) {
    for gid in 0..st.glyphs {
        loca.u32(glyf.len());
        if gid == DOT_GID {
            st.lsb[gid] = dot_glyph(glyf);
        } else {
            let g = if gid == 0 { &f.notdef } else { &f.glyphs[gid - FIRST_GID] };
            composite(glyf, g, st, gid);
        }
        glyf.pad(4);
    }
    loca.u32(glyf.len());
}

fn head(st: &Stats, b: &mut Buf) {
    for v in [0x00010000, 0x0000199A, 0, 0x5F0F3CF5] {
        b.u32(v);
    }
    b.u16s(&[1 | 8, UPM]);
    for v in [0, 3874003200, 0, 3874003200] {
        b.u32(v);
    }
    b.u16s(&[st.xmin, st.ymin, st.xmax, st.ymax, 0, 12, 2, 1, 0]);
}

fn hhea(st: &Stats, b: &mut Buf) {
    b.u32(0x00010000);
    b.u16s(&[ASC, DESC, 0, st.advmax, st.minlsb, st.minrsb, st.extent, 1, 0, 0]);
    b.u32(0);
    b.u32(0);
    b.u16s(&[0, st.glyphs as i64]);
}

fn hmtx(st: &Stats, b: &mut Buf) {
    for gid in 0..st.glyphs {
        b.u16s(&[st.adv[gid], st.lsb[gid]]);
    }
}

fn maxp(st: &Stats, b: &mut Buf) {
    b.u32(0x00010000);
    let md = st.maxdots;
    b.u16s(&[st.glyphs as i64, 16, 1, md * 16, md, 2, 0, 0, 0, 0, 0, 0, md, 1]);
}

fn post(b: &mut Buf) {
    b.u32(0x00030000);
    b.u32(0);
    b.u16s(&[-150, 100]);
    for _ in 0..5 {
        b.u32(0);
    }
}

fn os2(st: &Stats, b: &mut Buf) {
    b.u16s(&[4, st.advsum / st.glyphs as i64, 400, 5, 0, 780, 720, 0, 100, 780, 720, 0, 400, 100, 350, 0]);
    b.u8(2);
    for _ in 1..10 {
        b.u8(0);
    }
    b.u32(3);
    b.u32(1 << 16 | 1 << 27);
    b.u32(1 << 4);
    b.u32(0);
    b.tag("NONE");
    b.u16s(&[0x40 | 0x80, st.firstcp, st.lastcp, ASC, DESC, 0, ASC.max(st.ymax), (-DESC).max(-st.ymin)]);
    b.u32(1 | 1 << 20);
    b.u32(0);
    b.u16s(&[XH, CAP, 0, 32, 0]);
}

/// A stretch of consecutive code points (and so of consecutive gids).
struct Run {
    c0: i64,
    c1: i64,
    gid: i64,
}

fn runs(f: &Font) -> Vec<Run> {
    let mut rs = Vec::new();
    let mut i = 0;
    while i < f.glyphs.len() {
        let start = i;
        while i + 1 < f.glyphs.len() && f.glyphs[i + 1].cp == f.glyphs[i].cp + 1 {
            i += 1;
        }
        rs.push(Run { c0: f.glyphs[start].cp, c1: f.glyphs[i].cp, gid: (start + FIRST_GID) as i64 });
        i += 1;
    }
    rs
}

fn cmap(f: &Font, b: &mut Buf) {
    let rs = runs(f);
    let bmp: Vec<&Run> = rs.iter().filter(|r| r.c0 <= 0xFFFF).collect();
    let segs = bmp.len() as i64 + 1;
    let len4 = 16 + segs * 8;
    let off4 = 4 + 4 * 8;
    let off12 = off4 + len4;
    b.u16s(&[0, 4]);
    for (p, e, off) in [(0, 3, off4), (0, 4, off12), (3, 1, off4), (3, 10, off12)] {
        b.u16s(&[p, e]);
        b.u32(off);
    }
    let sr = 2 * (1 << log2(segs));
    b.u16s(&[4, len4, 0, segs * 2, sr, log2(segs), segs * 2 - sr]);
    bmp.iter().for_each(|r| b.u16(r.c1.min(0xFFFE)));
    b.u16s(&[0xFFFF, 0]);
    bmp.iter().for_each(|r| b.u16(r.c0));
    b.u16(0xFFFF);
    bmp.iter().for_each(|r| b.u16((r.gid - r.c0) & 0xFFFF));
    b.u16(1);
    for _ in 0..segs {
        b.u16(0);
    }
    b.u16s(&[12, 0]);
    b.u32(16 + rs.len() as i64 * 12);
    b.u32(0);
    b.u32(rs.len() as i64);
    for r in &rs {
        b.u32(r.c0);
        b.u32(r.c1);
        b.u32(r.gid);
    }
}

fn fvar(b: &mut Buf) {
    b.u16s(&[1, 0, 16, 2, 2, 20, 6, 12]);
    for (tag, lo, def, hi, name) in [("wght", 100, 400, 900, 256), ("ROND", 0, 100, 100, 257)] {
        b.tag(tag);
        b.u32(lo << 16);
        b.u32(def << 16);
        b.u32(hi << 16);
        b.u16s(&[0, name]);
    }
    for (name, w, r) in [(258, 100, 100), (259, 400, 100), (260, 900, 100), (261, 100, 0), (262, 400, 0), (263, 900, 0)] {
        b.u16s(&[name, 0]);
        b.u32(w << 16);
        b.u32(r << 16);
    }
}

fn stat(b: &mut Buf) {
    b.u16s(&[1, 1, 8, 2]);
    b.u32(20);
    b.u16(0);
    b.u32(0);
    b.u16(2);
    b.tag("wght");
    b.u16s(&[256, 0]);
    b.tag("ROND");
    b.u16s(&[257, 1]);
}

fn gvar(st: &Stats, b: &mut Buf) {
    let mut data = Buf::default();
    dot_variations(&mut data);
    let arr = 20 + (st.glyphs as i64 + 1) * 4;
    b.u16s(&[1, 0, 2, 0]);
    b.u32(arr);
    b.u16s(&[st.glyphs as i64, 1]);
    b.u32(arr);
    for gid in 0..=st.glyphs {
        b.u32(if gid <= DOT_GID { 0 } else { data.len() });
    }
    b.0.extend_from_slice(&data.0);
}

const NAMES: &[(i64, &str)] = &[
    (
        0,
        "Copyright 2026 The Tenten Project Authors. Glyph bitmaps from Fusion Pixel Font, Copyright (c) 2022, TakWolf (https://takwolf.com), which includes Ark Pixel Font (c) 2021 TakWolf, Cubic 11, and Galmuri (c) 2019-2025 Lee Minseo.",
    ),
    (1, "Tenten TC"),
    (2, "Regular"),
    (3, "0.100;NONE;TentenTC-Regular"),
    (4, "Tenten TC Regular"),
    (5, "Version 0.100"),
    (6, "TentenTC-Regular"),
    (13, "This Font Software is licensed under the SIL Open Font License, Version 1.1."),
    (14, "https://openfontlicense.org"),
    (256, "Weight"),
    (257, "Roundness"),
    (258, "Thin"),
    (259, "Regular"),
    (260, "Black"),
    (261, "Pixel Thin"),
    (262, "Pixel Regular"),
    (263, "Pixel Black"),
];

fn name(b: &mut Buf) {
    let (mut strs, mut recs) = (Buf::default(), Buf::default());
    for &(id, s) in NAMES {
        let off = strs.len();
        s.bytes().for_each(|c| strs.u16(c as i64));
        recs.u16s(&[3, 1, 0x409, id, s.len() as i64 * 2, off]);
    }
    let n = NAMES.len() as i64;
    b.u16s(&[0, n, 6 + n * 12]);
    b.0.extend_from_slice(&recs.0);
    b.0.extend_from_slice(&strs.0);
}

fn build(f: &Font) -> Vec<u8> {
    let glyphs = f.glyphs.len() + FIRST_GID;
    let mut st = Stats {
        glyphs,
        adv: vec![0; glyphs],
        lsb: vec![0; glyphs],
        xmin: 1 << 30,
        ymin: 1 << 30,
        xmax: -(1 << 30),
        ymax: -(1 << 30),
        advmax: 0,
        minlsb: 1 << 30,
        minrsb: 1 << 30,
        extent: 0,
        maxdots: 0,
        advsum: 0,
        firstcp: f.glyphs[0].cp.min(0xFFFF),
        lastcp: f.glyphs[f.glyphs.len() - 1].cp.min(0xFFFF),
    };
    let (mut glyfb, mut locab) = (Buf::default(), Buf::default());
    glyf(f, &mut st, &mut glyfb, &mut locab);
    let mut t: Vec<(&str, Buf)> = Vec::new();
    let mut add = |tag, fill: &dyn Fn(&mut Buf)| {
        let mut b = Buf::default();
        fill(&mut b);
        t.push((tag, b));
    };
    add("OS/2", &|b| os2(&st, b));
    add("STAT", &stat);
    add("cmap", &|b| cmap(f, b));
    add("fvar", &fvar);
    add("glyf", &|b| b.0 = glyfb.0.clone());
    add("gvar", &|b| gvar(&st, b));
    add("head", &|b| head(&st, b));
    add("hhea", &|b| hhea(&st, b));
    add("hmtx", &|b| hmtx(&st, b));
    add("loca", &|b| b.0 = locab.0.clone());
    add("maxp", &|b| maxp(&st, b));
    add("name", &name);
    add("post", &post);
    assemble(&t)
}

fn assemble(ts: &[(&str, Buf)]) -> Vec<u8> {
    let mut out = Buf::default();
    let n = ts.len() as i64;
    let sr = 16 * (1 << log2(n));
    out.u32(0x00010000);
    out.u16s(&[n, sr, log2(n), n * 16 - sr]);
    let mut off = 12 + n * 16;
    let mut headoff = 0;
    for (tag, b) in ts {
        out.tag(tag);
        out.u32(checksum(&b.0) as i64);
        out.u32(off);
        out.u32(b.len());
        if *tag == "head" {
            headoff = off as usize;
        }
        off += (b.len() + 3) & !3;
    }
    for (_, b) in ts {
        out.0.extend_from_slice(&b.0);
        out.pad(4);
    }
    let adj = 0xB1B0AFBAu32.wrapping_sub(checksum(&out.0));
    out.0[headoff + 8..headoff + 12].copy_from_slice(&adj.to_be_bytes());
    out.0
}

fn die(msg: &str, code: i32) -> ! {
    eprintln!("{msg}");
    process::exit(code)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        die("usage: tenten <in.bdf> <out.ttf> [charset.txt]", 64);
    }
    let keep = args.get(3).map(|p| charset(&fs::read_to_string(p).unwrap_or_else(|_| die("tenten: cannot read charset", 1))));
    let input = fs::read_to_string(&args[1]).unwrap_or_else(|_| die("tenten: cannot read bdf", 1));
    let f = parse(&input, keep.as_deref());
    let out = build(&f);
    fs::write(&args[2], &out).unwrap_or_else(|_| die("tenten: cannot write font", 1));
    let ndots: usize = f.glyphs.iter().map(|g| g.dots.len()).sum::<usize>() + f.notdef.dots.len();
    println!("glyphs {} dots {} bytes {}", f.glyphs.len(), ndots, out.len());
}
