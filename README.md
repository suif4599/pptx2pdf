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
