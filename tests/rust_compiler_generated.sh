#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Generated PI32 C on a natural32-bit ABI, retaining all layout/PCM assertions.
# Usage: bash tests/rust_compiler_generated.sh CORE_OUTPUT BRIDGE_OUTPUT ZIG [OUTPUT]
set -eu
cd "$(dirname "$0")/.."
core=$(realpath "${1:?generated core output directory}")
bridge=$(realpath "${2:?generated application output directory}")
zig=$(realpath "${3:?pinned Zig compiler}")
out=${4:-build/compiler-generated-regression}
mkdir -p "$out" build/gen
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
native=(-target x86-linux-musl -msse2 -mfpmath=sse -ffp-contract=off -fexcess-precision=standard -O2 -std=gnu11 -w -ffunction-sections -fdata-sections)
"$zig" cc "${native[@]}" -c "$core/libcore.rlib.c" -o "$out/core.o"
objects=()
for package in hal types formats engine rt app jam_bridge;do
    "$zig" cc "${native[@]}" -c "$bridge/libfm1_$package.rlib.c" -o "$out/$package.o"
    objects+=("$out/$package.o")
done
common=(-DEN_HOST -D_POSIX_C_SOURCE=200809L -Ibuild/gen -Ifirmware/src -Ifirmware/src/dsp -Wl,--gc-sections -static)
rust=(-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_VOICES=1 -DEN_RUST_WORKER=1 -DEN_RUST_FM6=1 -DEN_RUST_FACTORY=1 -DEN_RUST_FORMATS=1 -DEN_RUST_STORAGE=1 -DEN_RUST_HARMONY=1 -DEN_RUST_APP=1)
web=(tests/reference/compiler_web_trace.c tests/reference/compiler_engine_observer.c)
abi=tests/reference/compiler_serial_abi.c
"$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB "${web[@]}" -lm -o "$out/c0"
"$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB "${rust[@]}" "${web[@]}" "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated"
"$out/c0" "$out/c0.pcm" > "$out/c0.json"
"$out/generated" "$out/generated.pcm" > "$out/generated.json"
cmp "$out/c0.pcm" "$out/generated.pcm"
cmp "$out/c0.json" "$out/generated.json"
"$zig" cc "${native[@]}" "${common[@]}" tests/reference/effects_reference.c -lm -o "$out/c0-effects"
"$zig" cc "${native[@]}" "${common[@]}" "${rust[@]}" tests/reference/effects_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-effects"
for case in 0 1 2 3 4 5 6 7 8 20 30 31;do
    "$out/c0-effects" "$case" "$out/c0.trace" > "$out/c0-effects-$case.json"
    "$out/generated-effects" "$case" "$out/generated.trace" > "$out/generated-effects-$case.json"
    cmp "$out/c0.trace" "$out/generated.trace"
    cmp "$out/c0-effects-$case.json" "$out/generated-effects-$case.json"
    sha256sum "$out/generated.trace" > "$out/effects-$case.sha256"
done
python3 tests/reference/runner_widgets_extract.py . build/gen/runner_widgets.inc
for profile in policy draw;do
    runner_flags=()
    if [ "$profile" = draw ];then runner_flags+=(-DRUNNER_DRAW_TEST=1);fi
    "$zig" cc "${native[@]}" "${common[@]}" "${runner_flags[@]}" tests/reference/runner_policy_reference.c -lm -o "$out/c0-runner-$profile"
    "$zig" cc "${native[@]}" "${common[@]}" "${runner_flags[@]}" -DEN_RUST_APP=1 tests/reference/runner_policy_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-runner-$profile"
    "$out/c0-runner-$profile" > "$out/c0-runner-$profile.bin"
    "$out/generated-runner-$profile" > "$out/generated-runner-$profile.bin"
    cmp "$out/c0-runner-$profile.bin" "$out/generated-runner-$profile.bin"
    sha256sum "$out/generated-runner-$profile.bin" > "$out/runner-$profile.sha256"
done
python3 tools/ui_abi.py --check
"$zig" cc "${native[@]}" "${common[@]}" tests/reference/ui_state_reference.c -lm -o "$out/c0-ui-state"
"$zig" cc "${native[@]}" "${common[@]}" -DEN_RUST_APP=1 tests/reference/ui_state_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-ui-state"
"$out/c0-ui-state" > "$out/c0-ui-state.bin"
"$out/generated-ui-state" > "$out/generated-ui-state.bin"
cmp "$out/c0-ui-state.bin" "$out/generated-ui-state.bin"
sha256sum "$out/generated-ui-state.bin" > "$out/ui-state.sha256"
sha256sum "$core/libcore.rlib.c" "$bridge"/*.rlib.c "$out/generated.pcm" > "$out/identities.sha256"
echo 'Generated-C natural32-bit factory,12 effects traces, full Runner policy/pixels and UI layout/defaults match current C0.'
