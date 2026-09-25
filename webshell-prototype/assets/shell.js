// The only frontend transport. Only pages explicitly trusted by the native runtime receive it.
(() => {
  let latest = null;
  let context = null;
  const listeners = new Set();
  const post = message => window.webkit.messageHandlers.shell.postMessage(JSON.stringify(message));
  window.addEventListener('shell:context', event => { context = event.detail; });
  window.addEventListener('shell:workspaces', event => {
    latest = event.detail;
    for (const listener of listeners) listener(latest);
  });
  window.shell = Object.freeze({
    get context() { return context; },
    workspaces: Object.freeze({
      get: () => latest,
      subscribe(callback) { listeners.add(callback); if (latest) callback(latest); return () => listeners.delete(callback); },
      activate(id) { post({type:'workspace_activate', id}); }
    }),
    surfaces: Object.freeze({toggle(id) { post({type:'surface_toggle', id}); }}),
    inputRegions: Object.freeze({set(rects) { post({type:'input_regions', rects}); }})
  });
})();
