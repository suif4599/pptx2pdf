const MIME = {
  wasm: 'application/wasm',
  js: 'application/javascript',
  data: 'application/octet-stream',
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const name = url.pathname.split('/').pop();
    const obj = await env.LO_ASSETS.get(name);
    if (!obj) return new Response('not found', { status: 404 });
    const headers = {
      'Content-Type': MIME[name.split('.').pop()] || 'application/octet-stream',
      'Access-Control-Allow-Origin': '*',
      'Cache-Control': 'public, max-age=31536000, immutable',
    };
    return new Response(obj.body, { headers });
  },
};
