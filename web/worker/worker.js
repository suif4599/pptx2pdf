const MIME = {
  wasm: 'application/wasm',
  js: 'application/javascript',
  data: 'application/octet-stream',
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const name = url.pathname.replace(/^\/+lo\/+/, '');
    const headers = {
      'Content-Type': MIME[name.split('.').pop()] || 'application/octet-stream',
      'Access-Control-Allow-Origin': '*',
      'Cache-Control': 'public, max-age=31536000, immutable',
      'Vary': 'Accept-Encoding',
    };
    if ((request.headers.get('Accept-Encoding') || '').includes('br')) {
      const br = await env.LO_ASSETS.get(name + '.br');
      if (br) {
        headers['Content-Encoding'] = 'br';
        headers['Content-Length'] = String(br.size);
        return new Response(br.body, { headers });
      }
    }
    const obj = await env.LO_ASSETS.get(name);
    if (!obj) return new Response('not found', { status: 404 });
    return new Response(obj.body, { headers });
  },
};
