#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Check executed melodic paths in transport_reference.c's diagnostic traces.

This is the native adapter's private little-endian trace, not a saved file or
wire format. Parity compares every byte separately before this coverage check.
"""
import json
from pathlib import Path
import sys


def coverage(path, expected_calls):
    data = path.read_bytes()
    assert sys.byteorder == "little", "native trace parser requires little endian"
    words = memoryview(data).cast("I")
    position = calls = 0
    result = dict(recorded=False, motif=False, inverted=False, alternate=False,
                  restated=False, bar_answer=False, loop_answer=False,
                  bass_steps=False, max_midi_gates=0)
    while position < len(words):
        if words[position] == 0x4d:  # MIDI event: tag/frame/status/data1/data2
            position += 5
            continue
        assert words[position] == 0x41, (path, position, "audio tag")
        count = words[position + 1]
        position += 2 + 2 * count  # stereo int32 PCM
        position += 11  # transport state
        assert words[position] == 0x53, (path, position, "scheduling tag")
        position += 19 + 36 + 42  # state, patterns and delayed drum slots
        assert words[position] == 0x4c, (path, position, "melodic tag")
        state = words[position:position + 45]
        result["recorded"] |= bool(state[19] and state[16])
        played = bool(state[20] and not state[19] and not state[24])
        result["motif"] |= played
        result["inverted"] |= played and bool(state[21])
        result["alternate"] |= played and bool(state[22])
        result["restated"] |= played and bool(state[23])
        result["bar_answer"] |= bool(state[9])
        result["loop_answer"] |= bool(state[10])
        result["bass_steps"] |= state[29] < 16
        # Raw arrays after the 45 scalar words: MIDI notes start at word52.
        notes = data[(position + 45 + 52) * 4:(position + 45 + 54) * 4]
        result["max_midi_gates"] = max(result["max_midi_gates"],
                                        sum(note != 0 for note in notes))
        position += 45 + 62
        calls += 1
    assert position == len(words), (path, position, "truncated trace")
    assert calls == expected_calls, (path, calls, expected_calls)
    return result


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "build/transport-parity")
    results = {case: coverage(out / f"rust-{case}.trace", 10000)
               for case in range(35, 51)}
    aggregate = {
        key: (max(r[key] for r in results.values()) if key == "max_midi_gates"
              else any(r[key] for r in results.values()))
        for key in next(iter(results.values()))
    }
    assert all(value for key, value in aggregate.items() if key != "max_midi_gates"), aggregate
    assert aggregate["max_midi_gates"] == 8, aggregate
    report = dict(aggregate=aggregate, cases=results)
    (out / "melodic-coverage.json").write_text(json.dumps(report, indent=2) + "\n")
    print("Melodic trace coverage: " + json.dumps(aggregate, sort_keys=True))


if __name__ == "__main__":
    main()
