# Native UI and save integration audit

This is a source audit and a bounded rendering check, not a claim that any
adapter is complete or that an installed game's full route was tested.

## RealLive window toolbar change

`adapter/window_buttons.rs` translates WAKU BTN patterns, corner-relative
boxes, global/per-window switches and extra-button switches into shared image
Layers and interaction-only UI widgets. Hover/press/activation identity comes
from shared UI. Clear, voice replay, read skip, auto and WBCALL are native
policies; a native long-operation fence retains the paused wait while an extra
button's script runs. History uses shared modal UI and a bounded native text
history. Its appearance is not an original-engine backlog skin reconstruction.

Text-window closing removes the toolbar; temporary hiding covers nine-slice
frames too. Pause and page-full waits observe auto timing and voice completion.
Audio prepared by UI replay does not acknowledge a nonexistent script syscall.

Compilation: `cargo build --release -p crysirror-desktop --bin crysirror`
completed successfully. No debug target was built.

Behavior regression: the three release-mode window-button tests passed,
covering corner-relative boxes, shared UI-driven atlas states, hide/auto
actions without text advancement, and WBCALL return fencing.
The existing `engine_bridge` release integration suite also passed all 15
tests. These checks are separate from installed-game behavior acceptance.

Rendering: the new desktop binary ran the owner-installed Little Busters
through 10,000 offscreen ticks with the prior story input track. The resulting
800×600 frame contains the Auto, Skip, Backward, Forward, Save, Load, Status
and Close artwork. This checked a generated offscreen artifact, not a live
desktop screenshot. It does not establish that every button's downstream
script/menu operation is implemented.

Baseball: the opt-in PT00 DLL test passed against owner-provided PT00.dll for
2,000 simulation frames. Archive inspection found the original PT00 call sites.
Starting isolated helper scenes without their parent call/memory context is
not a valid playable-game acceptance test; full baseball play remains unverified.

## Confirmed remaining gaps

* RealLive: slot-header queries and global persistence exist, but the adapted
  save module has no local-slot save/load execution or complete native slot
  menu. Runtime checkpoint/restore explicitly returns Unsupported. Some system
  UI switches are remembered without a corresponding rendered UI operation.
  Motion-based SEL effects without shared motion transitions substitute a
  crossfade and increment a diagnostic counter.
* Siglus: WAKU button declaration, painting, shared UI observations and native
  callback admission are already implemented; it does not have RealLive's
  exact missing-BTN path. Original-save stage selection reconstruction calls
  `prepare_stage_btnselitems` / `prepare_saved_stage_btnselitems`, which only
  record unsupported-layout notes. The plain message profile rejects nonzero
  novel/name-window and several alternate message modes. Original-save MWND
  reconstruction and movie resource reconstruction have explicit restrictions.
  CAPTURE_FOR_OBJECT currently substitutes a whole-frame capture for a limited
  capture. The joypad configuration command logs an error and returns.
* AVG32: native fullscreen requests are produced by the menu, but the adapter
  drains host requests using only a Quit predicate, dropping fullscreen
  requests. Unknown scenario subcommands commonly warn and continue, which
  can leave partial behavior instead of a visible integration failure.
  Framework checkpoints are unavailable; original SAVE.INI is a separate path.
  Rendering still submits a composed 640×480 PDT surface; moving raster
  algorithms to a shared crate alone does not establish the owner-required
  semantic resource/operation integration.

These findings prioritize local save/load, restored selection UI, dropped host
requests and warning-only paths for follow-up. They do not enumerate every
opcode or certify the rest of any adapter.
