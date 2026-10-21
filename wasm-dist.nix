{ runCommand, python3, brotli, emf2svg-wasm, pptx-fix-wasm
, corefonts, wineFonts, liberation_ttf, dejavu_fonts, wqy_microhei }:
let
  envPath = builtins.getEnv "LO_WASM_ARTIFACTS";
  artifacts =
    if envPath == "" then null
    else builtins.path { path = envPath; name = "lo-wasm-artifacts"; };
  optArts = if artifacts == null then null else runCommand "lo-wasm-opt" {
    passAsFile = [ "appendScript" ];
    appendScript = ''
      import json, os, sys

      d, fonts_dir = sys.argv[1], sys.argv[2]
      with open(os.path.join(d, "soffice.data"), "rb") as f:
          blob = f.read()
      with open(os.path.join(d, "soffice.data.js.metadata")) as f:
          meta = json.load(f)

      offset = meta["remote_package_size"]
      assert offset == len(blob)
      for name in sorted(os.listdir(fonts_dir)):
          with open(os.path.join(fonts_dir, name), "rb") as f:
              data = f.read()
          meta["files"].append({
              "filename": "/instdir/share/fonts/truetype/" + name,
              "start": offset,
              "end": offset + len(data),
          })
          blob += data
          offset += len(data)
      meta["remote_package_size"] = offset

      with open(os.path.join(d, "soffice.data"), "wb") as f:
          f.write(blob)
      with open(os.path.join(d, "soffice.data.js.metadata"), "w") as f:
          json.dump(meta, f, separators=(",", ":"))
    '';
    nativeBuildInputs = [ brotli ];
  } ''
    mkdir -p $out/fonts
    for p in ${corefonts} ${wineFonts} ${liberation_ttf} ${dejavu_fonts} ${wqy_microhei}; do
      find $p \( -name '*.ttf' -o -name '*.ttc' -o -name '*.otf' \) | while read -r f; do
        case "$(basename "$f")" in
          DejaVuSansCondensed*|DejaVuSerifCondensed*|DejaVuSans-ExtraLight.ttf|DejaVuSansMono-Oblique.ttf|DejaVuSansMono-BoldOblique.ttf) continue ;;
        esac
        cp "$f" $out/fonts/
      done
    done
    install -m 644 ${artifacts}/soffice.data ${artifacts}/soffice.data.js.metadata ${artifacts}/soffice.js ${artifacts}/favicon.ico $out/
    ${python3}/bin/python $appendScriptPath $out $out/fonts
    for f in $out/fonts/*; do
      ${brotli}/bin/brotli -q 11 -w 24 -c "$f" > "$f.br"
    done
    ${python3}/bin/python -c "import json,os,sys; d=sys.argv[1]; json.dump(sorted(n for n in os.listdir(d) if not n.endswith('.br')), open(os.path.join(d,'index.json'),'w'))" $out/fonts
    cp ${artifacts}/soffice.wasm $out/soffice.wasm
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/soffice.wasm > $out/soffice.wasm.br
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/soffice.data > $out/soffice.data.br
  '';
in
runCommand "lo-wasm-dist"
{
  passAsFile = [ "appendScript" ];
}
(
  if artifacts == null then ''
    echo "lo-wasm-dist: LO_WASM_ARTIFACTS not set." >&2
    echo "  LO_WASM_ARTIFACTS=~/lo-src/build-wasm/workdir/installation/LibreOffice/emscripten nix build --impure .#lo-wasm-dist" >&2
    exit 1
  '' else ''
    set -e

    mkdir -p $out/dist/vendor $out/dist/worker $out/r2/fonts
    cp ${./web/index.html} $out/dist/index.html
    cp ${./web/app.js} $out/dist/app.js
    cp ${./web/styles.css} $out/dist/styles.css
    cp ${./web/_headers} $out/dist/_headers
    cp ${./web/fix-worker.js} $out/dist/fix-worker.js
    cp ${./web/worker}/worker.js ${./web/worker}/wrangler.toml $out/dist/worker/

    cp ${emf2svg-wasm}/dist/emf2svg.js $out/dist/vendor/
    cp ${pptx-fix-wasm}/dist/pptx_fix_wasm.js $out/dist/
    cp ${emf2svg-wasm}/dist/emf2svg.wasm $out/r2/
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/r2/emf2svg.wasm > $out/r2/emf2svg.wasm.br
    cp ${pptx-fix-wasm}/dist/pptx_fix_wasm_bg.wasm $out/r2/
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/r2/pptx_fix_wasm_bg.wasm > $out/r2/pptx_fix_wasm_bg.wasm.br

    cp ${optArts}/fonts/* $out/r2/fonts/

    cp ${optArts}/soffice.wasm ${optArts}/soffice.wasm.br $out/r2/
    cp ${optArts}/soffice.data ${optArts}/soffice.data.br $out/r2/
    ${python3}/bin/python -c "import json,os,sys; d=sys.argv[1]; json.dump({n: {'raw': os.path.getsize(os.path.join(d,n)), 'br': os.path.getsize(os.path.join(d,n+'.br'))} for n in ('soffice.wasm','soffice.data')}, open(os.path.join(d,'sizes.json'),'w'))" $out/r2

    mkdir -p $out/dist/lo
    cp ${optArts}/soffice.js $out/dist/lo/
    chmod u+w $out/dist/lo/soffice.js
    # a Node-like process global (browser extensions) makes emscripten take the node path and throw on require
    sed -i 's/var ENVIRONMENT_IS_NODE=[^;]*;/var ENVIRONMENT_IS_NODE=false;/' $out/dist/lo/soffice.js
    cp ${optArts}/favicon.ico $out/dist/
    cp ${optArts}/soffice.data.js.metadata $out/dist/lo/

    cp $out/dist/lo/soffice.js $out/r2/soffice.js
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/r2/soffice.js > $out/r2/soffice.js.br
    cp $out/dist/lo/soffice.data.js.metadata $out/r2/
    ${brotli}/bin/brotli -q 11 -w 24 -c $out/r2/soffice.data.js.metadata > $out/r2/soffice.data.js.metadata.br

    V=$(sha256sum $out/r2/soffice.wasm $out/r2/soffice.data $out/dist/lo/soffice.js $out/r2/pptx_fix_wasm_bg.wasm $out/r2/emf2svg.wasm $out/dist/pptx_fix_wasm.js | sha256sum | cut -c1-12)
    chmod u+w $out/dist/app.js
    sed -i "s/^const V = '[^']*';/const V = '$V';/" $out/dist/app.js
  ''
)
