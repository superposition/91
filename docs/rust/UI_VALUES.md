# Knob values, ranges and mood

Seven existing UI consumers use `fm1-app::values`: parameter lookup, value
lookup, range lookup, acid working-copy refresh/kind, mood adjustment and mood
application. The original bodies remain selected by `EN_RUST_APP=0`. The private
ABI generator freezes the inherited 200–239 pseudo-control codes, the 240 FX
base, 35 descriptor pages and Jam's Ionian/Aeolian mapping. All 176 parameter
ranges come from the maintained authoritative `fm1-types` metadata.

The bridge holds raw foreground roots and lends UI only within short closures.
Every loan ends before an engine, descriptor, project or reader callback. FX
descriptors can call a nested ordinary getter. The reader receives only its
derived initialized local destination span and retains no pointer. Progression
preview preserves the six-byte encoding and stored Play bit; chord kinds retain
the original three bytes. Acid refresh uses a 38-byte local packet and preserves
the 400 ms edit hold, masks and timestamp semantics. These packets add no
persistent payload. No complete UI, sample span or worker-owned voice is copied.

On the physical path, the named saved-IRQ hooks protect only bounded CPU0
publication copies: current parameter, Jam settings, bass/pluck/mod-step values
and acid fields. Exact incoming IRQ state is restored before Rust work.
Readonly harmony preview uses the separately qualified snapshot boundary.
Worker-private arrays are excluded; local IRQ exclusion cannot stop CPU1.

`BassStep` is a one-byte checked view over kind/note/octave fields. All 256 raw
encodings round-trip, including reserved kind and octave values. The existing
semantic setters remain responsible for their narrower musical limits. The
15-entry mood table keeps its 30-byte representation and replaces the active C
table; C0 retains its independent table. Disabled pluck stays disabled. Mood
arithmetic preserves C truncation for valid stored values and rejects signed
overflow before a write. There is no new persistent cache or counter.

`tests/rust_ui_values.sh <R01-source>` verifies all 440 immutable source hashes
and executes actual full-app consumers. The 4,096 records cover every parameter,
all 40 pseudo-controls, all 140 FX controls, packed bass/mod values, 256 mood
bytes, progression/Jam branches and acid-edit timing. Each record includes
explicit UI/project/global/store fields, PCM and framebuffer hashes. Structure
padding is never serialized. All six native/WASM streams are byte-exact:
52,072,448 bytes with SHA-256
`b24e4b6c20f883ed91e13e4693a83816868d6d5d9f059e0e0a524fc230066f8e`.
Five strict-provenance Miri cases include reader
reentry, nested lookup, partial-reader failure, bounded metadata indices and
untouched error outputs. Miri uses Rust C-ABI doubles, not physical IRQs.

The actual C acid reader also runs with only publication reads instrumented.
All 65,536 low IRQ-state combinations, a high-bit sentinel and nested enabled/
disabled callers produce 131,072 coherent packets over 4,980,736 read
opportunities, preserving guards and complete incoming IRQ state. The unmasked
negative control detects 1,212,416 incoherent bytes. Invalid kind, length and
null cases leave outputs untouched. This models interrupt exclusion and cannot
prove physical interrupt latency. Pure getter success does not increment the
hot-path application counter; the original encoder execution witness remains
unchanged. No additional counter is introduced.

Remaining #15 work includes setters, MIDI helpers, bounded text and widgets,
page composition and visualizers. MCU ROM/RAM/stack and interrupt latency remain
the separate adopted-compiler/physical gates. No browser or device interaction
is claimed by these source-level checks.
