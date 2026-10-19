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
      # headless pptx/ppt-to-pdf spike: same nixpkgs expression, stripped of
      # everything a conversion-only build cannot use; sizing experiment
      lo-min = (pkgs.callPackage "${nixpkgs}/pkgs/applications/office/libreoffice" {
        variant = "stable";
        withHelp = false;
        withFonts = false;
        withJava = false;
        kdeIntegration = false;
        langs = [ ];
      }).overrideAttrs (old: {
        # upstream bug: svl/uui keep a curl external dep when --disable-curl
        patches = old.patches ++ [ ./disable-curl-deps.patch ];
        # static single-binary target: bundled externals produce .a, system
        # ones would stay dynamic — drop every --with-system-* flag
        configureFlags =
          lib.filter (f: !lib.hasPrefix "--with-system-" f && !lib.hasPrefix "--with-boost" f)
            old.configureFlags
          ++ [
          "--enable-wasm-strip"
          "--with-wasm-module=impress"
          "--disable-gui"
          "--disable-gtk3"
          "--disable-gtk4"
          "--disable-qt5"
          "--disable-qt6"
          "--disable-dbus"
          "--disable-dconf"
          "--disable-gio"
          "--disable-gstreamer-1-0"
          "--disable-cups"
          "--disable-curl"
          "--disable-openssl"
          "--disable-nss"
          "--disable-gpgmepp"
          "--with-tls=no"
          "--without-webdav"
          "--disable-breakpad"
          "--disable-crashdump"
          "--disable-database-connectivity"
          "--disable-firebird-sdbc"
          "--disable-postgresql-sdbc"
          "--disable-mariadb-sdbc"
          "--disable-lpsolve"
          "--disable-coinmp"
          "--disable-pdfimport"
          "--disable-pdfium"
          "--disable-poppler"
          "--disable-skia"
          "--disable-opengl"
          "--disable-cairo-canvas"
          "--disable-sdremote"
          "--disable-sdremote-bluetooth"
          "--disable-extension-integration"
          "--disable-extensions"
          "--disable-xmlhelp"
          "--disable-libcmis"
          "--disable-ldap"
          "--disable-lotuswordpro"
          "--without-parquet-filter"
          "--disable-librelogo"
          "--disable-zxing"
          "--disable-opencl"
          "--disable-randr"
          "--disable-eot"
          "--disable-epm"
          "--disable-rust-uno"
          "--enable-python=no"
          "--disable-dynamic-loading"
          "--enable-customtarget-components"
          "--enable-mergelibs=more"
          "--with-theme=colibre"
          "--with-build-platform-configure-options=--without-system-fontconfig --without-system-freetype --without-system-zlib"
        ];
      });
    in
    {
      packages.${system} = {
        default = pptx2pdf;
        inherit pptx2pdf mf-cli emf2svg-conv lo-min;
      };

      apps.${system}.default = {
        type = "app";
        program = "${pptx2pdf}/bin/pptx2pdf";
      };

      homeManagerModules.default = import ./home-manager.nix self;

      devShells.${system} = {
        default = pkgs.mkShell {
          packages = [ python pkgs.inkscape mf-cli emf2svg-conv pkgs.libreoffice ] ++ fonts;
          FONTCONFIG_FILE = fontsConf;
        };
        # incremental build shell for the lo-min spike; source tree in ~/lo-src
        lo = pkgs.mkShell {
          inputsFrom = [ lo-min ];
          NIX_CFLAGS_COMPILE = "-I${pkgs.librdf_rasqal}/include/rasqal -DSAL_LOG_WARN=1 -DSAL_LOG_INFO=1";
        };
      };
    };
}
