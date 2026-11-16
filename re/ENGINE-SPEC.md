# spotify-rs engine spec

Reverse-engineered from the Microsoft Store build of Spotify for Windows
(`SpotifyAB.SpotifyMusic_1.308.620.0_x64__zpdnekdrzrea0`).

## 1. What the real app actually is

The Store build is **not** Electron. It is:

| Component | Size | Role |
|---|---|---|
| `Spotify.exe` | 4 MB | launcher |
| `Spotify.dll` | 44 MB | the engine (C++), exports exactly one symbol: `RunWinMain` |
| `libcef.dll` | 285 MB | Chromium Embedded Framework — renders the UI |
| `Apps/xpui.spa` | 12 MB | the UI, a ZIP of 614 files (React SPA) |
| `Apps/login.spa` | 7 MB | the login UI |

`Spotify.dll` PE export table:

```
ordinal base 1, functions 1, names 1
  [1] 0x0005bdf8  RunWinMain
```

So the whole engine is behind one entry point. That is the seam we replace.

`xpui.spa` is a plain ZIP with a `/` root entry (zip-slip breaks naive
extractors). Contents are numbered chunks plus named bundles
(`xpui-routes-*.js`, `xpui-snapshot.css`, `dwp-*.js` = "desktop web player").

## 2. The UI ↔ engine interface: Esperanto

The UI does **not** talk to the engine over Electron IPC. REA's analysis
found `context_bridge_apis: 0` and `ipc.operations: 0` — there is no
Electron layer at all. Instead the UI uses Spotify's internal **Esperanto**
RPC:

```js
transport.callSingle({ service, method, payload }, cb) -> Promise<Uint8Array>
transport.callStream({ service, method, payload }, cb)
```

`payload` is a protobuf-encoded request; the response is protobuf bytes.
The transport object itself is **not** in the bundle — module `21255`
(`em.X()`) is injected by the native host. That injected module is exactly
what we replace.

Service ids are proto package paths, e.g.
`spotify.desktop.media_file_access_esperanto.proto.DesktopMediaFileAccess`.

### Services the UI calls (10 services, 29 methods)

Recovered from `7309.js` (`static SERVICE_ID`, `static METHODS`,
`static DECODERS`) and cross-checked against `Spotify.dll` strings.

| Service | Methods |
|---|---|
| `spotify.analytics_esperanto.proto.Analytics` | `subMessage` (stream) |
| `spotify.desktop.commands_esperanto.proto.DesktopCommands` | `subscribeCommand` (stream) |
| `spotify.desktop.issue_reporter_esperanto.proto.DesktopIssueReporter` | `submitIssue` |
| `spotify.desktop.media_file_access_esperanto.proto.DesktopMediaFileAccess` | `selectFiles`, `selectLocalFiles`, `getCoverArt`, `registerFile`, `readFileChunk`, `releaseFile` |
| `spotify.desktop.proxy_settings_esperanto.proto.DesktopProxySettings` | `getProxySettings`, `setProxySettings` |
| `spotify.display_segments_esperanto.proto.v1.DisplaySegmentsService` | `getDisplaySegments` |
| `spotify.image.esperanto.proto.Image` | `getImage`, `getCacheStatus`, `setProjectionMap` |
| `spotify.offline_playable_cache_esperanto.proto.OfflinePlayableCache` | `getTracks`, `hasEnoughTracks`, `subscribeTracks`, `subscribeHasEnoughTracks`, `removeTrack` |
| `spotify.prefetch_esperanto.proto.Prefetch` | `getEventHistory`, `getQueue` |
| `spotify.restrictions.permissions.permissions_proto.Permissions` | `canSeeQueue`, `canAddToQueue`, `canSetQueue`, `canPlayContent`, `canSkipPodcastVideoAd`, `canSwitchToMusicVideo`, `observeInvalidation` (stream) |

`DesktopCommands.subscribeCommand` is the pub/sub channel the UI uses for
transport commands (play/pause/seek/...).

### Message schemas

