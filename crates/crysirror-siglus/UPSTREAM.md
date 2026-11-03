# Siglus resource integration

The Siglus resource implementation is maintained by xmoezzz within CrySirror.
This crate owns native G00 layouts and metadata; common binary, image and codec
operations use shared engine services. License: MPL-2.0.

The runtime decoder is the reference for palette alpha, forward overlapping
LZSS matches, display cropping, image centers, original sizes, missing PATNO
slots and negative-length reused cuts. These policies stay in this crate.

Engine integration:

- Checked little-endian reads use crysirror-core::binary.
- RGBA size checks, red/blue conversion and bounded JPEG decoding use the shared
  crysirror-image service. Window/GPU/audio SDK dependencies are absent.
- Added 64 MiB input/decompression/surface/total-frame budgets, 8192 dimension
  and 4096 cut limits; invalid ranges and coordinate overflows return errors.
  Files exceeding this implementation profile fail visibly.
- The generic image plugin accepts only one frame without native pivot/crop
  metadata. Native objects must explicitly select frames and apply metadata;
  the shared renderer never guesses these semantics.

The original SceneVm, CommandContext, waits, forms/syscalls and original save
codecs are not replaced by a new interpreter or by the data VM. Their integration
is still pending; this port is an executable native-resource slice only.

CrySirror now also owns standalone crysirror-audio and crysirror-text services.
Those are engine implementations, not copied Siglus subsystems. Future native
integration maps audited native policies to them; see docs/ENGINE_SYSTEMS.md.
