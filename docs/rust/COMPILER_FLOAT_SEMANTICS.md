# Preserving floating semantics through generated C

The proposed [compiler addition](../../rust/toolchain/mrustc-float-semantics.patch) repairs reproduced `f32` constant folding, signed-zero constant identity and floating remainder semantics. Fresh generated core and application libraries now reproduce the complete factory audio fingerprint on both natural i386 and diagnostic WASM. No firmware DSP expression or comparison tolerance was changed.

The patch is based on mrustc `5205980b25d4d15494abdb45103e59e8dc309970` **after** the existing combined PI32 patch. It was built in a separate local clone; the root compiler checkout/binary and live/device builds were untouched. [compiler-float-proof.json](compiler-float-proof.json) records source, compiler, patch and output identities.

## The failure and its isolation

The original generated-code browser run first differs after about 105.912 seconds: factory block 36,490, right sample 7. Expected signed PCM is -3,997; generated code produces -3,996. Their normalized Float32 bit patterns are `b9f9d000` and `b9f9c000`.

The same failure occurs with unmodified generated C compiled for static i386 Linux/musl using SSE2, strict binary32 evaluation and no packing. All original generated layout assertions pass naturally. C0 completes 55,000 blocks with frozen hash `baeff971`; the original generated build produces `158b0971`, with the same blit/MIDI/clock counts. This rules out the WASM core-packing workaround as the cause of this discrepancy.

Selecting C effects makes the complete WASM run pass. Selecting C voices or C FM6 leaves the identical first mismatch. A scalar effects trace then finds an earlier state difference: the vinyl low-pass coefficient is `3e785438` in C0 and `3e785434` in generated C after the first block.

This minimal expression demonstrates a compiler defect independently of the instrument:

```rust
(-6.28318530717959_f32 * 300.0_f32).to_bits()
```

| Evaluation | Result |
| --- | --- |
| Standard rustc, constant or runtime | `c4eb9e95` |
| Original mrustc, runtime operand | `c4eb9e95` |
| Original mrustc, constant expression | `c4eb9e94` |

The original compiler stores parsed literals as `double` and folds the multiplication before applying binary32 rounding. It emits `-1.8849555922e+03f` rather than the correctly rounded sequential-f32 result, -1884.9556884765625. A diagnostic correction of this coefficient alone does not repair the full audio trace; no coefficient workaround is retained. The complete compiler repair fixes the reproduced trace.

## Repair and controls

| Defect | Repair |
| --- | --- |
| Unrounded f32 literal operands and double-width constant chains | Narrow after the literal's type is resolved; perform f32 arithmetic with float operands and round each result in MIR propagation and const evaluation |
| Structural constant comparison merged +0 and -0 | Compare stored floating bit patterns with an IEEE total-order key, preserving finite numeric ordering while distinguishing representations |
| Rust `%` emitted IEEE `remainderf/remainder` | Emit truncating `fmodf/fmod`; implement the corresponding typed MIR/const-evaluator operation |

The regression gate compares **49 bit-pattern cases** with rustc 1.94.1. It covers literal-to-integer rounding, chained addition/subtraction/multiplication/division, noncontracted products, underflow/overflow, signed zeros, const items/functions, f64 controls and both signs of remainder operands. The original compiler differs in 21 of the 37 ordinary-expression/runtime records and aborts when evaluating floating remainder through a const function. Failed outputs and logs remain under `build/compiler-float-vectors/` and `build/compiler-remainder-constants/`.

An additional **28 numeric comparison predicates** protect the distinction between representation identity and Rust comparison semantics. For f32 and f64, +0 equals -0, their `<` and `>` comparisons are false, and NaN equality/order remains unordered. Ordinary constant expressions, const-function evaluation and runtime calls all produce the expected mask `000020b1`. This probe extension required no further implementation change.

Native remainder probes link the real platform math library. The fixed instrument WASM has **zero imports**, so an unprovided math function cannot silently become a zero-returning environment stub. No floating library placeholder or undefined-symbol exemption is added.

