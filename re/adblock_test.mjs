// Verify the ad blocker intercepts requests.
const port = process.argv[2] || "9344";
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

const expression = `(async () => {
  const probe = async (url) => {
    try {
      const r = await fetch(url, { method: 'GET' });
      return { url, status: r.status, statusText: r.statusText };
    } catch (e) {
      return { url, error: String(e) };
    }
  };
  const blocked = await probe('https://spclient.wg.spotify.com/ads/v2/config');
  const sponsored = await probe('https://spclient.wg.spotify.com/sponsoredplaylist/v1/sponsored?market=from_token');
  const normal = await probe('https://open.spotify.com/');
  return JSON.stringify({ blocked, sponsored, normal, counter: window.__spotifyRsBlocked || 0 });
})()`;

const out = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
console.log(out.result.value);
ws.close();
