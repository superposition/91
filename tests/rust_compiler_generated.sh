#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Generated PI32 C on a natural32-bit ABI, retaining all layout/PCM assertions.
# Usage: bash tests/rust_compiler_generated.sh CORE_OUTPUT BRIDGE_OUTPUT ZIG [OUTPUT] [FROZEN_SOURCE]
set -eu
cd "$(dirname "$0")/.."
repo=$PWD
core=$(realpath "${1:?generated core output directory}")
bridge=$(realpath "${2:?generated application output directory}")
zig=$(realpath "${3:?pinned Zig compiler}")
out=${4:-build/compiler-generated-regression}
baseline=${5:-}
mkdir -p "$out" build/gen
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
native=(-target x86-linux-musl -msse2 -mfpmath=sse -ffp-contract=off -fexcess-precision=standard -O2 -std=gnu11 -w -ffunction-sections -fdata-sections)
"$zig" cc "${native[@]}" -c "$core/libcore.rlib.c" -o "$out/core.o"
objects=()
for package in hal types formats engine rt app jam_bridge;do
    "$zig" cc "${native[@]}" -c "$bridge/libfm1_$package.rlib.c" -o "$out/$package.o"
    objects+=("$out/$package.o")
done
common=(-UNDEBUG -include tests/reference/compiler_assertions.h -DEN_HOST -D_POSIX_C_SOURCE=200809L -Ibuild/gen -Ifirmware/src -Ifirmware/src/dsp -Wl,--gc-sections -static)
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
python3 tests/reference/ui_frame_extract.py . build/gen/ui_frame_port.inc port
cp tests/reference/ui_frame_frozen.inc build/gen/ui_frame_frozen.inc
"$zig" cc "${native[@]}" "${common[@]}" -DEN_RUST_HANDOFF=1 tests/reference/ui_frame_reference.c -lm -o "$out/c0-ui-frame"
"$zig" cc "${native[@]}" "${common[@]}" -DEN_RUST_HANDOFF=1 -DEN_RUST_APP=1 tests/reference/ui_frame_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-ui-frame"
"$out/c0-ui-frame" > "$out/c0-ui-frame.bin"
"$out/generated-ui-frame" > "$out/generated-ui-frame.bin"
cmp "$out/c0-ui-frame.bin" "$out/generated-ui-frame.bin"
python3 tests/reference/ui_frame_extract.py . build/gen/ui_accel_current.inc accel
for profile in ui_controls usb_trs;do
    options=()
    if [ "$profile" = ui_controls ];then options+=(-DEN_RUST_APP=1 '-DUI_ACCEL_SOURCE="ui_accel_current.inc"');else options+=(-DEN_RUST_USB=1);fi
    "$zig" cc "${native[@]}" "${common[@]}" tests/reference/${profile}_reference.c -lm -o "$out/c0-$profile"
    "$zig" cc "${native[@]}" "${common[@]}" "${options[@]}" tests/reference/${profile}_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-$profile"
    "$out/c0-$profile" > "$out/c0-$profile.bin"
    "$out/generated-$profile" > "$out/generated-$profile.bin"
    cmp "$out/c0-$profile.bin" "$out/generated-$profile.bin"
done
python3 tests/reference/ui_frame_extract.py . build/gen/ui_button_current.inc button_current
python3 tests/reference/ui_frame_extract.py . build/gen/ui_button_port.inc button_port
cp tests/reference/ui_button_frozen.inc build/gen/ui_button_frozen.inc
python3 tests/reference/ui_frame_extract.py . build/gen/ui_classify_current.inc classify
cp tests/reference/ui_classify_frozen.inc build/gen/ui_classify_frozen.inc
for profile in buttons classify;do
    if [ "$profile" = buttons ];then source='-DUI_BUTTON_SOURCE="ui_button_current.inc"';else source='-DUI_CLASSIFY_SOURCE="ui_classify_current.inc"';fi
    "$zig" cc "${native[@]}" "${common[@]}" "$source" tests/reference/ui_${profile}_reference.c -lm -o "$out/c0-ui-$profile"
    "$zig" cc "${native[@]}" "${common[@]}" "$source" -DEN_RUST_APP=1 tests/reference/ui_${profile}_reference.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-ui-$profile"
    "$out/c0-ui-$profile" > "$out/c0-ui-$profile.bin"
    "$out/generated-ui-$profile" > "$out/generated-ui-$profile.bin"
    cmp "$out/c0-ui-$profile.bin" "$out/generated-ui-$profile.bin"
