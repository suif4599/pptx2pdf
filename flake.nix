{
  description = "pptx2pdf: convert legacy-formula pptx decks to PDF";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      lib = nixpkgs.lib;
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [ "corefonts" ];
      };
      wineFonts = pkgs.callPackage "${nixpkgs}/pkgs/applications/emulators/wine/fonts.nix" { };
      fonts = [ pkgs.corefonts wineFonts pkgs.liberation_ttf pkgs.dejavu_fonts pkgs.noto-fonts-cjk-sans pkgs.noto-fonts-cjk-serif ];
      fontsConf = pkgs.writeText "fonts.conf" ''
        <?xml version="1.0"?>
        <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
        <fontconfig>
          ${toString (map (f: "<dir>${f}</dir>") fonts)}
          <cachedir>/tmp/fontcache</cachedir>
        </fontconfig>
      '';
      python = pkgs.python3.withPackages (ps: [ ps.python-pptx ps.tqdm ps.pillow ]);
      rustSrc = lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
          let rel = lib.removePrefix (toString ./.) path;
          in lib.any (p: lib.hasPrefix p rel) [ "/Cargo.toml" "/Cargo.lock" "/src" "/third_party" ];
      };
      mf-cli = pkgs.rustPlatform.buildRustPackage {
        pname = "mf-cli";
        version = "0.1.0";
        src = rustSrc;
        cargoLock.lockFile = ./Cargo.lock;
        meta.mainProgram = "mf-cli";
      };
      emf2svg-conv = pkgs.stdenv.mkDerivation {
        pname = "emf2svg-conv";
        version = "unstable";
        src = ./third_party/libemf2svg;
        nativeBuildInputs = [ pkgs.cmake pkgs.pkg-config ];
        buildInputs = [ pkgs.libpng pkgs.freetype pkgs.fontconfig pkgs.libxml2 pkgs.zlib ];
        cmakeFlags = [ "-DCMAKE_BUILD_TYPE=Release" "-DCMAKE_SKIP_BUILD_RPATH=TRUE" ];
        buildPhase = ''
          runHook preBuild
          cmake --build . --target emf2svg-conv -j$NIX_BUILD_CORES
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          install -Dm555 emf2svg-conv $out/bin/emf2svg-conv
          mkdir -p $out/lib
          cp -P libemf2svg.so* $out/lib/
          runHook postInstall
        '';
        postFixup = ''
          patchelf --add-rpath $out/lib $out/bin/emf2svg-conv
        '';
        meta.mainProgram = "emf2svg-conv";
      };
      pptx2pdf = pkgs.writeShellScriptBin "pptx2pdf" ''
        set -euo pipefail
        export FONTCONFIG_FILE=${fontsConf}
        export SAL_LOG="-WARN"
        export PATH="${lib.makeBinPath [ python pkgs.inkscape pkgs.libreoffice mf-cli emf2svg-conv ]}:$PATH"
        flags=()
        in=""
        out=""
        for arg in "$@"; do
          case "$arg" in
            --nosvg) flags+=(--nosvg) ;;
            -h|--help) echo "usage: pptx2pdf [--nosvg] <input.pptx|input.ppt> [output.pdf]"; exit 0 ;;
            *) if [ -z "$in" ]; then in="$arg"; else out="$arg"; fi ;;
          esac
        done
        if [ -z "$in" ]; then
          echo "usage: pptx2pdf [--nosvg] <input.pptx|input.ppt> [output.pdf]" >&2
          exit 1
        fi
        if [ -z "$out" ]; then
          out="''${in%.*}.pdf"
        fi
        work=$(mktemp -d)
        trap 'rm -rf "$work"' EXIT
        src="$in"
        case "''${in##*.}" in
          [pP][pP][tT])
            soffice --headless -env:UserInstallation="file://$work/lo" \
              --convert-to pptx --outdir "$work" "$in" > /dev/null
            src="$work/$(basename "''${in%.*}").pptx"
            ;;
        esac
        python ${./fix_formulas.py} "''${flags[@]+"''${flags[@]}"}" "$src" "$work/work.pptx"
        soffice --headless -env:UserInstallation="file://$work/lo" \
          --convert-to pdf --outdir "$work" "$work/work.pptx" > /dev/null
        mv "$work/work.pdf" "$out"
        echo "saved: $out"
      '';
    in
    {
      packages.${system} = {
        default = pptx2pdf;
        inherit pptx2pdf mf-cli emf2svg-conv;
      };

      apps.${system}.default = {
        type = "app";
        program = "${pptx2pdf}/bin/pptx2pdf";
      };

      homeManagerModules.default = import ./home-manager.nix self;

      devShells.${system}.default = pkgs.mkShell {
        packages = [ python pkgs.inkscape mf-cli emf2svg-conv pkgs.libreoffice ] ++ fonts;
        FONTCONFIG_FILE = fontsConf;
      };
    };
}
