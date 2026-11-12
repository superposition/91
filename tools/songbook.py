"""Bounded Songbook v2 packs and three authored, transposable compositions."""
import json,re,pathlib
H,EXTRA,CHART,VOICE,META=1120,1152,1280,1536,2560

def programs():
    from fm1_jam import ROOT
    text=(ROOT/'firmware/src/dsp/fm6_dy.h').read_text(encoding='utf-8')
    body=text.split('static const uint8_t FM6_DY',1)[1].split('= {',1)[1]
    body=re.sub(r'/\*.*?\*/','',body,flags=re.S)
    rows=re.findall(r'\{([^{}]+)\}',body)
    source=[list(map(int,re.findall(r'\d+',r))) for r in rows]
    mellow=source[0][:];bark=source[0][:];bloom=source[0][:]
    # Envelope and touch changes retain the six-operator program's identity.
    for op in range(6):
        mellow[op*17+13]=(mellow[op*17+13]&3)|(3<<2)
        bark[op*17+14]=min(99,bark[op*17+14]+(5 if op%2==0 else 0))
        bloom[op*17+1]=min(bloom[op*17+1],35)
        bloom[op*17+7]=0
    items=[dict(name=n,data=d) for n,d in [('Mellow Tine',mellow),('Bark EP',bark),('Bloom EP',bloom),('Round FM',source[2]),('Articulated FM',source[9])]]
    for v in items:v['data'][118:128]=list(v['name'][:10].ljust(10).encode('ascii'))
    return items

def factory(song=0):
    from fm1_jam import factory as legacy
    d=legacy();d['version']=2
    d['name']=['Velvet Changes','Afterhours','Neon Turnaround'][song]
    # Each mode has the same functional indices; this preserves the home mode
    # when the player changes songs. Sixth roots describe the upper structure.
    major=[(0,7,0,6),(2,5,0,1),(4,7,0,1),(5,0,0,6),(7,2,1,3),(9,0,0,1),(11,2,1,2),
           (9,10,1,7),(5,5,1,4),(4,5,1,7),(2,7,1,3),(6,9,1,2),(0,0,0,0),(7,8,1,7),(5,5,0,0),(0,0,0,0)]
    minor=[(0,0,1,4),(2,5,1,2),(3,10,0,6),(5,8,0,1),(7,8,1,7),(8,8,0,0),(11,2,1,2),
           (9,10,1,7),(5,5,1,4),(7,2,1,3),(10,5,1,3),(6,9,1,2),(0,0,1,4),(7,8,1,7),(8,3,0,6),(0,0,1,4)]
    d['nodes']=[]
    for mode,rows in enumerate([major,minor]):
        for bass,sixth,scale,label in rows:
            d['nodes'].append(dict(bass=bass,sixth=sixth,scale=scale,label=label,mode=mode,emphasis=0,
                                  borrow=9 if label in (0,1) else 0 if label==6 else 1))
    d['cadence']=[[0,1,4],[16,17,20]]
    # Two destinations per bar: A and B, major and minor. Cadence answers are
    # explicit in the long versions instead of stretching every chord twice.
    charts=[
      [[[0,0,5,5,1,1,4,4,2,7,1,4,12,14,1,4], [3,3,8,8,2,2,7,7,1,1,4,13,0,0,4,4]],
       [[0,0,12,12,3,3,10,4,2,7,1,4,0,14,1,4], [3,3,10,10,2,7,1,4,3,8,2,7,1,4,0,0]]],
      [[[0,0,0,0,3,3,10,10,2,2,5,5,1,1,4,4], [8,8,1,4,0,0,7,7,1,1,4,4,0,0,4,4]],
       [[0,0,0,0,3,3,10,10,2,2,5,5,1,1,4,4], [8,8,1,4,0,0,7,7,1,1,4,4,0,0,4,4]]],
      [[[0,0,9,9,5,5,10,10,1,1,4,4,0,7,1,4], [3,3,11,11,2,7,1,4,14,8,2,7,1,4,0,0]],
       [[0,0,7,7,3,3,10,10,2,2,5,5,1,1,4,4], [8,8,11,11,2,7,1,4,8,8,2,7,1,4,0,0]]]
    ][song]
    d['charts']=[]
    for mode in range(2):
        versions=[]
        for bars in (8,16):
            parts=[]
            for part in range(2):
                a=charts[mode][part]
                if bars==16:
                    # Restate the motif, then develop a second eight-bar answer.
                    a=a[:12]+[1,4,0,0]+a[:12]+[1,4,0,0]
                parts.append([x+16*mode for x in a])
            versions.append(parts)
        d['charts'].append(versions)
    d['forms']=[dict(name='Authored',roles=[0,1,0,3] if song==2 else [0,0,2,3]),
                dict(name='Vamp',roles=[0]),dict(name='AABA',roles=[0,0,2,3]),dict(name='ABAB',roles=[0,1,0,1])]
    d['voice_programs']=programs()
    d['parts']=dict(keys=[0,2,1][song],bass=4 if song==2 else 3,swell=[2,5,19][song],
                    keys_gain=45,bass_gain=35,swell_amount=30,variation=1,keys_gate=65,bass_gate=58)
    d['song_id']=song
    # A composed straight-sixteenth pocket. Velocity and syncopation carry the
    # feel; independent random delays split the six-DX render into tiny jobs.
    for setting in d['sounds']:
        if setting['parameter'] in (47,163):setting['value']=0
    for drum in d['drums']:drum['comp']=0x4441 if song!=1 else 0x0441;drum['comp_gate']=65
    d['sources']=['Original composition and arrangement; Harris-informed Citypop.',
        'https://jazzworkshops.com/workshop-item/evolutionary-voicings-part-1/',
        'DX voices adapted from PERF-1 public-domain DY bank; see tools/fm6_dy.txt and LICENSING.md.']
    return d

