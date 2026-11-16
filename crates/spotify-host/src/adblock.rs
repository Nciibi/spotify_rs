//! Ad and telemetry blocking.
//!
//! Three layers, all of them local:
//!
//! * a **native** layer that blackholes whole hosts through WebView2's
//!   `--host-resolver-rules`, so the request never leaves the machine;
//! * a **network** layer inside the page that filters `fetch`,
//!   `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon` and image
//!   sources by host and by path;
//! * a **cosmetic** layer that hides the ad containers the web player
//!   renders, so no slot is left behind.
//!
//! The path patterns come from the endpoints the official client actually
//! calls, recovered from its own cache (see `re/out/api-surface.txt`):
//! `spclient.wg.spotify.com/ads/v2/config`,
//! `spclient.wg.spotify.com/sponsoredplaylist/v1/sponsored` and the
//! `gander` notification service. Strings in `re/out/spotify-strings.txt`
//! gave the ad-event and audio-segment telemetry schemas.

/// Hosts that only ever serve advertising or telemetry.
pub const BLOCKED_HOSTS: &[&str] = &[
    // Spotify's own ad and tracking hosts.
    "ads.service.scdn.co",
    "ads.i.scdn.co",
    "ad.scdn.co",
    "pixel.spotify.com",
    // Google ad networks.
    "ade.googlesyndication.com",
    "pagead2.googlesyndication.com",
    "tpc.googlesyndication.com",
    "googleads.g.doubleclick.net",
    "stats.g.doubleclick.net",
    "securepubads.g.doubleclick.net",
    "ad.doubleclick.net",
    "www.googleadservices.com",
    "googleadservices.com",
    "adservice.google.com",
    "www.googletagmanager.com",
    "www.google-analytics.com",
    "ssl.google-analytics.com",
    // Other ad networks, trackers and telemetry.
    "abetter.bassilive.com",
    "o2.ak.adsystem.com",
    "match.adsrvr.org",
    "px.adsrvr.org",
    "cdn.taboola.com",
    "trc.taboola.com",
    "static.criteo.net",
    "ib.adnxs.com",
    "sb.scorecardresearch.com",
    "www.quantserve.com",
    "sentry.io",
    "js.sentry-cdn.com",
];

/// Path prefixes that carry advertising or sponsored content.
pub const BLOCKED_PATHS: &[&str] = &[
    "/ads/",
    "/ads/v2/",
    "/ad/",
    "/ad-event/",
    "/adevent/",
    "/sponsoredplaylist/",
    "/sponsored/",
    "/gander/",
    "/einkompaniet/",
    "/flexads/",
    "/audio-ads/",
];

/// URL suffixes for advertising endpoints.
pub const BLOCKED_SUFFIXES: &[&str] = &[
    "/ads/v2/config",
    "/sponsoredplaylist/v1/sponsored",
    "/GetUserHasUnreadNotification",
];

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

/// The stylesheet that hides the ad slots the web player renders.
pub fn injected_css() -> String {
    r#"
/* Advertising slots. */
[data-testid="ad-slot"],
[data-testid="ad-unit"],
[data-testid="ad-break"],
[data-testid="advertisement"],
[data-testid="banner-ad"],
[data-testid="sponsored-playlist"],
[data-testid="sponsored-content"],
[data-testid="ad-container"],
.advertisement,
.ad-container,
[class^="ad-container"],
[class*="AdContainer"],
[class*="AdSlot"],
[class*="BannerAd"],
[class*="SponsoredContent"],
[class*="ads-ui"],
#ads-container {
  display: none !important;
}

/* The player marks sponsored rows with a "Sponsored" label. */
li:has(> div > div > span:text-is("Sponsored")),
[role="row"]:has(span:text-is("Sponsored")),
[role="row"]:has(span:text-is("Ad")) {
  display: none !important;
}

/* The "Ad break" countdown / "Advertisement" badge in the player bar. */
[class*="AdBreak"],
[class*="ad-break"],
[data-testid="ad-break-label"] {
  display: none !important;
}
"#
    .to_string()
}

/// The script that removes any ad element that slipped past the stylesheet.
///
/// Returns an expression that evaluates to the sweep function.
pub fn cosmetic_script() -> String {
    r#"
(() => {
  const SELECTORS = [
    '[data-testid="ad-slot"]',
    '[data-testid="ad-unit"]',
    '[data-testid="ad-break"]',
    '[data-testid="advertisement"]',
    '[data-testid="banner-ad"]',
    '[data-testid="sponsored-playlist"]',
    '[data-testid="sponsored-content"]',
    '[data-testid="ad-container"]',
    '.advertisement',
    '.ad-container',
    '[class*="AdContainer"]',
    '[class*="AdSlot"]',
    '[class*="SponsoredContent"]',
    '#ads-container'
  ];
  const LABELS = ['Sponsored', 'Ad', 'Advertisement'];

  const sweep = () => {
    for (const selector of SELECTORS) {
      for (const node of document.querySelectorAll(selector)) node.remove();
    }
    // Rows whose visible label is exactly "Sponsored" or "Ad".
    for (const label of document.querySelectorAll('span, div')) {
      if (label.children.length !== 0) continue;
      const text = (label.textContent || '').trim();
      if (!LABELS.includes(text)) continue;
      const row = label.closest('li, [data-testid="context-item-link"], [role="row"]');
      if (row) row.remove();
      else label.remove();
    }
  };

  return sweep;
})()
"#
    .to_string()
}

