#!/usr/bin/env python3
"""Optional connected FM-1 acceptance run; writes evidence, never saves a slot.

Requires mido, python-rtmidi, numpy and sounddevice. Runs all three full songs,
changes role profiles and transfers a pack while recording fixed-level USB.
"""
import argparse,json,time,wave
from pathlib import Path
import mido,numpy as np,sounddevice as sd
from jam_midi import Device
from fm1log import decode,decode_extra,decode_diag

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--seconds',type=int,default=600)
    ap.add_argument('--audio-device',type=int,required=True)
    ap.add_argument('--out',default='build/hardware/songbook')
    a=ap.parse_args();folder=Path(a.out);folder.mkdir(parents=True,exist_ok=True)
    telemetry=[];extra=[];diag=[];trace=[];audio=[];flags=[];events=[];errors=[];before=None;final=None
    class Tap:
        def __init__(self,source):self.source=source
        def iter_pending(self):
            for msg in self.source.iter_pending():
                row=decode(msg.data) if msg.type=='sysex' else None
                if row:telemetry.append(row)
                if msg.type=='sysex':
                    for decoder,dest in ((decode_extra,extra),(decode_diag,diag)):
                        detail=decoder(msg.data)
                        if detail:dest.append(dict(ms=telemetry[-1]['ms'] if telemetry else 0,**detail))
                yield msg
        def close(self):self.source.close()
    def capture(data,frames,info,status):
        audio.append(data.copy())
        if status:flags.append(str(status))
    with Device() as dev:
        dev.inp=Tap(dev.inp);before=dev.status();original=dev.read()
        (folder/'active-before.jmp').write_bytes(original)
        try:
            assert dev.features()['factory_songs']==3
            dev.control('play',0);dev.control('hold',0);dev.control('bars',16)
            dev.control('form',0);dev.control('song',0)
            deadline=time.monotonic()+5
            while dev.song_status()['pack_version']!=2 or dev.song_status()['loading'] or dev.status()['pending']:
                if time.monotonic()>deadline:raise TimeoutError('factory song activation')
                time.sleep(.1)
            dev.request(13,[1]);dev.control('play',1)
            with sd.InputStream(device=a.audio_device,samplerate=44100,channels=2,dtype='int16',callback=capture):
                start=time.monotonic();armed=0;printed=-1;done=set()
                while time.monotonic()-start<a.seconds:
                    if (folder/'stop').exists():raise InterruptedError('acceptance stopped by stop file')
                    elapsed=time.monotonic()-start
                    if time.monotonic()-armed>3:
                        dev.out.send(mido.Message('sysex',data=[125,80,70,1]));armed=time.monotonic()
                    for at,field,value in [(180,'song',1),(360,'song',2),(540,'touch',60),
                            (542,'brightness',45),(544,'keys_program',0),(546,'bass_program',3),
                            (548,'swell',40),(550,'hold',1),(560,'hold',0),(562,'idea',1)]:
                        if elapsed>=at and at not in done:
                            dev.control(field,value);events.append(dict(seconds=elapsed,field=field,value=value));done.add(at)
                    if elapsed>=570 and 'upload' not in done:
                        active=dev.read();dev.audition(active);events.append(dict(seconds=elapsed,upload_bytes=len(active)));done.add('upload')
                    state=dev.status();state['seconds']=elapsed;trace.append(state)
                    block=int(elapsed//30)
                    if block!=printed:
                        printed=block
                        print(json.dumps(dict(seconds=round(elapsed),state=state,telemetry=telemetry[-1:])),flush=True)
                    time.sleep(.25)
            final=dev.status()
            assert final['xruns']==before['xruns'],'audio xruns increased'
            assert telemetry,'no hardware telemetry'
            for field in ('xruns','late','crashes'):
                assert telemetry[-1][field]==telemetry[0][field],field+' increased'
            assert max(t['shed'] for t in telemetry)==0,'load shedding'
            assert not flags,'USB recording callback flags'
            assert len({s['crc32'] for s in trace})>=3,'not all songs activated'
            assert all(s['key']==before['key'] and s['minor']==before['minor'] for s in trace),'home key changed'
        except Exception as e:
            errors.append(repr(e))
        finally:
            try:
                dev.request(13,[0]);dev.control('play',0)
                dev.control('bars',before['bars']);dev.control('key',before['key']+12*before['minor'])
                dev.control('hold',before['hold']);dev.control('energy',before['energy'])
                dev.control('song',0);time.sleep(.5)
            except Exception as e:errors.append('cleanup: '+repr(e))
    samples=np.concatenate(audio) if audio else np.zeros((0,2),dtype='int16')
    with wave.open(str(folder/'device-songbook.wav'),'wb') as f:
        f.setparams((2,2,44100,0,'NONE','not compressed'));f.writeframes(samples.tobytes())
    report=dict(before=before,after=final,seconds=len(samples)/44100,errors=errors,capture_flags=flags,
                peak_pcm16=int(np.abs(samples.astype('int32')).max()) if len(samples) else 0,
                rms_pcm16=float(np.sqrt(np.mean(samples.astype('float64')**2))) if len(samples) else 0,
                telemetry_frames=len(telemetry),events=events)
    if telemetry:
        report.update(cpu_max=max(t['cpu'] for t in telemetry),cpu1_max=max(t['cpu1'] for t in telemetry),
                      render_max_us=max(t['max_us'] for t in telemetry),shed=max(t['shed'] for t in telemetry),
                      deltas={k:telemetry[-1][k]-telemetry[0][k] for k in ('xruns','late','crashes')})
    for name,value in [('report',report),('telemetry',telemetry),('status',trace),('extra',extra),('diag',diag)]:
        (folder/(name+'.json')).write_text(json.dumps(value,indent=2),encoding='utf-8')
    print(json.dumps(report,indent=2))
    if errors:raise SystemExit(1)

if __name__=='__main__':main()
