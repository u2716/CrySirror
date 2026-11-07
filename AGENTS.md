# CrySirror development rules

Read STATUS.md, docs/ARCHITECTURE.md and docs/NATIVE_ADAPTERS.md before editing.

- Engine onboarding has two modes under ADR 0102 and docs/ENGINE_ONBOARDING.md:
  `adopt` implements/maintains native VM semantics over shared capabilities;
  `integrated` maintains a complete runtime behind CrySirror-owned host/output
  services. Record the chosen mode in the crate README. Existing modes do not
  change automatically; new unclassified work defaults to `adopt`.
- Siglus, BGI and Softpal use `integrated` backends: retain their complete
  implementations as internal modules of workspace crates and present their
  frame/PCM output through CrySirror. The host owns the window/event loop and
  output devices. Their existing capability-extraction code remains preserved,
  inactive in the default player; `codex/bgi-extraction-backup` preserves the
  BGI extraction work separately. Other current adapters retain their `adopt` boundary unless
  explicitly migrated. Integrated runtimes follow ADR 0102's lifecycle,
  frame/PCM, documentation, storage and platform requirements.

- For `adopt`, integrations grow CrySirror's reusable capabilities. A new VM
  must call CrySirror APIs through a thin semantic/syscall adapter; it must not
  bring a second general-purpose engine implementation or renderer with it.
- In `adopt`, do not host a legacy engine and upload its composed screen.
  Do not keep reusable image/text/UI/layer/effect/audio algorithms private to a VM
  merely to get a game running. Add missing capabilities to CrySirror first.
- `adopt` acceptance requires engine-owned resources/operations/state usable without
  that VM. A running game, matching screenshot or registered factory alone
  does not establish capability extraction. `integrated` acceptance instead
  requires the explicit host/output/platform boundary and scoped behavior
  evidence in docs/ENGINE_ONBOARDING.md.

- Owner requirement (2026-10-30): adopted VM and adapter source must be
  maintained as first-class CrySirror workspace crates. Do not use an
  `*-upstream` engine package or an engine Git dependency as the maintained
  integration boundary. Existing such dependencies are
  migration debt, not accepted completed integrations. Renaming/moving the
  package alone does not satisfy this requirement. In `adopt`, extract reusable image,
  text, UI, composition, effects and audio mechanisms into shared engine APIs;
  retain only original VM and native semantic/resource/wait/save policy in
  the adapter. In `integrated`, complete native runtime modules may remain
  internal under the documented host seam. Preserve source provenance and
  applicable licence notices in both modes.

- Async/multithreaded execution is an engine-wide requirement (ADR 0050).
  Asset loading and pose workers in ADR 0049 are only stage one; every subsystem
  needs explicit task/dependency, completion and ownership boundaries.

- Default host: wgpu + winit. Switch and PS Vita are deferred; no other console.
- RFVP and Siglus remain separate original VMs, never aliases for the data VM.
- Preserve opcode, value, wait, resource and original-save semantics in adapters.
- Inspect complete upstream source at pinned commits before implementing hooks.
- Do not invent commit IDs, native APIs, successful builds or hardware results.
- Core/compat must stay free of window/GPU/audio dependencies and unsafe code.
- Keep the renderer independent of winit; the desktop crate owns the event loop.
- Unknown native adapters/layouts fail visibly instead of silently succeeding.
- Use client-scoped handles and capability roots for shared-service calls.
- Keep project/VM data distinct from GPU/device pointers and executable code.
- Never distribute user game data, fonts, credentials or proprietary SDK files.
- Document implementation, compilation and behavior validation separately.
- Run Python tooling tests and cargo checks/tests when the environment permits.
  Use `cargo check/test --release` for local validation (owner preference).
- Current contracts belong in adapter docs, native findings beside their code,
  and dated behavior evidence in focused docs/validation notes. Do not append
  implementation specifications or temporary build/agent coordination to
  STATUS.md or HANDOFF logs. An ignored GPU test is not a pass.
- After API changes, regenerate docs/api and log fingerprint changes (docs/api/README.md).

## Engine onboarding skill

Use `.agents/skills/crysirror-engine-onboarding/SKILL.md` for native engine
adoption/reverse implementation or integrated-runtime host work. It supplies
safe analysis boundaries and routes to the mode-specific contracts.
