---
name: crysirror-platform-host
description: Maintain CrySirror platform hosts and release packaging across desktop, iOS, Android, browser, PS Vita and Switch. Use for session/view lifecycle, input/NLS, audio interruption, scoped storage, native bridges, signing, build-only CI and artifact verification.
---

# Platform hosts, lifecycle and release artifacts

Read `AGENTS.md`, [native packaging](../../../docs/NATIVE_PACKAGING.md),
[onboarding contracts](../../../docs/ENGINE_ONBOARDING.md) and the affected
host/build source. Reuse the shared player/session and current scripts. Read
only the platform-specific implementation relevant to the task.

## Choose the actual host and build boundary

| Target | Inspect |
| --- | --- |
| Desktop | [desktop host](../../../crates/crysirror-desktop/src/app.rs), [desktop guide](../../../docs/DESKTOP_API.md) and existing native dependencies. |
| macOS/iOS | [Apple frontend](../../../platform/apple/Sources/NativePlayer.swift), [project](../../../platform/apple/project.yml) and [C boundary](../../../platform/include/crysirror.h). |
| Android | [Activity](../../../platform/android/app/src/main/java/org/crysirror/player/MainActivity.java), [JNI](../../../crates/crysirror-mobile/src/android.rs), [manifest](../../../platform/android/app/src/main/AndroidManifest.xml) and [Gradle](../../../platform/android/app/build.gradle.kts). |
| Browser | [web host](../../../crates/crysirror-web/src/lib.rs) and [web build/serve tool](../../../tools/web_host.py). |
| PS Vita | [platform contract](../../../platform/psv/PLATFORM.md), [player](../../../platform/psv/player/src/main.rs) and native GXM/GXP inputs. |
| Switch | [platform contract](../../../platform/switch/PLATFORM.md), [libnx shell](../../../platform/switch/runtime/source/main.c), [Makefile](../../../platform/switch/runtime/Makefile) and native deko3d/DKSH inputs. |

Desktop compilation does not validate mobile lifecycle, browser workers or
console execution. A freestanding target check is not the std-based player.
Preserve native console graphics and the selected `adopt`/`integrated` runtime;
do not create a private per-engine host, renderer or output device.

## Maintain lifecycle and bridge ownership

Trace startup/readiness, attach/detach, resize, suspend/resume, restart, failure
and destroy through the actual callbacks and session owner. View recreation
does not automatically mean VM recreation. Process death is separate from
in-process Activity/view replacement and requires a real persistence policy.

For [C/JNI sessions](../../../crates/crysirror-mobile/src/lib.rs), preserve opaque
session IDs, surface generations and ownership of native window/layer leases.
A late detach must not remove a replacement view. Stop/join resource/audio/GPU
producers before releasing their last device/view references, without holding a
global registry lock across waits. Release returned native strings/references
with their declared owner; keep pointers and native handles out of VM/save data.

Suspend must have explicit simulation, audio and input behavior. Prevent stale
press edges, accumulated background elapsed time, duplicated clock advancement
or replayed callbacks on resume. Verify both successful and failed startup/close
paths. Use [memory management](../crysirror-memory-management/SKILL.md) for old/new
surface overlap, device loss, deferred retirement and shutdown allocations.

## Preserve user-facing behavior and storage

- Map pointer/touch/controller input to the original game canvas independently
  of presentation scale, orientation or display mode. Define focus, repeat,
  capture, modal ownership, text/IME and platform menu/dialog handling.
- Keep native font/NLS/encoding rules per adapter. A locale, runtime language
  option and filename/byte encoding may be different settings. Do not insert
  global transcoding or silently change save identity when adding a selector.
- Use actual permitted storage: Apple scoped access, Android document/provider
  access, browser-injected sources or native console roots. A URI is not an OS
  path. Keep read-only game data, writable saves and derived caches separate.
- Audit network source propagation with
  [network storage](../crysirror-network-storage/SKILL.md), including permission,
  suspend/cancellation and backend availability. Do not bulk-copy remote games
  merely to satisfy a native path assumption.
- Define audio interruption/device replacement, pause/flush/restart and final
  drain ownership. A muted run or visible frame does not verify audio behavior.
- Reconstruct supported state through
  [native saves](../crysirror-native-save/SKILL.md); retained process memory alone
  is not crash/process-death persistence.

Unsupported platform capabilities must be visible. Adding a menu item, enum,
feature flag or package entry is not a connected implementation.

## Build and package the intended target

Use [package_platform.py](../../../tools/package_platform.py) for its supported
Apple/Android routes, the console player/Makefile for VPK/NRO, and the web tool
for browser artifacts. Inspect current arguments/toolchain requirements rather
than inventing commands or installing an unrelated SDK set. Use existing Release
target directories and matching binaries; `--skip-rust` cannot establish that an
old library matches changed source or the requested architecture/features.

Review target architecture, exported C/JNI symbols, native library dependencies,
loader paths, resources and shader catalogues. The Android helper currently
checks required JNI symbols and bundled native libraries; Apple device and
simulator slices remain distinct. A framework, app, APK, VPK, NRO and web site
are different artifacts, not interchangeable evidence.

Use CrySirror's own icons and preserve applicable source/notices. Do not package
user games, fonts, keys or proprietary SDK payloads without appropriate task
scope and distribution authority. The separate
[game packager](../../../tools/package_game.py) is not a general native-game
asset inventory or a cross-compiler; inspect its supported input profile first.

## Signing, CI and publication

Inspect existing signing configuration before changing it. A local ad-hoc macOS
signature is not notarization; an unsigned APK or signing-disabled iOS build is
not a signed distribution. Use the owner's intended identity/provisioning and
keep secrets outside source/logs/artifacts. Preserve package IDs and signing
identity for upgrades; do not uninstall an existing app or discard its saves to
work around a mismatch. Account/keychain steps follow the existing task scope.

Keep [GitHub CI](../../../.github/workflows/build.yml) compilation/package-only.
Do not add unit/regression tests, game runs, emulator boot or hardware execution
to CI. Inspect the real matrix, toolchain pins, native dependencies, features,
generated shader inputs and artifact paths. Bound caches/artifact retention and
avoid copying whole target trees or private SDK/data into uploads.

Compilation, packaging, signature verification, installation, launch and real
device behavior are separate results. Build-only CI does not certify game
compatibility or lifecycle behavior. Publishing, installing on a device or
using signing credentials must be within the user's existing authorization;
otherwise finish the requested reviewable local artifact without publishing.
Do not create an extra confirmation step when that action is already authorized.

## Validate the changed boundary and document it

Use the smallest authorized Release check or lifecycle exercise. Record the
actual target/toolchain/features, artifact, signing state and observed behavior.
Keep emulator/simulator and physical-device results separate. Do not claim a
universal binary, signed release or multi-platform pass from one host build.
Review [API compatibility](../../../docs/api/README.md#compatibility-assessment)
before changing C/JNI/command schemas, defaults, IDs or lifecycle completion.

Update the host's contract/platform matrix and focused validation notes. Do not
record temporary build coordination in STATUS.md. Adding this guidance needs no
build, signing action, device access or test run.
