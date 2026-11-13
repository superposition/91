# Measuring the endpoint decision table

The first actual vendor-LTO inspection exposed a cost hidden by the source:
the function-local `const` table became 32 bytes of stack initialization.
Making the same immutable table `static` placed it in read-only storage.
The decision bytes and all endpoint traces stayed identical.

| Inspection result | Local constant | Read-only static |
| --- | ---: | ---: |
| Function code | 224 B | 106 B |
| Saved registers/return address | 16 B | 8 B |
| Local frame | 32 B | 0 B |
| Isolated frame total | 48 B | 8 B |
| Addressable read-only table | 0 B | 32 B |
| Allocated text, including the same inspection sink | 226 B | 140 B |

The final prologue saves `{rets, r4}` and allocates no local stack. There are no
calls. The 32-byte table is a named symbol in the only allocated ELF section,
whose flags are `AX`, without `W`. Neither version adds writable `.data` or
`.bss`. The output comes from the real `fm1_rust_usb_endpoint` export and all
seven selected Rust libraries, emitted by the pinned bootstrap compiler and
linked by the vendor clang driver with LTO, r3/fprev1 and contraction disabled.

```sh
JIELI_TOOLCHAIN=/path/to/vendor/toolchain \
  bash tests/compile_usb_endpoint_leaf.sh CORE_OUTPUT BRIDGE_OUTPUT
```

The script creates an inspection ELF with that export as its entry. Its
freestanding unwind symbol uses an explicit inspection-only infinite-loop
sink, which is disconnected from the leaf and occupies the same two bytes in
both results. It supplies no physical DMA/stop fact and is never flashed.
Raw `ld` cannot consume the emitted LLVM bitcode; the vendor clang LTO driver
performs that step.

The complete host, healthy endpoint, retained-source correction, TX and reset
corpora remain exact after this change, and exhaustive endpoint plus retained
root Miri checks pass. See [the combined proof](combined-host-manual-checkpoint-20261112.json).

This eight-byte frame is an isolated measurement. A full firmware link may
change inlining/register allocation, and its C callers, other Rust functions
and interrupt overlap still need a complete stack bound. No deadline or
physical execution claim follows from this inspection.
