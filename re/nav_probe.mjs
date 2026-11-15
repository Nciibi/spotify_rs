const port = process.argv[2] || "9342";
const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = list.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
let id = 0;
const pending = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); }
});
const send = (method, params = {}) => new Promise((r) => {
  const mid = ++id; pending.set(mid, r);
  ws.send(JSON.stringify({ id: mid, method, params }));
});

const expr = `(() => {
  const nav = document.querySelector('[data-testid="global-nav-bar"]');
  const buttons = nav ? Array.from(nav.querySelectorAll('button, a')).map(b => ({
    text: (b.textContent || '').trim().slice(0, 30),
    testid: b.getAttribute('data-testid'),
    hidden: b.getAttribute('data-spotify-rs-hide'),
    tag: b.tagName
  })) : [];
  return JSON.stringify({
    hasNav: !!nav,
    buttons: buttons.slice(0, 20),
    hiddenCount: document.querySelectorAll('[data-spotify-rs-hide]').length,
    styleText: (document.getElementById('spotify-rs-style') || {}).textContent ? 'yes' : 'no',
    hasHideRule: (document.getElementById('spotify-rs-style') || {textContent:''}).textContent.includes('data-spotify-rs-hide')
  }, null, 2);
})()`;

const out = await send("Runtime.evaluate", { expression: expr, returnByValue: true });
console.log(out.result.value);
ws.close();
