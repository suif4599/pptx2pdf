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

Then create `pptx2pdf.nix` and import it in `flake.nix`:

```nix
{ inputs, ... }: {
  home.packages = [ inputs.pptx2pdf.packages.x86_64-linux.default ];
}
```

After writing either file, import it in `flake.nix`:

```nix
imports = [ ./pptx2pdf.nix ];
```

## Usage

```bash
pptx2pdf [--nosvg] <input.pptx|input.ppt> [output.pdf]
```

Binary `.ppt` input is converted to pptx with LibreOffice first.

When the output path is omitted, generate a `.pdf` file with the same name in the input file directory.
