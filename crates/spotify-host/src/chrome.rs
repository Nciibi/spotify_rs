//! Styling for the embedded Spotify client.
//!
//! The page is the real web player, so the goal is to make it feel like a
//! native desktop application — and then a little better than the stock one:
//!
//! * the website-only chrome (sign up, premium upsells, legal links, cookie
//!   wall) is pruned, because a paid desktop client never shows it;
//! * an ambient wash behind the interface is tinted from the current cover
//!   art, the way the native client's " colourful background" does, something
//!   the plain web player has no access to;
//! * rounded corners reveal the system Mica backdrop and the scrollbars,
//!   separators and focus rings are tuned to match the rest of the shell.
//!
//! The selectors come from the live page, see `re/probe_page.mjs`.

/// The stylesheet that turns the web player into a desktop window.
pub fn injected_css() -> String {
    r#"
/* Let the window's Mica backdrop show through the rounded corners. */
html, body {
  background: transparent !important;
  overscroll-behavior: none;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}

/* Ambient wash tinted from the current cover art (`--sr-accent-rgb`). */
#sr-ambient {
  position: fixed;
  top: 0; left: 0; right: 0;
  height: 46vh;
  z-index: 0;
  pointer-events: none;
  background:
    radial-gradient(120% 100% at 12% -10%,
      rgba(var(--sr-accent-rgb, 30, 215, 96), 0.42), transparent 70%),
    linear-gradient(180deg,
      rgba(var(--sr-accent-rgb, 30, 215, 96), 0.22), transparent);
  opacity: 0;
  transition: background 700ms ease, opacity 700ms ease;
}
html[data-sr-accent="1"] #sr-ambient { opacity: 1; }

/* The main application surface floats above the wash. */
#main-view,
main,
.main-view-container {
  position: relative;
  z-index: 1;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 20px 70px rgba(0, 0, 0, 0.6),
              0 0 0 1px rgba(255, 255, 255, 0.04);
}

/* Compact, unobtrusive scrollbars. */
* {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.22) transparent;
}
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.18);
  border-radius: 8px;
  border: 3px solid transparent;
  background-clip: content-box;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.34);
  background-clip: content-box;
}

/* Softer separators. */
hr, [class*="Separator"], [class*="separator"] {
  border-color: rgba(255, 255, 255, 0.08) !important;
}

/* Rounded artwork and cards with a gentle, consistent hover. */
img, [data-testid="card-image"] { border-radius: 8px; }
[class*="CardWrapper"],
[data-testid="card-click-handler"],
[class*="Tile"] {
  border-radius: 10px;
  transition: background-color 160ms ease, transform 160ms ease;
}
[class*="CardWrapper"]:hover,
[data-testid="card-click-handler"]:hover { transform: translateY(-2px); }

/* A slightly rounder, floating now-playing bar. */
footer, [data-testid="now-playing-widget"], [data-testid="player-controls"] {
  backdrop-filter: blur(22px) saturate(140%);
  -webkit-backdrop-filter: blur(22px) saturate(140%);
}

/* Keyboard focus that matches the rest of the application. */
:focus-visible {
  outline: 2px solid rgb(var(--sr-accent-rgb, 30, 215, 96)) !important;
  outline-offset: 2px !important;
}
:focus:not(:focus-visible) { outline: none !important; }

/* Elements the host marked as unwanted. */
[data-spotify-rs-hide] { display: none !important; }

/* ---- Website-only chrome ------------------------------------------------ */

/* The "Skip to main content" link is a web accessibility affordance. */
a.e-101111-skip-link { display: none !important; }

/* "Sign up" next to "Log in": a desktop client only signs in. */
[data-testid="signup-button"] { display: none !important; }

/* The sidebar footer carries the legal links and the language picker. */
[data-testid="left-sidebar-legal-links"],
[data-testid="language-selection-button"] { display: none !important; }

/* The "Preview of Spotify" upsell above the player bar. */
[data-testid="signup-bar"] { display: none !important; }

/* The marketing footer at the bottom of every page. */
[data-testid="footer-div"],
.main-view-container__mh-footer-container { display: none !important; }

/* Free-tier upgrade cards nagging in the sidebar and on the home page. */
[data-testid="upgrade-premium-button"],
[data-testid="premium-card"],
[class*="PremiumUpsell"],
[class*="TrialBanner"] { display: none !important; }

/* The header's marketing buttons. The stable `href` handles "Install App";
   `:has-text()` is used instead of `:text-is()` because Chromium ships the
   former but not the latter. The JS prune is the primary mechanism and these
   rules are the no-flash fallback. */
a[href="/download"] { display: none !important; }
[data-testid="global-nav-bar"] button:has-text("Premium"),
[data-testid="global-nav-bar"] button:has-text("Support"),
[data-testid="global-nav-bar"] button:has-text("Download"),
[data-testid="global-nav-bar"] a:has-text("Install App") { display: none !important; }

