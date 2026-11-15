// Find the DOM selectors of the elements we want to hide.
const port = process.argv[2] || "9340";

const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = list.find((t) => t.type === "page");
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

const expression = `(() => {
  const describe = (el) => {
    if (!el) return null;
    const path = [];
    let node = el;
    while (node && node.nodeType === 1 && path.length < 5) {
      let desc = node.tagName.toLowerCase();
      if (node.id) desc += '#' + node.id;
      const testid = node.getAttribute && node.getAttribute('data-testid');
      if (testid) desc += '[data-testid=' + testid + ']';
      const cls = node.getAttribute && node.getAttribute('class');
      if (cls) desc += '.' + cls.trim().split(/\\s+/).slice(0, 2).join('.');
      path.push(desc);
      node = node.parentElement;
    }
    return path.join(' < ');
  };

  const results = {};

  // The "Preview of Spotify" upsell banner.
  const banner = Array.from(document.querySelectorAll('*')).find(
    (el) => el.children.length === 0 && (el.textContent || '').trim() === 'Preview of Spotify'
  );
  results.banner = describe(banner ? banner.closest('section, div[class*=anner], [class*=promo], [class*=upsell]') || banner.parentElement : null);

  // The header navigation links (Premium / Support / Download / Install App).
  const premium = Array.from(document.querySelectorAll('a,button')).find(
    (el) => (el.textContent || '').trim() === 'Premium'
  );
  results.premiumLink = describe(premium);
  results.premiumParent = describe(premium ? premium.parentElement : null);

  // The account / signup cluster in the header.
  const signup = Array.from(document.querySelectorAll('button,a')).find(
    (el) => (el.textContent || '').trim() === 'Sign up'
  );
  results.signup = describe(signup);

  // About Ads link.
  const aboutAds = Array.from(document.querySelectorAll('a')).find(
    (el) => (el.textContent || '').trim() === 'About Ads'
  );
  results.aboutAds = describe(aboutAds);

  // Header landmark.
  results.header = describe(document.querySelector('header, [role=banner], nav'));

  // Any element whose text mentions advertising.
  results.adTexts = Array.from(document.querySelectorAll('*'))
    .filter((el) => el.children.length === 0 && /(sponsored|advertisement|\bAd\b)/i.test((el.textContent || '').trim()))
    .slice(0, 8)
    .map((el) => describe(el));

  return JSON.stringify(results, null, 2);
})()`;

const out = await send("Runtime.evaluate", { expression, returnByValue: true });
console.log(out.result.value);
ws.close();
