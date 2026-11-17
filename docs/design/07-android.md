# Relay: the phone side (Android)

Status: **draft for approval** · 2026-11-17 · Items marked **(call)** are proposals.
"(unverified)" means I did not confirm it in a primary source this session.

Test hardware: 2 Windows PCs and 1 phone. Android app is roadmap after P3, but the core must not
block it, and the phone should be useful in testing as early as is sensible.

## 1. What the phone does at each stage

| Stage | Phone role | What is needed |
|-------|-----------|----------------|
| P0 | None. | Nothing. Keep core free of Windows-only code. |
| P1 | None. | Nothing. Test transfers PC to PC. |
| P2 | Optional: a browser or `curl` is not a member. | Nothing. Do not build a web client (**call**). |
| P3 demo (**cheap option, no app**) | A plain Wi-Fi hotspot, the "third link". | The owner turns on the phone's normal hotspot in system settings. Both PCs join it. `link` sees a third interface and fails over to it. No Relay code on the phone. |
| After P3, step 1 | Real member: join, send, receive. | Tauri Android build, foreground service, SAF folder picker, NSD discovery. |
| After P3, step 2 | Host or deputy (hotspot you run yourself). | Local-only hotspot control. Likely not worth it early. |

Why the cheap option first: it exercises the P3 failover with zero Android work and uses the one
phone for a real test. Caveat: a phone hotspot often shares mobile data, so the PCs may also get
internet through it. Relay must treat that as just another path, never as required (matches "works
offline").

Client isolation on some hotspots or routers can block peer-to-peer traffic (unverified). If the
PCs cannot see each other over the phone hotspot, that is a test finding, not a Relay bug.

## 2. How the Rust core reaches Android

Tauri v2 has an Android target. Prerequisites are Android Studio, the SDK, NDK, and these Rust
targets: `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`,
`x86_64-linux-android`
([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).

Native code goes in a **mobile plugin**: a Kotlin class extending `app.tauri.plugin.Plugin` with
`@TauriPlugin` and `@Command` methods. Rust calls it through `PluginHandle::run_mobile_plugin`,
and Kotlin returns data with `invoke.resolve`. Native commands run on the main thread, so long work
must move to a coroutine. Kotlin can also call Rust over JNI
([Tauri mobile plugins](https://v2.tauri.app/develop/plugins/develop-mobile/)).

```
Tauri UI (web view) ── invoke ──▶ relay-app (Rust)
                                      │ calls only through node
                                  relay-node ─▶ modules ... (pure Rust, same crates as Windows)
                                      │ trait objects
                                  platform traits ◀── Kotlin plugin (NSD, SAF, service, hotspot)
```

| Module | Portable? | Platform piece |
|--------|-----------|----------------|
| common, proto (PR) | yes | None. |
| identity (ID) | yes | Key storage: Android Keystore or app-private files **(call)**. Keys must be loaded through a `KeyStore` trait, not a path. |
| store (ST) | yes, with a path given in | App-private dir from the platform. No hard-coded paths. |
| discovery (DS) | needs platform code | Beacon and mDNS in Rust work in principle. NSD is the supported Android path ([NSD guide](https://developer.android.com/develop/connectivity/wifi/use-nsd)). Kotlin feeds results through a `DiscoverySource` trait. |
| link (LK) | needs platform code | Interface list and change events. Android gives these through `ConnectivityManager` (unverified here). Hotspot control is separate (section 3). |
| session (SS) | yes | QUIC over UDP runs in Rust. Needs only a bound socket. |
| membership (MB) | yes | Approval prompt goes through the UI, as on desktop. |
| transfer (TX) | needs platform code | Files are content URIs, not paths (section 3). Reads and writes go through a `BlobSource`/`BlobSink` trait. |
| share (SH) | needs platform code | Folder grant maps to a SAF tree URI. Path checks become URI-tree checks. |
| audit (AU) | yes | Uses store. |
| node (ND) | yes | Hosts the foreground-service lifecycle hooks (start, stop, `onTimeout`). |
| app / cli | UI yes, cli no | The same web UI; the CLI is desktop only. |

## 3. Android constraints and the rule each implies

| Constraint | Fact and source | Design rule for the core |
|------------|-----------------|--------------------------|
| Background execution | Doze suspends network access and ignores wake locks while the device is idle. A foreground service stops the app counting as idle for App Standby, but the page does not say foreground services are exempt from Doze ([Doze and Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)). | Treat "process paused or killed" as normal. Sessions and transfers must resume from disk (already in `02-protocol.md`). Never rely on an in-memory-only transfer state. |
| Foreground service | From Android 14 (API 34) each foreground service declares a type plus a type permission. `dataSync` fits file transfer and needs no runtime permission ([service types](https://developer.android.com/develop/background-work/services/fgs/service-types)). | `node` exposes `start()` and `stop()` that a service can call. Show progress as events, so a notification can mirror them. |
| `dataSync` time limit | When targeting Android 15, `dataSync` gets 6 hours per 24 hours in the background. The system calls `Service.onTimeout` and the service must stop within seconds ([timeout](https://developer.android.com/develop/background-work/services/fgs/timeout)). | `node` needs a `pause_all(reason)` that flushes chunk maps fast. A forced stop must never corrupt `.relaypart` state. |
| Nearby permission | On Android 13+ (API 33) Wi-Fi Aware, Wi-Fi Direct and the local-only hotspot need `NEARBY_WIFI_DEVICES`. NSD is not in that list ([Wi-Fi permissions](https://developer.android.com/develop/connectivity/wifi/wifi-permissions)). Below API 33, `ACCESS_FINE_LOCATION` applies. | Permission state is UI and platform business. A module reports `RLY-DS-xxx permission missing` as `Degraded`, never crashes. |
| Multicast and mDNS | Wi-Fi drivers often filter multicast to save power. `WifiManager.MulticastLock` (needs `CHANGE_WIFI_MULTICAST_STATE`) asks the system to deliver it (unverified, from general knowledge). NSD discovery is costly and should stop when the UI pauses ([NSD guide](https://developer.android.com/develop/connectivity/wifi/use-nsd)). | Discovery sits behind a trait. Keep the signed UDP beacon (already planned), but also support "poll when asked" so the phone can run it briefly. Typed address and QR join must always work. |
| Hotspot control | `startLocalOnlyHotspot` exists since API 26, gives a system-chosen SSID and password, no internet (unverified). It needs `NEARBY_WIFI_DEVICES` on Android 13+ (verified, link above). | `link` must not assume it can pick the SSID or password. Hotspot is a `HotspotProvider` trait. The Windows one is host-run. The phone one may only return credentials to show or encode in a QR. |
| Scoped storage | The folder picker `ACTION_OPEN_DOCUMENT_TREE` grants a directory tree. It returns URIs, not paths. `takePersistableUriPermission` keeps it across reboots, but the grant is lost if the folder is moved or deleted. The root of internal storage and `Download` cannot be picked ([storage access framework](https://developer.android.com/training/data-storage/shared/documents-files)). | `share` stores a grant as an opaque `GrantRef` plus a display name, not a `PathBuf`. `transfer` reads and writes through byte-stream traits. A stale grant is an error code, and re-asking the user is the fix. |
| Display name vs file name | Display name is provider-specific and may not be the real file name; size may be null (same source). | Offer manifests must not trust name or size from the platform. Size comes from reading, and names are sanitized in `transfer`. |
| Battery | Wake locks and multicast locks cost power (same Doze source). | Heartbeats (250 ms active, 2 s standby, **call** in `02-protocol.md`) need a "low power" profile with longer intervals and a clock injected into `link`. Never hard-code intervals. |

## 4. Do now, P0 to P3

1. Keep every module free of `std::fs` paths and `#[cfg(windows)]` outside one `platform` crate.
2. Put discovery sources, hotspot control, key storage, and file access behind traits in the
   module that uses them. Windows implementations are the first impls, not the only ones.
3. Files are never `PathBuf` in the transfer or share interfaces. Use `BlobSource`, `BlobSink`
   and `GrantRef` (with a Windows impl that wraps paths).
4. All transfer and session state is on disk and safe to kill at any instant. Test it in `sim`
   with a kill at every step.
5. Heartbeat and timeout values come from a config struct, not constants.
6. `node` exposes `pause_all`, `resume_all`, and a progress event stream, so a service wrapper can
   be added without touching modules.
7. Build core crates for `aarch64-linux-android` in CI from P1 on (compile check only). This
   catches desktop-only dependencies early.
8. Never block on a UI approval inside a module. Approvals are events and replies, because the
   phone UI may be gone when they arrive.

## 5. Open questions for the owner

1. Which phone model and Android version is the test phone? (It decides which permission and
   foreground-service rules apply in practice.)
2. Is a phone hotspot with no app enough for the P3 demo, with a real Android member after P3?
3. Does the phone's hotspot allow devices to reach each other, or does it isolate clients? A
   five-minute test with two PCs would show it.
4. Is the Android member app required for the resume story, or is a working host-on-Windows demo
   plus a documented Android plan enough?
