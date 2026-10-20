{ rustPlatform, wasm-bindgen-cli, lld, rustSrcFull }:

rustPlatform.buildRustPackage {
  pname = "pptx-fix-wasm";
  version = "0.1.0";

  src = rustSrcFull;
  cargoLock.lockFile = ./../Cargo.lock;

  nativeBuildInputs = [ wasm-bindgen-cli lld ];

  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";

  buildPhase = ''
    runHook preBuild
    cargo build --release --target wasm32-unknown-unknown -p pptx-fix-wasm --offline
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/dist
    wasm-bindgen --target web --out-dir $out/dist target/wasm32-unknown-unknown/release/pptx_fix_wasm.wasm
    runHook postInstall
  '';
}
