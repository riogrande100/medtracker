# Mobile / desktop target status

Phase 1 behavior is verified on the macOS host only (Rust tests, Vitest, `cargo build`, frontend build).
No device, simulator, Windows, or Android run has been performed.

## Reproducible scaffold commands

```sh
npm install
npm run tauri:ios-init       # tauri ios init   -> src-tauri/gen/apple (generated, git-ignored)
npm run tauri:android-init   # tauri android init -> src-tauri/gen/android (generated, git-ignored)
```

- iOS: `tauri ios init --ci` succeeded on this host (Xcode present). Not yet built or run on a simulator/device;
  signing (Apple team ID) and a simulator run remain open.
- Android: BLOCKED. `tauri android init` fails: Android SDK/NDK not found (`ANDROID_HOME`/`NDK_HOME` unset).
- Windows and macOS desktop: DEFERRED by owner decision D-29 (2026-11-18); no desktop release in the first release. Android build setup is pending on the Mac Studio (B0-10).

## Open items

- Device-local timezone: Phase 1 uses the OS fixed UTC offset at entry (no IANA zone id, no DST rules engine).
  Must be revisited per target before promising `device_local` schedules across travel/DST.
- Notifications are a stub (`src-tauri/src/notifications.rs`); nothing is scheduled.
