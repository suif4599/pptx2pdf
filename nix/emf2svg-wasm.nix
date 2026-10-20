{ stdenv, emscripten, libemf2svg, zlib, libpng, wrapper }:

stdenv.mkDerivation {
  pname = "emf2svg-wasm";
  version = "unstable";

  src = libemf2svg;

  nativeBuildInputs = [ emscripten ];

  patches = [ ../patches/emf2svg-no-fonts.patch ];

  buildPhase = ''
    runHook preBuild

    mkdir -p $TMP/deps/zlib $TMP/deps/png
    tar xf ${zlib.src} -C $TMP/deps/zlib --strip-components=1
    tar xf ${libpng.src} -C $TMP/deps/png --strip-components=1

    export EM_CACHE=$TMP/em-cache
    mkdir -p $EM_CACHE

    (
      cd $TMP/deps/zlib
      emcc -O2 -c adler32.c crc32.c deflate.c infback.c inffast.c inflate.c inftrees.c trees.c uncompr.c zutil.c
    )
    (
      cd $TMP/deps/png
      cp scripts/pnglibconf.h.prebuilt pnglibconf.h
      emcc -O2 -I. -I$TMP/deps/zlib -c $(ls *.c | grep -v -E 'example|test')
    )

    LIBS="$TMP/deps/zlib/*.o $TMP/deps/png/*.o"
    SRC=src/lib
    UEMF=vendor/libuemf

    emcc -O2 \
      -I inc -I vendor/libuemf/include -I . -I $TMP/deps/png -I $TMP/deps/zlib \
      -DEMf2SVG_NO_FONTS=1 \
      ${wrapper} \
      $SRC/emf2svg.c $SRC/emf2svg_utils.c $SRC/emf2svg_img_utils.c $SRC/emf2svg_print.c $SRC/pmf2svg.c $SRC/pmf2svg_print.c \
      $SRC/emf2svg_clip_utils.c $SRC/emf2svg_rec_control.c \
      $SRC/emf2svg_rec_object_creation.c $SRC/emf2svg_rec_path.c \
      $SRC/emf2svg_rec_clipping.c $SRC/emf2svg_rec_drawing.c \
      $SRC/emf2svg_rec_bitmap.c $SRC/emf2svg_rec_object_manipulation.c \
      $SRC/emf2svg_rec_comment.c $SRC/emf2svg_rec_transform.c \
      $SRC/emf2svg_rec_state_record.c \
      $UEMF/uemf.c $UEMF/uemf_utf.c $UEMF/uemf_endian.c $UEMF/upmf.c \
      $LIBS \
      -o emf2svg.js \
      -sMODULARIZE=1 -sEXPORT_NAME=Emf2Svg \
      -sENVIRONMENT=web,worker \
      -sALLOW_MEMORY_GROWTH=1 \
      --no-entry \
      -sEXPORTED_FUNCTIONS=_emf2svg_convert,_emf2svg_free_out,_malloc,_free \
      -sEXPORTED_RUNTIME_METHODS=UTF8ToString,stringToUTF8,lengthBytesUTF8,getValue,setValue,HEAPU8,HEAP32

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    echo "export default Emf2Svg;" >> emf2svg.js
    mkdir -p $out/dist
    cp emf2svg.js emf2svg.wasm $out/dist/
    runHook postInstall
  '';
}
