# Loader block packaging

`fm1-tooling::lz4` implements the existing greedy LZ4 block compressor and its
prefix-aware decoder. It preserves candidate replacement order, the 65,535-byte
distance bound, extension bytes and the final literal rule. It runs only in the
host tooling graph, using host allocations; it adds no firmware or codec-WASM
dependency or payload.

`fm1-tooling::build::loader_wrap` uses the shared Rust JLFS entry writer and CRC
to produce the same outer header, load-address header and 4,096-byte compressed
blocks as the frozen packager. Oversized compressed blocks retain the existing
`loader: block N does not compress below 4096 B` refusal. The Python
`tools/build.py::ldr_wrap` and `tools/lz4blk.py` APIs are transport bindings into
these implementations. Compiler execution, linker/image/MMIO/stub/SDK checks
and the complete build graph remain separate #22 work.

The native host CLI also works without Python:

```sh
fm1-rs tools lz4-compress input.bin block.bin
fm1-rs tools lz4-decompress block.bin input-again.bin
fm1-rs tools loader-wrap loader.bin ota.bin
```

All commands operate on local files; none opens USB/MIDI or flashes firmware.

The compatibility corpus compares the live bindings against independently
loaded frozen Python, checking the source paths to prevent accidentally loading
the oracle twice. It covers all 440 pinned source hashes, 224 compressor vectors
through the 1,100,000-byte input boundary, 540 malformed/prefix decoder cases,
length extension boundaries, 36 loader wrappers/refusals and three native CLI
commands. Accepted truncated final literals and zero-distance legacy decoder
behavior remain explicit compatibility cases. Invalid offsets/extensions return
errors without unsafe reads. The new output cap refuses expansion beyond
1,100,000 bytes; a separate frozen witness confirms that previously unbounded
expansion. Compressed input has 10,000 bytes of additional framing allowance.

Reproduce on Windows or Linux:

```sh
python tests/rust_build_policy.py FROZEN_R01_SOURCE
```

This is a packaging checkpoint. It does not replace the firmware's update
loader, modify the pinned recovery stub or qualify a new physical image.
