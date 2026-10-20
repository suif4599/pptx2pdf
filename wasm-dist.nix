{ runCommand, python3, emf2svg-wasm, pptx-fix-wasm
, corefonts, wineFonts, liberation_ttf, dejavu_fonts, wqy_microhei }:
let
  envPath = builtins.getEnv "LO_WASM_ARTIFACTS";
  artifacts =
    if envPath == "" then null
    else builtins.path { path = envPath; name = "lo-wasm-artifacts"; };
in
runCommand "lo-wasm-dist"
{
  passAsFile = [ "appendScript" ];
  appendScript = ''
    import json, os, sys

    src_dir, out_dir, fonts_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(os.path.join(src_dir, "soffice.data"), "rb") as f:
        blob = f.read()
    with open(os.path.join(src_dir, "soffice.data.js.metadata")) as f:
        meta = json.load(f)

    offset = meta["remote_package_size"]
    assert offset == len(blob)
    fonts = sorted(os.listdir(fonts_dir))
    for name in fonts:
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

    with open(os.path.join(out_dir, "soffice.data"), "wb") as f:
        f.write(blob)
    with open(os.path.join(out_dir, "soffice.data.js.metadata"), "w") as f:
        json.dump(meta, f, separators=(",", ":"))
  '';
}
(
  if artifacts == null then ''
    echo "lo-wasm-dist: LO_WASM_ARTIFACTS not set." >&2
    echo "  LO_WASM_ARTIFACTS=~/lo-src/build-wasm/workdir/installation/LibreOffice/emscripten nix build --impure .#lo-wasm-dist" >&2
    exit 1
  '' else ''
    set -e
    A=${artifacts}

    mkdir -p $out/dist/vendor $out/dist/fonts $out/dist/worker $out/r2
    cp ${./web/index.html} $out/dist/index.html
    cp ${./web/app.js} $out/dist/app.js
    cp ${./web/styles.css} $out/dist/styles.css
    cp ${./web/_headers} $out/dist/_headers
    cp ${./web/worker}/worker.js ${./web/worker}/wrangler.toml $out/dist/worker/

    cp ${emf2svg-wasm}/dist/emf2svg.js ${emf2svg-wasm}/dist/emf2svg.wasm $out/dist/vendor/
    cp ${pptx-fix-wasm}/dist/pptx_fix_wasm.js ${pptx-fix-wasm}/dist/pptx_fix_wasm_bg.wasm $out/dist/

    for p in ${corefonts} ${wineFonts} ${liberation_ttf} ${dejavu_fonts} ${wqy_microhei}; do
      find $p \( -name '*.ttf' -o -name '*.ttc' -o -name '*.otf' \) -exec cp {} $out/dist/fonts/ \;
    done
    ls $out/dist/fonts > $out/fonts-index.tmp
    ${python3}/bin/python -c "import json,sys; json.dump([l.strip() for l in open(sys.argv[1])], open(sys.argv[2],'w'))" $out/fonts-index.tmp $out/dist/fonts/index.json
    rm $out/fonts-index.tmp

    ${python3}/bin/python $appendScriptPath $A $out/r2 $out/dist/fonts
    cp $A/soffice.wasm $out/r2/
    mkdir -p $out/dist/lo
    cp $A/soffice.js $out/dist/lo/
    chmod u+w $out/dist/lo/soffice.js
    # a Node-like process global (browser extensions) makes emscripten take the node path and throw on require
    sed -i 's/var ENVIRONMENT_IS_NODE=[^;]*;/var ENVIRONMENT_IS_NODE=false;/' $out/dist/lo/soffice.js
    cp $A/favicon.ico $out/dist/
    cp $out/r2/soffice.data.js.metadata $out/dist/lo/
  ''
)
