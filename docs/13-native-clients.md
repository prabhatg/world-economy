# 13. Native clients

Recommendation from a design review of the native-client options (Flutter + frb,
Tauri v2, per-platform native, Rust-native UI, Compose Multiplatform, React Native).
Items marked **(verify)** depend on platform capabilities or store policies that must
be re-checked before implementation.

## 13.1 Key idea: the UI is thin, the daemon does the work

Almost everything hard in a volunteer client is not UI: coordinator protocol, work
cache, scheduling, idle/thermal/battery policy, signature verification, sandboxed
engine processes, GPU detection, engine updates. All of that lives in a **headless
Rust daemon that is also the CLI/Docker client**. The UI shell is a thin front end
over local IPC, which makes the shell choice reversible.

Because work units name a content-hashed `engine_version`, the daemon can download
and swap signed engine binaries on desktop without reinstalling the app.

## 13.2 Architecture

```
 ┌──────────────── desktop (Win / macOS / Linux) ────────────────┐
 │                                                               │
 │  UI shell (Flutter)  ◄── local IPC ──►  worldd (Rust daemon)  │
 │   status, settings,     UDS / named      coordinator client,  │
 │   "my world" globe      pipe, versioned  cache, scheduler,    │
 │   (embedded web         JSON-RPC or      idle/thermal/battery │
 │   viewer)               protobuf         policy, signatures,  │
 │                                          engine hot-swap,     │
 │  localhost web viewer ◄── served by ──── GPU detection        │
 │  (zero-shell UI, Linux fallback)              │ pipes only    │
 │                                               ▼               │
 │                                  world-engine workers         │
 │                                  (one per work unit,          │
 │                                   sandboxed, no network)      │
 └───────────────────────────────────────────────────────────────┘
```

1. **`worldd` daemon** (same binary as the headless/CLI/Docker client). Runs
   **per user**, not as a system service — idle detection and GPU access are simpler,
   and app sandboxes tolerate it:
   - Windows: Task Scheduler at logon (or Run key)
   - macOS: LaunchAgent via `SMAppService`
   - Linux: `systemd --user` unit
   - Idle detection natively in the daemon: `GetLastInputInfo` (Windows),
     `CGEventSourceSecondsSinceLastEventType` (macOS), XScreenSaver / Wayland
     `ext-idle-notify` (Linux).
2. **`world-engine` workers**, spawned per work unit with pipes only; the engine links
   no network stack at all:
   - Linux: network namespace + Landlock + seccomp (or bubblewrap); GPU device nodes
     allowed.
   - macOS: `sandbox-exec` profile denying network — deprecated API, still functional
     **(verify)**.
   - Windows: AppContainer / restricted token + Job Object — least mature path; fallback
     is running the **wasm32 engine under wasmtime** (sandboxed by construction, ~1.5–3×
     slower, same platform class as browser clients for validation).
3. **UI shell** talks only to the daemon. The daemon also serves the web viewer on
   `localhost` (BOINC-manager pattern) — a working UI from day one and the Linux
   fallback.
4. **Mobile folds daemon-core + engine into the app** (no separate daemon):
   - Android: WorkManager worker (charging + unmetered) in a separate
     `android:process`; consider `isolatedProcess="true"` to drop network permission
     **(verify native-library loading)**.
   - iOS: in-process under `BGProcessingTask`, small segments with ~1-minute
     checkpoints; expect minutes of compute per night.
   - Store builds embed the engine (Apple 2.5.2 and Google Play forbid downloading
     executable code); coordinator capability matching handles version lag.

## 13.3 UI technology

- **Flutter + flutter_rust_bridge** is the single shell for desktop and mobile
  (ADR-0022, accepted) — *on the condition that it owns only presentation and IPC*.
  frb wraps the IPC client on desktop and daemon-core on mobile.
- **Globe:** embed the TypeScript deck.gl/CesiumJS viewer in a webview
  (`webview_windows` on Windows, `flutter_inappwebview` on macOS, `webview_flutter` on
  mobile). On Linux, open the daemon's localhost viewer in the browser until a Flutter
  webview is viable. `flutter_gpu` is not relied on **(verify status)**.
- **Fallback:** Tauri v2 is the named desktop fallback. If Flutter desktop webviews
  prove unreliable, a Tauri shell can reuse the viewer verbatim over the same daemon and
  IPC with no engine changes.
- **Native glue** (Swift / Kotlin / C++) is confined to idle detection, background
  services, store entitlements and the macOS screensaver host.

## 13.4 Options considered

| Criterion | Flutter + frb | Tauri v2 | Per-platform native | Rust-native (egui/Slint) |
|---|---|---|---|---|
| Desktop + mobile, one codebase | Yes | Yes (mobile thinner) | No (4×) | Desktop yes, mobile weak |
| Reuse of TS globe viewer | Via webview; Linux fragmented | Direct; Linux WebKitGTK WebGL slow | Via platform webviews | No (new renderer) |
| Tray / autostart / updater | Community plugins; no updater | Built-in, incl. signed updater | Native, best | Crates; no updater |
| Background engine model | Separate daemon | Sidecar built in | Native | Separate daemon |
| Shell size | 20–30 MB | 5–10 MB | 5–20 MB | ~10 MB |
| Solo + AI maintenance | Medium (Dart, but known) | Low–medium | High | Medium |
| Fit with founder's skills | Excellent | Moderate | Low | Low–moderate |

Rejected: Compose Multiplatform (JVM size for a background app, weak tray/Linux),
React Native (thin desktop support), Dioxus/Slint (mobile not mature enough).

## 13.5 Updates, signing, distribution

- Updates are driven by the daemon (download signed bundle, swap, relaunch), or
  Sparkle / WinSparkle / velopack.
- Signing: Apple Developer Program + notarization; Windows OV/EV certificate or Azure
  Trusted Signing (eligibility for new organizations **(verify)**); Linux Flatpak +
  AppImage.
- Direct distribution first (DMG, MSIX/installer, Flatpak, AppImage, APK/F-Droid);
  app stores later.

## 13.6 Build order

0. `worldd` + `world-engine` + CLI/Docker client (needed for M1 cloud spot workers),
   IPC schema, Linux sandbox, localhost viewer.
1. Desktop Flutter shell (tray, status, settings, idle rules) with webview globe;
   macOS notarization, Windows signing, updater; direct distribution.
2. GPU compute in the engine (wgpu first as one backend for Vulkan/Metal/DX12;
   CUDA later) — independent of the shell.
3. Android (Play + direct APK / F-Droid).
4. iOS, explore-first.
5. Screensavers: Windows `.scr` first; macOS `.saver` (native Swift/Metal, fragile on
   recent macOS) last; Linux low priority.

## 13.7 Risks

- **Apple review:** guideline 2.4.2 forbids unrelated background processes and
  excessive battery/heat. Science compute is not banned, but the app's primary purpose
  must visibly be *the user's world*; `BGProcessingTask` is the only sanctioned path
  **(verify current wording)**.
- **Google Play:** foreground-service type limits (e.g. `dataSync` time caps;
  `specialUse` needs a declaration) may cap compute; direct APK is the escape hatch
  **(verify)**.
- **Linux WebKitGTK WebGL performance** — mitigated by the browser fallback.
- **Windows sandboxing without network** — wasmtime fallback.
- **Windows signing cost and SmartScreen reputation** for a new publisher.
- **Thermal/battery complaints** are the top reason volunteer clients get uninstalled
  or rejected — conservative defaults in the daemon.
