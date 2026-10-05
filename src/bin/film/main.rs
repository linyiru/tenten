//! film: renders the Tenten launch film. Every frame is drawn from the font's
//! own bitmaps and dot geometry, the score is synthesized from the same
//! events, and ffmpeg only encodes. Standard library only.
//!
//!   film [--bdf F] [--out build/film/tenten.mp4] [--size 1920x1080] [--fps 60]
//!        [--from S] [--to S] [--still T,T,...]

mod audio;
mod raster;
mod scene;

use std::io::Write;
use std::process::{Command, Stdio};
use std::{env, fs, process, thread};

use raster::Canvas;
use scene::{Frame, Glyphs};
use tenten::dot_half;

const BG: [f32; 3] = [0.035, 0.035, 0.034];
const OFF: [f32; 3] = [0.13, 0.13, 0.125];
const ON: [f32; 3] = [0.96, 0.94, 0.89];
const HUD_ON: [f32; 3] = [0.62, 0.61, 0.58];
const HUD_PITCH: f32 = 4.0;

fn mix(a: [f32; 3], b: [f32; 3], u: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u]
}

fn draw(g: &Glyphs, f: &Frame, cv: &mut Canvas) {
    let k = cv.w as f32 / 1920.0;
    let (w, h) = (cv.w as f32, cv.h as f32);
    let [cx, cy, pitch] = f.cam;
    let p = pitch * k;
    let (i0, i1) = ((cx - w / 2.0 / p).floor() as i32 - 1, (cx + w / 2.0 / p).ceil() as i32 + 1);
    let (j0, j1) = ((cy - h / 2.0 / p).floor() as i32 - 1, (cy + h / 2.0 / p).ceil() as i32 + 1);
    for i in i0..=i1 {
        let rond = f.rond(i);
        // Near Black the dots touch; a hair of overlap hides the seam that
        // two half-covered anti-aliased edges would leave between them.
        let half = dot_half(f.wght, rond) * p + 0.35 * scene::seg(f.wght, 800.0, 900.0);
        let hot = f.hot.filter(|h| h.0 == i).map_or(0.0, |h| h.1);
        let sx = (i as f32 + 0.5 - cx) * p + w / 2.0;
        for j in j0..=j1 {
            let sy = (j as f32 + 0.5 - cy) * p + h / 2.0;
            let pres = f.presence(i, j);
            match f.lit.get(&(i, j)) {
                Some(l) => {
                    let col = if l.b <= 1.0 { mix(OFF, ON, l.b) } else { ON.map(|c| c * l.b) };
                    cv.dot(sx, sy, half * l.s, rond, col, pres);
                }
                None => {
                    let row = (0..12).contains(&j);
                    let col = if hot > 0.0 && row { mix(OFF, [0.42, 0.41, 0.39], hot) } else { OFF };
                    cv.dot(sx, sy, half * 0.72, rond, col, pres);
                }
            }
        }
    }
    if f.hud_alpha > 0.0 {
        let p = HUD_PITCH * k;
        for hud in &f.hud {
            let x0 = hud.x * k - g.width(&hud.text) as f32 * p * hud.align;
            for (ch, c0) in g.layout(&hud.text) {
                for &(c, r) in g.cells(ch) {
                    let (x, y) = (x0 + (c0 + c) as f32 * p + p / 2.0, hud.y * k + r as f32 * p + p / 2.0);
                    cv.dot(x, y, dot_half(500.0, 100.0) * p, 100.0, HUD_ON, f.hud_alpha);
                }
            }
        }
    }
}

/// One finished frame. Fast camera moves are averaged over a half-frame
/// shutter, enough subframes that no dot jumps more than ~4 px.
fn render(g: &Glyphs, t: f32, fps: f32, w: usize, h: usize, index: u64, out: &mut Vec<u8>) {
    let k = w as f32 / 1920.0;
    let (a, b) = (scene::camera(t - 0.25 / fps), scene::camera(t + 0.25 / fps));
    let edge = (w as f32 / 2.0) / a[2].min(b[2]);
    let disp = ((a[0] - b[0]).abs() + (a[1] - b[1]).abs()) * a[2].max(b[2]) + (a[2] - b[2]).abs() * edge;
    let n = ((disp * k / 4.0).ceil() as usize).clamp(1, 12);
    let mut acc = Canvas::new(w, h, BG);
    if n == 1 {
        draw(g, &scene::frame(g, t), &mut acc);
    } else {
        acc.scale(0.0);
        for s in 0..n {
            let ts = t + ((s as f32 + 0.5) / n as f32 - 0.5) * 0.5 / fps;
            let mut cv = Canvas::new(w, h, BG);
            draw(g, &scene::frame(g, ts), &mut cv);
            acc.accumulate(&cv, 1.0 / n as f32);
        }
    }
    acc.bloom(0.75, 0.55);
    let fade = scene::frame(g, t).fade;
    acc.finish(index, fade, out);
}