done
python3 tools/ui_input_abi.py --check
python3 tests/reference/ui_input_host.py . build/gen/ui_input_current.inc
"$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB '-DEN_VERSION="0.3.0"' tests/reference/ui_input_reference.c firmware/src/dsp/amb.c -lm -o "$out/c0-ui-input"
"$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB '-DEN_VERSION="0.3.0"' "${rust[@]}" tests/reference/ui_input_reference.c firmware/src/dsp/amb.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-ui-input"
"$out/c0-ui-input" > "$out/c0-ui-input.bin"
"$out/generated-ui-input" > "$out/generated-ui-input.bin"
cmp "$out/c0-ui-input.bin" "$out/generated-ui-input.bin"
python3 tools/ui_performance_abi.py --check
python3 tools/ui_values_abi.py --check
if [ -n "$baseline" ];then
    baseline=$(realpath "$baseline")
    (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/frozen-source.log"
    python3 tests/reference/ui_input_host.py "$baseline" build/gen/ui_input_frozen.inc
fi
for profile in ui-performance jam-labels ui-automatic ui-values;do
    driver=tests/reference/ui_performance_reference.c
    if [ "$profile" = jam-labels ];then driver=tests/reference/jam_labels_reference.c;fi
    if [ "$profile" = ui-automatic ];then driver=tests/reference/ui_automatic_reference.c;fi
    if [ "$profile" = ui-values ];then driver=tests/reference/ui_values_reference.c;fi
    "$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB '-DEN_VERSION="0.3.0"' "$driver" firmware/src/dsp/amb.c -lm -o "$out/c0-$profile"
    "$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB '-DEN_VERSION="0.3.0"' "${rust[@]}" "$driver" firmware/src/dsp/amb.c "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-$profile"
    "$out/c0-$profile" > "$out/c0-$profile.bin"
    "$out/generated-$profile" > "$out/generated-$profile.bin"
    cmp "$out/c0-$profile.bin" "$out/generated-$profile.bin"
    if [ -n "$baseline" ];then
        "$zig" cc "${native[@]}" "${common[@]}" -DEN_WEB '-DEN_VERSION="0.3.0"' '-DUI_INPUT_HOST="ui_input_frozen.inc"' "$driver" "$baseline/firmware/src/dsp/amb.c" -lm -o "$out/frozen-$profile"
        "$out/frozen-$profile" > "$out/frozen-$profile.bin"
        cmp "$out/frozen-$profile.bin" "$out/generated-$profile.bin"
    fi
done
python3 tools/usb_reference.py . "$out/usb-source"
python3 tests/reference/usb_queue_reference.py . "$out/queue-source"
for profile in control transport queues tx host host_tx host_control;do
    source=(-I"$out/usb-source" -DFELUCCA_CDC=1 -DFELUCCA_UAC=1 -DUSB_DX=1 -DFELUCCA_OTA=1)
    fixture="tests/reference/usb_${profile}_reference.c"
    if [ "$profile" = queues ];then source=(-I"$out/queue-source" -Ifirmware/hal);fixture=tests/reference/usb_queue_reference.c;fi
    if [ "$profile" = tx ];then source+=(-DUSB_TX_ACTIVE_CONFIG_ONLY=1);fi
    if [ "$profile" = control ];then source+=(-DUSB_CONTROL_CONFIRMED_ABORTS=1);fi
    case "$profile" in host|host_tx|host_control) source+=(-DPERF_USB_HOST=1);;esac
    "$zig" cc "${native[@]}" "${common[@]}" "${source[@]}" "$fixture" -lm -o "$out/c0-usb-$profile"
    "$zig" cc "${native[@]}" "${common[@]}" "${source[@]}" -DEN_RUST_USB=1 "$fixture" "$abi" "${objects[@]}" "$out/core.o" -lm -o "$out/generated-usb-$profile"
    "$out/c0-usb-$profile" > "$out/c0-usb-$profile.bin"
    "$out/generated-usb-$profile" > "$out/generated-usb-$profile.bin"
    cmp "$out/c0-usb-$profile.bin" "$out/generated-usb-$profile.bin"
