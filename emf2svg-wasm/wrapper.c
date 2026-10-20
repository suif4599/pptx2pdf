#include <emf2svg.h>
#include <emscripten/emscripten.h>
#include <stdlib.h>
#include <string.h>

EMSCRIPTEN_KEEPALIVE
int emf2svg_convert(const char *in, int len, char **out, int *out_len)
{
    generatorOptions options;
    memset(&options, 0, sizeof(options));
    options.emfplus = 1;
    options.svgDelimiter = 1;
    options.nameSpace = NULL;
    return emf2svg((char *)in, (size_t)len, out, (size_t *)out_len, &options);
}

EMSCRIPTEN_KEEPALIVE
void emf2svg_free_out(char *out)
{
    free(out);
}
