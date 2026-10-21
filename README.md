# pptx2pdf

Correctly render PPTX files containing old-style OLE equation objects (Equation.3 / Equation.DSMT4 from MathType 3.0) and non-standard EMF images as PDFs

## Features

- Automatically locate and replace both types of content while maintaining position, size, and hierarchy:
  - OLE equation objects: Re-render the WMF preview as SVG
  - EMF images: Convert via `libemf2svg` to SVG and embed using `asvg:svgBlip`
- `--nosvg` fallback to full grid mode
- Self-contained fonts: corefonts, wine Symbol, Liberation, DejaVu, Noto CJK

Rendering cache stored in `~/.cache/pptx2pdf/<hash>/`

## Installation

Add input to `flake.nix`:

```nix
inputs.pptx2pdf.url = "github:<owner>/pptx2pdf";
```

Three install modes are provided through the home-manager module:

```nix
{ inputs, ... }: {
  imports = [ inputs.pptx2pdf.homeManagerModules.default ];

  programs.pptx2pdf = {
    enable = true;
    # useSystemLibreOffice = true; the wrapper calls `soffice` from PATH
    # staticLibreOffice = true will install the stripped single-binary LibreOffice
  };
}
```

Equivalent packages without home-manager: `pptx2pdf` (default), `pptx2pdf-system`, `pptx2pdf-static`.

## WASM dist

Full in-browser conversion (formula fix + LibreOffice render, nothing leaves the
machine). Build the static dist (LO artifacts come from the out-of-tree
emscripten build; everything else is built by the flake):

```bash
LO_WASM_ARTIFACTS=~/lo-src/build-wasm/workdir/installation/LibreOffice/emscripten \
  nix build --impure .#lo-wasm-dist
```

Output layout: `result/dist/` is the site (open with any COOP/COEP header
server), `result/r2/` holds everything the page loads from the `lo/` path —
`soffice.js`, `soffice.wasm`, the fonts-appended `soffice.data` and its
metadata, the bundled fonts, and the two wasm modules used by the fix step —
each with a `.br` variant served when the client accepts brotli.
The Worker in `dist/worker/` serves them (`wrangler deploy`; upload every file
in `r2/`, including `fonts/` and `sizes.json`).
Local test:

```bash
nix shell nixpkgs#python3 -c python3 /tmp/wasm-serve.py result/dist result/r2 8931
```

then open http://127.0.0.1:8931 — open a pptx/ppt, watch the progress bar, get
an in-page PDF preview and download. The page requires cross-origin isolation
(SharedArrayBuffer); the shipped `_headers` handles it on Cloudflare Pages.

Fonts: the bundled set includes WenQuanYi Micro Hei as a CJK fallback; when the
browser grants font access, fonts installed on the machine are scanned, cached
in IndexedDB, and used instead. Chinese slide text therefore renders in both
cases (the CJK fallback is a sans font, so serif families are substituted).

## Usage

```bash
pptx2pdf [--nosvg] <input.pptx|input.ppt> [output.pdf]
```

Binary `.ppt` input is converted to pptx with LibreOffice first.

When the output path is omitted, generate a `.pdf` file with the same name in the input file directory.

## License

This project is licensed under the **GNU General Public License v3.0**. See the [LICENSE](LICENSE) file for the full license text.

## Third-party components

Everything below is shipped in the browser dist (`dist/` + `r2/`) unless noted otherwise. Licenses were read from each project's own license file; package-manager metadata (nixpkgs) is cited where the upstream file is not distributed.

### Engine and conversion code

- [LibreOffice](https://www.libreoffice.org) core (the `soffice.js` / `soffice.wasm` / `soffice.data` build, version 26.8.0.3, impress-only) — **MPL-2.0**. Built out-of-tree with local patches (curl-external removal, FS export, memory-growth flags, font strip lists); the patched tree and build flags are described in this repo, satisfying MPL-2.0 source availability.
- Third-party libraries statically linked into the LibreOffice wasm (from LO's external inventory: Boost, expat, fontconfig, FreeType, graphite2, HarfBuzz, Hunspell, ICU, lcms2, libjpeg-turbo, libpng, libtiff, libwebp, libxml2, libxslt, pixman, zlib, zstd, and the LibreOffice import-filter libraries libcdr/libmspub/libvisio/libwpd & co.) — each under its **own upstream license** (MIT-style, BSD-style, Zlib, BSL-1.0, FTL, ICU, LGPL-2.1+/MPL-2.0 depending on the library); see the [LibreOffice license overview](https://www.libreoffice.org/about-us/licenses/) and each library's source tree.
- [libemf2svg](https://github.com/kakwa/libemf2svg) (compiled to `emf2svg.wasm`, EMF→SVG) — **GPL-2.0-or-later**. It embeds [libuemf](https://github.com/kakwa/libemf2svg/tree/master/vendor/libuemf) (**GPL-2.0**, see its COPYING) and statically links zlib (**Zlib**) and libpng (**libpng-2.0**).

### Formula-fix pipeline (`pptx_fix_wasm_bg.wasm`)

Own code under GPL-3.0, statically linking these Rust crates (transitive dependencies are recorded in `Cargo.lock`; each carries its upstream license):

- [metafile-rs](https://github.com/Dantusaikamal/metafile-rs) (WMF→SVG) — **Apache-2.0**
- [resvg](https://github.com/linebender/resvg), usvg, [tiny-skia](https://github.com/linebender/tiny-skia) (SVG rasterization) — **MPL-2.0**
- [fontdb](https://github.com/RazrFalcon/fontdb) — **MIT**
- zip, quick-xml, roxmltree, flate2, png, ttf-parser, wasm-bindgen, js-sys — **MIT** / **MIT OR Apache-2.0**

### Bundled fonts (`lo/fonts/`, also appended into `soffice.data`)

- Microsoft core fonts (Andale Mono, Arial, Comic Sans MS, Courier New, Georgia, Impact, Times New Roman, Trebuchet MS, Verdana, Webdings) — **Microsoft "TrueType core fonts for the Web" EULA** (unfree, redistributable under its terms). Two binding obligations on anyone redistributing this dist: (1) every copy of the fonts must be complete and unmodified and **accompanied by a copy of the EULA** — the build ships it next to the fonts as `lo/fonts/eula.html` (R2) and as `dist/licenses/msttcorefonts-eula.html`; keep them with any redistribution. (2) The font files shipped in this dist are unmodified copies of the Microsoft core fonts, including all original copyright and trademark notices.
- Wine fonts (Wine Tahoma, Wine Symbol, Marlett et al., from the wine `fonts` package) — **LGPL-2.1-or-later**.
- [Liberation fonts](https://github.com/liberationfonts/liberation-fonts) — **SIL OFL 1.1**; the "Liberation" trademark is not licensed.
- [DejaVu fonts](https://dejavu-fonts.github.io) — **DejaVu fonts license** (permissive, Bitstream Vera derived).
- [WenQuanYi Micro Hei](https://wenq.org) — **Apache-2.0** (per its nixpkgs registration; derived from Droid Sans Fallback).

### Build-time tooling (not distributed in the dist)

Nix / nixpkgs (MIT), the Emscripten SDK & LLVM (Apache-2.0-with-LLVM-exceptions / MIT / University of Illinois BSD-style), Rust (MIT OR Apache-2.0). No license obligation follows from these into the built artifacts beyond what is listed above.