done
# Corrected reset behavior deliberately differs from the legacy C witness.
# Compare both natural32 implementations to their independently compiled native
# counterparts, while each fixture also asserts its own reset invariants.
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/reference-native"
reset=(-I"$out/usb-source" -DFELUCCA_CDC=1 -DFELUCCA_UAC=1 -DFELUCCA_OTA=1)
for profile in c0 rust;do
    selected=();if [ "$profile" = rust ];then selected+=(-DEN_RUST_USB=1);fi
    objects_for_profile=();if [ "$profile" = rust ];then objects_for_profile+=("${objects[@]}" "$out/core.o" "$abi");fi
    "$zig" cc "${native[@]}" "${common[@]}" "${reset[@]}" "${selected[@]}" tests/reference/usb_reset_reference.c "${objects_for_profile[@]}" -lm -o "$out/generated-reset-$profile"
    "$out/generated-reset-$profile" > "$out/generated-reset-$profile.bin"
    library=();if [ "$profile" = rust ];then library+=("$out/reference-native/release/libfm1_jam_bridge.a");fi
    cc -O2 -UNDEBUG -ffp-contract=off -std=gnu99 -ffunction-sections -fdata-sections -Wl,--gc-sections -Ifirmware/src "${reset[@]}" "${selected[@]}" tests/reference/usb_reset_reference.c "${library[@]}" -lpthread -ldl -lm -o "$out/native-reset-$profile"
    "$out/native-reset-$profile" > "$out/native-reset-$profile.bin"
    cmp "$out/native-reset-$profile.bin" "$out/generated-reset-$profile.bin"
done
lease=(-I"$out/usb-source" -DPERF_USB_HOST=1 -DFELUCCA_CDC=1 -DFELUCCA_UAC=1 -DFELUCCA_OTA=1)
for mode in correction healthy;do
    mode_flags=();if [ "$mode" = healthy ];then mode_flags+=(-DUSB_LEASE_HEALTHY=1);fi
    for profile in c0 rust;do
        selected=();library=();objects_for_profile=()
        if [ "$profile" = rust ];then
            selected+=(-DEN_RUST_USB=1)
            library+=("$out/reference-native/release/libfm1_jam_bridge.a")
            objects_for_profile+=("${objects[@]}" "$out/core.o" "$abi")
        fi
        "$zig" cc "${native[@]}" "${common[@]}" "${lease[@]}" "${selected[@]}" "${mode_flags[@]}" tests/reference/usb_lease_reference.c "${objects_for_profile[@]}" -lm -o "$out/generated-lease-$mode-$profile"
        "$out/generated-lease-$mode-$profile" > "$out/generated-lease-$mode-$profile.bin"
        cc -O2 -UNDEBUG -ffp-contract=off -std=gnu99 -ffunction-sections -fdata-sections -Wl,--gc-sections -Ifirmware/src "${lease[@]}" "${selected[@]}" "${mode_flags[@]}" tests/reference/usb_lease_reference.c "${library[@]}" -lpthread -ldl -lm -o "$out/native-lease-$mode-$profile"
        "$out/native-lease-$mode-$profile" > "$out/native-lease-$mode-$profile.bin"
        cmp "$out/native-lease-$mode-$profile.bin" "$out/generated-lease-$mode-$profile.bin"
    done
    if [ "$mode" = healthy ];then cmp "$out/generated-lease-$mode-c0.bin" "$out/generated-lease-$mode-rust.bin";fi
done
sha256sum "$core/libcore.rlib.c" "$bridge"/*.rlib.c "$out/generated.pcm" > "$out/identities.sha256"
echo 'Generated-C natural32 factory,12 effects,Runner,UI layout/frame/encoders/buttons/classification,TRS and USB control/transport/queues match C0.'
