#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Optional connected FM-1 acceptance run; writes evidence, never saves a slot.

Rust supplies the schedule, validation, report and WAV sink. These adapters open
MIDI/audio devices, copy recorder buffers and provide monotonic-clock facts.
Requires mido, python-rtmidi, numpy and sounddevice for connected operation.
"""
import argparse
import json
import tempfile
import time
from pathlib import Path
import mido
import numpy as np
import sounddevice as sd
from jam_midi import Device
from fm1log import decode, decode_extra, decode_diag
from rust_authoring import call as policy


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--seconds', type=int, default=600)
    ap.add_argument('--audio-device', type=int, required=True)
    ap.add_argument('--out', default='build/hardware/songbook')
    a = ap.parse_args()
    folder = Path(a.out)
    folder.mkdir(parents=True, exist_ok=True)
    telemetry, extra, diag, trace, audio, flags, events, errors = ([] for _ in range(8))
    before = final = None

    class Tap:
        def __init__(self, source): self.source = source
        def iter_pending(self):
            for msg in self.source.iter_pending():
                row = decode(msg.data) if msg.type == 'sysex' else None
                if row: telemetry.append(row)
                if msg.type == 'sysex':
                    for decoder, dest in ((decode_extra, extra), (decode_diag, diag)):
                        detail = decoder(msg.data)
                        if detail: dest.append(dict(ms=telemetry[-1]['ms'] if telemetry else 0, **detail))
                yield msg
        def close(self): self.source.close()

    def capture(data, frames, info, status):
        audio.append(data.copy())
        if status: flags.append(str(status))

    with Device() as dev:
        def apply(actions):
            for action in actions:
                kind = action['kind']
                if kind == 'control':
                    dev.control(action['field'], action['value'])
                    if 'seconds' in action:
                        events.append(dict(seconds=action['seconds'], field=action['field'], value=action['value']))
                elif kind == 'request': dev.request(action['command'], action['data'])
                elif kind == 'arm': dev.out.send(mido.Message('sysex', data=[125, 80, 70, 1]))
                elif kind == 'upload':
                    active = dev.read()
                    dev.audition(active)
                    events.append(dict(seconds=action['seconds'], upload_bytes=len(active)))
                else: raise RuntimeError('unknown acceptance action')

        dev.inp = Tap(dev.inp)
        before = dev.status()
        original = dev.read()
        (folder/'active-before.jmp').write_bytes(original)
        try:
            apply(policy('accept-init', features=dev.features()))
            deadline = time.monotonic() + 5
            phase = 0
            while True:
                reply = dev.song_status() if phase < 2 else dev.status()
                decision = policy('accept-ready', phase=phase, reply=reply, now=time.monotonic(), deadline=deadline)
                phase = decision['phase']
                if decision['kind'] == 'ready': break
                if decision['kind'] == 'sleep': time.sleep(.1)
            apply(policy('accept-start'))
            with sd.InputStream(device=a.audio_device, samplerate=44100, channels=2, dtype='int16', callback=capture):
                start = time.monotonic()
                state = dict(armed=0, printed=-1, done=0, uploaded=False)
                while policy('accept-window', elapsed=time.monotonic()-start, seconds=a.seconds):
                    elapsed = time.monotonic()-start
                    decision = policy('accept-step', state=state, elapsed=elapsed, now=time.monotonic(), stop=(folder/'stop').exists())
                    apply(decision['actions'])
                    state = decision['state']
                    status = dev.status()
                    status['seconds'] = elapsed
                    trace.append(status)
                    if decision['print']:
                        print(json.dumps(dict(seconds=decision['printed_seconds'], state=status, telemetry=telemetry[-1:])), flush=True)
                    time.sleep(.25)
            final = dev.status()
            policy('accept-validate', before=before, after=final, telemetry=telemetry, trace=trace, flags=flags)
        except Exception as error:
            errors.append(repr(error))
        finally:
            try:
                apply(policy('accept-cleanup', before=before))
                time.sleep(.5)
            except Exception as error:
                errors.append('cleanup: '+repr(error))
    samples = np.concatenate(audio) if audio else np.zeros((0, 2), dtype='int16')
    with tempfile.NamedTemporaryFile(dir=folder, suffix='.pcm', delete=False) as raw:
        raw.write(samples.tobytes())
        raw_path = Path(raw.name)
    try:
        report = policy('accept-finalize', folder=str(folder), pcm=str(raw_path), before=before, after=final,
                        errors=errors, flags=flags, events=events, telemetry=telemetry, trace=trace, extra=extra, diag=diag)
    finally:
        raw_path.unlink()
    print(json.dumps(report, indent=2))
    raise SystemExit(policy('accept-exit', errors=errors))


if __name__ == '__main__': main()
