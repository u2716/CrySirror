# Native application packages

CrySirror's native application frontends use the shared player library and
`crysirror-mobile` C/JNI boundary. `crysirror-desktop` still supplies the
`crysirror` CLI; its reusable library is named `crysirror_player`.

`platform/apple` contains a SwiftUI folder chooser and native CAMetalLayer
views for macOS and iOS. `platform/android` contains an Android folder importer
and SurfaceView frontend. All application icons come from `icon/crysirror.svg`
and its existing PNG exports. No Siglus icons, game files or system fonts are
included in the packages.

The packaging structure was informed by the owner's Siglus project at commit
`256c398e06fbcbae8e956eb046c831b3d6c8a593` (MPL-2.0), inspected in the local
Cargo reference checkout. The frontends here are CrySirror host implementations;
they do not link that project's game launcher or renderer.

## Local builds

Run from the workspace root. Apple builds require Xcode and XcodeGen. Android
builds require Cargo NDK, an Android SDK/NDK, Gradle 9 and Java 17. Installed
Rust targets must match the command. The helper uses the existing Cargo
`target` directory and builds Release configurations.

```sh
python3 tools/package_platform.py macos
python3 tools/package_platform.py ios-sim
python3 tools/package_platform.py ios
python3 tools/package_platform.py xcframework
python3 tools/package_platform.py android
# Optional Android emulator ABI:
python3 tools/package_platform.py android --abi x86_64
```

`--skip-rust` packages already-built native libraries. Android defaults to
`arm64-v8a`; the optional x86_64 target is a separate native build. No emulator
or Simulator is launched by these commands.

Outputs:

- macOS: `target/platform/apple/Build/Products/Release/CrySirror.app`, with an
  embedded dylib and local ad-hoc signature.
- iOS Simulator: `target/platform/apple/Build/Products/Release-iphonesimulator/CrySirror.app`.
- iOS device: `target/platform/apple/Build/Products/Release-iphoneos/CrySirror.app`.
- Reusable iOS library: `platform/apple/Vendor/CrySirror.xcframework` (arm64
  device and arm64 Simulator slices).
- Android: `platform/android/app/build/outputs/apk/release/app-release-unsigned.apk`.

Device iOS apps and Android APKs require the owner's signing configuration
before installation/distribution. The helper does not look up signing identities,
provisioning profiles or passwords. The macOS output is a host-architecture app;
it is not a universal binary. No game assets are bundled.

## Ownership and storage

A session owner thread creates and owns the Runtime, native adapters, shared
wgpu renderer and shared audio host. Frontend callbacks update one bounded
mailbox containing latest dimensions/pointer/held buttons and accumulated press
edges. Resource/composition/render-extraction tasks retain their existing shared
worker boundaries. A session status response reports loading, running, failed or
stopped. Boot failures and unsupported VM operations are visible to the launcher.

The C boundary accepts a retained CAMetalLayer or ANativeWindow. `start` and
`attach` return a Surface generation. `detach(session, generation)` waits until
that particular view is no longer used; a late detach cannot remove a successor.
The Runtime, VM, GPU device/renderer, resource caches and frame counter remain
owned by the same session while its native view is absent. `destroy` stops and
joins the complete session. Android's factory owns an acquired NDK window lease
and drops it after its Surface; Apple retains its layer until detach returns.
Session IDs are opaque integers, never VM or GPU pointers.
The renderer crate does not depend on UIKit, Android views or winit.

Apple file selection balances security-scoped URL access for the duration of
the selected source. Android SAF document trees are imported through their
provider into application storage, preserving file bytes/names; `content://`
URIs are not passed to Rust as filesystem paths. Imports are bounded and failed
imports remove only their newly-created destination. Saves are separate from
the selected game directory. Current launchers select one game at a time; Apple
selection is not persisted across launches, and Android remembers its last
successful import.

Boot/archive completion polling and GPU initialization check cancellation before
continuing; detaching during archive preparation does not wait for that
preparation to finish because the view has not been acquired by the owner yet.
The instance registry is not held while waiting for Surface retirement.

The host supports touch/pointer clicks, right-click/menu, basic hardware keyboard
buttons, resize, suspend/resume and native close requests. Full native IME,
platform file dialogs requested by a running VM, clipboard UI and software
keyboard integration remain separate host work. Android Surface recreation and
Activity recreation within the same process reconnect the existing session. Per-holder
session/generation bindings prevent callbacks from removed views affecting a
new game or view. Apple representable-view recreation likewise detaches and
reattaches without restarting the VM. Background/no-Surface periods suspend
simulation/audio, clear held buttons, and reset elapsed time on resume. Runtime
state is retained in process memory; this does not persist a live VM across OS
process death. Original native save operations remain adapter-owned. Native VM API coverage is the
same adapter coverage as the desktop; packaging does not establish compatibility
with every game. iOS code-signing policies can impose additional restrictions on
native plugins; no arbitrary external plugin code is loaded by this frontend.

## Native text encoding

The launcher probes the selected game's native adapter and exposes only its
existing override choices. `auto` leaves the adapter's default/detection policy
unchanged. The same policy is available to the CLI:

```sh
./target/release/crysirror play /path/to/game --nls gbk
```

| Adapter | Choices | Native configuration |
| --- | --- | --- |
| RealLive | sjis, gbk, big5, utf8, western, korean | encoding |
| AVG32 | sjis, gbk, big5, utf8 | encoding |
| RFVP | sjis, gbk, utf8 | nls |
| SoftPAL | sjis, gbk, utf8 | nls |
| NScripter | sjis, utf8 | encoding |
| BGI | sjis, gbk | gbk boolean |

Other adapters expose no override through this host yet. Their own Unicode or
native parsing rules remain in force. No global byte conversion is inserted
between the VM and resources. RealLive retains its per-scenario RLdev metadata
precedence; SoftPAL's separate native `lang` setting is not changed by `nls`.
Unsupported choices fail before VM construction. Encoding overrides change
runtime/snapshot configuration without relocating native save directories.

ABI declarations and ownership requirements are in
[`platform/include/crysirror.h`](../platform/include/crysirror.h).

Platform references:
[Apple security-scoped file imports](https://developer.apple.com/documentation/swiftui/view/fileimporter%28ispresented%3Aallowedcontenttypes%3Aoncompletion%3A%29),
[Android document tree access](https://developer.android.com/training/data-storage/shared/documents-files).

## Console seams

PS Vita and Switch have SDK-free host scaffolds, not playable ports. See
[`platform/psv`](../platform/psv/README.md) and
[`platform/switch`](../platform/switch/README.md). Their device backends, full
Runtime portability and packages remain separate work.
