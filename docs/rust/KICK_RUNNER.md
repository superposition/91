# Kick Runner application slice

With `EN_RUST_APP=1`, the existing game uses safe Rust state transitions,
spawning, scoring, steering, restart/button decisions, track/effect masks, LED
selection and complete drawing composition. The C0 bodies remain selectable.
Shared UI bookkeeping (button timestamps, held keys, activity and MIDI
dispatch) remains in the UI controller until the next part of issue #15.

## State and effects

`fm1_app::runner::State` replaces the existing C game object in place. It is
236 bytes: a 108-byte prefix including its 20-byte banner and explicit three
padding bytes, then sixteen 8-byte rows. Each row has a binary32 arrival beat,
an eight-lane wall mask, gate mask and two status bytes. There is no extra
game object, heap, framebuffer or saved format. Best score and rows born keep
their original run-lifetime behavior; reset does not erase them.

The 16-byte observation copies the clock, sixteenth counter, playback state
and kick glow. The xorshift32 RNG and binary32 operation order are unchanged.
SELECT detents retain the original sign behavior: a delta of +7 moves one
lane. Track/effect IDs arrive from the named C instrument constants and are
checked before shifting. No storage, MIDI or public WASM IDs change.

The bridge has no callbacks and retains no pointer. Mutable game borrows end
before platform or engine calls. Update has separate clock and row phases;
row resolution returns a banner action, then the caller samples its deadline
after the borrow ends. This preserves the original order of clock reads,
messages and effects, including immediate stopped-command reentry.

```mermaid
sequenceDiagram
    participant C as Application adapter
    participant G as Rust game state
    participant E as Engine/platform
    C->>E: Capture clock and observations
    C->>G: Advance clock, spawn and decay
    G-->>C: Release game borrow
    loop Each of 16 rows
        C->>G: Resolve arrival and score
        G-->>C: Release borrow; optional banner
        opt New banner
            C->>E: Sample banner deadline
        end
    end
    C->>G: Compute effect and track masks
    G-->>C: Release borrow; return masks
    C->>E: Dispatch effects and mutes
```

## Drawing and retained DMA

`runner_draw` passes the three actual fonts, four theme colors and a half
index to `fm1_app::runner_draw`. The renderer uses the existing 240×120 active
strip inside the 240×144 pool. Projection, ring order, row order, antialiased
segments, glows, ship, labels and help/game-over text retain C0 behavior.
There is no LCD call while Rust borrows pixels or fonts.

The C adapter requires confirmed `LCD_READY()` before entering the renderer.
Rust initializes the active prefix through `MaybeUninit`, then forms a typed
pixel slice over only that prefix. The borrow ends before `lcd_blit`; the LCD
may retain the source until its next confirmed completion. Unknown completion
quarantines the old strip. Both the C0/provider and Rust/provider game adapters
are tested against a retained-source double with failed and late readiness.

Drawing rejects invalid float/state ranges before raster work. Segments have
a fixed work bound. Text uses small NUL-terminated integer writers, with no
allocator or generic floating formatting. The old game-over local was 32
bytes; two full-width u32 scores require 34 including NUL. Rust uses a bounded
40-byte local. Frozen comparisons cover the original defined score range,
and a separate canary test covers both maximum u32 scores safely.

## Reproduction and limits

```sh
sh tests/rust_runner_policy.sh /path/to/verified/R01/source
sh tests/rust_emulator_parity.sh
sh tests/rust_simulator.sh /path/to/verified/R01/source
```

The dedicated corpus compares 72,192 complete policy records, including every
game field, banner tail, all rows and ordered callbacks. It exercises clock
wrap, RNG trajectories, collision precedence, score wrap, ship loss, full-track
gates, held input and encoder consumption. A callback double accesses game
state only after the Rust call returns. Another corpus compares every pixel
in 1,024 screen halves across lane positions, row masks, colors, game-over
states, glows and scanlines. The oracles are exact hash-checked R01 C functions;
the optional original source is verified against all 440 source hashes.
Both families execute current C0 and Rust production adapters natively and
in WASM. The real instrument additionally enters, plays, pauses, steers,
restarts and exits the game through its unchanged 24-export browser ABI.

See `kick-runner-proof.json` for qualified artifacts and footprint accounting.
The whole MCU code/stack map and physical display/abort latency remain target
qualification work. Source-level native/WASM checks do not constitute browser,
physical MIDI or device QA. The page controller and visualizers still keep
issue #15 open.