## Generated-code evidence

Fresh Rust 1.90 core metadata/C and all five application libraries were emitted from the repaired clone. Every generated layout assertion remains enabled. Native i386 uses no packing; the WASM diagnostic keeps `-fpack-struct=4` for core only to model PI32's u64 field alignment, while all application units remain unpacked.

- Natural i386 factory: all **14,080,000 stereo Float32 values** match C0, hash `baeff971`, 4,679 blits and 11,531 outgoing MIDI packets including 6,215 clocks.
- Twelve focused native effects cases: **6,040,443 frames** compare complete PCM, USB output, scalar state, ring buffers and RNG traces. These include compressor/limiter transitions, injected nonfinite input guards, effect priorities, external clock, impulse tails and a zero-input null case. Initial effects coefficients also match C0 exactly.
- Six WASM scenarios compare **39,611,888 Float32 values**: factory, saved pack, effects panel, DJ snips, worker ownership and four FM6 algorithms. Factory/saved fingerprints remain `baeff971` / `28722449`; effects remain `fd659ab9`. The module retains 24 public exports, fixed 32 MiB memory and zero imports.
- The maintained regression commands below pass against fresh fixed-compiler outputs. A separate clean clone accepts the old combined patch followed by this addition and produces the identical combined diff.

```sh
bash tests/rust_compiler_float.sh MRUSTC_BINARY CORE_OUTPUT ZIG_BINARY
bash tests/rust_compiler_generated.sh CORE_OUTPUT BRIDGE_OUTPUT ZIG_BINARY
node tests/host/compiler_wasm_imports.mjs FIXED_INSTRUMENT_WASM
```

The native commands require Linux with static i386 execution, pinned Zig 0.17.1 and rustup's installed 1.94.1 toolchain. The generated-output directories contain the real core and five crate `.rlib.c` / `.rlib` files. Raw exploratory commands are retained separately under `build/compiler-investigation/`; the two maintained scripts provide the repeatable gates.

## Adoption and qualification boundary

The additional patch SHA-256 is `f4f469d7184d6cdb134b6f8e751fa1534f5a5556f90c4658defd02aefa9a50cd`. The resulting combined diff is `9c6321c51f606dfc7b60151020023c8b01d311b7e8f11577bf2057f912ad9d26`. Parent review adopted that exact combined diff in `mrustc-pi32v2.patch` and its verification manifest. A fresh checkout needs only the combined patch; the separate addition records the repair against the earlier compiler. The verifier continues to reject any unrecorded compiler diff. The old root compiler binary, diff and manifest were archived before applying the addition; rebuilding the compiler/core/application outputs is a separate gate from the clone evidence above.

Vendor compilation, exported-static/map checks, physical atomics, stack/RAM/cycle qualification and device testing remain separate gates. The diagnostic MCU atomic helper is serial-only and panic traps remain active. These results establish compiler-semantic defects and their repair; they do not establish a cause for the separate device USB failure.

## Adopted compiler rebuild

[compiler-adoption-proof.json](compiler-adoption-proof.json) records the fresh
root compiler/core and six-crate vendor-LTO rebuild. The 49 floating records,
28 comparisons, immutable/exported-static section checks, natural-i386 factory
and all 12 focused effects traces pass again with graphics, storage and harmonic
paths selected. The factory remains `baeff971`; effects cover 6,040,443 frames.

The private MCU link uses the repaired compiler, current DSP, storage, harmony
and queue guard, with application graphics and the new hardware layer disabled.
Its image is 574,704 bytes, ordinary RAM is 97,988/98,304 and the pool is
339,404/344,064. The image body has 2,544 bytes of linker headroom. Strict register
access, soft-double, allocator, TLS and unwind-table checks pass; the abort-only
compatibility handlers remain explicitly recorded. Installer fake-device and
actual-package simulated-NOR checks pass, with the pinned recovery stub intact.
This image was never flashed and supplies no runtime deadline or stack evidence.
