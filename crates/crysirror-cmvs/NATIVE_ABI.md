# CMVS script ABI

The initial instruction profile is `Pc20160201X64` for the Windows x64 CMVS
build dated 2016-02-01. The Amatsutsumi sample executable has SHA-256
`dc74d2001387ae1aefbdc4204216de5ebb9d3896009280720c4e7a91b42d84f5`.
The referenced static analysis locates the script loader at `0x140080530`, the
VM loop at `0x140067930` and the syscall dispatcher at `0x1400823a0`.

The selected Amatsutsumi installation supplies that exact `cmvs64.exe` and
`data/pack/start.ps3` (SHA-256 `c338313949a803e834acf0d746c93f09c4d41f558af8fb6c13ab1ee55aff2bd0`).
The disk script header version is `0x27947`, the entry PC is zero and the code
segment is 4,726 bytes. Its `ps.cpz`, `script.cpz` and `bg.cpz` headers carry
`CPZ6`. The hashes identify the reference sample. Callers select the VM ABI
explicitly and pass the prepared reader for the supported PS2A layout.

| Admission | ABI identity and scope |
| --- | --- |
| Registered as `pc-20160201-x64` | CMVS Windows AMD64, PE COFF timestamp `0x56aedd70`, recognized config and `PS2A` startup layout. The VM and syscall tables refer to this profile. |
| Registered as `cvns-20040515-x86` | CVNS Windows i386, PE COFF timestamp `0x40a51966`, `cvns.cfg` and `PSS0` startup layout. CVNS uses its own syscall table. |
| Candidate `20160201` x86 | The reference catalogue records the same number of syscall slots at a different RVA and in a different CPU ABI. The launcher has no registered x86 profile. |
| Candidate `20171114` x64 | The reference catalogue records 992 slots and separate native handlers. Admission requires a profile with verified inheritance and separate behavior for changed slots. |

## CVNS Windows 2004-05-15

The original i386 VM loop is `0x424850`, and the dispatcher at `0x432550`
contains 544 jump-table entries starting at `0x2000`. CVNS `0x2000` advances
and yields. The 229 entries targeting `0x432fd5` return `0x4000` and retain
the stack and registers. `0x2002` consumes an unsigned millisecond period
from SP−4 and changes frame cadence. `0x2003` returns the active period in
system integer result 0 without consuming operands. `0x43a7a0` initializes
the period to 17 ms. `0x43a730` skips missed deadlines and admits one frame.
A zero period cannot advance that scheduler and the adapter reports it as
invalid. Other CVNS selectors require their own recovered contract before
they can reach a handler.

`0x210e` sets the audit-enable word at `+0x4fa4` and returns `0x4000`.
`0x433bb0` visits one of eight stripes of the original bit and integer
banks per admitted frame, compares each cell with its mirror and rotates
the stripe. A mismatch sets bit 1 of the engine error word at `+22500`.
The original bank accessors also check or synchronize mirrored values.
The CVNS adapter currently stops at `0x210e` until this bank contract is
represented in the VM.

In the supplied `start.pss`, the audit starts at code offset `0x1e`.
The next native calls fill global banks in this order. Each fill consumes
three words in the order first index, count and value from the top of stack.

| Offset | Selector | Native contract |
| --- | --- | --- |
| `0x44` | `0x2108` | `0x42c460` fills packed bits through the mirrored setter at `0x433910`. |
| `0x6a` | `0x2109` | `0x42c4d0` fills integer words through `0x4339e0`. |
| `0x90` | `0x210a` | `0x42c540` copies the resolved string into consecutive cells through `0x433a50`. CVNS has 256 string cells of 1,024 bytes. |
| `0xaa` | `0x2023` | `0x429610` binds a numbered resource source under the install root through `0x40a190`. This call consumes two words. |

The `0x423290` loader uses a 48-byte PSS0 header and the label, code,
mutable-data and text segments in the same order as PS2A. PSS0 payloads go
directly into the shared 11/5 LZSS decoder when word 9 is nonzero. The
transform on PS2A payloads is specific to PS2A. CVNS `0x41ee80` uses the
shared CPZ1 reader with the 64-byte `.rdata` table at RVA `0x5dd80`, adding
10 to each table byte when preparing the client-scoped key. `0x401250`
maps `CVNS_SYSTEM_MAIN/WINDOW_SIZE` `1` and `2` to 800×600 and 1024×768
respectively, with 640×480 as the default.

## CMVS frame and device queries

For the two x64 CMVS profiles, `0x2002` consumes no operands and copies the
CMainFrameWnd word at `+0x3c` into integer result 0. IDA resolves virtual
slot 1 to `0x1400094d0` in 2016 and `0x140020e40` in 2021. Mode 1
increments the current count once per gate at `+0x28`. Mode 2 copies it
from `+0x2c` to `+0x3c` and clears the
current count and the parity word at `+0x40`. The host call boundaries for
the two modes are tracked in SATD. The 2016 case `0x22ff` reads CD3DDevice
word `+0x90` and writes its boolean value to integer result 0 without
consuming operands. The device constructor `0x14001d5d0` clears the word.
Startup `0x140011320` loads `WINDOW_RESOLVING/OTHERS_DISPLAY_USE`,
normalizes it and calls the shared field setter `0x14002e850` on the device.
The Windows executable can source this setting from `CR_OthersDisplayUse`
in its registry parameter store. CrySirror uses the scoped `cmvs.cfg`.
The 2021 case returns zero directly.

The 2016 `0x2300` replaces target 0 or 1 with a square backing of 1024
pixels, or 2048 if either canvas axis exceeds 1024. `0x2301` retires the
selected target. Both consume one word and ignore selectors outside 0–1.
The draw loop binds targets 0 and 1 around groups 8 and 9 respectively,
before group 0. `0x202b` sets the record's source selector at `+6200`.
Zero uses the object's bitmap; nonzero fetches the target at selector
minus one through `0x14001c530`. Restore uses the native `06a0` records
and `0x14001dc70` to recreate backing and its recorded pixel data.
The draw queue at `0x14003de80` places eligible records with a nonzero
destination word at `+6224` in group `destination + 7`. The version 3 draw
record writer and reader store the texture source and destination words
separately at offsets 16 and 20. When the renderer binds a target, it saves
the current viewport, color surface and depth surface and restores them
after the pass through `0x140026b80`. The native color formats are
`X8R8G8B8` and `X1R5G5B5`.
CrySirror's shared persistent render targets supply destination ownership,
routed drawing and readback. The CMVS group, source, color format and save
mapping is tracked in SATD.

Both x64 profiles use four scene slots for `0x2290`–`0x2297`, at engine
`+4024` in 2016 and `+4032` in 2021. Each command replaces the selected
scene when SP−4 is 0–3. `0x2290` resolves two script string operands at SP−8
and SP−12 and consumes three words. `0x2296` selects its scene with SP−4
and consumes two words. The other six consume just the slot word.
The two loaders are `0x140040250` and `0x140052340`. Each profile keeps
its own scene initialization and pipeline behavior.

### Ground and walking scenes

`0x2290` loads ground and water names through image roots. Type `0x50`
queues ground quads centered at world depths 0, 10 and 20 with a world Y of
−0.1, then draws a translucent plane at world Y 0. The plane follows the
camera's X and Z. Beyond each 10-unit threshold only one ground segment
recycles per frame. The update follows drawing in
`0x140049c20` / `0x14005c970`. The world view uses the left-handed camera
and perspective paths at `0x140050380` / `0x140062b00`.

`0x2291` initializes the same camera to `[0, 1.5, 0]` with pitch 35°
and yaw 0°, and loads three resident sounds into slots 16, 17 and 20.
Type `0x51` queues transitions 1–3 through `0x22af`, and `0x22ae` reports
the active phase or −1 when idle. Phase 90 waits 600 ms and advances on the
next positive tick unless a queued phase takes precedence. Walking phases
cap each clock update at 200 ms, produce a single cue at their native
thresholds and retain the final camera pose. Landing stops streamed SE
channel 10. Releasing the scene through `0x22a0` frees resident slots
16–20 and stops that same channel. Types `0x50` and `0x51` have no
restorable body in the original slot format.

### Staggered quad scene

`0x2292` reclaims one slot word, in the 2016 and 2021 dispatch cases at
`0x14007d700` and `0x140090780`. Type `0x52` loads nine images through the
registered image bank. It assembles a ground plane, two large textured quads
and 24 smaller rotating quads. The latter share six textures and alternate
yaw direction over a five-second revolution. Their twelve upper groups
retain independent alpha values. The scene uses the current camera.

`0x22af` queues mode 1 or 2. `0x22ae` reports −1 while a mode is queued,
otherwise the active mode or 0 when idle. The next update after entering a
mode plays the native resident cue. Mode 1 plays slot 16 (`SE006.ogg`) and
brings in the twelve groups at 300 ms intervals. Mode 2 starts slot 17
(`SE006c.wav`), raises the large foreground quad and fades the same groups
at 300 ms intervals starting at 1,500 ms. At 7,000 ms the foreground quad
returns to its starting height and slot 17 stops. Each mode's clock caps a
frame at 200 ms. The rotation clock uses the full frame delta. Releasing
the scene frees its textures and resident slots 16 and 17.

### Rule-image panels

