//! Helpers for building the JavaScript the host injects into the page.
//!
//! Everything the host adds to the page is injected *before* the page's own
//! scripts run, so the interface never flashes in its unstyled state and the
//! network hooks are in place before the first request goes out.

/// Encode a Rust string so it can be embedded in a script as a literal.
pub fn json(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

/// Build a script that injects a stylesheet as early as the DOM allows.
///
/// The style element is attached to `document.head` when it exists and to
/// `document.documentElement` otherwise. At document-start neither exists
/// yet, so a mutation observer on the document picks the root element up as
/// soon as the parser creates it, which is still before the first paint.
pub fn style_script(id: &str, css: &str) -> String {
    format!(
        r#"
(() => {{
  const id = {id};
  if (document.getElementById(id)) return;
  const style = document.createElement('style');
  style.id = id;
  style.textContent = {css};
  const attach = () => {{
    const root = document.head || document.documentElement;
    if (!root) return false;
    root.appendChild(style);
    return true;
  }};
  if (!attach()) {{
    const observer = new MutationObserver(() => {{
      if (attach()) observer.disconnect();
    }});
    observer.observe(document, {{ childList: true, subtree: true }});
  }}
}})();
"#,
        id = json(id),
        css = json(css)
    )
}

/// Build a script that runs `body` as soon as the document is parsed.
///
/// `body` must be a JavaScript expression that evaluates to a function.
pub fn on_ready(body: &str) -> String {
    format!(
        r#"
(() => {{
  const body = {body};
  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', body, {{ once: true }});
  }} else {{
    body();
  }}
}})();
"#,
        body = body
    )
}

/// Build a script that keeps `body` running against every DOM change.
///
/// `body` must be a JavaScript expression that evaluates to a function. The
/// observer is throttled so a chatty page cannot turn the sweep into a busy
/// loop.
pub fn watch(body: &str) -> String {
    format!(
        r#"
(() => {{
  const body = {body};
  let queued = false;
  const schedule = () => {{
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {{
      queued = false;
      try {{ body(); }} catch (e) {{}}
    }});
  }};
  try {{
    const observer = new MutationObserver(schedule);
    observer.observe(document, {{ childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'style', 'hidden'] }});
  }} catch (e) {{}}
  schedule();
}})();
"#,
        body = body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_quotes() {
        assert_eq!(json("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn style_script_mentions_the_id_and_the_css() {
        let script = style_script("spotify-rs-style", "body { color: red; }");
        assert!(script.contains("spotify-rs-style"));
        assert!(script.contains("color: red"));
    }
}
