//! Ad and telemetry blocking.
//!
//! Two layers:
//!
//! * a native layer that blackholes whole hosts through WebView2's
//!   `--host-resolver-rules`, so the request never leaves the machine;
//! * a page layer that filters `fetch`, `XMLHttpRequest`, `WebSocket` and
//!   `sendBeacon` by path, and hides the ad containers that the web player
//!   renders.
//!
//! The path patterns come from the endpoints the official client actually
//! calls (recovered from its own cache, see `re/out/api-surface.txt`).

/// Hosts that only ever serve advertising or telemetry.
pub const BLOCKED_HOSTS: &[&str] = &[
    "ads.service.scdn.co",
    "ade.googlesyndication.com",
    "pagead2.googlesyndication.com",
    "www.googleadservices.com",
    "adservice.google.com",
    "ad.doubleclick.net",
    "stats.g.doubleclick.net",
    "googleads.g.doubleclick.net",
    "pixel.spotify.com",
    "abetter.bassilive.com",
    "ads.i.scdn.co",
    "sentry.io",
    "o2.ak.adsystem.com",
    "match.adsrvr.org",
    "px.adsrvr.org",
];

/// Path prefixes that are advertising or sponsored content.
pub const BLOCKED_PATHS: &[&str] = &[
    "/ads/v2/",
    "/sponsoredplaylist/",
    "/ads/",
    "/ad/",
    "/gander/",
    "/einkompaniet/",
];

/// URL suffixes for advertising endpoints.
pub const BLOCKED_SUFFIXES: &[&str] = &["/ads/v2/config", "/sponsoredplaylist/v1/sponsored"];

/// Build the `--host-resolver-rules` value for the WebView2 browser process.
///
/// Mapping a host to `0.0.0.0` makes every lookup for it fail immediately,
/// so no connection is ever attempted.
pub fn host_resolver_rules() -> String {
    let mut rules = Vec::with_capacity(BLOCKED_HOSTS.len());
    for host in BLOCKED_HOSTS {
        rules.push(format!("MAP {host} 0.0.0.0"));
    }
    rules.join(", ")
}