`0x2293` reclaims the slot word and initializes the camera to
`[0, 3.2, 0]` with zero pitch and yaw. Type `0x53` loads the two panel
images and the overlay `__try007a.pb3`. Each panel independently falls back
from `try007c3.pb3` / `try007c4.pb3` to `try007c1.pb3` / `try007c2.pb3`
when the preferred image width differs from 512. That choice also selects
the corresponding `try007d*.pb3` rule image.

`0x22af` queues keys 1–5. Key 1 starts phase 10 and reveals the panels
over 6,000 ms. The x64 pixel shader multiplies source alpha by
`clamp(2 × progress − rule_green, 0, 1)` after interpolating the rule texels.
The equivalent shared mask coverage has feather 0.5. Keys 2 and 3 hide and
show the panels. Key 4 starts phase 11 and opens them by 5° over 800 ms.
Key 5 starts phase 12 and reduces their alpha for 500 ms before hiding
them. The separate idle swing advances through 2,000 ms ramps separated
by 200 ms holds. `0x22ae` returns −1 while a key is queued and otherwise
returns the phase. Completed reveals release the two rule images.

### Rising meshes and billboards

`0x2294` reclaims the slot word and initializes the camera to `[0, 1, 0]`
with zero pitch and yaw. Type `0x54` loads five rotating planes, a
512-particle billboard texture and resident slot 16 (`st03_magic1b.wav`).
The rotation period is 6,000 ms. The billboards rise and wrap by 10 world
units. Native random draws set their origin, travel period and blink
delay. Billboards use additive blending and face the camera.

`0x22af` key 10 holds the five planes at their current growth and key 11
resumes their staggered growth without its initial 1,500 ms delay. The
first growing plane plays the resident cue. Key 20 fades the planes and
particles, and key 22 fades only particles. Both use the same retained
1,000 ms fade clock. Key 50 replaces the billboard image with
`try008z2.pb3`. The 200 ms cap applies to the scene tick. `0x22ae` reports
the native phase, or −1 while a new phase is queued. `0x22a0` releases the
textures and resident cue.

### Projected atlas actors

`0x2295` loads `ci001a.pb3`, `ci001b.pb3`, `ci001c.pb3` and resident
slots 16 and 17 (`SE035a.ogg` and `SE037b.ogg`). Type `0x55` uses the
projected sprite camera and a 256×192 atlas cell with origin [128, 96].
Each strip loops over four frames with a 100 ms frame period. The loader
and transition helpers are anchored by `0x140053900`, `0x140053640`,
`0x140054ba0` and `0x140056080` in the 2021 binary. The 2016 loader is
`0x140040b10`. Actor movement and rotation records cap delta at 200 ms.
The scene-level jump and departure clocks use the full delta.

`0x22af` keys 1–4, 6–8, 10–14 and 17 select the native choreography.
Keys 5, 15 and 16 immediately select a different atlas strip. Keys 10
and 11 can interrupt the jumping sequence at its original boundaries.
Key 17 waits for the current approach sector to change before the
departure begins. `0x22ae` reports −1 while a trigger is queued and
otherwise returns the active phase. The repeated effects duplicate
resident samples and can overlap. `0x22a0` releases both resident slots
and all three atlas images.

### Rotating quad scene

`0x2296` uses its top stack word as a slot in 0–3 and reclaims two words.
The two x64 dispatch cases are `0x14007da40` and `0x1400909f0`. Type `0x56`
loads `po_bg510c.pb3` and `it008d2.pb3` through the registered image bank,
draws an opaque ground quad and draws seven independently phased translucent
quads using the current camera. The update in `0x140047910` / `0x14005ab40`
caps the frame delta at 200 ms, advances rotation in tenths of a degree
and drives the shared vertex alpha from 255 to 192 and back over 2 seconds.
The scene has no audio, trigger transitions or native save body.
`0x22a0` releases the scene and its textures. CrySirror initializes the
rotation clock to zero and reports idle (−1) through `0x22ae`; the original
allocators do not set the clock or the extra status words read by the native
query.

### Orbiting atlas scene

`0x2297` reclaims one slot word. The two x64 dispatch cases are
`0x14007db10` and `0x140090ac0`. Type `0x57` resets the camera position,
pitch and yaw. It loads nine registered images, including the atlas for
six bands of 32 orbiting quads. Each atlas cell consumes one draw from the
shared MSVC random sequence. The angles start at 11° spacing. The first
update changes the spacing to 11.25° and applies opposite rotation to
adjacent bands. Their height wraps every 1.8 world units. Four additional
planes rotate on a separate 30-second period. The clocks cap each frame
delta at 200 ms.

The only `0x22af` trigger is key 10. It replaces two registered pictures,
reveals the overlay and shows the four rotating planes. Repeating the
trigger reloads the same two images and preserves both animation clocks.
`0x22ae` reports 0. Releasing the scene withdraws all nine texture slots.

In the 2021 executable, `MESSAGE_INFO/FONT` is a CP932 family name read
with `GetPrivateProfileStringA` and passed through font availability and
selection helpers `0x14001fcd0` and `0x140020150`. The word at `+1552` is
`MESSAGE_INFO/TEXT_REVISION_ALPHA`. In `0x22f9`, SP−4 selects read or write.
Read returns the enabled bit in system integer result 0. Write normalizes
SP−8 to a boolean and updates existing text renderers through `0x140085560`
and `0x14006c360`, invalidating their glyph caches when the setting changes.
Both forms reclaim eight bytes. The write leaves result registers intact.
The outline rasterizer at `0x14006b8e0` maps its 17 coverage bins to
`[0, 31, 60, 87, 112, 135, 156, 175, 192, 207, 220, 231, 240, 247, 252, 254, 255]`
when enabled and `[0, 15, 31, …, 255]` when disabled. The image-font branch
bypasses both tables. CrySirror maps shared 4×4 outline coverage through
these tables and keys cached masks by the raster profile. Already emitted
glyphs retain the profile used to draw them. The menu reflects the setting
at ID 32931. The 2016 dispatcher routes `0x22f9` to its original no-op.

## CMVS x64 script containers and VM

| Boundary | Layout and evidence | Implemented path |
| --- | --- | --- |
| Disk script | `PS2A` and a 48-byte header, then transformed payload. In order, the decoded segments are u32 little-endian labels, code, mutable script data and raw text. `.ps2` and `.ps3` can both carry this container. | Bounded disk decoding and segment validation. A CSV2 memory image uses a separate, already-decoded layout. |
| Compressed payload | Subtract `0x7c`, XOR with the low byte of `(key >> 3) + (key >> 24)`, rotate right by `((key >> 20) % 5) + 1`, then expand 11/5 LZSS when the packed size is nonzero. | Shared `crysirror-resource` LZSS with exact input/output sizing. |
| Instruction stream | Little-endian u16 opcodes, with PCs and direct targets measured in bytes from code start. Label operands are u16 indices into the label table. | One-instruction structural decoding. Raw operands are preserved for later dispatch. |

| Opcode or family | Decoded operands and native behavior |
| --- | --- |
| `0200`–`0202` | Integer, string-reference and float expressions. The layouts contain tagged postfix terms and terminate at `020f`. Nested indices use integer expressions. Raw literal and reference bits are retained. |
| `0400`, `0401`, `0402` | Direct or conditional branch with a u32 target. |
| `0403`, `0407` | Two u32 operands, with the target in the second word. |
| `0405`, `0410` | Label index. `0410` saves `PC + 4` before branching to the label. |
| `0411`, `0412` | u16 byte-count stack operand. `0411` pops a same-script return PC and the indicated argument bytes, then leaves the call frame. `0412` discards the indicated bytes. |
| `0413` | No encoded operands. Pops the same-script return PC and then the low 16 bits of `result × 4` argument bytes. Leaves the call frame. |
| `0416` | No encoded operands. Calls the current script's byte offset in the 32-bit result register, pushes PC+2, and establishes a new call frame. |
| `0430` | No encoded operands. Pushes the 32-bit result register. |
| `0414` | u16 tail operand for the script-return layout. |
| `0440`, `0442` | u16 stack byte count and u32 immediate. |
| `2000`–`27ff` | Low 11 bits select a native system command. A decoded selector carries no guarantee of a registered handler. |

The x64 VM dispatch jump table at RVA `0x67d58` and index table at
`0x67d90` route `0413` to `0x140067b9f` and `0416` to
`0x140067c2d`. The decoder recognizes the 18 explicit non-system
instruction IDs in this dispatcher. Evaluation of each expression
operand type and operator is a separate compatibility boundary.

## Executable startup slice

The following operations are connected to the CMVS session's logical state.
The parameter stack is byte-addressed, with arguments stored as little-endian
32-bit words. Native globals start zero or empty for this build. The bank
limits are 4,096 integers, 262,144 packed bits, 2,048 floats and 128 strings of
1,024 bytes. The integer expression-result slot `4096` is separate from the
ordinary integer range.

