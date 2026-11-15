//! Styling for the embedded Spotify client.
//!
//! The page is the real web player, so the goal is to make it feel like a
//! native desktop application: rounded corners that reveal the system Mica
//! backdrop, tighter spacing, proper scrollbars and a cleaner sidebar.

pub fn injected_css() -> String {
    r#"
/* Let the window's Mica backdrop show through the rounded corners. */
html, body {
  background: transparent !important;
  overscroll-behavior: none;
}

/* The main application surface. */
#main-view,
main {
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.55);
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
::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.32); background-clip: content-box; }

/* Softer separators. */
hr, [class*="Separator"] {
  border-color: rgba(255, 255, 255, 0.07) !important;
}

/* Rounded artwork and cards. */
img { border-radius: 6px; }

[class*="CardWrapper"],
[data-testid="card-click-handler"] {
  border-radius: 8px;
  transition: background-color 120ms ease;
}

/* Keyboard focus that matches the rest of the application. */
:focus-visible {
  outline: 2px solid #1ed760 !important;
  outline-offset: 2px !important;
}
:focus:not(:focus-visible) { outline: none !important; }

/* Slightly larger touch targets for the transport controls. */
button[aria-label*="Play"],
button[aria-label*="Pause"],
button[aria-label*="Next"],
button[aria-label*="Previous"] {
  transition: transform 90ms ease;
}
button[aria-label*="Play"]:active,
button[aria-label*="Pause"]:active {
  transform: scale(0.94);
}

/* The bottom player bar floats above the backdrop. */
footer, [data-testid="now-playing-widget"] {
  backdrop-filter: blur(18px);
}

/* Kill the "listen on another device" nudge that the web player shows. */
[data-testid="upgrade-button"],
button[data-testid="upgrade-button"] { display: none !important; }

/* Elements the host marked as unwanted. */
[data-spotify-rs-hide] { display: none !important; }

/* The sign-up promo bar along the bottom. */
[data-testid="signup-bar"] { display: none !important; }

/* The cookie consent wall. */
#onetrust-consent-sdk,
#ot-sdk-container,
.onetrust-pc-dark-filter,
[id^="ot-sdk-"] { display: none !important; }

/* "About Ads" has no place in an ad blocking client. */
[data-testid="left-sidebar-legal-links"] li:has(a:text-is("About Ads")),
[data-testid="left-sidebar-legal-links"] li:has(a:text-is("Cookies")) { display: none !important; }
"#
    .to_string()
}

/// Removes the parts of the web interface that a desktop client does not
/// need: the marketing links in the header and the signup prompts.
///
/// The elements are marked rather than removed, because React re-creates them
/// as soon as they leave the tree.
///
/// Returns an expression that evaluates to the pruning function.
pub fn prune_script() -> String {
    r#"
(() => {
  const DROP = ['Premium', 'Support', 'Download', 'Install App', 'Sign up'];

  return function prune() {
    const nav = document.querySelector('[data-testid="global-nav-bar"]');
    if (!nav) return;
    for (const label of DROP) {
      for (const node of nav.querySelectorAll('button, a')) {
        if ((node.textContent || '').trim() !== label) continue;
        // Keep the account button, it is how the user signs in.
        if (node.getAttribute('data-testid') === 'signup-button') continue;
        if (node.closest('[data-testid="account-button"]')) continue;
        if (node.getAttribute('data-spotify-rs-hide') === '1') continue;
        node.setAttribute('data-spotify-rs-hide', '1');
        const holder = node.parentElement;
        if (holder && holder.children.length === 1 && !holder.getAttribute('data-spotify-rs-hide')) {
          holder.setAttribute('data-spotify-rs-hide', '1');
        }
      }
    }
  };
})()
"#
    .to_string()
}

/// Script that reports page state back to the host and wires the shortcuts.
pub fn injected_script() -> String {
    let css = injected_css();
    let prune = prune_script();
    format!(
        r#"
(() => {{
  // The stylesheet has to be injected before the first paint.
  const addStyle = () => {{
    if (document.getElementById('spotify-rs-style')) return;
    const style = document.createElement('style');
    style.id = 'spotify-rs-style';
    style.textContent = {css};
    (document.head || document.documentElement).appendChild(style);
  }};
  if (document.head) addStyle();
  else document.addEventListener('DOMContentLoaded', addStyle);

  // Remove the marketing chrome that only makes sense on the website.
  const PRUNE = {prune};
  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', PRUNE);
  }} else {{
    PRUNE();
  }}
  try {{ new MutationObserver(PRUNE).observe(document, {{ childList: true, subtree: true }}); }} catch (e) {{}}

  // Report the visible track so the host can show a native notification.
  const describe = () => document.title || '';
  window.addEventListener('DOMContentLoaded', () => {{
    setInterval(() => {{
      const title = describe();
      if (window.__spotifyRsLastTitle !== title) {{
        window.__spotifyRsLastTitle = title;
        try {{ window.chrome && window.chrome.webview && window.chrome.webview.postMessage(JSON.stringify({{ type: 'title', value: title }})); }} catch (e) {{}}
      }}
    }}, 1000);
  }});
}})();
"#,
        css = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into()),
        prune = prune
    )
}