/// The script injected before any page script runs.
pub fn injected_script() -> String {
    let hosts = serde_json::to_string(BLOCKED_HOSTS).unwrap_or_else(|_| "[]".into());
    let paths = serde_json::to_string(BLOCKED_PATHS).unwrap_or_else(|_| "[]".into());
    let suffixes = serde_json::to_string(BLOCKED_SUFFIXES).unwrap_or_else(|_| "[]".into());

    format!(
        r#"
(() => {{
  const HOSTS = new Set({hosts});
  const PATHS = {paths};
  const SUFFIXES = {suffixes};
  const blocked = (raw) => {{
    if (!raw) return false;
    let url;
    try {{ url = new URL(raw, location.href); }} catch (e) {{ return false; }}
    if (HOSTS.has(url.hostname)) return true;
    for (const p of PATHS) if (url.pathname.startsWith(p)) return true;
    for (const s of SUFFIXES) if (url.pathname.endsWith(s)) return true;
    return false;
  }};

  // fetch
  const realFetch = window.fetch;
  window.fetch = function (input, init) {{
    const url = typeof input === 'string' ? input : (input && input.url);
    if (blocked(url)) {{
      window.__spotifyRsBlocked = (window.__spotifyRsBlocked || 0) + 1;
      return Promise.resolve(new Response(null, {{ status: 204, statusText: 'Blocked by spotify-rs' }}));
    }}
    return realFetch.apply(this, arguments);
  }};

  // XMLHttpRequest
  const realOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url) {{
    if (blocked(url)) {{
      this.__spotifyRsBlocked = true;
      return;
    }}
    this.__spotifyRsBlocked = false;
    return realOpen.apply(this, arguments);
  }};
  const realSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function () {{
    if (this.__spotifyRsBlocked) {{
      try {{ Object.defineProperty(this, 'readyState', {{ value: 4, configurable: true }}); }} catch (e) {{}}
      try {{ this.status = 204; }} catch (e) {{}}
      this.dispatchEvent(new Event('readystatechange'));
      this.dispatchEvent(new Event('load'));
      this.dispatchEvent(new Event('loadend'));
      return;
    }}
    return realSend.apply(this, arguments);
  }};

  // WebSocket
  const RealWebSocket = window.WebSocket;
  function BlockedSocket() {{
    this.readyState = 3;
    this.send = () => {{}};
    this.close = () => {{}};
    this.addEventListener = () => {{}};
    this.removeEventListener = () => {{}};
    setTimeout(() => {{ try {{ this.onclose && this.onclose({{ code: 1006 }}); }} catch (e) {{}} }}, 0);
  }}
  BlockedSocket.prototype = RealWebSocket.prototype;
  window.WebSocket = function (url, protocols) {{
    if (blocked(url)) {{
      window.__spotifyRsBlocked = (window.__spotifyRsBlocked || 0) + 1;
      return new BlockedSocket();
    }}
    return protocols === undefined ? new RealWebSocket(url) : new RealWebSocket(url, protocols);
  }};
  window.WebSocket.prototype = RealWebSocket.prototype;

  // beacons
  if (navigator.sendBeacon) {{
    const realBeacon = navigator.sendBeacon.bind(navigator);
    navigator.sendBeacon = function (url, data) {{
      if (blocked(url)) return false;
      return realBeacon(url, data);
    }};
  }}

  // Ad imagery and iframes
  const realSetAttribute = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function (name, value) {{
    if ((name === 'src' || name === 'data-src') && blocked(value)) {{
      window.__spotifyRsBlocked = (window.__spotifyRsBlocked || 0) + 1;
      return;
    }}
    return realSetAttribute.call(this, name, value);
  }};
}})();
"#
    )
}

/// Stylesheet that hides the ad slots the web player renders.
pub fn injected_css() -> String {
    r#"
/* Hide advertising slots. */
[data-testid="ad-slot"],
[data-testid="ad-unit"],
[data-testid="ad-break"],
[data-testid="advertisement"],
[data-testid="sponsored-playlist"],
[data-testid="banner-ad"],
.advertisement,
.ad-container,
[class^="ad-container"],
[class*="AdContainer"],
[class*="AdSlot"],
[class*="BannerAd"] {
  display: none !important;
}

/* The player marks sponsored rows with a "Sponsored" label. */
[data-testid="context-item-link"]:has(> *:only-child [data-encore-id="textAdditiveStrong"]),
li:has(> div > div > span:text-is("Sponsored")) {
  display: none !important;
}
"#
    .to_string()
}

/// Script that removes any ad element that slipped past the CSS.
pub fn cosmetic_script() -> String {
    r#"
(() => {
  const SELECTORS = [
    '[data-testid="ad-slot"]',
    '[data-testid="ad-unit"]',
    '[data-testid="ad-break"]',
    '[data-testid="advertisement"]',
    '[data-testid="banner-ad"]',
    '.ad-container',
  ];
  const sweep = () => {
    for (const selector of SELECTORS) {
      for (const node of document.querySelectorAll(selector)) node.remove();
    }
    // Rows whose visible label is exactly "Sponsored" or "Ad".
    for (const label of document.querySelectorAll('span, div')) {
      if (label.children.length === 0) {
        const text = (label.textContent || '').trim();
        if (text === 'Sponsored' || text === 'Ad') {
          const row = label.closest('li, [data-testid="context-item-link"], [role="row"]');
          if (row) row.remove();
          else label.remove();
        }
      }
    }
  };
  sweep();
  try { new MutationObserver(sweep).observe(document, { childList: true, subtree: true }); } catch (e) {}
})();
"#
    .to_string()
}