| Opcode | Guard and effects | State or next boundary |
| --- | --- | --- |
| `0200` | Evaluate the reviewed integer literals, variables, label and frame/data operands, and operators. Write result and slot `4096`, set condition from nonzero. | Unhandled descriptor forms fail at their native offset. |
| `0201` | Evaluate text/variable references and the accumulator/assignment operations. Preserve the numeric condition outside nested evaluation. | Keep CP932 text as bytes. Script-data and frame-local writes require bounded space for the full C string and roll back with the expression. |
| `0202` | Evaluate the reviewed numeric tags/operators and preserve result bits. Set the condition for a nonzero or unordered result. | Comparison flags follow the x64 COMISS/UCOMISS instruction sequences. |
| `0430` | Push the expression result by four bytes. An uninitialized result or exhausted stack is an error. | The next instruction sees the updated SP. |
| `2000` | Return `0xc000`, advance two bytes and yield with no stack reclamation. | Caller starts the next native slice later. |
| `201d`–`201f` | Read SP−4, write the game ID, startup field `+12236` or archive mode through the `0x140008040` policy. Return `0x4004`. | Mode `10` selects CPZ6 in the sample. |
| `2090`, `234d` | Set `+12084` or subobject field `+1532` to 1 and return `0x4000`. | VM retains the flags for later consumers. |
| `2091`–`2094` | Read start, count and value at SP−4, SP−8 and SP−12. Validate the entire range, then fill bits, integers, raw float words or string cells. Return `0x400c`. | String fill accepts supported native reference regions. |
| `2013` | SP−4 holds the BGM source slot and SP−8 holds a path reference. The sample first reaches this request at code PC `0x00de`. | Register the scoped directory or archive source. Playback is a separate host operation. |

The syscall result word uses bit `0x8000` to yield, bit `0x4000` to advance
the PC and bit `0x2000` to increment the repeat counter. The low byte gives
the byte count to reclaim from the parameter stack when advancing. The startup
slice advances and reclaims operands only after a connected operation succeeds.
Unsupported VM instructions report the code PC. Unhandled system calls return
their opcode and PC with the stack unchanged. The remaining host bindings
include text, Layer, UI, audio, input and media. The pinned binary inventory lists 976 dispatcher case slots, which
include default/alias entries and give no count of independently usable syscalls.

For string expressions, `0120` reads the loaded text segment and
`0121` reads or writes a string-bank slot. `0122` loads a tagged reference
from a frame argument and does not write to that argument. `0125` and
`0127` read or write C strings at a script-data byte offset or a frame-base
positive offset. The resolved high bits of a reference choose the same
storage regions. Untouched stack reservation bytes stay unknown.

The integer and float evaluators keep array operands as lazy descriptors.
Fixed-stride forms add a 32-bit index times four to the base. Strided forms
add index times the encoded unsigned stride, and two-index forms add the
second index times four. Offsets wrap at 32 bits and are interpreted as
signed byte offsets by the frame and script-data readers. The integer
reader converts float descriptors using CVTTSS2SI. Its `012a` immediate
and the float writer's integer frame/data destinations use the low word of
the 64-bit conversion. Integer writes to `010f` affect registers 0–6.
Integer reads from `010f` also convert float registers 7–10. The float
writer preserves raw bits for those float results.

### Arithmetic syscalls

The entries from `20d5` through `20dc` advance and reclaim four bytes per
operand. Operands in the table start at SP−4 and proceed down the stack.

| Opcode | x64 handler RVA | Operands and effects |
| --- | --- | --- |
| `20d5` | `74b50` | Start and end. Advance the same MSVC random stream as `20d4` and return `start + rand() % (end − start)` in integer result 0, with wrapping subtraction and addition. A zero divisor is a VM error. |
| `20d6` | `74ba0` | Base and signed exponent. Return wrapping 32-bit exponentiation in result 0. For exponents at most 1, return the base. |
| `20d7` | `74bf0` | Value, scale and mode. Mode 0 returns the wrapped signed product divided by 1000. Modes 1 and 2 square `sqrt(value) × scale / 1000` and truncate with CVTTSD2SI. Mode 2 subtracts that result from the input using wrapping arithmetic. Other modes consume the operands and keep result 0. |
| `20d8`, `20d9`, `20dc` | `74cd0`, `74d10`, `74e10` | Signed angle and scale. `20d8` converts degrees to hundredths with wrapping multiply. `20dc` takes only the angle and fixes scale at 4096. Write scaled cosine and sine to integer results 1 and 2 through CVTTSD2SI. |
| `20da`, `20db` | `74d50`, `74dc0` | x1, y1, x2 and y2. Use wrapping coordinate differences and `atan2(dy, dx)`. The helper converts with negative π before truncation, then reverses the angle modulo 36000. Result 4 is the angle in hundredths for `20db` and whole degrees for `20da`. Result 5 is truncated `dx / cos(angle)` or wrapping `abs(dy)` when `dx` is zero. |
| `20e0` | `74e50` | Six raw float words `x, y, z, pitch, yaw, roll` in pop order. Convert degrees to radians in binary32, then rotate about Y, X and Z in that order using binary64 intermediates. Scale each output by 100, truncate to a signed 32-bit integer and divide by 100 in binary32. Write raw float results 8, 9 and 10 and reclaim 24 stack bytes. |

The rotation helpers divide the angle by 18000 and use the native binary32 π
constant widened to binary64. Native math-library parity near integer
truncation boundaries remains to be checked across host platforms.

### Byte-string syscalls

These entries resolve string arguments through `0x14006b850`. Operands
below are in SP−4, SP−8, SP−12, SP−16 order. Result means system integer
register 0. All routes advance with flag `0x4000` and reclaim four bytes
per operand. `20fe` consumes zero bytes and has no effects.

| Opcode | x64 handler RVA | Operands and effects |
| --- | --- | --- |
| `20f0`, `20f2` | `743a0`, `74470` | Destination and source. Call `lstrcpyA` or `lstrcatA` and leave the result register alone. |
| `20f1`, `20f5` | `74400`, `74690` | String and comparison string. Return 1 on ASCII-case-insensitive equality or a prefix match, respectively. Valid CP932 pairs compare byte for byte. |
| `20f3` | `744d0` | Destination, source, start byte and byte count. Copy complete encoded characters whose starting bytes precede the signed end offset. Return the number of characters. Leave the destination alone for an empty interval. |
| `20f4` | `745a0` | Destination, delimiter and source. Copy up to the first matching byte or CP932 pair of the delimiter. The search advances one byte per candidate. Return the byte offset, or 0 after copying all bytes with no match. |
| `20f6` | `74700` | Text to integer. Lowercase `0x` chooses hexadecimal. Decimal treats nondigits as zero digits and a minus anywhere negates the result. Accumulation wraps at 32 bits. |
| `20f7`, `20f9` | `74740`, `74800` | Source and a string whose first byte is the needle. Return the first or last matching byte offset, or `0xffffffff`. The terminator is excluded. |
| `20f8` | `747c0` | Return `lstrlenA` byte length. |
| `20fa` | `74880` | Destination and signed word. Format with the native fixed pattern `%d-`. |
| `20fb` | `748c0` | Return the weighted sum of signed bytes at offsets 0, 1, 2, 3, 5, 6, 8 and 9, with powers of ten from `10^7` to `10^0`. The original includes the ASCII digit bias. |
| `20fc`, `20fd` | `74990`, `74a00` | Destination, pattern and one integer or string argument. The formatter at RVA `6ed0` receives only that argument. |
| `20ff` | `74a80` | Destination and new extension. Replace bytes following the last single-byte dot, or append a dot and the extension. Skip lead/trail pairs when looking for a dot. |

CrySirror bounds string syscall writes to the resolved string-bank, script
text, script-data or frame region. Native resolver `0x14006b850` exposes the
loaded text bytes to native calls. The VM copies the text segment on the first
write, and again only if a checkpoint shares the modified backing. Bytecode
and the source archive remain immutable. String expressions retain their own
destination rules. Native copies into bank cells (`0x140085e50`) write a
terminator and keep the following bytes intact. `20fb` reads the whole fixed
range even when it contains a NUL, with region bounds and frame initialization
checked before a result is published. CrySirror rejects unsupported overlap.
The formatter handles literals, `%%`, `%c`, `%d`, `%u`, `%z`, `%x`, `%o`,
`%s` and `%r`. `%x` and `%o` emit zero-padded, fixed-width digits and consume
the argument even at width zero. `%r` preserves existing destination bytes
from the current output offset up to their next NUL and takes no argument.
`%ld`, `%lu`, `%lx` and `%lo` still read a 32-bit argument. Uppercase numeric
letters and `%S` use the original formatter's CP932 width mapping. `%S`
counts an explicit width as twice that many output bytes and pads with ASCII
spaces. The signed-byte check at `0x140006e2b` leaves single-byte kana unchanged.
`%t`/`%T` read the variadic word as signed i64. The integer caller loads it
through `r8d`, so its upper half is zero. At width zero the signed formatter
starts with the wrapped `10^19` divisor and divides it by ten between output
digits. That wrapped negative divisor can produce a negative digit index.
`%f`/`%F` read
the same zero-extended word as binary64. Every representable input is zero or
a positive subnormal and produces zero integer and fraction digits. The native
grammar accepts `f[+-]width.precision`; fraction scale wraps at 32 bits. CrySirror
rejects scratch underflow, invalid divisors, pointer reinterpretation and reads
of uninitialized native arguments at the syscall PC.

## Control transfer and resource lifetime

