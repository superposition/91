# Foreground harmony observations

Read-only progression capture and chord preview now obtain their existing
208-byte state and 48-byte configuration under the named saved-IRQ hook on
the physical target. The helper restores the caller's complete incoming IRQ
state before entering Rust math or lending the caller's output buffer. An
already-disabled outer context stays disabled. Hosted native/WASM execution
retains its serialized source boundary.

Only `harmony_preview` and `harmony_capture` use this read-only boundary.
Transition/publication and pad-retarget paths remain with their existing
engine ownership. Every copied field is CPU0 harmony, history, scale or
parameter state. No worker-private voice/sample span is read; masking CPU0
IRQs is not a CPU1 ownership claim. Existing stack packets are reused, with
one temporary saved flags word and no new persistent buffer or counter.

`tests/rust_ui_harmony_snapshot.sh` extracts the actual read-only wrappers and
injects an interrupt opportunity after each byte in a test implementation of
the producer copy. It covers 65,536 low-word IRQ patterns with a high-word
sentinel, both outer-context states, and preview/capture: 262,144 calls and
67,108,864 byte-copy opportunities. Protected packets are coherent, incoming
flags are retained, and no output changes before the math phase. The negative
control intentionally violates the hosted serialization assumption and
confirms that the same unmasked copy produces mixed generations.

The probe uses the actual named IRQ-state implementation with fake register
access, plus C implementations of the Rust math call boundary. Existing
frozen C/current C/Rust harmony parity separately checks actual algorithms.
This qualifies source order and admission structure; actual target interrupt
latency and physical timing still require the hardware gate.