/* The cookie consent wall. */
#onetrust-consent-sdk,
#ot-sdk-container,
.onetrust-pc-dark-filter,
[id^="ot-sdk-"] { display: none !important; }

/* "About Ads" has no place in an ad blocking client. */
[data-testid="left-sidebar-legal-links"] li:has(a:text-is("About Ads")),
[data-testid="left-sidebar-legal-links"] li:has(a:text-is("Cookies")) { display: none !important; }

/* The desktop client does not have a "back to website" breadcrumb. */
[data-testid="web-player-banner"] { display: none !important; }

/* Keep reduced-motion users comfortable. */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
"#
    .to_string()
}

/// The script that removes the parts of the web interface a desktop client
/// does not need.
///
/// React re-creates the marketing nodes as soon as they leave the tree, so
/// they are marked with an attribute and hidden with CSS instead of being
/// removed.
///
/// Returns an expression that evaluates to the pruning function.
pub fn prune_script() -> String {
    r#"
(() => {
  const HIDE_TEXT = ['Premium', 'Support', 'Download', 'Install App', 'Sign up', 'Explore Premium'];
  const PROMO_TEXT = [
    'Create your first playlist',
    "Let's find some podcasts to follow",
    'Upgrade to Premium',
    'Get Premium'
  ];
  const HIDE_SELECTORS = [
    'a.e-101111-skip-link',
    '[data-testid="signup-button"]',
    '[data-testid="left-sidebar-legal-links"]',
    '[data-testid="language-selection-button"]',
    '[data-testid="signup-bar"]',
    '[data-testid="footer-div"]',
    '[data-testid="upgrade-premium-button"]',
    '[data-testid="premium-card"]',
    '[data-testid="web-player-banner"]',
    '.main-view-container__mh-footer-container'
  ];

  const hide = (node) => {
    if (!node || node.getAttribute('data-spotify-rs-hide') === '1') return;
    node.setAttribute('data-spotify-rs-hide', '1');
  };

  return function prune() {
    for (const selector of HIDE_SELECTORS) {
      for (const node of document.querySelectorAll(selector)) hide(node);
    }

    // The header's marketing buttons.
    const nav = document.querySelector('[data-testid="global-nav-bar"]');
    if (nav) {
      for (const node of nav.querySelectorAll('button, a')) {
        const label = (node.textContent || '').trim();
        if (!HIDE_TEXT.includes(label)) continue;
        // "Log in" is how a new user signs in; it is never hidden.
        if (node.getAttribute('data-testid') === 'login-button') continue;
        hide(node);
        const holder = node.parentElement;
        if (holder && holder.children.length === 1) hide(holder);
      }
    }

    // The promotional cards in the sidebar.
    for (const needle of PROMO_TEXT) {
      for (const leaf of document.querySelectorAll('span')) {
        if (leaf.children.length !== 0) continue;
        if ((leaf.textContent || '').trim() !== needle) continue;
        const card = leaf.closest('section') || leaf.closest('div');
        if (card) hide(card);
      }
    }
  };
})()
"#
    .to_string()
}

/// Sample the dominant colour of the current cover art.
///
/// The native client tints the area behind the interface with the palette of
/// the playing album; the web player does not. A hidden, CORS-enabled clone
/// of the artwork is drawn to a tiny canvas and its average non-grey colour
/// is published as `--sr-accent-rgb` for the stylesheet above. If the CDN
/// refuses the cross-origin read, the fallback Spotify green is left in
/// place, so nothing breaks.
pub fn accent_script() -> String {
    r#"
(() => {
  const FALLBACK = '30, 215, 96';
  const SELECTORS = [
    '[data-testid="now-playing-widget"] img',
    '[data-testid="now-playing-cover"] img',
    '.now-playing img',
    'footer img'
  ];
  let lastSrc = '';

  const cover = () => {
    for (const s of SELECTORS) {
      const node = document.querySelector(s);
      if (node && node.src) return node.src;
    }
    return '';
  };

  const sample = (src) => {
    const probe = new Image();
    probe.crossOrigin = 'anonymous';
    probe.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = canvas.height = 16;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(probe, 0, 0, 16, 16);
        const { data } = ctx.getImageData(0, 0, 16, 16);
        let r = 0, g = 0, b = 0, weight = 0;
        for (let i = 0; i < data.length; i += 4) {
          const cr = data[i], cg = data[i + 1], cb = data[i + 2];
          const max = Math.max(cr, cg, cb), min = Math.min(cr, cg, cb);
          // Ignore near-black and near-white pixels — they carry no tint.
          if (max < 24 || min > 232 || max - min < 12) continue;
          r += cr; g += cg; b += cb; weight++;
        }
        if (!weight) return;
        const rgb = [r / weight, g / weight, b / weight]
          .map((v) => Math.max(0, Math.min(255, Math.round(v))));
        document.documentElement.style.setProperty('--sr-accent-rgb', rgb.join(', '));
        document.documentElement.setAttribute('data-sr-accent', '1');
      } catch (e) { /* tainted canvas: keep the fallback */ }
    };
    probe.src = src;
  };

  return function accent() {
    const src = cover();
    if (!src || src === lastSrc) return;
    lastSrc = src;
    sample(src);
  };
})()
"#
    .to_string()
}