The machine validates instruction boundaries for entry, labels and taken
transfers. Integer and numeric frame/data stores are journaled until a complete
expression succeeds. Parameter stack reservations retain unknown bytes outside
the immediate word. Ordinary calls save a return PC and establish a frame base.
Cross-script loads are distinct: `2081` resolves a script through the registered
script sources, pops two arguments and saves repeat, PC+2 and caller slot on the
byte stack before entering a common slot. `0414` restores that context. `2080`
prepares replacement slot zero before the owner captures the shared scene.
It preserves common slots and globals, clears the stack, call frames and repeat
count, and drops callbacks owned by the replaced slot. The in-memory native
copy has bit zero set, and the admitted VM has bit zero cleared. The old script
remains active until preparation and checkpoint capture both succeed.
The `22b2` scene checkpoint uses a detached VM copy at PC+2 and resets its
repeat count. Bit 0 is set in the retained copy and cleared in the live
VM after capture. It retains the scoped shared scene and the supported
audio source and transport records. Persistent CSV2 serialization and
restoration are separate boundaries.

`2003` loads a fresh `main.ps3` image from the registered script sources,
preserving admitted globals, registered sources, common scripts and common
callbacks. The session drops its main checkpoint and scene objects,
movies, audio, text/history, sequence header, and clear/camera state. The
native dispatcher returns zero because the replacement supplies its own
entry PC and empty parameter stack. The source and image must pass
preparation before the owner starts cleanup.

`2010`–`2014`, `2019` and `201a` register game-relative resource paths in native
slot order. A trailing separator denotes a directory. A CPZ6 source uses the
shared [mode 1 decoder](../crysirror-formats-arc/src/legacy/f/cpz6-mode1.md).
`2015` and `2016` set the loose movie root and writable save suffix respectively.
The save capability remains separate from game assets. The CPZ6 table is read
from the selected ABI's file-backed `.rdata` range at RVA `0xd9c40` and remains
inside the client's decode context. The byte table and game assets are private.
Reading it needs no executable hash gate or native code execution.

`2007` calls `0x140007d80` (multimedia tick count minus the boot origin,
wrapped to unsigned 32 bits and divided by 1000). CrySirror samples its
owner's monotonic clock at the syscall and records the origin at session
publication. It returns seconds in system integer result 0 with no operands.
`200d` calls `GetLocalTime` at `0x14006ba70` and puts `YYYYMMDD`, `HHMMSS`
and Sunday-zero weekday in integer results 4, 5 and 6 without operands. A
client-scoped CivilTime task samples this clock. The pending syscall and its
PC stay in place until the task returns, without advancing the VM frame.
`201b`, `20e9` and `23b5` all branch to `0x1400825b1`. They write zero to
integer result 0, advance, and consume no operands.

The `2008/2009` commands read the current VM frame time/delta. `200b/200c`
reset or query one of ten wrapping millisecond counters and reclaim one word.
`200e/200f` mark and query the independent stopwatch.
`2088` registers an owner slot, target PC and three native words, reclaiming
20 bytes. `2089` and `208c` select automatic callback dispatch and its inhibit
flag. `208a` invokes the registered `(slot & 63)` target using a cross-script
return context. `208b` disables an entry. A cooperative instruction budget or
an asynchronous I/O poll does not begin a new VM frame or advance its timers.
The headless path dispatches registered per-frame callbacks for the supported
startup state. Message-window and close-event scheduling depend on the native
message and input mappings.

## Title controls and child objects

The PC 20160201 x64 dispatcher uses `2022` and `2023` to replace or
remove a child of an existing scene slot. `2050` resets its draw record.
The draw setters `2044`–`2048` select the parent with a negative child
word and select one of 768 child indices otherwise. A child created under
an initialized parent inherits position, opacity and the parent's atlas
when its own image is absent. Its draw order is within the parent's queue
entry. Objects created before parent initialization use independent order.

`2031` (`0x14006c760` → `0x14003aa30`) releases the scene slot's
own image at SP−4 and leaves the object and draw records allocated.
An already empty image slot succeeds. A missing or invalid object latches
frame error bit `0x10`. The result registers are preserved and the command
returns `0x4004`.

`2321` (2016 `0x14006da70`, 2021 `0x140093f30`) writes the draw
record's DWORD at +88. Its stack words are scene slot, signed child and
mode. Negative child values select the root. The child limit is 768 in
2016 and 1024 in 2021. Missing objects and out-of-range child indices
latch `0x10` and `0x20` respectively. The handler preserves the result
registers and reclaims 12 bytes (`0x400c`). In ordinary 2D drawing with
the second texture, mode 4 multiplies source alpha by the screen-space
matte before source-over blending. An inheriting child uses the parent's
matte in mode 4. The draw record retains this mode across checkpoints.
The 2021 CG viewer sets mode 4 on slot 11 at `omake.ps3+0x50074`.

| Call | Stack words in native order | Completion |
| --- | --- | --- |
| `2087` | merged-state flag | Changes how button image records are assigned. |
| `2210`, `221b` | group, scene slot, and for `221b` pointer dimensions | Replaces one of six groups. |
| `2211` | group | Releases the group's button records. |
| `2212` | group, ID | Appends a node with insertion-order links. Result 0 is allocation success. |
| `2213` | group, ID, x, y, w, h | Sets the first matching node's half-open hit rectangle. |
| `2214` | group, ID, state, child, crop x/y/w/h, origin x/y | Stores a state record with signed 16-bit geometry. |
| `2215` | group | Applies initial image records and resets the current button. |
| `2216`, `2217` | group | Queries hover or processes navigation, hover and confirm/cancel edges. Result 0 contains the button ID or native sentinel. |
| `221a` | group, ID, warp flag | Selects an enabled button and reports success. |

The calls advance and reclaim four bytes per argument after the shared
Layer batch accepts the affected atlas geometry. Hit testing uses the
shared half-open rectangle helper. The engine owns image resources and
layer rendering.

## Movie and camera records

Fullscreen movie `20c0` starts the scoped file from the `2015` root
and returns an open result in system register 0 after the first picture
is published. `20c1` removes the fullscreen slot, and `20c2` reports its
frame index or -1 when absent/ended. `230a` removes one of three separate
scene movie slots and succeeds when the slot is already empty.
CMV demux, JBPD/JBPE decoding, Ogg sound and picture clocks belong to
CrySirror Video/Media. The original frame query uses the media
owner's published frame count. Camera settings are retained as native
words while the implemented 2D draw modes require no projection.

## Message window records

`2140` and `215d` allocate a window (slot 0–7) and its referenced text
record (slot 0–11). Arguments name the slots followed by x, y, width
and height. `215d` additionally supplies logical width and height.
`2141` removes the window and its referenced text record. `214b`
queries existence, `214c` applies temporary hiding when shown,
`214d` queries temporary hiding and `214e` changes the shown flag.
`2142`–`2149` retain the native text order, dimensions, pen origin,
colors, timing and decoration rules. `2150`, `215a` and `215b`
address the two native position records and the active position.
`215f` tests a half-open rectangle relative to the window position.
It scales the pointer from the canvas to the window's logical dimensions
using the original unsigned multiplication and division.

Window image loads (`2170`) use the same scoped image path as `2030`.
Window objects and children share the retained layer bridge. `2171`
releases just the window image. `2178` replaces the selected object and
hides it, `2179` removes it, and `217b` queries object presence.
`217a` sets its visibility,
`217c` its logical size and `217f` clears its quake-follow flag.
`2180`, `2182`–`2186` initialize or change native draw records.
`2190`–`2192` store button image records, hit geometry and enable
state. `21cf` replaces the window object's atlas animation record,
`21d0` assigns one of 64 linked drawings, `21d1` chooses the first
record, and `21d2` drops the animation while retaining its last draw
properties. Negative origin components preserve the previous value.
Native frame boundaries advance durations up to the 200 ms delta cap.
Unknown commands retain their PC. All listed calls reclaim four bytes
per input word on completion. `2194` returns window button hit results
in system registers 4–6, and `2195`–`2197` query or apply button states.
`218d` copies opacity, position and order to integer registers 0–3
and local scale to float registers 8–9. `20ec` applies a uniform scale
to the window and text. The inclusive [0.95, 1.05] interval snaps to 1.0.

`211f` scans text using the current text slot's cell widths and line
break rules. It returns success, maximum line byte count, number of
lines and last line byte count in registers 0, 1, 2 and 4. It keeps the
live pen position. `2155` stores the raw per-glyph parameter.

`2152` and `2157` append once at repeat zero. They wait at the current
instruction while the glyph reveal is active and reclaim all three
argument words at a yielding completion. Only `2152` honors skip and
confirm for early completion. The cells decode CP932 and publish
explicit positions to the shared positioned-glyph renderer. Ruby layout,
line breaks and the native glyph reveal descriptors stay in the adapter.
`2153` waits at the page, routes mouse releases past enabled window
buttons and clears the wait cursor on advance. `2158` uses an independent
unconditional auto timer and the same glyph-count and voice-wait durations.
`2112` returns ruby-aware line counts and `2119` returns the native
encoded display byte count, excluding escapes and ruby annotations.
`212f`–`2136` own the
128-entry, newest-first history and its native fields.

`2225` starts the supported step or damped-cosine quake, `2226`
queries its remaining count, `2227` pauses or resumes it, `2228`
waits for its completion or cancels with a frame yield, and `2229`
releases it. `220c` reports whether an existing quake is paused.
The sampled offset is applied in canvas pixels to qualifying shared
scene and message-window image layers. Clearing quake-follow on the
parent prevents its layer and children from receiving the offset.

