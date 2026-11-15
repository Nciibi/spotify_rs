// Probe the page rendered inside our WebView2 host.
const port = process.argv[2] || "9340";

const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = list.find((t) => t.type === "page");
if (!page) {
  console.log("no page target");
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));

let id = 0;
const pending = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) {
    pending.get(m.id)(m.result);
    pending.delete(m.id);
  }
});
const send = (method, params = {}) =>
  new Promise((r) => {
    const mid = ++id;
    pending.set(mid, r);
    ws.send(JSON.stringify({ id: mid, method, params }));
  });

const expression = `JSON.stringify({
  title: document.title,
  w: innerWidth,
  h: innerHeight,
  fetchPatched: String(window.fetch).includes('Blocked by spotify-rs'),
  styleInjected: !!document.getElementById('spotify-rs-style'),
  blockedCount: window.__spotifyRsBlocked || 0,
  bodyBg: getComputedStyle(document.body).backgroundColor,
  elements: document.body.querySelectorAll('*').length,
  text: (document.body.innerText || '').replace(/\\s+/g, ' ').slice(0, 240)
})`;

const out = await send("Runtime.evaluate", { expression, returnByValue: true });
console.log(out.result.value);
ws.close();
