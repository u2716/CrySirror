# Retained native player sessions — 2026-11-04

## Implementation

Native view lifetime is now separate from Runtime/VM lifetime. One session owner
constructs the Runtime once and retains the renderer/device, resource caches,
pending engine work, immutable frame snapshot and frame counter while its native
view is absent. Reattachment replaces only the Surface and its configuration.
The shared rendering backend remains independent of UIKit, Android and winit.

The C/JNI boundary adds attach/detach with Surface generations. A caller's detach
waits for retirement of its own generation. Late callbacks cannot remove a
successor. JNI factories own acquired ANativeWindow references; Surface drops
before its factory/native lease. C callers retain their native layer until
detach or session destruction returns. Registry lookup clones an Arc and
releases the global registry lock before any retirement wait.

Android binds callbacks to each SurfaceHolder and retains the session ID across
Activity recreation within the same process. Explicit library/game shutdown destroys the
session; Surface or temporary Activity destruction does not. A process-scoped
plain session receipt and weak UI owner avoid retaining destroyed Activities. Apple view dismantling/window detachment
retires its binding and reconnects the same Player session on recreation. Old
Apple bindings cannot resize or inject input into a successor view.

Background/no-Surface periods pause simulation/audio and clear held input. Time
sampling resets on resume. Pending archive startup checks cancellation and
releases its decode ticket; GPU initialization is polled with cancellation.
A Surface removed while startup is still preparing resources has not yet been
acquired by the render owner, so it can be detached without waiting for archive
preparation. Lost swap chains recreate their Surface from the retained factory.
Android import disables Play while an import is in flight, preventing its
completion callback from shutting down a subsequently started game.

This retains process memory. OS process death still requires native saves;
there is no automatic serialization or alteration of original VM save formats.
Full native IME, game-requested OS dialogs and clipboard UI remain separate work.
Console host scaffolds are unchanged by this implementation.

## Compilation

Release checks passed for the mobile/player Rust library, the desktop test
sources and arm64 Android/iOS Simulator targets. Swift source type-checks passed
for macOS and iOS Simulator; Java Release compilation passed. The C header
passed a C syntax check. macOS, Android, iOS device and arm64 Simulator packages were rebuilt and their
attach/detach entry points verified in the actual artifacts. macOS signature
verification passed; the APK native library matches Gradle's stripped output.
Native application package results are recorded in
`target/verification/mobile-retention/builds.json` and its adjacent build output.

Four regression scenarios are included in embedded.rs: detach during boot,
pending-view replacement with lease retirement, waiting for active retirement,
and ignoring late destruction of a predecessor. They were compiled, not run.

## Behavior validation

No applications, games, tests, emulator or device session were executed for this
follow-up. Compilation and package/export checks do not establish runtime
behavior, GPU-driver recovery, audio continuity or hardware compatibility.

Lifecycle references:
[Android Surface callbacks](https://developer.android.com/reference/android/view/SurfaceHolder.Callback),
[NDK native window references](https://developer.android.com/ndk/reference/group/a-native-window),
[Apple view dismantling](https://developer.apple.com/documentation/swiftui/uiviewrepresentable/dismantleuiview(_:coordinator:)).