/// Styling for the sign-in surface, derived from the real client's `login.spa`
/// (`re/target/login/index.html` and `login.css`).
///
/// The web login at `accounts.spotify.com` is functional but uses the website's
/// typography and a flat background. The desktop client's login uses the
/// SpotifyMixUI/Circular stack, a softly lit dark card and a green accent. We
/// cannot ship the bundled fonts cross-origin, but the stack below uses them
/// wherever the system already has them and falls back cleanly.
pub fn login_css() -> String {
    r#"
/* Match the desktop login's typography. */
html, body, input, button {
  font-family: 'SpotifyMixUI', 'CircularSp', 'Circular',
               -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
  -webkit-font-smoothing: antialiased;
}

/* The desktop login floats a card over a dark, green-lit backdrop. */
body {
  background:
    radial-gradient(90% 60% at 50% -10%, rgba(30, 215, 96, 0.16), transparent 70%),
    #000 !important;
}
section {
  border-radius: 16px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6),
              0 0 0 1px rgba(255, 255, 255, 0.05);
  padding: 32px !important;
}
h1 { letter-spacing: -0.04em; }

/* Rounded, roomy form controls like the desktop client's. */
input[data-testid="login-username"],
input[id="username"],
button[type="submit"] {
  border-radius: 8px !important;
  min-height: 48px !important;
}
button[data-testid="login-button"] {
  border-radius: 999px !important;
}

/* Focus rings in the app's green. */
:focus-visible { outline: 2px solid #1ed760 !important; outline-offset: 2px !important; }
"#
    .to_string()
}

/// Build the single initialization script the host injects before the page.
///
/// It adds the stylesheet, runs the prune and accent passes immediately,
/// re-runs them whenever React updates the tree, and keeps a slow poll as a
/// safety net for mutations that do not trigger the observer.
pub fn injected_script() -> String {
    let css = serde_json::to_string(&injected_css()).unwrap_or_else(|_| "\"\"".into());
    let login_css = serde_json::to_string(&login_css()).unwrap_or_else(|_| "\"\"".into());
    let prune = prune_script();
    let accent = accent_script();

    format!(
        r#"
(() => {{
  // Initialization scripts may run before <head>/<body> exist. Defer the DOM
  // work instead of throwing, so the prune and accent passes below always
  // get installed.
  const addStyle = () => {{
    const host = document.head || document.documentElement;
    if (!host) {{ document.addEventListener('DOMContentLoaded', addStyle, {{ once: true }}); return; }}
    if (!document.getElementById('sr-style')) {{
      const style = document.createElement('style');
      style.id = 'sr-style';
      style.textContent = {css};
      host.appendChild(style);
    }}
    // The sign-in flow lives on accounts.spotify.com. Give it the desktop
    // login's typography and card treatment (from the real login.spa).
    if (location.hostname === 'accounts.spotify.com' &&
        !document.getElementById('sr-login-style')) {{
      const login = document.createElement('style');
      login.id = 'sr-login-style';
      login.textContent = {login_css};
      host.appendChild(login);
    }}
  }};
  const addAmbient = () => {{
    if (document.getElementById('sr-ambient')) return;
    if (!document.body) {{ document.addEventListener('DOMContentLoaded', addAmbient, {{ once: true }}); return; }}
    const ambient = document.createElement('div');
    ambient.id = 'sr-ambient';
    document.body.appendChild(ambient);
  }};
  addStyle();
  addAmbient();

  const prune = {prune};
  const accent = {accent};

  const run = () => {{ prune(); accent(); }};
  run();
  if (document.readyState !== 'loading') run();
  else document.addEventListener('DOMContentLoaded', run);

  const observer = new MutationObserver(() => run());
  observer.observe(document.documentElement || document, {{
    childList: true, subtree: true, attributes: true,
    attributeFilter: ['src', 'class', 'data-testid']
  }});
  setInterval(run, 2000);
}})();
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_hides_the_website_chrome() {
        let css = injected_css();
        assert!(css.contains("[data-testid=\"signup-button\"]"));
        assert!(css.contains("[data-testid=\"left-sidebar-legal-links\"]"));
        assert!(css.contains("[data-testid=\"footer-div\"]"));
    }

    #[test]
    fn prune_keeps_the_login_button() {
        assert!(prune_script().contains("login-button"));
    }

    #[test]
    fn bootstrap_wires_every_layer() {
        let script = injected_script();
        assert!(script.contains("sr-style"));
        assert!(script.contains("sr-ambient"));
        assert!(script.contains("MutationObserver"));
        assert!(script.contains("setInterval"));
        assert!(script.contains("--sr-accent-rgb"));
    }

    #[test]
    fn ambient_css_reads_the_accent_variable() {
        assert!(injected_css().contains("--sr-accent-rgb"));
    }
}
