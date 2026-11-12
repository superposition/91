# Project and persistent storage

The first checkpoint moves the A/B NOR algorithm into the allocator-free,
`forbid(unsafe_code)` `fm1-engine::persistence` module. `storage.c` selects it
with `EN_RUST_STORAGE=1`; the C0 path remains selectable. Flash read, erase,
program, IRQ and suspend primitives are unchanged platform hooks.

The caller retains the existing 3,840-byte scratch buffer. Each synchronous
callback finishes before returning, retains no slice/pointer, and cannot re-enter
the store. No extra flash buffer or heap is introduced. The 32-byte commit record
uses explicit little-endian fields, never a Rust struct image.

The exact legacy rules are intentional: read both headers first; A wins a tie;
compare sequences numerically; fall back from a rotten newer payload; ignore
slot and reserved header fields when loading. Sequence `0xffffffff + 1` wraps
to zero without changing the legacy numeric ordering. Hook failures retain their
original integer result. Payload pages precede the final header program, followed
by header/payload readback; readback failure returns `-7`, even when a reboot can
already load the newly committed copy.

`tests/rust_persistence.sh IMMUTABLE_R01_SOURCE` first verifies all 440 baseline
source hashes, then runs unchanged independent C fixtures against frozen R01,
current C0 and the Rust production adapter. All three native outputs and both
actual WASM executions match: 47,709 cases, 2,671,720 bytes,
SHA-256 `13be5492724249e90816eee1886438f7022012dcd5d1f9a036e0c7e234546ad3`.
The corpus covers all 31 objects, zero/max/page-boundary sizes, tie/wrap/corrupt
selection, positive and negative I/O errors, and cuts after erase/every program
operation with partial-program prefixes. It compares I/O ordering, result,
guarded loaded bytes and both resulting NOR sectors. Separate safe-core tests
exercise all commit boundaries and 32,768 malformed headers; the existing C
storage regression also passes through the Rust adapter.

Project/default/global/extension/session policy is the next checkpoint. This
checkpoint alone does not claim R06 completion, device execution or physical
power-cut testing.
