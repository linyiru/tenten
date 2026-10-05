//! The score, synthesized from the picture's own events. 120 bpm, D minor
//! (Dm Bb F C). A lattice row is a pitch on the D minor pentatonic, top row
//! highest; ROND turns the pad from sine to square, wght sets its level.

use std::f32::consts::TAU;

use crate::scene::{self, Ev};

pub const SR: usize = 48000;
const BEAT: f32 = 0.5;
const BAR: f32 = 2.0;
const PENTA: [i32; 5] = [0, 3, 5, 7, 10];
const PROG: [[i32; 4]; 4] = [[38, 62, 65, 69], [34, 62, 65, 70], [41, 60, 65, 69], [36, 60, 64, 67]];

fn mtof(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// Row 0 (top) is the highest note: two and a half octaves over 12 rows.
pub fn row_midi(r: i32) -> f32 {
    let i = 11 - r.clamp(0, 11);
    (50 + PENTA[(i % 5) as usize] + 12 * (i / 5)) as f32
}

fn chord(t: f32) -> [i32; 4] {
    PROG[((t / BAR) as usize) % 4]
}

struct Rng(u64);

impl Rng {
    fn noise(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

/// An RBJ biquad.
#[derive(Clone, Copy)]
struct Bq {
    b: [f32; 3],
    a: [f32; 2],
    z: [f32; 2],
}

impl Bq {
    fn new(kind: u8, f: f32, q: f32) -> Bq {
        let w = TAU * f / SR as f32;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        let (b, a0, a1, a2) = match kind {
            b'l' => ([(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0], 1.0 + al, -2.0 * c, 1.0 - al),
            b'h' => ([(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0], 1.0 + al, -2.0 * c, 1.0 - al),
            _ => ([al, 0.0, -al], 1.0 + al, -2.0 * c, 1.0 - al),
        };
        Bq { b: [b[0] / a0, b[1] / a0, b[2] / a0], a: [a1 / a0, a2 / a0], z: [0.0; 2] }
    }
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

struct Bus {
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Bus {
    fn new(n: usize) -> Bus {
        Bus { l: vec![0.0; n], r: vec![0.0; n] }
    }
    /// Adds a mono signal at t0 with an equal-power pan (−1..1).
    fn put(&mut self, t0: f32, sig: &[f32], gain: f32, pan: f32) {
        let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
        let (gl, gr) = (a.cos() * gain, a.sin() * gain);
        let s0 = (t0 * SR as f32).round() as isize;
        for (k, &v) in sig.iter().enumerate() {
            let i = s0 + k as isize;
            if i >= 0 && (i as usize) < self.l.len() {
                self.l[i as usize] += v * gl;
                self.r[i as usize] += v * gr;
            }
        }
    }
}

fn render(len: f32, mut f: impl FnMut(f32, usize) -> f32) -> Vec<f32> {
    (0..(len * SR as f32) as usize).map(|i| f(i as f32 / SR as f32, i)).collect()
}

fn ping(m: f32) -> Vec<f32> {
    let f = mtof(m);
    render(1.6, |t, _| {
        let env = (t / 0.002).min(1.0) * (-t * 4.5).exp();
        env * ((TAU * f * t).sin() + 0.22 * (TAU * 2.0 * f * t).sin() * (-t * 9.0).exp() + 0.06 * (TAU * 3.0 * f * t).sin())
    })
}

fn pluck(m: f32) -> Vec<f32> {
    let f = mtof(m);
    render(0.7, |t, _| {
        let env = (t / 0.003).min(1.0) * (-t * 8.0).exp();
        let ph = (f * t).fract();
        let tri = 4.0 * (ph - 0.5).abs() - 1.0;
        env * (0.75 * (TAU * f * t).sin() + 0.25 * tri * (-t * 14.0).exp())
    })
}

fn kick() -> Vec<f32> {
    let mut ph = 0.0;
    render(0.5, |t, _| {
        ph += TAU * (46.0 + 120.0 * (-t * 28.0).exp()) / SR as f32;
        let click = (-t * 400.0).exp() * 0.3;
        ((ph.sin() * 1.15).tanh() + click) * (-t * 6.5).exp()
    })
}

fn hat(rng: &mut Rng) -> Vec<f32> {
    let mut hp = Bq::new(b'h', 7500.0, 0.7);
    render(0.07, |t, _| hp.run(rng.noise()) * (-t * 55.0).exp())
}

fn clack(rng: &mut Rng, land: bool) -> Vec<f32> {
    let mut bp = Bq::new(b'b', if land { 1800.0 } else { 2600.0 + 900.0 * rng.noise().abs() }, 1.6);
    let len = if land { 0.03 } else { 0.014 };
    render(len, |t, _| bp.run(rng.noise()) * (1.0 - t / len).powi(3))
}

fn bass(m: f32) -> Vec<f32> {
    let f = mtof(m);
    let mut lp = Bq::new(b'l', 700.0, 0.9);
    render(0.42, |t, _| {
        let env = (t / 0.004).min(1.0) * (-t * 5.0).exp();
        lp.run(((TAU * f * t).sin() * 2.4).tanh() * 0.6 + 0.5 * (TAU * f * 0.5 * t).sin()) * env
    })
}

fn whoosh(rng: &mut Rng, len: f32) -> Vec<f32> {
    let mut v = Vec::with_capacity((len * SR as f32) as usize);
    let mut bp = Bq::new(b'b', 400.0, 0.8);
    for i in 0..(len * SR as f32) as usize {
        let u = i as f32 / (len * SR as f32);
        if i % 64 == 0 {
            let z = bp.z;
            bp = Bq::new(b'b', 300.0 + 5200.0 * (std::f32::consts::PI * u).sin().powi(2), 0.8);
            bp.z = z;
        }
        v.push(bp.run(rng.noise()) * (std::f32::consts::PI * u).sin().powi(2));
    }
    v
}

/// Schroeder reverb on a mono send.
fn reverb(x: &[f32], spread: f32) -> Vec<f32> {
    let mut y = vec![0.0; x.len()];
    for (ms, fb) in [(29.7, 0.84), (37.1, 0.83), (41.1, 0.82), (43.7, 0.81)] {
        let d = (ms * spread * SR as f32 / 1000.0) as usize;
        let (mut buf, mut lp) = (vec![0.0f32; d], 0.0f32);
        for i in 0..x.len() {
            let o = buf[i % d];
            lp += (o - lp) * 0.45;
            buf[i % d] = x[i] + lp * fb;
            y[i] += o * 0.25;
        }
    }
    for ms in [5.0, 1.7] {
        let d = (ms * spread * SR as f32 / 1000.0) as usize;
        let mut buf = vec![0.0f32; d];
        for (i, v) in y.iter_mut().enumerate() {
            let o = buf[i % d];
            let w = *v + o * 0.7;
            buf[i % d] = w;
            *v = o - w * 0.7;
        }
    }
    y
}

/// Interleaved 16-bit stereo PCM, normalized to −1 dBFS.
pub fn score(ev: &[Ev]) -> (Vec<i16>, usize) {
    let n = (scene::LEN * SR as f32) as usize;
    let mut rng = Rng(0x7e47e4);
    let (mut dry, mut music, mut drums, mut send) = (Bus::new(n), Bus::new(n), Bus::new(n), Bus::new(n));
    let mut kicks = Vec::new();

    for e in ev {
        match *e {
            Ev::Ping { t, row, col, k, n: total } => {
                let g = 0.2 * (1.0 - 0.6 * k as f32 / total as f32);
                let s = ping(row_midi(row) + 12.0);
                let pan = (col as f32 - 5.5) / 8.0;
                dry.put(t, &s, g, pan);
                send.put(t, &s, g * 0.8, 0.0);
            }
            Ev::Note { t, row, x, of } => {
                let s = pluck(row_midi(row) + 12.0);
                let g = 0.16 / (of as f32).sqrt();
                dry.put(t, &s, g, x * 1.4 - 0.7);
                send.put(t, &s, g * 0.5, 0.0);
            }
            Ev::Clack { t, slot, land } => {
                let s = clack(&mut rng, land);
                dry.put(t, &s, if land { 0.28 } else { 0.12 }, slot as f32 / 3.0 * 1.2 - 0.6);
            }
            Ev::Hit { t, wght } => {
                kicks.push((t, 1.0));
                for &m in &chord(t)[1..] {
                    let s = pluck(m as f32 + 12.0);
                    let g = 0.05 + 0.08 * (wght - 100.0) / 800.0;
                    music.put(t, &s, g, 0.0);
                    send.put(t, &s, g, 0.0);
                }
            }
            Ev::Last { t } => {
                let s = ping(74.0);
                dry.put(t, &s, 0.22, 0.0);
                send.put(t, &s, 0.5, 0.0);
            }
        }
    }

    // The bed: four on the floor from the downbeat, a break for the whip.
    let mut t = scene::S2_T0;
    while t < scene::END_T0 + 0.01 {
        if !(scene::WHIP_T0..scene::FLAP_A).contains(&t) && !kicks.iter().any(|k: &(f32, f32)| (k.0 - t).abs() < 0.01) {
            kicks.push((t, if t == scene::S2_T0 || t == scene::END_T0 { 1.15 } else { 0.85 }));
        }
        t += BEAT;
    }
    let k = kick();
    for &(t, g) in &kicks {
        drums.put(t, &k, 0.55 * g, 0.0);
    }
    let mut t = scene::FLAP_A + BEAT / 2.0;
    while t < scene::END_T0 {
        let h = hat(&mut rng);
        drums.put(t, &h, 0.09, 0.25);
        t += BEAT;
    }
    let mut t = scene::S2_T0;
    while t < scene::END_T0 {
        if !(scene::WHIP_T0..scene::FLAP_A).contains(&t) {
            let s = bass(chord(t)[0] as f32);
            music.put(t, &s, 0.32, 0.0);
        }
        t += BEAT / 2.0;
    }
    let w = whoosh(&mut rng, scene::WHIP_T1 - scene::WHIP_T0 + 0.1);
    dry.put(scene::WHIP_T0 - 0.05, &w, 0.25, 0.0);
    send.put(scene::WHIP_T0 - 0.05, &w, 0.2, 0.0);

    // The axes pad: ROND morphs sine → square, wght sets the level.
    let (p0, p1) = (scene::AXES_T0, scene::LAST_T + 1.0);
    let mut lp = [Bq::new(b'l', 2400.0, 0.7), Bq::new(b'l', 2400.0, 0.7)];
    let mut phase = [0.0f32; 3];
    let (mut sq, mut lvl) = (0.0, 0.0);
    for i in (p0 * SR as f32) as usize..((p1 * SR as f32) as usize).min(n) {
        let t = i as f32 / SR as f32;
        if i % 64 == 0 {
            sq = scene::squareness(t);
            lvl = 0.35 + 0.65 * (scene::wght(t) - 100.0) / 800.0;
        }
        let env = scene::seg(t, p0, p0 + 0.4) * (1.0 - scene::seg(t, scene::OUT_T0, scene::LAST_T + 0.8));
        let notes = chord(t.min(scene::END_T0 - 0.01));
        let mut v = 0.0;
        for (j, &m) in notes[1..].iter().enumerate() {
            phase[j] = (phase[j] + mtof(m as f32) / SR as f32).fract();
            let x = TAU * phase[j];
            let mut sqw = 0.0;
            for h in (1..16).step_by(2) {
                sqw += (h as f32 * x).sin() / h as f32;
            }
            v += x.sin() * (1.0 - sq) + sqw * 4.0 / std::f32::consts::PI * sq * 0.6;
        }
        let v = v * 0.045 * lvl * env;
        music.l[i] += lp[0].run(v);
        music.r[i] += lp[1].run(v * 0.98);
        send.l[i] += v * 0.4;
    }

    // Kick sidechain on the music bus.
    let mut duck = vec![1.0f32; n];
    for &(t, g) in &kicks {
        let s0 = (t * SR as f32) as usize;
        for (k, d) in duck.iter_mut().enumerate().skip(s0).take(SR / 3) {
            let dt = (k - s0) as f32 / SR as f32;
            *d = d.min(1.0 - 0.55 * g.min(1.0) * (-dt * 11.0).exp());
        }
    }

    let mono: Vec<f32> = (0..n).map(|i| send.l[i] + send.r[i]).collect();
    let (rl, rr) = (reverb(&mono, 1.0), reverb(&mono, 1.07));
    let mut mix = Vec::with_capacity(n * 2);
    for i in 0..n {
        for (d, m, k, r) in [(dry.l[i], music.l[i], drums.l[i], rl[i]), (dry.r[i], music.r[i], drums.r[i], rr[i])] {
            mix.push(((d + m * duck[i] + k + r * 0.35) * 1.2).tanh());
        }
    }
    let peak = mix.iter().fold(1e-6f32, |a, &v| a.max(v.abs()));
    let g = 0.89 / peak;
    (mix.iter().map(|&v| (v * g * 32767.0) as i16).collect(), n)
}

pub fn wav(pcm: &[i16]) -> Vec<u8> {
    let data = pcm.len() as u32 * 2;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&(SR as u32).to_le_bytes());
    b.extend_from_slice(&(SR as u32 * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    pcm.iter().for_each(|v| b.extend_from_slice(&v.to_le_bytes()));
    b
}
