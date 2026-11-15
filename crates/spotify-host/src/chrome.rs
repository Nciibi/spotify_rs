//! Styling for the embedded Spotify client.
//!
//! The page is the real web player, so the goal is to make it feel like a
//! native desktop application: the marketing chrome of the website is hidden,
//! rounded corners reveal the system Mica backdrop, and the scrollbars and
//! separators match the rest of the application.
//!
//! The selectors come from the live page, see `re/probe_page.mjs`.

/// The stylesheet that turns the web player into a desktop window.
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

/* The bottom player bar floats above the backdrop. */
footer, [data-testid="now-playing-widget"] {
  backdrop-filter: blur(18px);
}

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

/* The header's marketing buttons. */
[data-testid="global-nav-bar"] button:text-is("Premium"),
[data-testid="global-nav-bar"] button:text-is("Support"),
[data-testid="global-nav-bar"] button:text-is("Download"),
[data-testid="global-nav-bar"] a:text-is("Install App") { display: none !important; }

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
  const HIDE_TEXT = ['Premium', 'Support', 'Download', 'Install App', 'Sign up'];
  const PROMO_TEXT = ['Create your first playlist', "Let's find some podcasts to follow"];
  const HIDE_SELECTORS = [
    'a.e-101111-skip-link',
    '[data-testid="signup-button"]',
    '[data-testid="left-sidebar-legal-links"]',
    '[data-testid="language-selection-button"]',
    '[data-testid="signup-bar"]',
    '[data-testid="footer-div"]',
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
}
