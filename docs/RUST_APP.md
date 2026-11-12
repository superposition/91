# UI and graphics port

The first `EN_RUST_APP` checkpoint replaces the production strip renderer with
`fm1-app::graphics`. It does **not** complete issue #15: the page controller,
input policy, visualizer state and Kick Runner remain C at this checkpoint.

`fm1-app` is `no_std`, forbids unsafe code and depends on `fm1-types`,
`fm1-formats` and `fm1-engine`. The engine does not depend on the application.
There is no allocator, generic floating-point formatter or new pixel buffer.

## Production path

The existing `cv_begin`, `cv_band`, `cv_pset`, `cv_rect`, `cv_line`, scanline,
text, width and palette functions call the narrow `rust_app.h` bridge. The
UI's alpha pixel helper uses the same renderer. C0 bodies remain selectable.
The native Rust simulator and Rust-enabled web build enable this flag.

The safe renderer receives an initialized, fixed-width canvas descriptor and
borrows the existing strip for one call. It clips rectangles/pixels, bounds
line work, validates glyph spans, keeps byte-oriented font substitution and
preserves swapped RGB565 blending, scanlines and draw order. Text is bounded
to 256 bytes; normal UI strings fit this bound. A separate small text writer
provides NUL termination and manual integer output for subsequent UI work.
The controller has not adopted that writer yet.

The bridge never calls application or platform callbacks and retains no
borrow. It initializes only the active strip through `MaybeUninit` before
forming initialized pixel slices. An uninitialized pool tail is never read.

## Display ownership

`lcd_blit` may retain the source after returning. `LCD_READY()` must return 1
only when that transfer completed or was confirmed stopped. A void sync call
is insufficient on hardware: unknown completion quarantines the strip, skips
all pixel operations and blits, and requires a fresh successful `cv_begin`
before reuse. The next UI frame invalidates its redraw signatures to retry.

The physical build must supply the confirmed-ready hook; otherwise enabling
`EN_RUST_APP` produces a compile error. Native/web software frontends have
synchronous display sinks and use their existing sync hook. Hardware stop,
timeout latency and recovery qualification remain part of the HAL work.
When the physical provider is present, C0 drawing also respects the same
quarantine. Thus `HAL=1, APP=0` can remain selectable without overwriting a
retained source; the original `HAL=0, APP=0` path has unchanged behavior.

The DMA double retains the actual source pointer, checks its bytes until
completion and exercises failed begin, failed scan-before-blit, late completion
and fresh-begin recovery in both C0 and Rust graphics profiles. It also verifies
that quarantined operations never enter the Rust canvas bridge.

## Memory and interface

The renderer reuses `cv_px[240 * 144]` (69,120 bytes). The visualizer's existing
7,200-byte union is unchanged. Added mutable static storage is four 32-bit
counters at stat indices 80–83 plus one C lease-state byte (linker alignment
may add padding). Call-local state is 24 bytes and the font view is 36 bytes
on 32-bit targets, 56 bytes on the native 64-bit test target. No pointer is
stored in safe application state. MCU code size, stack and final memory maps
require the generated-C target qualification; WASM sizes are not MCU estimates.

The instrument keeps its 24 public WASM exports and 32 MiB memory. Stats 80
and 81 show real instrument canvas calls/pixel writes, 82 is reserved for
subsequent action dispatch and 83 counts rejected calls. Diagnostics stay out
of the player-facing UI.

## Independent proof

Run with Rust 1.94.1, clang, Node and Pillow on PATH:

```sh
sh tests/rust_app_graphics.sh
sh tests/rust_emulator_parity.sh
sh tests/rust_simulator.sh /path/to/verified/R01/source
```

The graphics test executes 78,353 cases and compares every output pixel,
metadata, width result and sync/blit observation across captured frozen C,
current C0 and the Rust production adapter, both natively and in WASM. The
captured C renderer and alpha helper are hash checked and retain their
license. Passing the optional R01 path additionally verifies all 440 original
source files and recompiles the original renderer. Fonts are generated once
from the same authoritative inputs for all variants; the captured corpus SHA
records the qualified font toolchain rather than assuming every Pillow
version generates identical assets.

Cases include all five fonts and all 256 glyph bytes, every RGB565 color in
alpha blending, clipped geometry, bands, scanline parity, complete/partial
blits, zero-size/clamped canvases and 0–256-byte text. Safe-core tests add
hostile coordinates, malformed font spans and canaries. The full instrument
suite checks strict PCM, stored bytes and framebuffer behavior; the five-way
simulator checks 97 scenarios and repeatability, including the exact known
frozen failure fingerprints rather than treating old assertions as successes.

No interactive browser, physical display or MIDI-device QA is claimed by
these source-level tests. See `rust/app-graphics-proof.json` for the qualified
artifacts.

## Remaining issue #15 inventory

| Area | Current state | Required boundary |
| --- | --- | --- |
| Strip/text raster, palette, alpha | Rust production path | Confirmed-ready LCD ownership |
| 43 views and 35 dial pages | C controller | Explicit initialized UI state and engine observations |
| Buttons, keys, encoders and LEDs | C policy | Bounded actions; preserve cadence and held/tapped edges |
| Themes/help/layout widgets | C composition | Bounded text and same primitive order |
| Visualizers | C state and algorithms | Reuse existing 7,200-byte union; no extra feedback canvas |
| Kick Runner | C state, policy and drawing | Fixed 16 rows; snapshot inputs and deferred effects |
| Settings/session persistence | Qualified Rust policy from #7 | Preserve its saved bytes and scratch lifetime |

Full UI effects must be dispatched only after mutable UI/project borrows end.
Stopped engine commands can drain immediately and reenter harmonic callbacks;
controller transitions must return actions or continuations instead of holding
an application borrow across those calls. Engine scale/chord observations use
only their initialized active prefixes. Captures must enumerate fields and
must not hash raw C padding.
