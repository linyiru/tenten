//! site: writes the specimen site into docs/ for GitHub Pages. One page per
//! locale, each set in its own font with its own sample text, and an English
//! index that links them. Every sample character is checked against the
//! locale's subset before anything is written. Standard library only.
//!
//!   site [--out docs]   (after scripts/build-fonts.sh)

use std::collections::HashSet;
use std::{env, fs, process};

const REPO: &str = "https://github.com/linyiru/tenten";

struct Page {
    dir: &'static str,
    lang: &'static str,
    tag: &'static str,
    face: &'static str,
    set: &'static str,
    charset: &'static str,
    /// The language's own name, for the switcher.
    native: &'static str,
    hero: &'static str,
    lead: &'static str,
    /// What the subset covers; `{n}` is its glyph count.
    covers: &'static str,
    axes: [&'static str; 2],
    instances: &'static str,
    download: &'static str,
    full: &'static str,
    footer: &'static str,
    /// (style, text) boards below the hero.
    boards: &'static [(&'static str, &'static str)],
    /// One line set in all six named instances.
    pangram: &'static str,
}

const PAGES: [Page; 4] = [
    Page {
        dir: "zh-TW",
        lang: "zh-Hant-TW",
        tag: "TC",
        face: "zh_tw",
        set: "Big5",
        charset: "big5-common",
        native: "繁體中文",
        hero: "點陣字體",
        lead: "可變的 CJK 點陣字體。每個字都是 12 × 12 格子上的點，變的是點本身：wght 是點的大小，ROND 是點的形狀，從方形像素到圓點。",
        covers: "Big5 常用字集：ASCII、Big5 符號與 5,401 個常用漢字，共 {n} 個字符。",
        axes: ["粗細", "圓度"],
        instances: "六個具名樣式",
        download: "下載",
        full: "完整字型（36,980 字符）",
        footer: "字型以 SIL Open Font License 1.1 授權，字形點陣來自 Fusion Pixel Font。程式碼以 MIT 授權。",
        boards: &[
            ("", "永和豆漿 03:45 → 台北車站\n下一班 自強號 Express 1234"),
            ("--rond:0", "天地玄黃　宇宙洪荒\n寒來暑往　秋收冬藏"),
            (
                "s24",
                "敏捷的棕色狐狸跳過了懶狗。The quick brown fox jumps over the lazy dog.\n「繁體中文」與『標點符號』，、。？！：；——……\nㄅㄆㄇㄈ ㄉㄊㄋㄌ ㄍㄎㄏ ㄐㄑㄒ ㄓㄔㄕㄖ ㄗㄘㄙ ㄧㄨㄩ",
            ),
        ],
        pangram: "點點 一點一點",
    },
    Page {
        dir: "zh-CN",
        lang: "zh-Hans-CN",
        tag: "SC",
        face: "zh_hans",
        set: "GB2312",
        charset: "gb2312",
        native: "简体中文",
        hero: "点阵字体",
        lead: "可变的 CJK 点阵字体。每个字都是 12 × 12 格子上的点，变化的是点本身：wght 控制点的大小，ROND 控制点的形状，从方形像素到圆点。",
        covers: "GB 2312 字符集：ASCII、GB 2312 符号与汉字，共 {n} 个字符。",
        axes: ["字重", "圆度"],
        instances: "六个命名样式",
        download: "下载",
        full: "完整字体（36,980 字符）",
        footer: "字体以 SIL Open Font License 1.1 授权，字形点阵来自 Fusion Pixel Font。代码以 MIT 授权。",
        boards: &[
            ("", "南京西路 03:45 → 上海虹桥\n下一班 复兴号 G1234 次"),
            ("--rond:0", "天地玄黄　宇宙洪荒\n寒来暑往　秋收冬藏"),
            (
                "s24",
                "敏捷的棕色狐狸跳过了懒狗。The quick brown fox jumps over the lazy dog.\n“简体中文”与《标点符号》，、。？！：；——……\nā á ǎ à  ō ó ǒ ò  ē é ě è  ī í ǐ ì  ū ú ǔ ù  ǖ ǘ ǚ ǜ",
            ),
        ],
        pangram: "点点 一点一点",
    },
    Page {
        dir: "ja",
        lang: "ja",
        tag: "JP",
        face: "ja",
        set: "JIS",
        charset: "jisx0208",
        native: "日本語",
        hero: "点々の字",
        lead: "可変の CJK ドットフォント。すべての字は 12 × 12 の格子に並ぶ点でできていて、変わるのは点そのものです。wght が点の大きさを、ROND が形を決め、四角いピクセルから丸い点まで変化します。",
        covers: "JIS X 0208：ASCII、半角カナ、JIS X 0208 の記号・かな・漢字、計 {n} 字。",
        axes: ["ウェイト", "丸み"],
        instances: "六つの名前付きスタイル",
        download: "ダウンロード",
        full: "フルセット（36,980 字）",
        footer: "フォントは SIL Open Font License 1.1、字形のドットは Fusion Pixel Font によります。コードは MIT ライセンスです。",
        boards: &[
            ("", "渋谷 03:45 → 東京駅\n次の列車 のぞみ 1234号"),
            ("--rond:0", "いろはにほへと　ちりぬるを\nわかよたれそ　　つねならむ"),
            (
                "s24",
                "色は匂へど散りぬるを。The quick brown fox jumps over the lazy dog.\n「日本語」と『約物』、。？！：；――……\nアイウエオ カキクケコ サシスセソ ｱｲｳｴｵ ｶｷｸｹｺ",
            ),
        ],
        pangram: "てんてん 点々",
    },
    Page {
        dir: "ko",
        lang: "ko",
        tag: "KR",
        face: "ko",
        set: "KSX",
        charset: "ksx1001",
        native: "한국어",
        hero: "도트글꼴",
        lead: "가변 CJK 도트 글꼴. 모든 글자는 12 × 12 격자 위의 점으로 이루어지고, 변하는 것은 점 자체입니다. wght는 점의 크기를, ROND는 모양을 정하며 네모난 픽셀에서 둥근 점까지 바뀝니다.",
        covers: "KS X 1001: ASCII, KS X 1001 기호·한글·한자와 현대 한글 음절 11,172자 전체, 모두 {n}자.",
        axes: ["굵기", "둥글기"],
        instances: "이름 붙은 스타일 여섯 가지",
        download: "내려받기",
        full: "전체 글꼴 (36,980자)",
        footer: "글꼴은 SIL Open Font License 1.1, 글자의 점은 Fusion Pixel Font에서 왔습니다. 코드는 MIT 라이선스입니다.",
        boards: &[
            ("", "강남역 03:45 → 서울역\n다음 열차 KTX 1234"),
            ("--rond:0", "다람쥐 헌 쳇바퀴에\n타고파 天地玄黃"),
            (
                "s24",
                "키스의 고유조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다.\n「한국어」와 『문장 부호』, . ? ! : ; …… The quick brown fox.\nㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ ㅏㅑㅓㅕㅗㅛㅜㅠㅡㅣ 가나다라마바사아자차카타파하",
            ),
        ],
        pangram: "점점 한점한점",
    },
];

const INSTANCES: [(&str, i32, i32); 6] = [
    ("Thin", 100, 100),
    ("Regular", 400, 100),
    ("Black", 900, 100),
    ("Pixel Thin", 100, 0),
    ("Pixel Regular", 400, 0),
    ("Pixel Black", 900, 0),
];

const CSS: &str = r#":root { --ink: #f4f1ea; --dim: #8a877f; --off: #242424; --bg: #0b0b0b; }
* { box-sizing: border-box; }
body { margin: 0; padding: 48px 48px 96px; background: var(--bg); color: var(--ink); font: 15px/1.6 system-ui, sans-serif; }
a { color: var(--ink); }
nav { display: flex; flex-wrap: wrap; gap: 4px 20px; margin: 0 0 48px; color: var(--dim); }
nav a { color: var(--dim); text-decoration: none; }
nav a[aria-current] { color: var(--ink); }
nav .home { margin-right: auto; color: var(--ink); }
.lead { max-width: 44em; margin: 0 0 32px; }
.meta { color: var(--dim); margin: 0 0 12px; }
/* One cell = font-size / 12. Sizes are multiples of 12px so every dot lands
   on whole pixels; the dark lattice behind is the same grid. */
.board {
  --size: 48px;
  --cell: calc(var(--size) / 12);
  font-size: var(--size);
  line-height: 1;
  font-variation-settings: "wght" var(--wght, 400), "ROND" var(--rond, 100);
  background-image: radial-gradient(circle at center, var(--off) 0 calc(var(--cell) * 0.3), transparent calc(var(--cell) * 0.3 + 0.5px));
  background-size: var(--cell) var(--cell);
  width: fit-content;
  max-width: 100%;
  overflow-x: auto;
  margin: 0 0 32px;
  white-space: pre;
  outline: none;
}
.s24 { --size: 24px; }
/* Below 4 px a cell, the lattice reads as a grey wash: leave it out. */
.s24, .grid .board { background-image: none; }
.s96 { --size: 96px; }
.live { --wght: var(--w); --rond: var(--r); }
body { --w: 400; --r: 100; }
.controls { display: flex; flex-wrap: wrap; gap: 8px 32px; margin: 0 0 24px; }
.controls label { display: inline-flex; gap: 10px; align-items: center; }
.controls output { min-width: 3ch; font-variant-numeric: tabular-nums; color: var(--dim); }
h2 { font-size: 15px; font-weight: 600; margin: 56px 0 16px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 8px 32px; }
.grid .board { --size: 24px; margin: 0; }
.grid p { margin: 16px 0 6px; color: var(--dim); }
ul.dl { padding: 0; list-style: none; }
ul.dl li { margin: 4px 0; }
footer { margin-top: 64px; color: var(--dim); max-width: 44em; }
.locales a { display: block; text-decoration: none; margin: 0 0 40px; }
.locales .meta { margin: 8px 0 0; }
@media (max-width: 720px) {
  body { padding: 24px 16px 64px; }
  .board { --size: 24px; }
  .s96 { --size: 48px; }
  .s24 { --size: 12px; }
}
"#;

const SCRIPT: &str = r#"<script>
  for (const [id, v] of [['w', '--w'], ['r', '--r']]) {
    const el = document.getElementById(id), out = el.nextElementSibling;
    el.oninput = () => { document.body.style.setProperty(v, el.value); out.value = el.value; };
  }
</script>"#;

fn die(msg: &str) -> ! {
    eprintln!("site: {msg}");
    process::exit(1)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn font_file(p: &Page) -> String {
    format!("Tenten{}-{}.woff2", p.tag, p.set)
}

fn face_css(p: &Page, prefix: &str) -> String {
    format!(
        "  @font-face {{ font-family: \"Tenten {tag}\"; src: url(\"{prefix}fonts/{file}\") format(\"woff2\"); font-weight: 100 900; }}\n",
        tag = p.tag,
        file = font_file(p)
    )
}

fn nav(current: Option<&str>, prefix: &str) -> String {
    let mut s = format!("<nav><a class=\"home\" href=\"{prefix}\">Tenten 點點</a>");
    let cur = |d: &str| if current == Some(d) { " aria-current=\"page\"" } else { "" };
    s += &format!("<a href=\"{prefix}\"{}>English</a>", if current.is_none() { " aria-current=\"page\"" } else { "" });
    for p in &PAGES {
        s += &format!("<a href=\"{prefix}{}/\" lang=\"{}\"{}>{}</a>", p.dir, p.lang, cur(p.dir), p.native);
    }
    s += &format!("<a href=\"{REPO}\">GitHub</a></nav>\n");
    s
}

fn head(lang: &str, title: &str, faces: &str, prefix: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<link rel=\"stylesheet\" href=\"{prefix}style.css\">\n<style>\n{faces}</style>\n"
    )
}

fn page(p: &Page, glyphs: usize) -> String {
    let fam = format!("font-family: \"Tenten {}\"", p.tag);
    let mut s = head(p.lang, &format!("Tenten {} — {}", p.tag, p.native), &face_css(p, "../"), "../");
    s += &nav(Some(p.dir), "../");
    s += &format!("<p class=\"lead\">{}</p>\n", esc(p.lead));
    s += &format!(
        "<div class=\"controls\"><label>wght {} <input id=\"w\" type=\"range\" min=\"100\" max=\"900\" value=\"400\"><output>400</output></label><label>ROND {} <input id=\"r\" type=\"range\" min=\"0\" max=\"100\" value=\"100\"><output>100</output></label></div>\n",
        p.axes[0], p.axes[1]
    );
    s += &format!("<div class=\"board s96 live\" style='{fam}' contenteditable spellcheck=\"false\">{}</div>\n", esc(p.hero));
    for (style, text) in p.boards {
        let (class, var) = if style.starts_with("--") { ("", format!("; {style}")) } else { (*style, String::new()) };
        let live = if var.is_empty() { " live" } else { "" };
        s += &format!("<div class=\"board {class}{live}\" style='{fam}{var}' contenteditable spellcheck=\"false\">{}</div>\n", esc(text));
    }
    s += &format!("<h2>{}</h2>\n<div class=\"grid\">\n", p.instances);
    for (name, w, r) in INSTANCES {
        s += &format!(
            "<div><p>{name} · wght {w} · ROND {r}</p><div class=\"board\" style='{fam}; --wght:{w}; --rond:{r}'>{}</div></div>\n",
            esc(p.pangram)
        );
    }
    s += "</div>\n";
    s += &format!(
        "<h2>{}</h2>\n<p class=\"meta\">{}</p>\n<ul class=\"dl\">\n",
        p.download,
        esc(&p.covers.replace("{n}", &thousands(glyphs)))
    );
    s += &format!("<li><a href=\"../fonts/{0}\">{0}</a></li>\n", font_file(p));
    s += &format!(
        "<li><a href=\"{REPO}/releases/latest\">Tenten{0}-{1}.ttf · Tenten{0}-Full.ttf</a> — {2}</li>\n</ul>\n",
        p.tag, p.set, p.full
    );
    s += &format!("<footer>{} <a href=\"{REPO}\">{REPO}</a></footer>\n{SCRIPT}\n</html>\n", esc(p.footer));
    s
}

fn index(counts: &[usize]) -> String {
    let faces: String = PAGES.iter().map(|p| face_css(p, "")).collect();
    let mut s = head("en", "Tenten 點点々 — a variable CJK dot font", &faces, "");
    s += &nav(None, "");
    s += "<p class=\"lead\">A variable CJK dot font. Every glyph is drawn as dots on a 12 × 12 lattice, and the dots are what vary: <code>wght</code> sets their size, <code>ROND</code> their shape, from square pixels to round dots. Four fonts, one per locale, each built from its own face of Fusion Pixel Font so every character has its local form.</p>\n";
    s += "<div class=\"controls\"><label>wght <input id=\"w\" type=\"range\" min=\"100\" max=\"900\" value=\"400\"><output>400</output></label><label>ROND <input id=\"r\" type=\"range\" min=\"0\" max=\"100\" value=\"100\"><output>100</output></label></div>\n";
    s += "<div class=\"locales\">\n";
    for (p, n) in PAGES.iter().zip(counts) {
        s += &format!(
            "<a href=\"{}/\" lang=\"{}\"><div class=\"board s96 live\" style='font-family: \"Tenten {}\"'>{}</div><p class=\"meta\">Tenten {} · {} · {} glyphs · {} →</p></a>\n",
            p.dir,
            p.lang,
            p.tag,
            esc(p.hero),
            p.tag,
            p.set.replace("KSX", "KS X 1001").replace("JIS", "JIS X 0208"),
            thousands(*n),
            p.native
        );
    }
    s += "</div>\n";
    s += &format!(
        "<footer>Fonts: SIL Open Font License 1.1, glyph bitmaps from <a href=\"https://github.com/TakWolf/fusion-pixel-font\">Fusion Pixel Font</a>. Code: MIT. After <a href=\"https://github.com/sundyme/punctum\">Punctum</a>. <a href=\"{REPO}\">{REPO}</a></footer>\n{SCRIPT}\n</html>\n"
    );
    s
}

fn thousands(n: usize) -> String {
    let d = n.to_string();
    let mut s = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            s.push(',');
        }
        s.push(c);
    }
    s
}

