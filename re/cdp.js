// Attach to a Chromium target over the DevTools protocol and report what the
// page logged, failed to load, and what globals it exposes.

const PORT = process.argv[2] || "9222";

async function targets() {
  const res = await fetch(`http://127.0.0.1:${PORT}/json/list`);
  return res.json();
}

function connect(wsUrl) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl);
    let id = 0;
    const pending = new Map();
    const events = [];

    ws.addEventListener("open", () => resolve(api));
    ws.addEventListener("error", reject);
    ws.addEventListener("message", (ev) => {
      const msg = JSON.parse(ev.data);
      if (msg.id && pending.has(msg.id)) {
        pending.get(msg.id)(msg.result);
        pending.delete(msg.id);
      } else if (msg.method) {
        events.push(msg);
      }
    });

    const api = {
      send(method, params = {}) {
        const mid = ++id;
        return new Promise((res) => {
          pending.set(mid, res);
          ws.send(JSON.stringify({ id: mid, method, params }));
        });
      },
      events,
      close: () => ws.close(),
    };
  });
}

const list = await targets();
const page = list.find((t) => t.type === "page" && t.url.includes("127.0.0.1"));
if (!page) {
  console.log("no matching page target");
  console.log(JSON.stringify(list.map((t) => ({ type: t.type, url: t.url, title: t.title })), null, 2));
  process.exit(1);
}

console.log(`target: ${page.title}  ${page.url}\n`);
const cdp = await connect(page.webSocketDebuggerUrl);

await cdp.send("Runtime.enable");
await cdp.send("Log.enable");
await cdp.send("Network.enable");

// Give the page a moment to emit anything.
await new Promise((r) => setTimeout(r, 4000));

const consoleMsgs = cdp.events.filter((e) => e.method === "Runtime.consoleAPICalled");
const exceptions = cdp.events.filter((e) => e.method === "Runtime.exceptionThrown");
const logs = cdp.events.filter((e) => e.method === "Log.entryAdded");
const failed = cdp.events.filter((e) => e.method === "Network.loadingFailed");

console.log(`=== console (${consoleMsgs.length}) ===`);
for (const e of consoleMsgs.slice(0, 25)) {
  const args = (e.params.args || []).map((a) => a.value ?? a.description ?? a.type).join(" ");
  console.log(`  [${e.params.type}] ${args.slice(0, 300)}`);
}

console.log(`\n=== exceptions (${exceptions.length}) ===`);
for (const e of exceptions.slice(0, 15)) {
  const d = e.params.exceptionDetails;
  console.log(`  ${d.text} ${d.exception?.description ?? ""}`.slice(0, 400));
}

console.log(`\n=== log entries (${logs.length}) ===`);
for (const e of logs.slice(0, 25)) {
  console.log(`  [${e.params.entry.level}] ${e.params.entry.text}`.slice(0, 300));
}

console.log(`\n=== failed requests (${failed.length}) ===`);
for (const e of failed.slice(0, 25)) {
  console.log(`  ${e.params.errorText} ${e.params.type}`.slice(0, 200));
}

// What does the page think it has?
const probe = await cdp.send("Runtime.evaluate", {
  expression: `JSON.stringify({
    readyState: document.readyState,
    bodyChildren: document.body ? document.body.children.length : -1,
    bodyHTML: document.body ? document.body.innerHTML.slice(0, 400) : "",
    hasWebpack: typeof globalThis.__webpack_require__,
    rspackChunk: typeof globalThis.rspackChunk,
    chunkKeys: globalThis.rspackChunk ? Object.keys(globalThis.rspackChunk).slice(0,5) : [],
    webpackChunk: typeof globalThis.webpackChunk,
    globals: Object.keys(globalThis).filter(k => /spotify|esperanto|native|desktop|bridge|host/i.test(k)).slice(0, 40),
  })`,
  returnByValue: true,
});
console.log("\n=== page probe ===");
console.log(probe?.result?.value ?? JSON.stringify(probe).slice(0, 800));

cdp.close();
