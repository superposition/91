#!/usr/bin/env python3
"""Reproducible chord-only standards arrangements for the FM-1 Songbook."""
import argparse
import copy
import hashlib
import json
import pathlib
import re
import zlib
from chord_chart import from_chords
from fm1_jam import ROOT, REST, THIRD, FIFTH, APPROACH, compile_pack, decompile

BLUE = 'Cm7 | Cm7 | Fm7 | Fm7 | Dm7b5 | G7b9 | Cm7 | Cm7 | Ebm7 | Ab7 | Dbmaj7 | Dbmaj7 | Dm7b5 | G7b9 | Cm7 | G7b9'
LEAVES = ('Am7 | D7 | Gmaj7 | Cmaj7 | F#m7b5 | B7 | Em9 | E7b9 | '
          'Am7 | D7 | Gmaj7 | Cmaj7 | F#m7b5 | B7 | Em9 | Em6 | '
          'F#m7b5 | B7alt | Em9 | E7b9 | Am7 | D7 | Gmaj7 | G6 | '
          'F#m7b5 | B7alt | Em9 Eb9 | Dm9 Db9 | Cmaj9 | B7b9 | Em9 | Em6')
SATIN = ('Dm7 G7 | Dm7 G7 | Em7 A7 | Em7 A7 | Am7 D7 | Abm7 Db7 | Cmaj7 | Em7b5 A7alt | '
         'Dm7 G7 | Dm7 G7 | Em7 A7 | Em7 A7 | Am7 D7 | Abm7 Db7 | Cmaj7 Dm7 | Ebdim7 Em7 | '
         'Gm7 C7 | Gm7 C7 | Fmaj7 | Fmaj7 | Am7 D7 | Am7 D7 | Dm7 G7 | Em7 A7 | '
         'Dm7 G7 | Dm7 G7 | Em7 A7 | Em7 A7 | Am7 D7 | Abm7 Db7 | Cmaj7 | Em7b5 A7alt')
WORKSHOP = 'Dm9 | G7b9 | Cmaj9 | A7alt | Dm9 | G9 | C6 | C6 | Dm7b5 | G7b9 | Cm9 | Cm6 | Fm9 | Bb9 | Ebmaj9 | G7alt'


def mask(*steps):
    return sum(1 << s for s in steps)


def voices():
    text = (ROOT/'firmware/src/dsp/fm6_dy.h').read_text(encoding='utf-8')
    body = re.sub(r'/\*.*?\*/', '', text.split('static const uint8_t FM6_DY', 1)[1].split('= {', 1)[1], flags=re.S)
    rows = [list(map(int, re.findall(r'\d+', r))) for r in re.findall(r'\{([^{}]+)\}', body)]
    ep = rows[0][:]
    # Algorithm 5: three parallel 2-op pairs. Reduce only modulators, retaining
    # carrier level and the human-authored envelopes. Keep concert-pitch keys.
    assert ep[110] == 4
    for op in (0,2,4):
        ep[op*17+14] = max(0, ep[op*17+14]-8)
    ep[117] = 24
    result = []
    for name, data in [('Round EP',ep),('Tine EP',rows[0]),('Drawbars',rows[6]),
                       ('Muted Gtr',rows[24]),('Round Bass',rows[2]),('Solid Bass',rows[9])]:
        data = data[:]
        if len(result)<4: data[117]=24
        data[118:128] = list(name[:10].ljust(10).encode('ascii'))
        result.append(dict(name=name,data=data))
    return result