/// Every character a page sets in its font must be in the font.
fn check(p: &Page, have: &HashSet<i64>) {
    let mut text: String = p.hero.to_string() + p.pangram;
    p.boards.iter().for_each(|b| text += b.1);
    let missing: Vec<char> = text.chars().filter(|&c| c != '\n' && c != ' ' && !have.contains(&(c as i64))).collect();
    if !missing.is_empty() {
        die(&format!("{}: not in Tenten{}-{}: {}", p.dir, p.tag, p.set, missing.iter().collect::<String>()));
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let out = args.iter().position(|a| a == "--out").and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| "docs".into());
    let mut counts = Vec::new();
    for p in &PAGES {
        let bdf = format!("source/fusion-pixel-12px-monospaced-{}.bdf", p.face);
        let data = fs::read_to_string(&bdf).unwrap_or_else(|_| die(&format!("cannot read {bdf} (run scripts/fetch-source.sh)")));
        let set = fs::read_to_string(format!("charsets/{}.txt", p.charset)).unwrap_or_else(|_| die("cannot read charset"));
        let f = tenten::parse(&data, Some(&tenten::charset(&set)));
        check(p, &f.glyphs.iter().map(|g| g.cp).collect());
        counts.push(f.glyphs.len());
    }
    fs::create_dir_all(format!("{out}/fonts")).unwrap_or_else(|_| die("cannot create output directory"));
    for (p, &n) in PAGES.iter().zip(&counts) {
        let woff = format!("build/{}", font_file(p));
        fs::copy(&woff, format!("{out}/fonts/{}", font_file(p)))
            .unwrap_or_else(|_| die(&format!("cannot copy {woff} (run scripts/build-fonts.sh)")));
        fs::create_dir_all(format!("{out}/{}", p.dir)).unwrap_or_else(|_| die("cannot create page directory"));
        fs::write(format!("{out}/{}/index.html", p.dir), page(p, n)).unwrap_or_else(|_| die("cannot write page"));
        println!("{out}/{}/index.html  {} glyphs", p.dir, n);
    }
    fs::write(format!("{out}/style.css"), CSS).unwrap_or_else(|_| die("cannot write style.css"));
    fs::write(format!("{out}/index.html"), index(&counts)).unwrap_or_else(|_| die("cannot write index"));
    fs::write(format!("{out}/.nojekyll"), "").unwrap_or_else(|_| die("cannot write .nojekyll"));
    println!("{out}/index.html");
}
