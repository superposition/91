# Scenes, tempo and page policy

Twelve existing application helpers now execute safe `fm1-app::performance`
policy under `EN_RUST_APP`: undo save/swap, tap tempo, scene store/apply/recall/
reset/tick, tempo glide start/tick, drop key changes and page transitions.
The original C bodies remain selectable with the flag off. Engine, parameter
and track effects plus bounded message formatting use a private callback ABI;
`tools/ui_performance_abi.py` checks its generated tags.

Every UI loan ends before a query, message or engine/application effect.
Scene recall rereads each remaining value after earlier callbacks, preserving
reentry and order instead of precomputing a stale batch. Page changes capture
the bass line before mutating page state, retain remembered groups and issue
the breadcrumb before changing the active page. No new mutable state, pixel
payload, storage buffer or diagnostic counter is added. Fixed page/routing
metadata remains compact and immutable; its C counterpart used by unported
rendering is explicitly transitional. Target footprint and stack gates remain
necessary.

The new bounded tap handler keeps the tap history but rejects a zero interval.
Two synthetic taps at the same clock value previously divided by zero. A
separate independently extracted R01 fixture records both the native `SIGFPE`
and UBSan diagnosis; this undefined input is excluded from compatibility
fuzzing. Rust guard tests check the rejected update and a successful later
nonzero interval. Bad scene/page/operation IDs and missing pointers are refused
before effects. The strict-provenance Miri case reenters page policy from a
track callback and changes a scene density and FX knob; later scene reads and
glide initialization must observe those changes.

`tests/rust_ui_performance.sh <R01-source>` runs the actual complete app/engine
with all twelve helpers over 4,096 deterministic cases. Its shared input
harness verifies all 440 immutable source hashes and captures PCM, all 193
semantic UI fields, explicit project/globals/slot data, platform state, stores
and framebuffer hashes. Each target compares original R01 C, current C0 and
Rust byte for byte. Two original-C settings-popup frames have different
framebuffer hashes between native and WASM (cases 1152 and 3456); separate
fixed frozen goldens retain them. Every other byte, including all PCM and
semantic fields, must also match across targets. There is no tolerance in
the current-C0/Rust comparisons. The original input corpus retains its strict
single cross-target golden.

`UI_CAPTURE_FRAME=<case>` can be defined in the test driver to emit the actual
RGB565 framebuffer at the first selected case for diagnosis. It changes only
test output. The Songbook-off label gap found during this corpus has its own
[regression and correction](JAM_LABELS.md).

DJ/AutoFX and other helpers, parameter/text/widget presentation, full page
composition and visualizers remain explicit #15 work. Message typography is
still rendered by the existing bounded C effects. Native/WASM validation does
not qualify physical controls, DMA, interrupt latency or device memory limits.
