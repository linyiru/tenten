# Tenten 點點

[![Release](https://img.shields.io/github/v/release/linyiru/tenten)](https://github.com/linyiru/tenten/releases/latest) [![Specimen](https://img.shields.io/badge/specimen-GitHub%20Pages-222)](https://linyiru.github.io/tenten/) [![Locales](https://img.shields.io/badge/locales-TC%20%C2%B7%20SC%20%C2%B7%20JP%20%C2%B7%20KR-555)](#fonts) [![Font license: OFL-1.1](https://img.shields.io/badge/font-OFL--1.1-blue)](OFL.txt) [![Code license: MIT](https://img.shields.io/badge/code-MIT-green)](LICENSE) [![Rust: std only](https://img.shields.io/badge/Rust-std%20only-orange?logo=rust)](Cargo.toml)

**English** · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

A variable CJK dot font. Every glyph is drawn as dots on a 12 × 12 lattice,
and the dots are what vary: `wght` sets their size, `ROND` their shape, from
square pixels to round dots.

*Tenten* is 点々 (*ten-ten*), Japanese for "dot by dot". It is written 點點 in
Chinese with the same meaning, and てんてん is also the everyday name of the
two-dot mark ゛. The idea comes from
[Punctum](https://github.com/sundyme/punctum), a 5 × 7 Latin dot font. Tenten
uses the same construction at the size Han characters need.

**Specimens:** [English](https://linyiru.github.io/tenten/) ·
[繁體中文](https://linyiru.github.io/tenten/zh-TW/) ·
[简体中文](https://linyiru.github.io/tenten/zh-CN/) ·
[日本語](https://linyiru.github.io/tenten/ja/) ·
[한국어](https://linyiru.github.io/tenten/ko/)

![Tenten in four locales](docs/specimen.png)

## Fonts

There is one font per locale, each built from that locale's face of Fusion
Pixel Font, so a character shared by several languages takes its local form
in each.

| font | locale | charset | glyphs | woff2 |
|---|---|---|---|---|
| `TentenTC-Big5` | Traditional Chinese | ASCII, Big5 symbols, the 5,401 common Big5 hanzi | 5,845 | 126 KB |
| `TentenSC-GB2312` | Simplified Chinese | GB 2312 | 7,368 | 142 KB |
| `TentenJP-JIS` | Japanese | JIS X 0208 and JIS X 0201 kana | 7,010 | 144 KB |
| `TentenKR-KSX` | Korean | KS X 1001 and all 11,172 modern Hangul syllables | 16,919 | 179 KB |
| `Tenten<L>-Full` | each of the above | every glyph in the face | 36,980 | — |

The `.woff2` files are in [docs/fonts/](docs/fonts/). The `.ttf` files, the
full fonts included, are on the
[releases page](https://github.com/linyiru/tenten/releases/latest). Each
family is named `Tenten TC`, `Tenten SC`, `Tenten JP` or `Tenten KR`, and sets
its OS/2 code page and Unicode ranges to match.

Axes and named instances:

| | `ROND` 100 (round) | `ROND` 0 (square) |
|---|---|---|
| `wght` 100 | Thin | Pixel Thin |
| `wght` 400 | Regular | Pixel Regular |
| `wght` 900 | Black | Pixel Black |

What the source faces do not have, and the subsets therefore skip:

| font | missing |
|---|---|
| TC | 66 symbols, mostly small-form punctuation, math operators and unit squares. No hanzi. |
| SC | 143 rare hanzi (鼗赝劐衮脔…) and 31 math symbols |
| JP | 33 math symbols. No kana or kanji. |
| KR | 36 hanja (磎恝旣擥笭…) and 187 symbols and unit squares. No Hangul. |

## How it is built

Glyph bitmaps come from the zh_TW, zh_Hans, ja and ko faces of
[Fusion Pixel Font](https://github.com/TakWolf/fusion-pixel-font) 12px
monospaced (BDF, v2026.09.25). `tenten` turns each lit pixel into a reference
to one shared dot glyph:

- UPM 1200, one lattice cell is 100 units, ascender 1000, descender −200
- glyph 1 is the dot: a 16-point quadratic contour
- every other glyph is a composite of dot references, 8 bytes per dot
- `gvar` holds deltas only for the dot glyph, so the whole font varies through one glyph's five tuples
- the square masters are the circle projected radially onto a square, so the two shapes interpolate point for point

```sh
scripts/fetch-source.sh   # download and verify the four BDFs into source/
scripts/build-fonts.sh    # cargo build --release, then build/*.ttf and the subset .woff2
cargo run --release --bin site   # docs/: the specimen site, one page per locale
```

`tenten [--locale TC|SC|JP|KR] <in.bdf> <out.ttf> [charset.txt]` builds one
font. The charset file keeps the characters it contains, read as UTF-8, and
leaving it out keeps every glyph. `build-fonts.sh` writes `.woff2` through
fontTools with brotli, from python or, failing that, `uvx`; set
`WOFF2_FULL=1` to compress the full fonts as well, which takes fontTools over
ten minutes each.

The charsets in `charsets/` were generated from Python's `big5`, `gb2312`,
`euc_jp` and `euc_kr` codecs. Where Windows or GB 18030 maps a code point to a
different character, both are kept: GB 2312 A1AA is U+2015 in the codec and
U+2014 in practice, and JIS `～` is U+301C or U+FF5E.

`site` writes the specimen site to `docs/`, which GitHub Pages serves: an
English index and one page per locale, each in its own font and `lang`. It
checks every sample character against the locale's subset first and stops if
one is missing.

## Film

`film` renders a 20-second launch film, picture and sound, from the zh_TW
BDF. It is plain Rust like the font builder; ffmpeg only encodes. The film is
on the [releases page](https://github.com/linyiru/tenten/releases/latest).

```sh
scripts/build-film.sh                  # build/film/tenten.mp4 and tenten.wav
scripts/build-film.sh --still 2,9.5,15 # review stills only
```

- Every frame is a pure function of time (`src/bin/film/scene.rs`), so frames
  render in parallel and in any order. A dot's size and shape come from
  `tenten::dot_half`, the font's own masters, so a dot on screen is the dot
  the font draws at that `wght` and `ROND`.
- The score (`audio.rs`) is synthesized from the event list the picture is
  timed by: each lit dot the playhead passes is a note, its row the pitch on
  a D minor pentatonic, and `ROND` turns the pad from sine to square.
- The rasterizer (`raster.rs`) draws anti-aliased rounded squares with a
  bloom, and fast camera moves are averaged over a half-frame shutter.

## License

- **Fonts** (everything `tenten` writes into `build/`, and `docs/fonts/`):
  SIL Open Font License 1.1, see [OFL.txt](OFL.txt). They are a Modified
  Version of Fusion Pixel Font, which includes glyphs from Ark Pixel Font,
  Cubic 11, and Galmuri. Their licenses are in [LICENSES/](LICENSES/).
- **Everything else in this repository** (the Rust source, scripts, charset
  lists, and specimen site): MIT, see [LICENSE](LICENSE).