def arrange(base, style):
    d=copy.deepcopy(base)
    d['voice_programs']=voices()
    d['parts'].update(keys=0,bass=4,swell=2,keys_gain=42,bass_gain=56,
                      swell_amount=0,variation=0,keys_gate=62,bass_gate=58,
                      touch=28,brightness=44,attack=0)
    # Full, restrained mix. No pitch effects; a small plate leaves room to play.
    settings={0:76,2:72,3:64,47:0,163:0,69:8,70:14,123:0,124:80,67:0,
              85:7,86:11,87:5,88:5,49:87,52:58,55:25,58:29,61:22,64:20}
    if style=='bossa':
        settings.update({3:54,52:35,55:0,58:26,64:33,86:5})
        lanes=[mask(0,6,8,14),mask(4,12),0,mask(0,2,4,6,8,10,12,14),0,mask(0,3,6,10,13)]
        events=[{0:0,6:FIFTH,8:0,14:FIFTH},{0:0,4:FIFTH,8:0,14:APPROACH},
                {0:0,6:FIFTH,8:0,12:THIRD,14:FIFTH},{0:0,6:FIFTH,8:0,14:APPROACH}]
        comp=mask(0,6,8,14)
    elif style=='soul':
        lanes=[mask(0,6,10),mask(4,12),mask(12),mask(0,2,3,6,8,10,11,14),mask(15),mask(7,13)]
        events=[{0:0,3:0,6:FIFTH,8:0,11:THIRD,14:FIFTH},
                {0:0,6:FIFTH,8:0,10:THIRD,15:APPROACH},
                {0:0,3:0,6:THIRD,8:0,11:FIFTH,14:0},
                {0:0,6:FIFTH,8:0,12:THIRD,15:APPROACH}]
        comp=mask(0,7,8,14)
    else:
        lanes=[mask(0,4,8,12),mask(4,12),mask(12),mask(0,2,4,6,8,10,12,14),mask(7,15),mask(3,11)]
        events=[{0:0,3:0,6:FIFTH,8:0,11:THIRD,14:0},
                {0:0,4:THIRD,6:FIFTH,8:0,14:APPROACH},
                {0:0,3:0,6:FIFTH,8:0,11:0,14:FIFTH},
                {0:0,4:THIRD,8:0,12:FIFTH,15:APPROACH}]
        comp=mask(0,6,8,14)
    d['sounds']=[dict(parameter=k,value=v) for k,v in settings.items()]
    d['bass']=[]
    for event in events:
        d['bass'].append(dict(notes=[event.get(s,REST) for s in range(16)],
            velocity=[(104 if s in (0,8) else 84 if s in (6,14) else 70) if s in event else 0 for s in range(16)],gate=58))
    d['drums']=[]
    for variant in range(4):
        lane=list(lanes)
        if variant in (1,3): lane[1]|=mask(14)
        if variant==3: lane[5]|=mask(14,15)
        d['drums'].append(dict(lanes=lane,comp=comp,comp_gate=62))
    d['sources']=['DX voices: PERF-1 public-domain DY bank; tools/fm6_dy.txt and LICENSING.md. Round EP reduces three modulator levels by8; no ROM voices added.']
    return d


def main():
    from songbook import factory
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--output',type=pathlib.Path,default=ROOT/'build/standards')
    output=ap.parse_args().output
    output.mkdir(exist_ok=True,parents=True)
    template=factory(0)
    items=[
        ('Blue Bossa',BLUE,'C','bossa','https://www.jazzguitar.be/blog/blue-bossa/',0),
        ('Autumn Leaves',LEAVES,'E','soul','https://www.jazzguitar.be/blog/autumn-leaves/',1),
        ('Satin Doll',SATIN,'C','funk','https://www.jazzguitar.be/blog/satin-doll/',2),
        ('Blue Bossa Deep',BLUE,'C','soul','https://www.jazzguitar.be/blog/blue-bossa/',None),
        ('Leaves Drawbars',LEAVES,'E','funk','https://www.jazzguitar.be/blog/autumn-leaves/',None),
        ('Satin Muted Guitar',SATIN,'C','funk','https://www.jazzguitar.be/blog/satin-doll/',None),
        ('Major Minor Lab',WORKSHOP,'C','soul','Original ii-V-I exercise, including major and minor cadences.',None),
    ]
    manifest=[];header=['/* Generated by tools/build_standards.py. Chords only; original accompaniment. */\n']
    for index,(name,chart,key,style,url,factory) in enumerate(items):
        base=arrange(template,style)
        if index==4: base['parts'].update(keys=2,keys_gain=30,keys_gate=48)
        if index==5: base['parts'].update(keys=3,keys_gain=45,keys_gate=48)
        sources=[url,'Chord-only arrangement; original groove and voice-leading, no melody copied.']
        if 'Leaves' in name:
            sources.append('Harmonic framework from published study. 6/9 colors reduced to6, B7b13/B7#9b13 voiced as7alt; F#m11 passing color omitted. This arrangement is explicit, not a note-for-note transcription.')
        d=from_chords(base,chart,key,name,voicing=1,sources=sources)
        b=compile_pack(d);assert compile_pack(decompile(b))==b
        stem='%02d-%s'%(index+1,name.lower().replace(' ','-'))
        (output/(stem+'.json')).write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')
        (output/(stem+'.jmp')).write_bytes(b)
        (output/(stem+'.chart.json')).write_text(json.dumps(dict(name=name,key=key,bars=[x.strip() for x in chart.split('|')],sources=sources),indent=2)+'\n',encoding='utf-8')
        for path in (output/(stem+'.json'),output/(stem+'.jmp')):
            (ROOT/'packs'/path.name).write_bytes(path.read_bytes())
        if factory is not None:
            header.append('static const uint8_t JP_SONG_%d[] = {\n'%factory)
            header.extend('    '+','.join(str(v) for v in b[o:o+24])+',\n' for o in range(0,len(b),24))
            header.append('};\n')
        manifest.append(dict(name=name,stem=stem,source_key=key,bars=len(chart.split('|')),
                             preset=index,bytes=len(b),crc32='%08x'%(zlib.crc32(b)&0xffffffff),sha256=hashlib.sha256(b).hexdigest()))
    (ROOT/'firmware/src/dsp/song_factory.inc').write_text(''.join(header),encoding='ascii')
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(manifest,indent=2))


if __name__=='__main__': main()