fn die(msg: &str) -> ! {
    eprintln!("film: {msg}");
    process::exit(1)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let bdf = opt("--bdf").unwrap_or_else(|| "source/fusion-pixel-12px-monospaced-zh_tw.bdf".into());
    let out = opt("--out").unwrap_or_else(|| "build/film/tenten.mp4".into());
    let size = opt("--size").unwrap_or_else(|| "1920x1080".into());
    let (w, h) = size.split_once('x').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))).unwrap_or_else(|| die("bad --size"));
    let fps: f32 = opt("--fps").and_then(|v| v.parse().ok()).unwrap_or(60.0);
    let from: f32 = opt("--from").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let to: f32 = opt("--to").and_then(|v| v.parse().ok()).unwrap_or(scene::LEN);

    let data = fs::read_to_string(&bdf).unwrap_or_else(|_| die(&format!("cannot read {bdf} (run scripts/fetch-source.sh)")));
    let g = Glyphs::new(&tenten::parse(&data, None));
    let dir = std::path::Path::new(&out).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    fs::create_dir_all(&dir).unwrap_or_else(|_| die("cannot create output directory"));

    if let Some(list) = opt("--still") {
        for t in list.split(',').filter_map(|v| v.parse::<f32>().ok()) {
            let mut rgb = Vec::new();
            render(&g, t, fps, w, h, (t * fps) as u64, &mut rgb);
            let path = dir.join(format!("still-{t:05.2}.png"));
            fs::write(&path, raster::png(w, h, &rgb)).unwrap_or_else(|_| die("cannot write still"));
            println!("{}", path.display());
        }
        return;
    }

    let ev = scene::events(&g);
    let (pcm, _) = audio::score(&ev);
    let wav = std::path::Path::new(&out).with_extension("wav");
    fs::write(&wav, audio::wav(&pcm)).unwrap_or_else(|_| die("cannot write score"));
    println!("{} ({} events)", wav.display(), ev.len());

    let mut ff = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            &format!("{w}x{h}"),
            "-r",
            &fps.to_string(),
            "-i",
            "-",
        ])
        .args(["-ss", &from.to_string(), "-i"])
        .arg(&wav)
        .args(["-map", "0:v", "-map", "1:a", "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p", "-tune", "film"])
        .args(["-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709"])
        .args(["-c:a", "aac", "-b:a", "256k", "-shortest", "-movflags", "+faststart"])
        .arg(&out)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| die("cannot start ffmpeg"));
    let mut pipe = ff.stdin.take().unwrap();

    let (n0, n1) = ((from * fps).round() as u64, (to * fps).round() as u64);
    let threads = thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let start = std::time::Instant::now();
    let mut k = n0;
    while k < n1 {
        let batch: Vec<u64> = (k..(k + threads).min(n1)).collect();
        let frames: Vec<Vec<u8>> = thread::scope(|s| {
            let hs: Vec<_> = batch
                .iter()
                .map(|&i| {
                    let g = &g;
                    s.spawn(move || {
                        let mut rgb = Vec::new();
                        render(g, i as f32 / fps, fps, w, h, i, &mut rgb);
                        rgb
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for f in &frames {
            pipe.write_all(f).unwrap_or_else(|_| die("ffmpeg closed the pipe"));
        }
        k += batch.len() as u64;
        if (k - n0) % (fps as u64 * 2) < threads {
            eprintln!("frame {k}/{n1}  {:.1}s", start.elapsed().as_secs_f32());
        }
    }
    drop(pipe);
    if !ff.wait().map(|s| s.success()).unwrap_or(false) {
        die("ffmpeg failed");
    }
    println!("{out}  {} frames in {:.1}s", n1 - n0, start.elapsed().as_secs_f32());
}
