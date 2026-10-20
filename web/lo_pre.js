// Runs inside every LO JS runtime; only the pthread worker has importScripts.
// With PROXY_TO_PTHREAD the embind UNO tree lives in that worker, so the page
// reaches it through the MessagePort that setupMainChannel publishes.
if (typeof importScripts === 'function') {
  const poll = setInterval(() => {
    if (!Module.uno_mainPort || !Module.uno) return;
    clearInterval(poll);
    Module.uno_mainPort.onmessage = (e) => {
      const port = Module.uno_mainPort;
      const msg = e.data;
      try {
        convert(msg);
        port.postMessage({ id: msg.id, ok: true });
      } catch (e) {
        let reason = (e && e.stack) || String(e);
        try {
          reason = String(Module.catchUnoException(e));
        } catch (ignored) {}
        port.postMessage({ id: msg.id, ok: false, error: reason });
      }
    };
  }, 50);

  function args(filter) {
    if (!filter) return new Module.uno_Sequence_com$sun$star$beans$PropertyValue(0, Module.uno_Sequence.FromSize);
    const value = new Module.uno_Any(Module.uno_Type.String(), filter);
    const pv = { Name: 'FilterName', Handle: -1, Value: value, State: 0 };
    return new Module.uno_Sequence_com$sun$star$beans$PropertyValue([pv]);
  }

  function convert(msg) {
    const css = Module.uno.com.sun.star;
    const desktop = css.frame.Desktop.create(Module.getUnoComponentContext());
    const loader = css.frame.XComponentLoader.query(desktop);
    const doc = loader.loadComponentFromURL(msg.url, '_default', 0, args(msg.filter));
    if (!doc) throw new Error(`cannot load ${msg.url}`);
    css.frame.XStorable.query(doc).storeToURL(msg.out, args(msg.storeFilter));
    css.util.XCloseable.query(doc).close(true);
  }
}
