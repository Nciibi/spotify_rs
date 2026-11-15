// Measure the time from launch until the embedded page has real content.
const { spawn } = require("node:child_process");
const http = require("node:http");

const PORT = 9350;
const exe = process.argv[2];
const args = process.argv.slice(3);

function targets() {
  return new Promise((resolve) => {
    const req = http.get({ host: "127.0.0.1", port: PORT, path: "/json/list", timeout: 2000 }, (res) => {
      let body = "";
      res.on("data", (c) => (body += c));
      res.on("end", () => {
        try {
          resolve(JSON.parse(body));
        } catch {
          resolve([]);
        }
      });
    });
    req.on("error", () => resolve([]));
    req.on("timeout", () => {
      req.destroy();
      resolve([]);
    });
  });
}

function evaluate(wsUrl, expression) {
  return new Promise((resolve) => {
    const ws = new WebSocket(wsUrl);
    let id = 0;
    const pending = new Map();
    const finish = (value) => {
      try {
        ws.close();
      } catch {}
      resolve(value);
    };
    ws.addEventListener("open", () => {
      const mid = ++id;
      pending.set(mid, finish);
      ws.send(JSON.stringify({ id: mid, method: "Runtime.evaluate", params: { expression, returnByValue: true } }));
    });
    ws.addEventListener("message", (e) => {
      const m = JSON.parse(e.data);
      if (pending.has(m.id)) {
        pending.get(m.id)(m.result?.result?.value);
        pending.delete(m.id);
      }
    });
    ws.addEventListener("error", () => finish(null));
    setTimeout(() => finish(null), 8000);
  });
}

(async () => {
  const start = Date.now();
  const child = spawn(exe, args, {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}` },
  });

  let page = null;
  while (Date.now() - start < 120000) {
    const list = await targets();
    page = list.find((t) => t.type === "page");
    if (page) break;
    await new Promise((r) => setTimeout(r, 50));
  }
  if (!page) {
    console.log("no page target");
    child.kill();
    process.exit(1);
  }
  const targetAppeared = (Date.now() - start) / 1000;

  let content = null;
  while (Date.now() - start < 120000) {
    const value = await evaluate(page.webSocketDebuggerUrl, "document.body ? document.body.querySelectorAll('*').length : 0");
    if (value && value > 400) {
      content = (Date.now() - start) / 1000;
      break;
    }
    await new Promise((r) => setTimeout(r, 100));
  }

  console.log(`page target : ${targetAppeared.toFixed(2)}s`);
  console.log(`content     : ${content ? content.toFixed(2) + "s" : "not reached"}`);
  child.kill();
})();