def compile_pack(d):
    import fm1_jam as j
    base=j.factory()
    for key in ('name','scales','forms','bass','drums','sounds'):base[key]=d[key]
    b=bytearray(j.compile_pack(base)[:1120]);b.extend(bytes(META-len(b)))
    b[3]=ord('2');b[4]=2;b[5]=j.integer(len(d['nodes']),1,32,'nodes')
    for mode,group in enumerate(d['cadence']):
        if len(group)!=3:raise ValueError('cadence needs tonic/ii/dominant')
        for n,node in enumerate(group):
            b[9+3*mode+n]=j.integer(node,0,b[5]-1,'cadence')
            if d['nodes'][node]['mode']!=mode:raise ValueError('cadence crosses mode')
    for i,node in enumerate(d['nodes']):
        off=j.NODE+i*8 if i<16 else EXTRA+(i-16)*8
        for k,(name,hi) in enumerate([('bass',11),('sixth',11),('scale',1),('label',7),('mode',1),('emphasis',15),('borrow',15)]):
            b[off+k]=j.integer(node.get(name,0),0,hi,name)
    b[H:H+4]=b'SNG2';p=d['parts'];voices=d['voice_programs']
    b[H+4]=j.integer(len(voices),1,8,'voice programs')
    for i,name in enumerate(('keys','bass')):b[H+5+i]=j.integer(p[name],0,len(voices)-1,name)
    b[H+7]=j.integer(p['swell'],0,23,'swell sound')
    for i,name in enumerate(('keys_gain','bass_gain','swell_amount')):b[H+8+i]=j.integer(p[name],0,100,name)
    b[H+11]=j.integer(p.get('variation',1),0,3,'variation');b[H+12]=j.integer(d.get('song_id',0),0,2,'song id')
    b[H+16]=j.integer(p.get('keys_gate',65),1,100,'keys gate');b[H+17]=j.integer(p.get('bass_gate',58),1,100,'bass gate')
    for off,name,default in [(18,'touch',75),(19,'brightness',50),(20,'attack',0)]:b[H+off]=j.integer(p.get(name,default),0,100,name)
    for mode in range(2):
        for length,bars in enumerate((8,16)):
            for part in range(2):
                chart=d['charts'][mode][length][part]
                if len(chart)!=bars*2:raise ValueError('chart needs two destinations per bar')
                padded=chart+[chart[-1]]*(32-len(chart))
                for k,node in enumerate(padded):
                    j.integer(node,0,b[5]-1,'chart node')
                    if d['nodes'][node]['mode']!=mode:raise ValueError('chart crosses mode')
                    b[CHART+((mode*2+length)*2+part)*32+k]=node
    for i,voice in enumerate(voices):
        if len(voice['data'])!=128:raise ValueError('DX program needs 128 bytes')
        b[VOICE+i*128:VOICE+(i+1)*128]=bytes(j.integer(x,0,127,'DX byte') for x in voice['data'])
    b.extend(json.dumps(d.get('sources',[]),ensure_ascii=False,separators=(',',':')).encode('utf-8'))
    if len(b)>j.MAX:raise ValueError('song exceeds 3840 bytes')
    return bytes(b)

