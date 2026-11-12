"""Render Jam Packs with the actual host engine; inspect/import selected MIDI tracks."""
import json
import os
import pathlib
import re
import subprocess
import sys

def midi_from_log(log,output,bpm):
    import mido
    tempo=mido.bpm2tempo(bpm); mf=mido.MidiFile(ticks_per_beat=480);tracks={};last={}
    meta=mido.MidiTrack();meta.append(mido.MetaMessage('set_tempo',tempo=tempo));mf.tracks.append(meta)
    for line in pathlib.Path(log).read_text().splitlines():
        match=re.match(r'(\d+) ms: ([0-9A-F]{2}) ([0-9A-F]{2}) ([0-9A-F]{2})',line)
        if not match:continue
        ms=int(match[1]);st,a,b=[int(match[i],16) for i in (2,3,4)]
        if st>=0xf0:continue
        ch=st&15
        if ch not in tracks:
            tracks[ch]=mido.MidiTrack();last[ch]=0;mf.tracks.append(tracks[ch])
            tracks[ch].append(mido.MetaMessage('track_name',name={0:'Keys',2:'Bass',4:'Swells',9:'Drums'}.get(ch,f'Channel {ch+1}')))
        tick=round(ms*480000/tempo)
        msg=mido.Message.from_bytes([st,a,b] if st&0xf0 not in (0xc0,0xd0) else [st,a]);msg.time=tick-last[ch];last[ch]=tick;tracks[ch].append(msg)
    mf.save(output)

def preview(root,pack,out,bars=16,bpm=100,key=0,minor=False,phrases=2):
    from fm1_jam import load
    out=pathlib.Path(out).resolve();out.mkdir(parents=True,exist_ok=True)
    (out/'preview.jmp').write_bytes(load(pack))
    script=f'jamstop\njampack preview.jmp\njam 1 {key+12*minor}\njam 2 {bars}\nsetpar 5 {bpm}\njam 0 1\nwait 60\nwav preview.wav\ntap PLAY\nwait {round(bars*4*60000/bpm*phrases)}\njamstop\nwait 2000\nwavstop\nshot preview.png\n'
    (out/'preview.amb').write_text(script)
    engine=root/'build/host/amb_host'
    env=os.environ.copy();env.pop('AMB_LEGACY_BOOT',None)
    if os.name=='nt':
        def linux(p):return '/mnt/'+p.drive[0].lower()+str(p)[2:].replace('\\','/')
        # No shell quoting or interpolation: bash script and all paths are separate argv.
        subprocess.run(['wsl','-d','Ubuntu-22.04','--cd',linux(root),'--','sh','host/build_host.sh'],check=True,env=env)
        subprocess.run(['wsl','-d','Ubuntu-22.04','--cd',linux(out),'--',linux(engine),'preview.amb','.'],check=True,env=env)
    else:
        subprocess.run(['sh','host/build_host.sh'],cwd=root,check=True,env=env)
        subprocess.run([str(engine),'preview.amb','.'],cwd=out,check=True,env=env)
    midi_from_log(out/'midi_out.txt',out/'preview.mid',bpm)
    return dict(wav=str(out/'preview.wav'),midi=str(out/'preview.mid'),screen=str(out/'preview.png'))

def inspect_midi(path):
    import mido
    mf=mido.MidiFile(path)
    result=[]
    for i,t in enumerate(mf.tracks):
        notes=[m.note for m in t if m.type=='note_on' and m.velocity]
        result.append(dict(track=i,name=t.name,notes=len(notes),range=[min(notes),max(notes)] if notes else [],channels=sorted({m.channel for m in t if hasattr(m,'channel')})))
    return dict(ticks_per_beat=mf.ticks_per_beat,tracks=result)

def import_midi(path,base,track,tonic,start_bar,role,minor=False):
    import mido
    from fm1_jam import decompile,load,REST
    d=decompile(load(base));mf=mido.MidiFile(path)
    if mf.type==2:raise ValueError('type 2 MIDI has independent sequences; export the selected sequence first')
    if not 0<=track<len(mf.tracks):raise ValueError('track out of range')
    if any(m.type=='time_signature' and (m.numerator,m.denominator)!=(4,4) for t in mf.tracks for m in t):raise ValueError('import currently requires 4/4 MIDI')
    start=(start_bar-1)*4*mf.ticks_per_beat;end=start+4*mf.ticks_per_beat;absolute=0;events=[]
    for msg in mf.tracks[track]:
        absolute+=msg.time
        if msg.type=='note_on' and msg.velocity and start<=absolute<end:events.append((absolute-start,msg.note,msg.velocity))
    if not events:raise ValueError('no notes in selected track/bar')
    if role=='bass':
        if len(d['bass'])>=8:raise ValueError('all eight bass pattern slots are used')
        notes=[REST]*16;vel=[0]*16
        for tick,n,v in events:
            step=min(15,round(tick*4/mf.ticks_per_beat));iv=(n-tonic+6)%12-6
            if notes[step]!=REST and notes[step]!=iv:raise ValueError('selected bass track is polyphonic at the sixteenth grid')
            notes[step]=iv;vel[step]=v
        d['bass'].append(dict(notes=notes,velocity=vel,gate=58))
    else:
        if d.get('version')==2:raise ValueError('v2 harmony uses authored charts; inspect the MIDI and edit chart destinations explicitly')
        groups={}
        for tick,n,v in events:groups.setdefault(tick//max(1,mf.ticks_per_beat//32),set()).add(n%12)
        added=0
        for pcs in groups.values():
            if len(pcs)!=4:continue
            match=None
            for root_pc in range(12):
                for scale,shape in enumerate(({0,4,7,9},{0,3,7,9})):
                    if {(x-root_pc)%12 for x in pcs}==shape:match=(root_pc,scale);break
                if match:break
            if not match:continue
            root_pc,scale=match;rel=(root_pc-tonic)%12
            if any(n['bass']==rel and n['sixth']==rel and n['scale']==scale and n['mode']==int(minor) for n in d['nodes']):continue
            if len(d['nodes'])>=16:break
            d['nodes'].append(dict(bass=rel,sixth=rel,scale=scale,label=4 if scale else 0,mode=int(minor)))
            for row in d['transitions']:row.append(0)
            row=[0]*len(d['nodes']);home=d['cadence'][int(minor)][0];row[home]=8;d['transitions'].append(row);d['transitions'][home][-1]=3;added+=1
        if not added:raise ValueError('no new major/minor sixth voicings found; inspect the MIDI or choose another bar')
    d['sources'].append(f'User-selected MIDI reference: {pathlib.Path(path).name}, track {track}, bar {start_bar}, role {role}; relative to tonic {tonic}.')
    return d