The VM updates gesture mode 0 (button hold and flick), mode 1 (region
drag) or mode 2 (unrestricted left-button drag) after refreshing frame
input. A completed hold may suppress ordinary action edges. `2362` reads
the hold state without consuming it and returns true only for selector 1.
`2353`, `2359`, `2360`, `2365` and `2368` expose the native flick,
wheel, hold, drag and release records.

`2364` copies the raw mode word and resets tracking through
`0x140053630`. Values outside 0–2 leave the recognizer inactive.
Mode 2 (`0x140054570`) starts tracking at any left press while the
gesture controls are enabled. Either axis crossing the configured
pixel threshold activates the drag. The activation frame reports zero
deltas. Subsequent frames measure from the press anchor. An active
release consumes the action edges and sets status 1 for that frame.
A release below the threshold remains an ordinary click.

`236a` reads the mode 2 release status, angle and speed into integer
results 0, 4 and 5 and pops no operands. The angle is the original
scaled-vector angle plus 90 modulo 360. Speed is the unsigned
wrapping product of 16 and the distance in tenths of a pixel divided
by the unsigned elapsed milliseconds. At zero elapsed time or when
either coordinate matches the press anchor, the measured angle and
speed are zero. `236b` consumes one threshold word, accepts 1–128
inclusive and preserves the previous setting for every other value.
The default threshold is four pixels. IDA resolves these handlers
to `0x140080020` → `0x140053760` and `0x140080070` → `0x140053600`.

`222a`–`222d` and `2234`–`2237` provide two native mask tracks. Each
loads the rule bitmap asynchronously, samples remaining time, and
commits a Cut by removing the target slot. The shared Layer mask uses
green-channel coverage with feather 0.5, yielding
`clamp(2 × progress − mask, 0, 1)`. Cut inverts the bitmap RGB with the
shared image effect and reverses progress. Simultaneous tracks require
distinct targets. The legacy CPU-device mask path is unmodeled.

### Text settings

The x64 handlers below read a mode at SP−4. Mode zero queries the
session setting and writes integer result 0. Any other mode writes using
SP−8 and preserves result 0. Every branch advances and consumes 8 bytes.
The speed and reveal handlers read SP−8 even on queries.

| Opcode | Handler RVA | Setting and write policy |
| --- | --- | --- |
| `22de` | `7c2d0` | Write the current default face from a CP932 reference. Query copies the face to the supplied writable reference and returns 1. Existing font objects keep their selected faces. |
| `22df` | `7c340` | Outline mode, signed wrapping absolute value modulo 3. |
| `22e0` | `7c3c0` | Bold, normalized to a boolean. Existing default-face text objects use the new weight for subsequent glyphs. |
| `22e1`, `22e2`, `22e3` | `7c410`, `7c480`, `7c4f0` | Text speed, reveal mode and reveal duration index, clamped as signed integers to `0..=10`, `0..=2` and `0..=4` respectively. |
| `22e4`, `22e6` | `7c560`, `7c620` | Skip read text only and auto-advance voice synchronization, normalized to booleans. |
| `22e7`, `22e8`, `22f8` | `7c670`, `7c6b0`, `7c6f0` | Auto-advance base delay, per-glyph delay and post-voice delay, stored as raw words. |
| `2106` | `728a0` | Text slot at SP−4 and face reference at SP−8. Rebind the slot's selected font, with an empty name selecting the current default. Advance and preserve the result. |

The message adapter resolves available names through the scoped font library.
Missing names use the engine default font. Each positioned glyph keeps its
alias and weight at creation, so changing either setting preserves existing
glyphs. Save checkpoints retain the explicit font aliases they use. GDI face
enumeration, native glyph coverage, the font picker and settings writeback
remain separate compatibility work.

`22d5` (`0x14007bee0`) uses the same mode/cleanup convention for the
`SYSTEM_INFO.ACTIVE_DRAW` boolean. The original polls audio and movies
before checking focus or active draw. CrySirror continues polling those
services while the script and scene frame are suspended, drops inactive
input edges and clears the elapsed frame baseline. A host with no focus
signal keeps running. The startup INI default is false.

### Live audio settings

`22d6` and `22d7` (`0x14007bf50` and `0x14007bfc0`) use the
mode and eight-byte cleanup convention of `22d8`. Changing `MUSIC_USE`
stops the two BGM channels if a track was playing and retains the current
names. Commands issued while disabled can replace those names. Re-enabling
reopens that track or intro/loop pair from the current BGM resource roots,
starts channel 1 at zero, and enables looping without a fade-in.

Changing `SE_USE` to false clears live effect channels 9–14 but retains
their six script records. Re-enabling opens the named records from the
current effect roots. It starts previously requested looping sounds and
prepares the other sounds paused. A non-looping record's start flag is
cleared. Missing resources leave that channel empty. Preparation and
publication preserve unrelated BGM, voice and movie channels. Settings and
stack cleanup follow commit of the replacement batch. The same publication
restores the per-slot gain multiplier and pan without starting an audibly
unconfigured voice. The multiplier is gain, as shown by
`0x140087af0` writing voice field +0x98 and the mix code at `0x140086e43`
multiplying it into amplitude without changing source advancement.
Effect commands `20b8` and `20b9` write their words both to device slot
9 + index and to the corresponding script record at +0x6a0 or +0x6a4.
They read an unsigned index from SP−4 and the setting from SP−8, consume
eight bytes and keep the result registers. Indices above 5 set error
`0x200`. The reopen helper restores the saved multiplier only if its
signed value is less than 256 and restores pan only if it is nonzero. A newly
loaded SE resets both settings. Voices have analogous
live setters `20ac` and `20ad` on slots 5–8, with error `0x400` on an
invalid index and no saved effect record. Stereo pan helper
`0x140087b30` leaves one side at 256 and subtracts the magnitude of the
signed word from the opposite side: positive attenuates left, negative
attenuates right. The mixer's unsigned compare applies attenuation only
when that coefficient is below 256.

In the unity `+0xa0` and device-global gain case, the native software
mixer multiplies the current base level by the raw multiplier and 256 in
wrapping 32-bit arithmetic, shifts right 16, then multiplies and shifts by
each pan coefficient in turn. The unity clamp is per side and follows the
pan product. Finally it multiplies signed PCM16 by the side coefficient
and shifts arithmetically right eight. The shared mixer applies these
steps at the voice fader, with its own PCM and fade clock. Native
device-global attenuation from `20a5` (`0x140070380` → `0x140087b10`)
consumes one word without changing a result.
The unsigned value caps at 256 and writes device field +0x974. That
field defaults to 256 and applies to the software mix of all sixteen
slots at `0x140089de0`, between the secondary and pan shifts. The
shared mixer retains the value for the client and applies it to live
and future CMVS software voices, including BGM and fullscreen movie
audio. The movie wrapper passes the same device to `0x1400379f0`, whose
fullscreen track uses device slot 2. The hardware-slot helpers
`0x140086ac0`, `0x140086af0` and `0x140089288` use the stored field when
starting an output. The setter itself leaves already-playing hardware
output alone. The shared adapter samples this field on each hardware play
command and retains the voice's gain afterward.
Separately, the BGM
ducking state machine at `0x140087df0` inspects voice slots 5–8 and
updates +0xa0 for slots 0 and 1. It leaves the voice, SE and independent
slots at unity for that stage.

`22d8` (`0x14007c030`) reads or writes `VOICE_USE`. Writes affect subsequent
voice requests and leave sources already playing alone. A query fetches only
the mode at SP−4. Every branch advances and consumes eight bytes.

`20a7` (`0x1400705f0` → `0x140088030`) takes voice index 0–3
and a nonzero loop flag in that order, consumes eight bytes and preserves
the result registers. It starts the existing prepared voice on slot 5–8
even if `VOICE_USE` changed after loading. The helper leaves an absent
or already active slot unchanged, and never seeks or reloads the decoder.
CrySirror also retains the decoder after EOF. Resuming it can cancel a
pending retirement or, if exhausted, report active until the next mix
without rewinding or emitting PCM. Invalid indices set error `0x400`.

`20ab` (`0x1400707f0` → `0x1400878a0`) reads one voice index,
returns −1 when the voice is absent or inactive, and otherwise returns the
decoder's cumulative PCM byte counter at +0xb4 in result 0. The counter
starts at zero in `0x140087667`. In `0x14008940b` the ordinary decoder
adds its returned input bytes before mono and sample-rate conversion. The
mono time-stretch branch adds the requested byte count at that step. This
query still needs a matching shared observation. It consumes four bytes,
and an invalid index sets error `0x400` while keeping the result.

`22d9`–`22dd` (`0x14007c0a0` through `0x14007c260`) fetch both mode and
SP−8 on reads and writes, then advance and consume eight bytes. A mode-zero
query writes result 0. Writes clamp signed inputs to 0–256 and preserve the
result. Helper `0x140017750` applies the master, music, effect, voice and
movie scales to the respective live channels. The master and category scales
multiply before shifting right eight bits. An effects-only change also
multiplies a positive per-slot volume word at `+0x69c + 0x34 × slot` before
shifting. The master path skips that extra multiply. A computed value above
256 leaves the live channel alone. Disabled music keeps its live gain.

The adapter submits live groups and retained media gains in one scoped batch.
Transport fades and decoder position survive a change. Source settings and VM
completion follow acceptance of the batch. Movie creation uses the current
setting. These settings still need native persistence.

