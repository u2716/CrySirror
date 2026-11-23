# KiriKiri bitmap redraw batches

## Diagnosis

The installed Riddle Joker script's Layer.fillOperateRect helper issues
128×64 Photoshop Overlay operations over a large bitmap. The adapter previously
submitted and awaited a whole-image worker job for every tile. A source trace
confirmed this path at the stalled startup/entry frontier. Intermediate worker
publications could expose the helper's temporary backing during text redraw.

The earlier desktop fence was too broad: it depended on the whole native frame
finishing, and could block the first presented picture. Desktop startup time
also used zero elapsed time until that first presentation. That global fence
has been replaced with a bitmap-redraw-specific observation that always allows
the first picture and native dialogs.

## Implementation

Consecutive independent Photoshop bitmap writes accumulate an immutable
ImageInput composite DAG. Shared engine workers still own all pixel math and
resources. Batches are bounded to 64 writes with simple sources of at most
65,536 pixels; self-dependent, large or unresolved sources retain the existing
single-operation/capture path.

Other native methods, native property writes and callback completion seal the
batch. ResourcePending retries the uncommitted native instruction after the
job completes. Pixel reads and subsequent copies therefore observe preceding
writes without replaying script side effects. Pending targets are GC roots,
and explicit target invalidation discards an unsubmitted batch.

onPaint callbacks now have a retained queue and cursor. Each callback finishes
its dependencies before the next callback enters TJS; remaining queued paint
objects are explicit GC roots. The redraw observation lasts through the
current bitmap callback, including an initial capture when needed, rather than
through unrelated waits or the whole VM frame.

## Validation

* Release-mode presentation/batch tests passed: ordinary startup waits remain
  presentable; bitmap redraw/modal observations behave independently; 130
  writes use three jobs. Direct script calls and onPaint callbacks, with and
  without holdAlpha, match the sequential shared-compositor reference exactly.
  Loop-side-effect counts and a following copy verify retry/ordering semantics.
* Existing release integration suites: engine_bridge 30 passed/1 ignored,
  native_layer_lifetime 3 passed, piled_copy 3 passed. The ignored case is not
  counted as a pass.
* The final desktop release build completed successfully. The same installed
  game and 9,000-step offscreen input route previously ended suspended inside
  tile processing with a black picture. The batch build reached the starry-sky
  message scene and a normal page wait, without a native error. A separate
  6,000-step run displayed the title-menu character background and buttons.

These are offscreen game executions and generated artifacts, not live-screen
inspection. They establish progress through the tested route and preserve
bitmap results; they do not measure interactive desktop FPS or certify every
transient frame, game route or native rendering profile.
