# ADR 0103: Game-space UI and presentation boundaries

Date: 2026-11-11
Status: accepted

## Context

Game windows, dialogue panels, settings pages and sliders need a coherent UI
system within the engine. Their geometry and appearance belong to the game;
the operating system's window frame, menu bar and display scaling belong to
the platform host. Treating these as one space produces misplaced controls,
inconsistent hit testing and settings pages whose appearance loses the game's
design.

## Decision

CrySirror maintains a distinct game-space UI system over shared Scene, Layer,
Text and Input capabilities. Window, panel, button, slider, scrollbar, list,
text field and dialogue controls share layout, styling, focus, interaction and
lifetime contracts. Authored games and adopted native adapters use the same
capabilities. Native scripts retain their own action and value semantics.

This decision specifies the design boundary; accepting it does not certify
that every control or adapter has completed the design.

### Spaces and ownership

| Space | Owner | Responsibilities |
| --- | --- | --- |
| Platform window/view | Desktop, mobile or browser host | OS frame, title bar, native menu, display scale, IME and OS events |
| Game UI surface | Shared engine UI | Logical canvas, layout, clipping, stacking, controls, focus and game-space windows |
| World-space UI | Shared engine UI and World | Surface placement/projection and mapping input back into its logical canvas |
| Native game policy | Adapter or game logic | Button actions, original ranges, waits, script callbacks, persistence and resource selection |

A game-space window has explicit outer bounds and a client area, with optional
game-drawn chrome, drag/resize constraints and modal ownership. Its title bar
and child controls participate in the game canvas. A request for a separate
native OS window goes through the host's window service with an explicit
lifecycle and input mapping.

### Coordinates and layout

- Each UI surface declares its logical size, viewport, scaling policy and
  clipping root. Presentation transforms are explicit, including letterboxing
  and display-scale conversion. A legacy 800-by-600 canvas keeps those logical
  units on a Retina display; OS device pixels do not become script coordinates.
- Controls use parent-local coordinates. Layout computes the same transformed
  geometry used for rendering, hit testing, pointer capture and IME placement.
  Scaling or moving a parent must not require rewriting its children's native
  coordinates.
- Content bounds, client-area bounds, scroll offsets and hit regions remain
  distinct. Nested clipping, hidden controls and empty clips have defined
  behavior. World projection converts into the same local input space.

### Controls and interaction

Shared controls define normal, hover, pressed, focused, disabled and selected
states. Pointer capture, keyboard/controller navigation, touch and text input
have explicit ownership. A modal game window scopes focus and input to its
subtree; closing it restores the preceding valid focus target.

Sliders and scrollbars expose range, value, step, orientation, track/thumb
geometry and change/commit events. Continuous drag, click-to-position and
keyboard increments use one value model. Original integer quantization,
nonlinear mappings and callback timing remain native policy where required.

Dispatch uses stable scoped identities and an ordered event stream. A callback
can close a window or replace controls without leaving stale capture or
delivering queued events to a destroyed identity. Rendering an updated frame
does not replay an activation or advance a suspended native wait.

### Visual design and skins

Layout, interaction and painting are separate. Shared style/skin data supports
game textures and atlases, nine-slice panels, fonts, state images, masks,
effects and animation. CrySirror also supplies a deliberately designed default
theme for authored UI. Theme metrics participate in layout and hit testing.

Adopted games retain their original artwork, labels, control geometry and
state transitions through these shared mechanisms. A generic settings panel
does not establish completion of a game's native settings interface. Missing
painting capabilities are added to the engine; a VM does not acquire a
private general-purpose widget renderer.

### Tasks, publication and lifecycle

UI mutation and event delivery belong to the session owner. Font shaping,
skin/image preparation and suitable layout work use bounded tasks with
explicit dependencies, cancellation and completion receipts under ADR 0050.
Workers consume immutable inputs; the owner publishes a consistent layout,
visual and hit-test state together. Pending resources cannot block the OS
event loop or acknowledge an unfinished native operation.

Controls, surfaces and cached resources have client-scoped ownership. Closing
a surface releases capture, focus, queued callbacks and presentation leases.
Resize, suspend, restore and device replacement preserve logical state while
reconstructing presentation resources. Saved data contains logical UI/native
state rather than OS, GPU or device handles.

## Relationship to other decisions

This extends the shared capability boundary in ADR 0047 and the async ownership
requirement in ADR 0050. ADR 0102 continues to govern onboarding modes:
`adopt` adapters translate native UI semantics into shared services;
`integrated` runtimes retain their documented internal UI and managed frame/input
boundary. This ADR does not change an adapter's onboarding mode.

## Acceptance

A control design covers appearance, layout, input, focus, modal behavior,
async publication and cleanup together. Adapter acceptance additionally covers
the original action/value policy and skin. Correct coordinates at different
viewport scales, usable slider interaction, modal close/return and resource
lifetime are independent of whether an entry point merely stopped reporting
an unsupported operation.
