# Native hosts and console scaffolds — 2026-11-04

## Implementation

The desktop CLI delegates to `crysirror_player`; the shared embedded session
owns Runtime/VM/render/audio work on a Rust owner thread. Apple SwiftUI and
Android SurfaceView frontends pass source selection, input, physical viewport
and lifecycle commands through `crysirror-mobile`. Icons are existing CrySirror
exports. Native NLS overrides use the corresponding adapter's configuration.

PS Vita and Switch have `host.toml` plans, platform documentation and the shared
`crysirror-console` coordinator. It is safe no_std code over core InputFrame,
with explicit pending simulation/presentation ownership and backend capability
requirements. No native console driver or console package is implemented.
Cross-checking also exposed core's missing serde alloc feature; enabling alloc
fixes standalone no_std vector declarations without introducing std.

## Compilation and artifact checks

- `cargo check --release -p crysirror-desktop`: passed.
- `cargo check --release -p crysirror-desktop --tests`: passed, including the
  new NLS tests' compilation. This command does not execute tests.
- `cargo build --release -p crysirror-desktop`: passed; output is the existing
  `target/release/crysirror` location.
- `python3 tools/console_host.py psv`: passed for
  `armv7-sony-vita-newlibeabihf` with nightly build-std=core,alloc.
- `python3 tools/console_host.py switch`: passed for
  `aarch64-nintendo-switch-freestanding` with nightly build-std=core,alloc.
- Apple Swift source type-checks passed for macOS and arm64 iOS Simulator.
- The C header passed `clang -fsyntax-only`.
- iOS device and arm64 Simulator static libraries compiled in Release.
  XCFramework creation and both device/Simulator application packages passed.
- CrySirror's iOS app-icon asset catalog compiled with actool for iOS 16.
- macOS application packaging and ad-hoc signature verification passed.
- Android Java Release application build passed with Java 17 and SDK 36.
- Android native arm64 Release library and final unsigned APK compiled.
  All five JNI exports, including initialization, were verified in the packaged
  library. Its SHA-256 matches Gradle's stripped output; dynamic dependencies
  were Android system libraries.

Local build output lives under `target`, ignored Apple Vendor/project outputs,
and ignored Android Gradle outputs. Existing warnings in native adapter sources
remain; these are not treated as runtime compatibility results.

## Behavior validation

The desktop tooling Python suite passed 42 tests. A bounded C ABI probe of the
desktop fixture returned its engine/title/options, and a missing source returned
an error. These do not validate a game session or presentation.

Applications, games, mobile devices, console hardware and emulators were not
launched for this change. No visual, audio, touch or performance behavior is
established by compilation. Console contract checks do not compile the complete
std-based Runtime or prove SDK/device integration. The initial Surface-loss restart is superseded by the
[retained-session change](mobile-retained-sessions.md);
remaining platform limitations, including missing native IME, are
listed in [native packaging](../NATIVE_PACKAGING.md).
