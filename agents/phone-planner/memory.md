# Phone planner memory

One file, newest facts first. Read this before touching `docs/design/07-android.md`.

## Scope rules
- Edit only `docs/design/07-android.md` and this file. Work in the cloud only.
- Branch: `claude/project-thread-ks7ux9`, based on the docs branch `claude/project-thread-t5x7gf`.
- Stack decision (Rust core + Tauri) is fixed. Do not reopen it.

## Basic checklist (every edit)
1. Read this file and section 3 of the Android doc.
2. Every Android fact has a link to a primary source or says "(unverified)".
3. Every proposal is marked **(call)**.
4. Mermaid diagrams use the repo palette (#FF6B35, #FFD166, #06D6A0, #118AB2, #073B4C).
5. Add what was learned here, with the date.

## Advanced checklist (before saying a spike plan is ready)
1. Each spike has one question, a method that works with 1 phone + 1 PC, a pass line, and a
   fallback.
2. Spikes are ordered by redesign cost if they fail.
3. Steps that need the physical phone are named as owner steps.
4. Re-read the Android "local network permission" page. It changed between Android 16 and 17.
5. Check target-SDK-dependent rules against the version the test phone runs.

## Facts verified 2026-11-17 (primary sources read)
- Android 17 (API 37): apps targeting 37+ need runtime `ACCESS_LOCAL_NETWORK` (Nearby devices
  group) for all LAN TCP and UDP, including plain NsdManager. Target <=36 keeps access via
  INTERNET for now. Denied TCP times out, denied UDP gives EPERM. The Android 16 opt-in
  (`RESTRICT_LOCAL_NETWORK` + `NEARBY_WIFI_DEVICES`) is deprecated.
- `dataSync` FGS: 6 h per 24 h in background when targeting Android 15+, resets on app foreground.
  `connectedDevice` FGS is not in that list; prerequisite can be `CHANGE_WIFI_MULTICAST_STATE`.
- Network change callbacks need no special permission to listen.
- Local-only hotspot: no internet, `NEARBY_WIFI_DEVICES` on target 33+.
- Tauri mobile plugins: permissions declared in `@TauriPlugin(permissions=[Permission(...)])`,
  auto `checkPermissions`/`requestPermissions`, `trigger()` for events, hooks `load` and
  `onNewIntent`.
- `tauri-plugin-background-service` 1.0.1 is third party (one maintainer), service type not
  documented.

## Still unverified
- MulticastLock description and permission (reference page would not render its class text).
- `startLocalOnlyHotspot` API level and who picks SSID and password.
- Whether the Rust side of a Tauri app survives the activity being destroyed under an FGS.
- quinn / rustls crypto backend building for `aarch64-linux-android`.
- Passing a SAF file descriptor to Rust, and its write speed.
- Test phone model and Android version (owner has not said).

## Gotchas
- developer.android.com reference pages (class docs) come back as navigation only when fetched.
  Guide pages work. Prefer guide pages as sources.
