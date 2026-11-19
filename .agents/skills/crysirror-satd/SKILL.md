---
name: crysirror-satd
description: Track incomplete CrySirror experiments and implementations as self-admitted technical debt. Use when starting or reviewing unfinished work, moving its code, or removing a debt record after full implementation and validation.
---

# Maintain CrySirror SATD records

Read [ADR 0105](../../../docs/adr/0105-self-admitted-technical-debt.md).
Create a record for every incomplete experiment or implementation, including
work isolated from normal execution. Use the owning component's `SATD.md`.
Give each gap a stable `SATD-<component>-<short-name>` ID and cross-reference
it at the affected code or contract. Record the affected behavior and profiles,
what currently works, the remaining cases, and the evidence required to close
it. Reuse an existing record for the same unresolved contract.

Append a dated history entry whenever work implements part of the contract.
Name the behavior delivered and what remains. When moving or renaming code,
move its marker, update the record's current location and append a dated
history entry with the old path, new path and effect on the outstanding work.
Move the record with component ownership and preserve the ID and history.

Remove the record and all its markers only after the entire recorded contract
has been implemented and verified at its normal entry points. Link closure
evidence in the completing change or the component's validation record.
If an experiment is withdrawn while its scope is unfinished, retain its record
and append the date, reason and last code location.

For native syscalls, follow
[engine semantics](../crysirror-engine-semantics/SKILL.md). Keep incomplete
draft handlers isolated and the unresolved syscall visibly unsupported.
An SATD record tracks work through completion under the same dispatch rule.
