{ runCommand }:
let
  artifactsPath = builtins.getEnv "LO_WASM_ARTIFACTS";
in
runCommand "lo-wasm-dist"
{
  inherit artifactsPath;
}
(
  if artifactsPath == "" then ''
    echo "lo-wasm-dist: the WASM build is not nixified yet." >&2
    echo "The emscripten artifacts are produced by the out-of-tree build in ~/lo-src/build-wasm." >&2
    echo "To assemble the deployable dist:" >&2
    echo "  LO_WASM_ARTIFACTS=$HOME/lo-src/build-wasm/workdir/installation/LibreOffice/emscripten \\" >&2
    echo "    nix build --impure .#lo-wasm-dist" >&2
    exit 1
  '' else ''
    mkdir -p $out/dist
    cp -L ${artifactsPath}/soffice.wasm ${artifactsPath}/soffice.js ${artifactsPath}/soffice.data $out/dist/
  ''
)
