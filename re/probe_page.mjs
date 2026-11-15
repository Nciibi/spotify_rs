// Launch Edge with a remote debugging port, open a URL and dump a structural
// outline of the page. Used to recover the real selectors of the elements the
// host wants to prune or hide.
//
//   node re/probe_page.mjs <url> [port]

import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const url = process.argv[2] || "https://open.spotify.com/";
const port = process.argv[3] || "9344";
const edge =
  process.env.EDGE ||
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";

const profile = mkdtempSync(join(tmpdir(), "probe-"));
const child = spawn(edge, [
  `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`,
  "--no-first-run",
  "--no-default-browser-check",
  "--headless=new",
  "--disable-gpu",
  "--window-size=1280,820",
  url,
]);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function endpoint() {
  for (let i = 0; i < 60; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/json/list`);
      const list = await res.json();
      const page = list.find((t) => t.type === "page" && t.webSocketDebuggerUrl);
      if (page) return page;
    } catch {
      /* not up yet */
    }
    await sleep(500);
  }
  throw new Error("edge never exposed a page target");
}

const page = await endpoint();
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));

let id = 0;
const pending = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) {
    pending.get(m.id)(m);
    pending.delete(m.id);
  }
});
const send = (method, params = {}) =>
  new Promise((r) => {
    const mid = ++id;
    pending.set(mid, r);
    ws.send(JSON.stringify({ id: mid, method, params }));
  });

// Let the app settle: the player renders its shell lazily.
await sleep(9000);

const expression = `(() => {
  const path = (el, depth = 6) => {
    const parts = [];
    let node = el;
    while (node && node.nodeType === 1 && parts.length < depth) {
      let desc = node.tagName.toLowerCase();
      if (node.id) desc += "#" + node.id;
      const testid = node.getAttribute && node.getAttribute("data-testid");
      if (testid) desc += "[data-testid=" + testid + "]";
      const role = node.getAttribute && node.getAttribute("role");
      if (role) desc += "[role=" + role + "]";
      const cls = node.getAttribute && node.getAttribute("class");
      if (cls) desc += "." + cls.trim().split(/\\s+/).slice(0, 3).join(".");
      parts.push(desc);
      node = node.parentElement;
    }
    return parts.join(" < ");
  };

  const text = (el) => (el.textContent || "").trim().replace(/\\s+/g, " ").slice(0, 90);

  const out = {};

  // Every element that carries a data-testid, grouped so the interesting
  // clusters can be picked out.
  const testids = Array.from(document.querySelectorAll("[data-testid]")).map(
    (el) => ({
      testid: el.getAttribute("data-testid"),
      tag: el.tagName.toLowerCase(),
      text: text(el),
      path: path(el, 3),
    })
  );
  out.testidCount = testids.length;
  out.testids = testids.filter((t) =>
    /ad|sponsor|legal|promo|sign|upsell|header|nav|sidebar|library|language|cookie|premium|download|install|support/i.test(
      t.testid + " " + t.text
    )
  );

  // Any element whose own text is short and meaningful, with its path.
  out.leafTexts = Array.from(
    document.querySelectorAll("span, a, button, h1, h2, h3, p, label")
  )
    .filter((el) => el.children.length === 0)
    .map((el) => text(el))
    .filter((t) => t.length > 0 && t.length < 40)
    .filter(
      (t, i, a) =>
        a.indexOf(t) === i &&
        /ad|sponsor|promo|sign|legal|cookie|premium|download|install|support|upgrade|preview|create|playlist|podcast|language|english|browse|about|privacy|safety/i.test(
          t
        )
    );

  // The direct children of the main landmark, one level deep.
  const main = document.querySelector('main, [data-testid="root-layout"]');
  out.mainChildren = main
    ? Array.from(main.children).map((el) => ({
        tag: el.tagName.toLowerCase(),
        testid: el.getAttribute("data-testid"),
        cls: (el.getAttribute("class") || "").slice(0, 90),
        text: text(el).slice(0, 70),
      }))
    : null;

  // Class names that look like ad containers anywhere on the page.
  out.adishClasses = Array.from(
    new Set(
      Array.from(document.querySelectorAll("[class]"))
        .map((el) => el.getAttribute("class") || "")
        .flatMap((c) => c.split(/\\s+/))
        .filter((c) => /^(ad|ads|sponsor|sponsored|promo|upsell|banner)/i.test(c))
    )
  ).slice(0, 40);

  // The left sidebar, outlined.
  const aside = document.querySelector('aside, nav, [data-testid="sidebar"]');
  out.sidebar = aside
    ? Array.from(aside.querySelectorAll(":scope > *, :scope > * > *")).map((el) => ({
        tag: el.tagName.toLowerCase(),
        testid: el.getAttribute("data-testid"),
        cls: (el.getAttribute("class") || "").slice(0, 80),
        text: text(el).slice(0, 70),
        path: path(el, 2),
      }))
    : null;

  // The header, outlined.
  const header = document.querySelector('header, [data-testid="global-nav-bar"]');
  out.header = header
    ? Array.from(header.querySelectorAll("button, a")).map((el) => ({
        tag: el.tagName.toLowerCase(),
        testid: el.getAttribute("data-testid"),
        text: text(el).slice(0, 40),
        path: path(el, 2),
      }))
    : null;

  // The ancestor chain of a few promotional texts, so the card that holds
  // them can be identified.
  const chain = (needle) => {
    const leaf = Array.from(document.querySelectorAll("*")).find(
      (el) =>
        el.children.length === 0 &&
        (el.textContent || "").trim().startsWith(needle)
    );
    if (!leaf) return null;
    const out = [];
    let node = leaf;
    for (let i = 0; i < 7 && node && node.nodeType === 1; i++) {
      out.push({
        depth: i,
        tag: node.tagName.toLowerCase(),
        testid: node.getAttribute("data-testid"),
        role: node.getAttribute("role"),
        cls: (node.getAttribute("class") || "").slice(0, 70),
        directText: Array.from(node.childNodes)
          .filter((n) => n.nodeType === 3)
          .map((n) => n.textContent.trim())
          .filter(Boolean)
          .join(" ")
          .slice(0, 60),
        buttons: Array.from(node.querySelectorAll(":scope > button, :scope > a, :scope > * > button"))
          .map((b) => text(b).slice(0, 30)),
      });
      node = node.parentElement;
    }
    return out;
  };

  out.promoChains = {
    createPlaylist: chain("Create your first playlist"),
    podcasts: chain("Let's find some podcasts to follow"),
    preview: chain("Preview of Spotify"),
    aboutAds: chain("About Ads"),
  };

  return JSON.stringify(out, null, 2);
})()`;

const res = await send("Runtime.evaluate", {
  expression,
  returnByValue: true,
});

if (res.result?.exceptionDetails) {
  console.error("page error:", JSON.stringify(res.result.exceptionDetails, null, 2));
} else {
  console.log(res.result.result.value);
}

ws.close();
child.kill();
try {
  rmSync(profile, { recursive: true, force: true });
} catch {
  /* best effort */
}