### Independent device channel

The x64 wrappers `0x1400712b0` through `0x1400714b0` take an unsigned slot
at SP−4. Zero selects hardware channel 15, separate from BGM, voice and the
six regular effects. Nonzero sets engine error bit `0x200` at +0x2f18,
preserves result registers and takes the same cleanup path. The desktop
adapter surfaces the error at the command after completing its stack
cleanup. The original reports the error after the VM returns to the
window loop (`0x14000e5fa`).

| Call | Following operands in stack order | Action and results | Bytes |
| --- | --- | --- | ---: |
| `2240` | Sound reference, unsigned gain, fade-in milliseconds, loop flag, immediate-start flag | Stops the previous channel before attempting replacement. Uses the `2012` sound archive/directory roots at +0x12b8 and forbids the alternate-directory branch of `0x140088390`. An empty or missing sound and a native name of 32 bytes or more return 0 in result 0. A prepared sound returns 1 and remains paused if immediate start is zero. The initial gain is the low-word product of master and operand shifted right eight, with levels above 256 leaving the default 256. The SE switch and SE category gain do not apply. | 24 |
| `2241` | — | Stops and releases channel 15. Stopping an empty slot has no effect. No result change. | 4 |
| `2242` | — | `0x140087870` returns −1 in result 0 unless the slot is playing. Otherwise it gives elapsed milliseconds from `0x1400877b0`, with zero before the first mixer fill. Looping does not reset it. The adapter uses the shared mixer transport clock, which can differ from the original device cursor. | 4 |
| `2243` | — | Writes the stereo mean absolute signed-16 PCM levels to result 4 and 5. `0x140086db0` records one pre-gain average per channel per block in a three-entry ring. `0x140087a70` reads the ring at the next write index. Missing or inactive slots return two zeroes. | 4 |
| `2244` | Unsigned Q8 multiplier | Writes voice +0x98 through `0x140087af0`. An empty slot is unchanged. The initial multiplier is 256 and a new sound resets it. No result change. The shared fader retains the onset fade and evaluates the wrapping multiplier at each quantized level. The native device fade clock still needs parity evidence. | 8 |

`20ae` uses the same level reader as `2243` for voice slots 5–8.
With an invalid voice index it sets error `0x400` and retains both results.
Both meter wrappers consume one word. Startup at `0x14000a9f4` selects
22,050 Hz for `SOUND_QUALITY=0` and 44,100 Hz for nonzero, then
`0x140089871` selects windows of 1,536, 2,048, 3,072 or 4,096 stereo
frames at 44,100 Hz for `SOUND_BUFFER` 0–3. At 22,050 Hz the counts are
halved. The INI fallback at `0x140013d2a` defaults both settings to zero,
and `0x140013e93` caps the unsigned buffer index at 3. The first two
completed windows still read zero because the ring starts empty.
The shared mixer uses these same durations on its own PCM clock. It
measures the rendered source before per-voice DSP and gain. Resampling
and buffering can differ from the original DirectSound path.

### Hardware sample slots

The x64 hardware sample catalogue is separate from the sixteen software
voices. The wrappers below select one of 24 entries at audio object
+0xa28 using an unsigned index at SP−4. A higher index sets error
`0x200` and retains the result. Each loaded slot owns one resident PCM
allocation, shared by its primary and duplicate playback instances.

| Call | Wrapper and helper | Contract | Bytes |
| --- | --- | --- | ---: |
| `2248` | `0x140070950` → `0x1400890c0` | Reads a filename reference at SP−8, releases the previous slot and prepares a resident sample from the `2012` roots. Names 32 bytes or longer and missing/invalid samples return 0. Success writes 1 in result 0. The original loader falls back to an 8 MiB PCM16 read when the decoder's byte size is unknown. | 8 |
| `2249` | `0x140070a20` → `0x140089010` | Destroys the primary and all 32 duplicate instances, then releases the sample buffer. | 4 |
| `224a` | `0x140070a70` → `0x140086af0` | Reads signed local gain at SP−8 and a nonzero loop flag at SP−12. If SE is enabled, starts or resumes the primary at its current cursor and sets the loop flag. Its gain is master × SE, with additional positive local gain and device attenuation. | 12 |
| `224b` | `0x1400709d0` → `0x140086b30` | Stops the primary and retains its cursor and allocated sample. Duplicates continue separately. | 4 |
| `224f` | `0x140070b00` → `0x140086ac0` → `0x140086370` | If SE is enabled, starts a nonlooping duplicate in the first free entry of a 32-entry table. Full and empty slots leave playback alone. It uses master × SE × device attenuation independently of the primary's local gain. | 4 |

Native `0x1400869d0` and `0x140086370` convert the computed Q8 word to a
DirectSound volume of `trunc(4000 × log10(level / 256))` hundredths of a dB,
using −10000 for zero. The adapter converts the accepted setting back to
linear amplitude. A primary with an unchanged cursor can change loop flags
on replay, as documented by Microsoft for secondary buffers. The shared
mixer starts a fresh instance when replaying a primary after natural EOF.
DirectSound-specific duplicate gain behavior, driver admission limits,
original decoder parity and the exact EOF cursor still need validation.
[Play](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418074(v=vs.85)),
[Stop](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418151(v=vs.85)),
[DuplicateSoundBuffer](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418041(v=vs.85)),
[SetVolume](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee418150(v=vs.85))

### Software voice controls

IDA 9.0 with Hex-Rays established the wrapper, device and decoder paths
below from the PC 20160201 x64 executable. The software mixer thread
at `0x140089d20` processes the source at `0x1400892f0` and releases
retiring slots through `0x1400876e0` when `0x140086db0` returns zero.

The SE pause `20b2` (`0x140070ee0` → `0x1400881e0`) takes one
unsigned slot index 0–5. It clears the active flag of a playing device
slot and clears the script record's started word, preserving the sample
name and decoder cursor. The SE restart `20b4` (`0x140070ff0` →
`0x1400880f0`) takes the index and loop flag, consumes two words and
rewinds an existing device decoder through its method at vtable +8
before starting it, even if the slot was active. Its live action is
gated by `SE_USE`. For a valid index it writes the loop and started words
regardless of that gate. Both commands preserve integer results and set
error bit `0x200` for invalid indices.

CrySirror parks the input transport with its decoder and gain state intact.
Restart waits for the shared seek to publish before advancing the PC. This
preserves the native synchronous call's script ordering without adding a
native repeat or admitting a script frame. It then applies the loop flag,
starts a new playback interval and enables transport. A disabled or empty
slot changes only the remembered loop and start words.

Software voice release has a separate two-command pair. `20bc` sets
device slot 5 + index to retirement state 3 through `0x1400879a0`.
`20bd` polls for the slot pointer to become null via `0x1400879d0`.
The mix worker releases the pointer when it next handles the retiring
voice. The wait returns `0xa000` until release, retaining its PC and
operand and incrementing the repeat counter. Once released, it returns
`0x4004`. Both wrappers accept indices 0–3 and preserve the result,
with error bit `0x400` on an invalid index. The invalid wait follows
the same `0xa000` path.

CrySirror marks the scoped voice and retires it on the next nonempty
mixer render. The owner wait repeats with its operands preserved until
that render publishes the released slot. For `20a7`, `20b2`, `20b4`,
`20bc` and `20bd`, the adapter latches the invalid-slot bit before
applying the native cleanup or wait. The native frame loop checks this
error word at `0x14000e5fa`.

