//! The protocol between the embedded page and the host.
//!
//! The page is the real Spotify web player, so the host does not implement a
//! player of its own. Instead it takes control of the one the page already
//! has: the script installed by [`bridge_script`] wraps
//! `navigator.mediaSession.setActionHandler` and keeps the handlers the
//! player registers. The host can then drive the real player by calling them,
//! which is exactly what the hardware media keys have to do.

use serde::Deserialize;

/// A transport command the host sends to the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transport {
    Play,
    Pause,
    TogglePlay,
    Next,
    Previous,
    Stop,
    Seek(f64),
}

impl Transport {
    /// The Media Session action this command maps to.
    fn action(self) -> Option<&'static str> {
        match self {
            Transport::Play => Some("play"),
            Transport::Pause => Some("pause"),
            Transport::TogglePlay => None,
            Transport::Next => Some("nexttrack"),
            Transport::Previous => Some("previoustrack"),
            Transport::Stop => Some("stop"),
            Transport::Seek(_) => Some("seekto"),
        }
    }
}

/// What the page reports about the current track.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PageState {
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: String,
}

/// A message the page sends to the host.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PageMessage {
    /// The current track changed.
    State(PageState),
    /// The number of requests the ad blocker stopped.
    Blocked(u64),
    /// A diagnostic message.
    Log(String),
}

/// Parse a message posted by the page.
///
/// Anything the page posts that does not match the protocol is ignored, so a
/// change in the web client cannot break the host.
pub fn parse(body: &str) -> Option<PageMessage> {
    serde_json::from_str(body).ok()
}

/// The script that installs the bridge.
///
/// It captures the player's Media Session handlers, publishes them under
/// `window.__spotifyRs` and reports the current track to the host whenever it
/// changes.
pub fn bridge_script() -> String {
    r#"
(() => {
  if (window.__spotifyRs) return;

  const handlers = {};
  const media = navigator.mediaSession;

  if (media && media.setActionHandler) {
    const real = media.setActionHandler.bind(media);
    media.setActionHandler = function (action, handler) {
      if (typeof handler === 'function') handlers[action] = handler;
      try { return real(action, handler); } catch (e) { return undefined; }
    };
  }

  const post = (payload) => {
    try {
      if (window.chrome && window.chrome.webview) {
        window.chrome.webview.postMessage(JSON.stringify(payload));
      }
    } catch (e) {}
  };

  // The player may keep its state in the DOM only, so read it from there as
  // well. These are the test ids the web player uses for the now playing bar.
  const fromDom = () => {
    const widget = document.querySelector('[data-testid="now-playing-widget"]');
    if (!widget) return null;
    const text = (selector) => {
      const node = widget.querySelector(selector);
      return node ? (node.textContent || '').trim() : '';
    };
    const title = text('[data-testid="context-item-link"]');
    if (!title) return null;
    return {
      title,
      artist: text('[data-testid="context-item-info-artist"]'),
      album: text('[data-testid="context-item-info-album"]')
    };
  };

  const fromMedia = () => {
    if (!media || !media.metadata) return null;
    const meta = media.metadata;
    const artwork = meta.artwork && meta.artwork.length
      ? meta.artwork[meta.artwork.length - 1].src
      : '';
    return {
      title: meta.title || '',
      artist: meta.artist || '',
      album: meta.album || '',
      artwork
    };
  };

  let last = '';

  const report = () => {
    const mediaState = fromMedia();
    const domState = fromDom();
    const state = {
      playing: !!(media && media.playbackState === 'playing'),
      title: (mediaState && mediaState.title) || (domState && domState.title) || '',
      artist: (mediaState && mediaState.artist) || (domState && domState.artist) || '',
      album: (mediaState && mediaState.album) || (domState && domState.album) || '',
      artwork: (mediaState && mediaState.artwork) || ''
    };

    const key = [state.playing, state.title, state.artist, state.artwork].join('|');
    if (key === last) return;
    last = key;
    post({ type: 'state', ...state });
  };

  window.__spotifyRs = {
    dispatch: (action, value) => {
      const handler = handlers[action];
      if (!handler) return false;
      try {
        if (action === 'seekto') handler({ seekTime: value || 0, fastSeek: false });
        else handler({});
        return true;
      } catch (e) {
        return false;
      }
    },
    handlers: () => Object.keys(handlers),
    state: report
  };

  post({ type: 'log', message: 'bridge installed' });

  setInterval(report, 500);
})();
"#
    .to_string()
}

/// The script that drives the page's player.
///
/// A command is a no-op when the page has not registered a handler for it,
/// which is the case until the player is ready.
pub fn transport_script(command: Transport) -> String {
    let call = match command {
        Transport::TogglePlay => {
            r#"
(() => {
  const bridge = window.__spotifyRs;
  if (!bridge) return;
  const playing = !!(navigator.mediaSession && navigator.mediaSession.playbackState === 'playing');
  bridge.dispatch(playing ? 'pause' : 'play');
})();
"#
            .to_string()
        }
        Transport::Seek(seconds) => {
            format!(
                r#"
(() => {{
  const bridge = window.__spotifyRs;
  if (!bridge) return;
  bridge.dispatch('seekto', {seconds});
}})();
"#,
                seconds = seconds
            )
        }
        other => {
            let action = other.action().unwrap_or("play");
            format!(
                r#"
(() => {{
  const bridge = window.__spotifyRs;
  if (!bridge) return;
  bridge.dispatch('{action}');
}})();
"#,
                action = action
            )
        }
    };

    call
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_state_message() {
        let message = parse(r#"{"type":"state","playing":true,"title":"Song","artist":"Band"}"#);
        match message {
            Some(PageMessage::State(state)) => {
                assert!(state.playing);
                assert_eq!(state.title, "Song");
                assert_eq!(state.artist, "Band");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn ignores_foreign_messages() {
        assert!(parse("not json").is_none());
        assert!(parse(r#"{"type":"somethingElse"}"#).is_none());
    }

    #[test]
    fn transport_script_dispatches_the_action() {
        let script = transport_script(Transport::Next);
        assert!(script.contains("dispatch('nexttrack')"));
    }

    #[test]
    fn seek_carries_the_position() {
        let script = transport_script(Transport::Seek(42.5));
        assert!(script.contains("42.5"));
    }
}