def decompile(b):
    import fm1_jam as j
    if len(b)<META or len(b)>j.MAX or b[:5]!=b'JMP2\2':raise ValueError('invalid v2 pack')
    base=bytearray(j.compile_pack(j.factory())[:1120]);base[16:40]=b[16:40]
    for start,end in [(40,56),(448,1120)]:base[start:end]=b[start:end]
    base[6:9]=b[6:9];d=j.decompile(bytes(base))
    d['version']=2;d['nodes']=[]
    for i in range(b[5]):
        off=64+i*8 if i<16 else EXTRA+(i-16)*8
        d['nodes'].append(dict(zip(['bass','sixth','scale','label','mode','emphasis','borrow'],b[off:off+7])))
    d['cadence']=[list(b[9:12]),list(b[12:15])];d.pop('transitions',None)
    d['charts']=[[[list(b[CHART+((m*2+l)*2+p)*32:CHART+((m*2+l)*2+p)*32+(16 if l==0 else 32)]) for p in range(2)] for l in range(2)] for m in range(2)]
    d['voice_programs']=[dict(name=bytes(b[VOICE+i*128+118:VOICE+i*128+128]).decode('ascii','replace').strip(),data=list(b[VOICE+i*128:VOICE+(i+1)*128])) for i in range(b[H+4])]
    d['parts']=dict(zip(['keys','bass','swell','keys_gain','bass_gain','swell_amount','variation'],b[H+5:H+12]));d['parts'].update(keys_gate=b[H+16],bass_gate=b[H+17])
    d['parts'].update(touch=b[H+18],brightness=b[H+19],attack=b[H+20]);d['song_id']=b[H+12];d['sources']=json.loads(b[META:] or b'[]');compile_pack(d);return d

def regenerate():
    """Regenerate reviewed factory JSON, binaries and the immutable C bank."""
    from fm1_jam import ROOT
    header=['/* Generated by python tools/songbook.py; GPL-3.0-only */\n']
    for song,stem in enumerate(('velvet-changes','afterhours','neon-turnaround')):
        d=factory(song);b=compile_pack(d)
        (ROOT/'packs'/f'{stem}.json').write_bytes((json.dumps(d,indent=2)+'\n').encode('utf-8'))
        (ROOT/'packs'/f'{stem}.jmp').write_bytes(b)
        header.append(f'static const uint8_t JP_SONG_{song}[] = {{\n')
        header.extend('    '+','.join(str(x) for x in b[i:i+24])+',\n' for i in range(0,len(b),24))
        header.append('};\n')
    (ROOT/'firmware/src/dsp/song_factory.inc').write_bytes(''.join(header).encode('ascii'))

if __name__=='__main__':regenerate()