`20aa` and `20b7` call `0x140087870` and `0x1400877b0` to read
elapsed playback time. The original reads the DirectSound play cursor,
accounts for its per-slot buffer count and subtracts the initial half-block
lead. CrySirror currently reads elapsed shared mixer frames since activation.
See [SATD-cmvs-software-playback-clock](SATD.md#satd-cmvs-software-playback-clock).

## Shared output and input

- Top-level object slots use `2020`, `2021` and `202f` for lifecycle.
  `2030` resolves an image in registered source order and decodes it with
  the shared image registry. `2040` initializes draw state and
  `2044`–`2048` update crop, position, origin, order and opacity.
  The adapter publishes image resources and retained layers. Child objects
  can use the parent's atlas in the implemented 2D draw mode. Separate
  child-image loads and projected draw modes remain pending.
- `2033` identifies a loaded object's image and returns width, height
  and source depth flag in registers 1–3. PB3 source depth comes from
  the shared header parser. Decoded opacity is independent of source depth.
- `22cb` maps the opaque native RGB background to the bottom retained
  layer. `2220`–`2224` map the solid overlay to shared Layer motion driven
  by CMVS frame time. The wait holds its PC and stack during the fade.
  Shared opacity quantization can differ from Direct3D 9 arithmetic.
- `22d1` queries or updates the native effect-enable flag. `21ea`–`21ee`
  allocate, fill, start, stop and query each draw state's eight native
  opacity keys. Per-frame sampling preserves the 200 ms delta cap, held
  keys, integer interpolation and terminal disposal. The shared Layer
  owner consumes the sampled opacity.
- `20b0`–`20b4` prepare, start, pause, release and rewind scoped effect
  voices through CrySirror's decoder and mixer. The sound setting and gains
  come from `cmvs.cfg`. A native main transfer with retained live sound
  still requires its original checkpoint fields.
- `20a0` takes a music name, fade milliseconds and loop flag. `20a4`
  takes two names before the fade and loop words. With a loop, the first
  name is an intro to the repeating second asset. Without a loop only
  the first asset plays. The native device alternates channels 0 and 1.
  `20a1` clears both and `20a2` fades the current channel and toggles
  the channel selector. These commands leave system result registers alone.
- The mapped `21a0` family reads and consumes per-action edges. The owner
  reads `key.cfg` and samples supported keyboard/pointer bindings when
  the VM admits its next native frame. Controller and native gesture
  classification remain pending.

## Startup persistence

`22b3` scans up to 256 CSV2 through CSV9 slot headers in the client's save
root, with a negative count selecting 10 slots. It sorts recognized labels
by the native packed timestamp and reclaims its four-byte argument after
publication. Recognizing a header gives menu metadata only. The caller owns
menu presentation and slot actions.

`22b5` (`0x140082340`) attempts original-slot restore through `0x140081960`.
Slots above 999 consume the four-byte operand and advance. Admitted restores
use the restored script context. The wrapper returns `0x2000` when the native
resume flag at `0x3998` is set, or zero otherwise. `22b6` (`0x1400812f0`) writes
slots 0–999 through `0x140080b20` and returns `0x4004` for all operands.
The writer uses a 600-byte header followed by three packed ranges. The range
sizes at 560, 564 and 568 describe the checkpoint, optional BMP preview and
live main-script image. The corresponding expanded sizes are at 580, 584
and 588. The third range is the expanded PS2A image, including its header,
labels, code, mutable data and text. Restore installs it through
`0x140080650`, then `0x140069fd0` overlays the checkpoint's `0x0900` record
onto the script's mutable data. That record stores a 32-bit byte length
followed by the data. Only the packed checkpoint bytes are transformed.
The writer adds `0x80` modulo 256 to each byte and XORs it with the
corresponding byte of the repeating ASCII key `DAME_SAVE_IMAGE`. The 16-bit
wrapping sum of the transformed bytes is written little-endian after the third
range. Header offset 540 stores that trailer address relative to the start
of the file. The writer sets the word at 546 to one and stores the current
thumbnail-enable word at 548. The packed ranges use the PS2A bytewise LZSS
layout with zero-filled history, initial position `0x7df`, an 11-bit ring
position and a 5-bit match length plus two. `0x140068210` writes one flag byte
for every eight tokens, least significant bit first. A set bit selects a
literal. The load needs the original scene, text, audio and script
reconstruction in addition to these container rules.

The original reader selects the transformed layout when header offset 540
plus two equals the file length. It restores the packed checkpoint bytes
with repeat XOR followed by `+0x80`, after summing the encoded bytes. Its
other path expands the checkpoint as stored. The isolated CSV2 codec admits
the latter form when the untransformed ranges end at EOF. The third range's
PS2A header retains its original packed-length, version and key words even
though its payload is already expanded. When thumbnails are disabled, the
writer omits the preview range and can retain a nonzero expanded preview
length at 584. The complete slot path is versioned separately from the
CSV2–CSV9 menu header scan. See [SATD-cmvs-x64-dispatch](SATD.md#satd-cmvs-x64-dispatch).

For save slots below 950, the writer checks whether the destination file
exists. `0x140068b50` copies an existing destination to slot 999 and records
the replaced slot in the undo state. The writer calls the CSS1 system-data
writer at `0x1400179e0` before packing the slot. A restore of slots 0–949
calls `0x1400818e0` before opening its input slot. That helper checks native
state words at `0x40c8` and `0x66c`, then writes slot 998. If automatic
thumbnail capture is enabled at `0x2f38` and its bitmap is absent, it captures
one before writing the backup. The `0x140069270` checkpoint writer uses a
3 MiB staging allocation and writes a `CSV2` header. Its body uses
family-specific record lengths, including short strings, indexed scene and
audio records, and the long `0x0540` message history record.

Every emitted record starts with a little-endian `u16` tag. The table lists
the remaining framing in the emitted stream. Restore at `0x140069fd0`
consumes these records by tag until the expanded checkpoint ends.

| Tags | Framing and contents from `0x140069270` |
| --- | --- |
| `0100`–`0105` | `u16` length and a NUL-terminated title or name. |
| `0106`–`0108`, `010a`, `0112`–`0115`, `0200`–`0203` | `u16` length set to 4, then a single word. The restore consumes `0113`–`0115` without applying their values. |
| `0109`, `010b` | `u16` length and 40 or 1,796 bytes of native state, respectively. |
| `0240`, `0241`, `0248` | `u16` length and the stack pointer, call depth or eleven native control words, respectively. |
| `0242`, `0243` | `u32` length and the 256-byte frame-offset table or the 65,536-byte parameter stack. |
| `0280`–`0283` | `u32` length and the gameplay integer, float, packed bit or string bank. The first three contain 8,192, 4,096 and 256 bytes. The string bank contains 64 consecutive NUL-terminated strings. |
| `0304` | Two consecutive `u16` lengths and NUL-terminated names, each name immediately following its own length. |
| `0310`, `0318`, `0540`, `0600`, `0610`, `0620`, `0630`, `06e0`, `0880`–`0883`, `0900` | `u32` length and family-specific bytes. `0310` carries 312 bytes, `0318` calls `0x140086b50`, `0540` contains history and `0900` contains the main script's mutable data. |
| `0380`, `0400`, `06a0`, `0700`, `0800`, `08c0`, `08c8`, `08e8` | `u16` slot, then `u32` length and family-specific bytes. The writer visits 12, 8, 2, 256, 4, 6, 8 and 3 slots respectively, emitting an entry for each present object. |

`22b8` (`0x14006b4a0`) captures a thumbnail in memory and returns `0x4000`.
The two D3D9 helpers read the logical client rectangle from the back buffer
or its system-memory copy. `SI_SamImageMode` at `0x2b60` selects direct
locking when set. Otherwise the original tries the copy path first and falls
back to locking. The shared presenter supplies that logical frame as an
immutable image. A readback failure leaves the prior thumbnail intact.
It uses `22b0` dimensions and a 10-bit fixed-point step per axis. Each output
pixel averages a block of exactly the integer part of that step, clipping
source samples to the framebuffer. A block with zero samples produces black.
The 24-bit BMP has bottom-up four-byte-aligned rows and declares its pixel
payload size. The encoder writes zeroes in the reserved bytes and row padding.
The capture and encode both complete before the owner replaces the retained
BMP and advances the VM. The stack and result registers retain their contents.
`2003` releases the retained thumbnail. A request with zero dimensions is
rejected before readback because the native sampler would divide by zero.

`22b1` stores the unsigned word at SP−4 in the native history-save
limit at object offset `0x2e2c` and returns `0x4004`. The startup default
is 32. The `0x0540` save record writer (`0x14005eb60`) counts entries
newest-first and emits at most this limit. CrySirror applies the same
limit to checkpoint capture. The live history and result registers retain
their contents.

`22b7` (`0x14007caf0` → `0x140093490`) queries the newest CSV2–CSV9
slot in a range. SP−4 is the first slot and SP−8 is the count. The wrapper
uses a wrapping addition for the exclusive end and compares it as signed
against 999. A negative first slot, an out-of-range end or an empty
unsigned scan returns −1. Every branch writes integer result 0 and
consumes eight bytes with `0x4008`.

The original opens `save%z3.dat` under the registered save prefix, reads
up to 64 bytes and packs the timestamp at offset 16. Only a greater
nonzero packed value replaces the selected slot, so ties preserve the
first slot. The native formatter uses the low three decimal digits of
the unsigned slot. A short successful read preserves the unread tail of
the session scratch. CrySirror snapshots at most one header per distinct
filename on a scoped storage worker and evaluates wrapping ranges with
the equivalent recurring scratch state. The result and scratch publish
together. This query reads headers only.

`22bb` (`0x14007cb60`) reads one unsigned slot at SP−4 and returns
`0x4004`. Integer result 0 is 1 for an existing CSV2–CSV9 slot and 0
for a missing file, different signature or slot above 999. The 64-byte
read updates its own scratch prefix. A short read retains bytes from
the preceding `22bb` call, including signature bytes it did not replace.
`22b7` uses a separate scratch area.

`22bc` (`0x14007cc60`) reads the unsigned slot at SP−4 and a string
reference at SP−8. It copies raw NUL-terminated comment bytes starting
at file offset 272 and returns `0x4008`. `22be` (`0x14007cdc0`) reads
the slot at SP−4 and stores the raw words at SP−8 and SP−12 as
little-endian fields at offsets 532 and 536. It returns `0x400c`.
Both write integer result 0 and accept slots 0–999 with CSV2–CSV9 signatures.
The native helpers rewrite the 600-byte prefix and leave its payload
tail intact. CrySirror commits through atomic scoped storage, returns 0
for absent, unrecognized or truncated headers, and reports storage errors
without reporting a successful write.

`22bd` (`0x14007cf10`) reads the signed slot, object slot, signed child
and two string destinations in SP−4 order. It consumes five words with
`0x4014`. A valid header returns the strings at offsets 16 and 272,
their packed timestamp in integer result 1, and header words 528, 532
and 536 in results 4–6. Result 0 reports recognition.
If the 16-bit field at 548 is nonzero, the command replaces a root or
child object with a saved preview. A nonzero packed size at 564 selects
the 11/5 bytewise LZSS stream starting at offset 600 plus the word at 560.
The expanded size comes from offset 584. A zero packed size selects an
opaque black preview at the dimensions configured by `22b0`. A negative
child operand replaces the root and destroys its old children. A nonnegative
child operand replaces just that child when the parent exists and the index
is below 768. The new object has no draw state. Image loader failure after
replacement still returns the recognized slot metadata. A negative save slot
or root index outside 0–255 writes only 0 to result 0. Nonnegative slots use
their last three decimal digits to select the file.

The native screenshot writer at `0x14006b4a0` produces a bottom-up
24-bit BMP. The preview reader hands the expanded blob to the memory
image loader at `0x140036700`. It selects BMP, PNG, PB3B, MSK0 or PB2A
by magic. PB3 type 1 uses the block side at header offset 24. Type 4 needs a
variant name and returns failure when called from the preview with no name.
Types 6 and 8 resolve the decoded `.pb3` base name through the asset search and
replace selected 8×8 tiles. Type 7 at `0x14009f6e0` expands `4 × width × height`
interleaved BGRA bytes using the stream offsets at 52 and 56 relative to byte 60.
MSK0 reads a raw one-byte plane using the data offset at 4,
width at 8 and height at 12. PB2A unpacks the 24-byte secondary header
at offset 8 using the key word three bytes from EOF and the 24-byte
subtractor table starting 27 bytes from EOF. Type is at offset 16, followed by
width, height and depth as 16-bit words. IDA routes PB2A types 1, 2,
3/4, 5 and 6 to `0x14009f330`, `0x14009ec00`, `0x14009f0d0`,
`0x14009e7a0` and `0x14009e1f0` respectively. Types 1 and 2 use 8×8
tiles and split-stream 11/5 LZSS. Types 3 and 4 use JBP1 with alpha runs
when depth is 32. Type 5 uses XOR-correlated planes for its first frame.
The memory entry point fixes the type 5 frame index at zero and passes a
null name to type 6, which rejects it. Named image loads can select type 6
variants. The [shared registry](../../docs/RESOURCES.md#native-pictures-and-textures)
decodes MSK0 grayscale files and handles the unnamed PB2A memory profile.

The preview BMP decoder `0x140035d50` reads bottom-up rows from byte 54,
with four-byte alignment and a source pixel width of `depth >> 3`. The
PNG decoder `0x140035ea0` only expands indexed PNG rows and copies the
leading three raw bytes of each source pixel. Both upload those bytes as
BGR, retain the fourth byte when the source stride is four bytes per pixel,
and supply opaque alpha otherwise. The PB2A memory wrapper leaves its alpha
flag uninitialized. CrySirror derives that flag from the encoded depth,
with type 5's explicit alpha plane enabled.

MSK0 returns an 8-bit plane. The native preview handoff at `0x140032880`
drops the depth, and its `0x1400301d0` scene copy requires four bytes per
pixel. The original copy has no defined output for the extra bytes.
CrySirror treats that input as an image upload failure, retains the
recognized save metadata and publishes an empty replacement object. The
last preview load failure is available in the adapter diagnostics.

The owner stages the two VM string writes, five integer results and stack
cleanup before submitting the scene replacement. Successful publication
commits the staged VM state. Scoped storage and malformed LZSS errors
leave the previous scene and VM state intact. A cleanly expanded preview
which the native image loader rejects still replaces the object and
returns recognized metadata.

`2128` loads `system.dat` in CSS1 encoding mode 1 through the save capability.
Its encrypted payload has an additive u16 checksum and a 10/6 LZSS stream.
The persisted ranges are bits 2048–262143, integers 2048–4095, floats
1024–2047 and strings 64–127. The two native name lists follow the strings.
A zero list count preserves the current list. A nonzero game ID must match
the current `201d` value. A missing file completes without mutation. Malformed
content or storage failures preserve state and leave the command pending.

CSS1 import reads through the save capability. CSS1 writes and whole-slot
CSV capture and restore remain pending.

## Effect records and object rasters

Offsets below are from the 2021-10-07 x64 build. `22a0` releases one of the four
`0xa0`-byte records at engine +4032. `22ae` reports the current status
according to type. Each object record owns an internal object whose children
it poses every frame:

| Selector | Type | Native init/step | Words after the slot and image name |
| --- | --- | --- | --- |
| `22a1` | 1 | `0x14005e370` / `0x14005e470` | mode, two unused floats, angle, speed, interval |
| `22a2` | 2 | `0x140060950` / `0x140060cd0` | area, height, speed, fade, count, follow |
| `22a3` | 3 | `0x14005f2a0` / `0x14005f360` | mode, depth, speed, count |
| `22a4` | 4 | `0x14005ec30` / `0x14005ed30` | as `22a1`, 2048 records, `/250` time base |
| `22a5` | 5 | `0x14005d440` / `0x14005dd10` | mode, centre, depth, direction, speed, count |
| `22a6` | 6 | `0x14005f4e0` / `0x14005fbd0` | mode, depth, speed, count |
| `22a7` | 7 | `0x1400604e0` / `0x140060600` | as `22a6` with the fade bytes of the mode |
| `22a8` | 8 | `0x14001ab50` / `0x14001afd0` | source object, tile size, one unused word |
| `22a9` | 9 | `0x14001b410` / `0x14001b4e0` | mode, speed, order, count |

Random values come from the MSVC `rand()` sequence seeded at startup. Type 6
particles settle on the triangles of the point lists of objects 20..=27
(`2330` new list, `2331` free, `2332` add a point, at most 128; triangles are
consecutive triples offset by the object's world position, 256 points in all).
`22af` key 1 replaces a type 6 mode. The wrapper consumes four words even
though it only passes the slot, key and value to the type dispatch.
A save keeps each object-effect record's command, image and
words (`0x1400525b0`); restoring starts it again (`0x140052e80`); type 8 records
are not kept.

`2328` (`0x14004f5d0`) starts a raster over the object's crop: mode 1 draws one
row per pixel line stretched by word 7 / 1000 about a pivot that swings by
`cos(36000·((phase + row·w9/w6) % w10)/w10)`, advancing the phase with the frame
delta; mode 16 draws word 7 echoes word 6 pixels apart either side with opacity
255/(n+1). While a raster runs the object itself is not drawn. `2329` stops it.

## Step list, slot files and scene movies

The step list at engine +4760 holds sixteen 96-byte records after a 20-byte
header and is part of the VM checkpoint. `2280` clears it, `2281` returns the
current record or -1, `2282` starts it, `2283` stops it, `2285` returns the
count, and `2286..=228b` append a wait, camera ease, object image, two world
moves and a window-part image. Each frame `0x14002a680` runs records until a wait
is pending; `2284` runs the rest without crossfades or tweens, then clears the list.

`228c` swaps two slot files through `savetemp.dat`, `228d` deletes one and
`228e` moves the second onto the first, keeping the replaced file as
`savedel.dat`. Changes below slot 10 rebuild the native load/save menu.

Scene movies at engine +4504 keep their own frame clock (class `0x14004a7d0`).
`2308`/`230e` load a movie paused on a first frame; `2309` plays a section from
its first to last frame (-1 for the end), looping or queued behind the running
one; `230b` releases and `230c` holds the clock; `230d` selects stepping (0),
single steps up to the `231d` frame (10) or the audio clock (15); `230f`, `231a`
and `231e` report the section, playing state and frame; `231b` shows a frame and
stops; `231c` starts the queued section. The media service presents the selected
frame.

## Picture stacks, lists and text fields

`2310..=2317` build an object picture from sixteen layers (CTexture +1216):
`2310` places an image, `2317` an opaque black rectangle and `2316` a copy of
another object's picture (layer 0 starts a new stack); `2311` releases a layer
or, for layer 0, the stack; `2313` greys and tints, `2314` multiplies by
colour/128, and `2315` filters (1 invert, 3 mosaic, 4 neighbour mix, 9 grey tint,
10 row blur, 11..=14 radial blurs, 15/16 spin blurs about (320, 240)); `2312`
bakes the stack into the object's image.

List objects (`2370`, class `0x14001b5c0`) count pointer hits only inside the
`237d` rectangle, measured from the object, and report the pointer relative to
the hit item through `237f`. `237c` leaves decided or cancelled input edges for
the script and `237e` disables keyboard movement. `2397` gives a text field draw
mode 4 with a screen-sized matte of its image's green channel.

## Developer and font records

`23a4..=23a8` keep text-debug records at engine +4768 in `<computer>_<n>.dvg`
beside the save slots, one CP932 line `kind,status,script,line,voice,name,text,
fixed name,fixed text,remark` each. `23a5` describes the current line, `23a6`
opens CTextDebugDlg 147 (name and text must be double-byte, the remark may not
hold a comma), `23a7` returns a record's voice and corrections and `23a8` sets
`<n>`. The host exposes no computer name, so the file uses the native fallback
`default`.

The 2021 image font at engine +4464 cuts square glyph cells from 8-bit sheets
(`23c0`), files one cell per sheet under a character (`23c1`) and gives a cell
its own advance (`23c2`). The text renderer draws a character from these cells
before using a system font, resampling the smallest larger cell for other sizes
(`0x140018d80`); proportional layout (`23ac`) uses the cell advance.