`re/out/esperanto-schema.json` holds the recovered field number + type for
every request and response message, extracted from the ts-proto encoders in
the bundle (e.g. `r.uint32(10).string(e.uri)` → `string uri = 1`).

Regenerate with `python re/schema.py`.

## 3. Backend endpoints

From `re/out/route-map.txt` (81 `open.spotify.com` routes = the layout
reference) and `re/out/spotify-strings.txt` (66k strings from the engine):

| Endpoint | Purpose |
|---|---|
| `https://spclient.wg.spotify.com` | the "web" RPC surface the engine uses |
| `https://spclient.wg.spotify.com/playlist/v2/playlist/` | playlists |
| `https://spclient.wg.spotify.com/identity/v3/user/username/` | identity |
| `https://clienttoken.spotify.com/v1/clienttoken` | client token (lifts Web API rate limits) |
| `https://login5.spotify.com/v3/login` | login |
| `https://apresolve.spotify.com/v2` | access points |
| `wss://dealer.g2.spotify.com:443` | Pathfinder WebSocket |
| `https://internal-podcast.spotify.net`, `https://podstart.spotify.com` | podcasts |

Hosts the engine talks to, read from `LocalState/Spotify/hosts.bnk`:

```
connectivity.backend_rpc_global_host    https://spclient.wg.spotify.com:443
connectivity.backend_rpc_regional_host   https://gew1-spclient.spotify.com:443
connectivity.backend_streaming_host      wss://gew1-dealer.g2.spotify.com:443
```

## 4. Free vs Premium

`LocalState/Spotify/offline.bnk` contains the account state:

```xml
<type>free</type>
<financial-product>pr:free,tc:0</financial-product>
<streaming>1</streaming>
<streaming-only-premium>0</streaming-only-premium>
```

* **librespot** (the native protocol, `ap.spotify.com:4070`) refuses
  non-Premium accounts: `librespot does not support "free" accounts.`
  It calls `exit(1)` — patched locally in
  `vendor/librespot/core/src/session.rs` to return an error instead.
* The Store app *does* play for Free accounts because the engine speaks the
  **web protocol** instead, and has its own DRM layer:
  `drm::KeyRequester`, `cpp-audio-drm_setup_impl`, `core-drm-feature`,
  `LicenseProvider@license@spotify`, `OfflineKeyStore`.
  For the Free tier it requests files with ad segments interleaved:
  `ad_clips_per_free_tier_ad_break`, `ad_flags=`,
  `StreamingAudioScanner: ad placement start_ms=%d duration_ms=%d`,
  `EmbeddedAdSegmentsDetector`, `media_prefetcher_feature_ads_segmented_files_enabled`.
* No Widevine/EME strings are present — the key comes from Spotify's own
  key service, not a CDM.

## 5. Target architecture

```
┌───────────────────────────────────────────────┐
│ browser host (WebView2 / OS Edge runtime)    │
│   serves the real xpui.spa                    │
│   + injected bootstrap that provides the      │
│     Esperanto transport (module 21255)        │
└───────────────────┬───────────────────────────┘
                    │  callSingle / callStream
                    │  {service, method, protobuf payload}
┌───────────────────▼───────────────────────────┐
│ Rust engine (spotify-rs)                     │
│   esperanto server  → dispatches by service  │
│   web-protocol client (spclient + CDN)        │
│   audio pipeline                               │
│   Web API client (client token)               │
└───────────────────────────────────────────────┘
```

The UI is unmodified, so the layout is exactly Spotify's. Only the engine
changes.

## 6. What was built

`spotify-rs` is a Rust workspace:

| Crate | Purpose |
|---|---|
| `crates/spotify-core` | OAuth PKCE, Web API client, models |
| `crates/spotify-playback` | librespot wrapper (native protocol audio) |
| `crates/spotify-ui` | fully native egui front end (139 MB RSS) |
| `crates/spotify-host` | WebView2 host: the real Spotify web client in a Rust shell |
| `apps/desktop` | `spotify-rs` (host), `spotify-rs-auth`, `spotify-rs-play` |

