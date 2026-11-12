# Foreground frame boundaries

`fm1-app::controller` owns frame ordering, splash dismissal, theme invalidation,
drum-drift generation tracking, overload escalation/recovery, manual Jam voice
handoff, game cadence, Songbook snapshot refusal and final frame advancement.
The existing application helpers remain explicit actions behind a narrow C
callback until their input/page/visualizer ports land. This checkpoint does not
claim those helper implementations have moved to Rust.

The safe core receives a `Port`, not an engine pointer. `with_ui` and
`with_frame` lend the existing carriers only inside a closure. The closure's
return type cannot retain its input reference, as a compile-fail test verifies.
Every query or application callback occurs after these closures return. The
unsafe bridge owns a small stack tuple of raw pointers and a function pointer;
it never holds an exclusive UI reference across C, stopped-command dispatch,
storage, LCD calls or a callback that can access UI again.

The seven old frame-local static words become one 28-byte `repr(C)` frame
carrier in their place. There is no extra UI object, observation framebuffer,
heap, payload or diagnostic counter. Counter 82 now counts frame policy as
well as initialization and game policy; it is not exclusively a game counter.
The renderer retains the existing strip/DMA readiness and quarantine rules.

Only the foreground may access UI and frame carriers. Physical scalar queries
copy their result under the named nested-safe IRQ save/restore hooks, restoring
the incoming state before any Rust policy, drawing or effect runs. Queries use
published CPU0 data and never inspect private worker buffers. These short
queries preserve the legacy phase-by-phase observation cadence; they do not
claim to capture all engine meters in one coherent generation. Future visual
observation packets must retain the stronger published-snapshot rules.

`tests/rust_ui_frame.sh` checks the exact R01 frame function, captured with its
page IDs and protected by SHA-256. Supplying the immutable R01 tree verifies all
440 source hashes and re-extracts that function independently. Native and WASM
execute 8,192 cases spanning clock/counter wrap, splash, themes, remote expiry,
load thresholds, manual voice transitions, game cadence and callbacks that
mutate UI. The corpus serializes every semantic UI field and effect order;
padding is excluded. The Rust side executes the actual production callback
extracted from `ui.c`. Three null-argument probes and a refused Songbook capture
also execute on both targets. A retained-reference compile-fail test and focused
timing boundaries supplement the independent C differential.

Full instrument regression and target compiler/footprint evidence are recorded
separately. Native/WASM success does not prove physical interrupt latency or
DMA completion, and no device writes are performed by these tests.