/// The script injected before any page script runs.
///
/// It installs the network hooks, the cosmetic stylesheet and the sweep that
/// removes ad elements the stylesheet could not hide. It also keeps a public
/// counter at `window.__spotifyRsBlocked` so a scenario can assert that ads
/// were actually turned away.
pub fn injected_script() -> String {
    let hosts = serde_json::to_string(BLOCKED_HOSTS).unwrap_or_else(|_| "[]".into());
    let paths = serde_json::to_string(BLOCKED_PATHS).unwrap_or_else(|_| "[]".into());
    let suffixes = serde_json::to_string(BLOCKED_SUFFIXES).unwrap_or_else(|_| "[]".into());
    let css = serde_json::to_string(&injected_css()).unwrap_or_else(|_| "\"\"".into());
    let sweep = cosmetic_script();

    format!(
        r#"
(() => {{
  const HOSTS = new Set({hosts});
  const PATHS = {paths};
  const SUFFIXES = {suffixes};

  // Cosmetic layer: stylesheet plus a MutationObserver-driven sweep.
  const style = document.createElement('style');
  style.id = 'sr-adblock-style';
  style.textContent = {css};
  (document.head || document.documentElement).appendChild(style);

  const sweep = {sweep};
  const sweepNow = () => {{ try {{ sweep(); }} catch (e) {{}} }};
  sweepNow();
  if (document.readyState !== 'loading') sweepNow();
  else document.addEventListener('DOMContentLoaded', sweepNow);
  new MutationObserver(sweepNow).observe(document.documentElement || document, {{
    childList: true, subtree: true
  }});

  const blocked = (raw) => {{
    if (!raw) return false;
    let url;
    try {{ url = new URL(raw, location.href); }} catch (e) {{ return false; }}
    if (HOSTS.has(url.hostname)) return true;
    for (const p of PATHS) if (url.pathname.startsWith(p)) return true;
    for (const s of SUFFIXES) if (url.pathname.endsWith(s)) return true;
    return false;
  }};

  const count = () => {{ window.__spotifyRsBlocked = (window.__spotifyRsBlocked || 0) + 1; }};
  window.__spotifyRsBlocked = 0;

  // fetch
  const realFetch = window.fetch;
  window.fetch = function (input, init) {{
    const url = typeof input === 'string' ? input : (input && (input.url || input.href || String(input)));
    if (blocked(url)) {{
      count();
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
      count();
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
      count();
      return new BlockedSocket();
    }}
    return protocols === undefined ? new RealWebSocket(url) : new RealWebSocket(url, protocols);
  }};
  window.WebSocket.prototype = RealWebSocket.prototype;

  // EventSource
  if (window.EventSource) {{
    const RealEventSource = window.EventSource;
    function BlockedSource() {{
      this.readyState = 2;
      this.close = () => {{}};
      this.addEventListener = () => {{}};
      this.removeEventListener = () => {{}};
    }}
    BlockedSource.prototype = RealEventSource.prototype;
    window.EventSource = function (url, config) {{
      if (blocked(url)) {{
        count();
        return new BlockedSource();
      }}
      return config === undefined ? new RealEventSource(url) : new RealEventSource(url, config);
    }};
    window.EventSource.prototype = RealEventSource.prototype;
  }}

  // beacons
  if (navigator.sendBeacon) {{
    const realBeacon = navigator.sendBeacon.bind(navigator);
    navigator.sendBeacon = function (url, data) {{
      if (blocked(url)) {{
        count();
        return false;
      }}
      return realBeacon(url, data);
    }};
  }}

  // Ad imagery. The `src` property setter has to be patched as well, because
  // assigning to it does not go through `setAttribute`.
  const realSetAttribute = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function (name, value) {{
    if ((name === 'src' || name === 'data-src' || name === 'srcset') && blocked(value)) {{
      count();
      return;
    }}
    return realSetAttribute.call(this, name, value);
  }};
  try {{
    const descriptor = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src');
    if (descriptor && descriptor.set) {{
      Object.defineProperty(HTMLImageElement.prototype, 'src', {{
        get: descriptor.get,
        set: function (value) {{
          if (blocked(value)) {{
            count();
            return;
          }}
          descriptor.set.call(this, value);
        }},
        configurable: descriptor.configurable,
        enumerable: descriptor.enumerable
      }});
    }}
  }} catch (e) {{}}
}})();
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_rules_cover_every_blocked_host() {
        let rules = host_resolver_rules();
        for host in BLOCKED_HOSTS {
            assert!(rules.contains(&format!("MAP {host} 0.0.0.0")), "missing {host}");
        }
    }

    #[test]
    fn rules_never_map_the_api_host() {
        assert!(!host_resolver_rules().contains("spclient.wg.spotify.com,"));
        assert!(!host_resolver_rules().contains("MAP open.spotify.com "));
    }

    #[test]
    fn script_carries_every_rule_set() {
        let script = injected_script();
        assert!(script.contains("ads.service.scdn.co"));
        assert!(script.contains("/sponsoredplaylist/"));
        assert!(script.contains("/ads/v2/"));
        assert!(script.contains("__spotifyRsBlocked"));
    }

    #[test]
    fn bootstrap_includes_cosmetic_layer() {
        let script = injected_script();
        assert!(script.contains("sr-adblock-style"));
        assert!(script.contains("MutationObserver"));
        assert!(script.contains("display: none"));
    }

    #[test]
    fn covers_recovered_endpoints() {
        assert!(BLOCKED_PATHS.contains(&"/ads/v2/"));
        assert!(BLOCKED_PATHS.contains(&"/sponsoredplaylist/"));
        assert!(BLOCKED_SUFFIXES.contains(&"/GetUserHasUnreadNotification"));
    }
}