### The web host

The Store client is the web client inside a bundled Chromium (CEF, 285 MB)
plus a native engine. The host keeps the web client — identical layout and
buttons — but swaps the browser for the OS provided WebView2 runtime, which
Windows already ships, and adds a native shell around it.

* `wry` 0.57 + `tao` 0.37, persistent profile in `%APPDATA%\spotify-rs\webview`
* Windows 11 treatment: Mica backdrop, rounded corners, dark caption
  (`DwmSetWindowAttribute`, see `crates/spotify-host/src/win.rs`)
* global media keys through a low level keyboard hook
  (`crates/spotify-host/src/hotkeys.rs`)
* an ambient colour wash sampled from the current cover art and applied behind
  the interface (`--sr-accent-rgb`, see `crates/spotify-host/src/chrome.rs`).
  The native client has this; the plain web player does not. If the CDN
  refuses the cross-origin canvas read, it falls back to Spotify green.

Measured against the installed client:

| | Store client | spotify-rs |
|---|---|---|
| binary | 285 MB `libcef` + 44 MB dll + 60 MB spa | **14.2 MB** |
| native processes | 8 | 1 (+8 WebView2) |
| window visible | 2.71 s | 0.07 s |
| content painted | — | 4.64 s warm / 5.61 s cold |

### The ad blocker

Two layers, driven by the endpoints recovered from the app's own cache
(`re/out/api-surface.txt`):

* **native** — `--host-resolver-rules` maps advertising hosts to `0.0.0.0`,
  so the request never leaves the machine;
* **page** — `fetch`, `XMLHttpRequest`, `WebSocket`, `sendBeacon` and image
  sources are filtered by host and by path (`/ads/v2/`,
  `/sponsoredplaylist/`, `/gander/`, ...), and ad containers are hidden.

Verified: `spclient.wg.spotify.com/ads/v2/config` and
`/sponsoredplaylist/v1/sponsored` both answer `204 Blocked by spotify-rs`,
ordinary requests pass through.

### The chrome

The web only marketing surfaces are pruned (React re-creates the nodes, so
they are marked with an attribute and hidden with CSS instead of removed):
`Premium`, `Support`, `Download`, `Install App`, `Sign up`, the
`signup-bar` promo, the OneTrust cookie wall and `About Ads`.

## 7. Open questions

1. The exact framing of the transport module (21255) that the native host
   injects — we define our own, but the bundle expects specific export
   names.
2. Which of the ~40 esperanto services the engine registers natively are
   also needed by the UI (`spotify.playback_esperanto.proto`,
   `spotify.metadata_esperanto.proto`, `spotify.collection_esperanto.proto`,
   `com.spotify.player.esperanto.proto`, ...) — the bundle's own service
   list must be enumerated more completely.
3. The free-tier audio path: `AudioFileSelection.proto` →
   `ExtendedAudioFile` → CDN resolve → `drm::KeyRequester` → segment
   fetch/decrypt → ad-segment scheduling. The account is on the Free tier
   (`<financial-product>pr:free,tc:0</financial-product>`), so librespot's
   native protocol refuses it. The web player plays it through the web
   protocol, which is what the host uses today.
4. The Web API returns `429` with `Retry-After: 86400` for the shared
   desktop client id. A personal app client id + secret (developer
   dashboard) plus a client token avoids it. Set `SPOTIFY_RS_CLIENT_ID` and
   `SPOTIFY_RS_CLIENT_SECRET`.
5. `xpui.spa` cannot be booted standalone: the entry chunk
   `xpui-snapshot.js` expects a global `__webpack_modules__` and a runtime
   chunk `7207.js` that are **not** in the `.spa`, in `Spotify.dll` or on
   any public CDN. The native host injects them. `xpui.app.spotify.com` is
   split-horizon DNS (NXDOMAIN from Google and Cloudflare DoH) so the
   desktop bundle cannot simply be fetched either.
