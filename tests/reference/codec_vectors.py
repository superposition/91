# SPDX-License-Identifier: GPL-3.0-only
"""Convert independently exported C/JS/Python fixtures to safe dispatch requests."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
fixtures=ROOT/'rust/fm1-formats/tests/formats'
vectors=[]
def add(name,op,args=(),blobs=(),expected=()):vectors.append(dict(name=name,op=op,args=list(args),blobs=[list(b) for b in blobs],expected=list(expected)))
def raw(n,seed=91,mul=37):return bytes((seed+i*mul)&255 for i in range(n))
def u16(b,i):return int.from_bytes(b[i:i+2],'little')
def u32(b,i):return int.from_bytes(b[i:i+4],'little')
def words(v):return b''.join((x&0xffffffff).to_bytes(4,'little')for x in v)
def get(path):return json.loads(path.read_text())
for i,(kind,data) in enumerate(get(ROOT/'build/formats/constructors.json')):
 b=bytes(data);name=f'C-{kind}-{i}'
 if kind.startswith('global'):add(name,50,[b[28] if len(b)==32 else 0,*b[29:32]], [b[4:12],b[12:28]],b)
 elif kind=='settings':
  n=u16(b,4);at=(24+n+1)&~1;add(name,51,[u16(b,at),u16(b,at+2)],[b[8:8+n],b[8+n:20+n],b[20+n:24+n]],b)
 elif kind=='voices':add(name,52,[],[b[8:8+u32(b,4)*128]],b)
 elif kind=='map':add(name,55,[b[3]-48,*b[5:8]],[b[8:8+b[4]*3]],b)
 elif kind=='kit':add(name,57,[u32(b,4),b[116]],[b[8:80],b[80:116],b[117:132]],b)
 elif kind=='bank':add(name,56,[b[5],u16(b,6),u16(b,8),*b[12:16]],[b[16:3776],b[3776:3836]],b)
 elif kind in ('extras','bass'):add(name,54,[int(kind=='bass'),b[5],*b[6:8]],[b[8:]],b)
 elif kind=='kinds':add(name,53,[],[b[4:]],b)
 elif kind=='legacy':
  count=u16(b,8);stride=count*2+74
  params=b''.join(b[20+i*stride:20+i*stride+count*2]for i in range(10));pools=b''.join(b[20+i*stride+count*2:22+i*stride+count*2]for i in range(10));pats=b''.join(b[22+i*stride+count*2:20+(i+1)*stride]for i in range(10))
  add(name,59,[count,u16(b,10),u16(b,12),u16(b,14),u16(b,16),u16(b,18)],[params,pools,pats],b)
  add(name+'-record',58,[u16(pools,0)],[params[:count*2],pats[:72]],b[20:20+stride])
c=get(fixtures/'c-reference.json');js=get(fixtures/'js-reference.json');py=get(fixtures/'python-reference.json')
for seed,row in enumerate(c['voices']):
 add(f'C-unpack-{seed}',20,[],[raw(128,seed)],row[0]);add(f'C-pack-{seed}',21,[],[raw(155,seed,53)],row[1])
for seed,row in enumerate(c['records']):add(f'C-record-{seed}',48,[0xab5],[b''.join((seed*19-i*3).to_bytes(2,'little',signed=True)for i in range(176)),b''.join(((seed*907+i*1637)&65535).to_bytes(2,'little')for i in range(36)),raw(96,seed),raw(18,seed,13),raw(12,seed,17)],row)
for seed,row in enumerate(c['frames']):add(f'C-frame-{seed}',29,[17,seed,3000],[raw(seed,seed)],row)
add('C-dump',25,[130],[words([i*9-100 if i!=23 else 8000 for i in range(24)])],c['dump'])
add('C-editor-voice',27,[15,16],[raw(128)],c['editor_voice'])
for i,p in enumerate(js['pieces']):
 b=bytes(p);n=128 if b[7]&1 else 0;add(f'JS-piece-{i}',41,[b[5],b[6],u16(b,8),b[7]],[b[14:390],b[390:396],b[396:396+n],b[396+n:-4]],b)
for i,row in enumerate(js['sets']):
 b=bytes(row);np,nv=b[23:25];pieces=[b[27+j*387:27+(j+1)*387]for j in range(np)];at=27+np*387
 add(f'JS-set-{i}',43,[b[21],b[22]],[b[5:21],words([v for p in pieces for v in [p[0],p[1],u16(p,3),p[2]]]),b''.join(p[5:381]for p in pieces),b''.join(p[381:387]for p in pieces),b[at:at+nv*128],b[at+nv*128:-4]],b)
for i,row in enumerate(js['backups'][:3]):
 b=bytes(row);at=8+u16(b,6);config=[];data=b''
 for j in range(b[5]):
  n=u16(b,at+1);config.extend([b[at],len(data),n]);data+=b[at+3:at+3+n];at+=3+n
 add(f'JS-backup-{i}',45,[],[b[8:8+u16(b,6)],words(config),data],b)
for p in sorted((ROOT/'packs').glob('*.jmp')):add('pack-'+p.stem,97,[],[p.read_bytes()],p.read_bytes())
for tail in [bytes(4),bytes([2,83,255,119])]:
 session=b'JSM'+bytes([1,1,0,0,16,0,3,60,0,0,0,100,0])+tail;add('session-'+str(tail),98,[],[session],session)
for i,row in enumerate(js['packed']):add(f'JS-pack7-{i}',72,[],[row['input']],row['encoded']);add(f'JS-unpack7-{i}',73,[len(row['input'])],[row['encoded']],words([len(row['input'])])+bytes(row['input']))
for i,p in enumerate(js['ota']):
 n=[0,1,6,7,8,31,32,127,128,511,512][i];add(f'JS-loader-{i}',93,[0xfedcba98,3],[raw(n)],p)
for seed,v in enumerate(py['vectors']):
 b=raw(seed*2,seed);add(f'PY-crc16-{seed}',70,[seed*257],[b],words([v['crc']]));add(f'PY-enc-{seed}',71,[seed*257],[b],v['enc']);add(f'PY-sfc-{seed}',9,[seed,0,0x980f],[b],v['sfc']);add(f'PY-key-{seed}',10,[],[raw(32,seed)],words([v['key']]));add(f'PY-marker-{seed}',12,[],[b],v['marker'])
add('PY-product',14,[],[py['package']],b'FM-1_7000300')
for seed,b in enumerate(py['entries']):add(f'PY-entry-{seed}',18,[seed*2053,0x12345678+seed*4096,0x100+seed,seed,255-seed,seed*1993],[raw(16,seed)],b)
for i,v in enumerate(py['compensation']):add(f'PY-compensate-{i}',13,[v['offset'],v['want']],[v['app']],v['bytes'])
def seven(data):
 value=int.from_bytes(data,'little');return bytes((value>>i)&127 for i in range(0,len(data)*8,7))
def ota(command,body):
 return b'\xf0'+seven(bytes([0,89,command,len(body)&255,len(body)>>8,0])+body+bytes([~sum(body)&255]))+b'\xf7'
def jam_reply(op,token,status,data):
 body=bytes([op,token,status])+bytes(v for b in data for v in [b>>4,b&15]);return bytes([240,125,80,97])+body+bytes([-sum(body)&127,247])
data=bytes([0,255,128,17,42]);nibbles=bytes(v for b in data for v in [b>>4,b&15]);request=bytes(js['jam_request']);reply=jam_reply(7,11,3,data)
for op in [6,90]:add('PY-nibble-encode-'+str(op),op,[],[data],nibbles)
for op in [7,89]:add('PY-nibble-decode-'+str(op),op,[],[nibbles],data)
for op in [3,84]:add('PY-sum7-'+str(op),op,[],[data],words([-sum(data)&127]))
for op in [36,79]:add('PY-jam-reply-'+str(op),op,[7,11,3],[data],reply)
for op in [37,86]:add('PY-jam-reply-parse-'+str(op),op,[],[reply],words([7,11,3])+data)
for op in [38,88]:add('JS-jam-request-parse-'+str(op),op,[],[request[1:-1]],words([1,0]))
add('JS-jam-request',85,[1,0],[],request);add('PY-jam-preflight',78,[],[request[1:-1]],words([0]))
add('PY-jam-begin',80,[],[bytes([1,2,1,2,3,4,5,6,7,8])],words([130,0x12345678]))
add('PY-jam-chunk',81,[],[bytes([3,0,5])+nibbles],words([3,5])+data)
add('PY-jam-control',82,[],[bytes([7,1,2])],words([7,130]));add('PY-value14',91,[],[bytes([1,2])],words([130]))
add('PY-dx-ack',83,[3,16],[],bytes([240,125,80,87,3,16,247]))
for kind in [70,90,91]:
 payload=words([0,0x12345678,0xffffffff,0x80000001]);frame=bytes([240,125,80,kind])+seven(payload)+bytes([247]);add('PY-word-frame-'+str(kind),76,[kind],[payload],frame);add('PY-word-parse-'+str(kind),77,[kind],[frame],payload)
for op in [39,74]:add('PY-ota-encode-'+str(op),op,[17],[data],ota(17,data))
add('PY-ota-body',75,[],[bytes([0,89,17,5,0,0])+data+bytes([~sum(data)&255])],words([17,5])+data)
add('PY-ota-wire',87,[],[ota(17,data)],words([17,5])+data);add('PY-ota-wire-old',40,[],[ota(17,data)],words([17])+data)
for kind,args,expected in [(0,[255],[240,125,80,66,1,127,247]),(1,[255,0xffff],[240,125,80,66,4,127,127,127,247]),(2,[],[240,125,80,66,5,247]),(3,[],[240,125,80,66,6,247]),(4,[],[240,125,80,80,2,247]),(5,[175,0xffffffff],[240,125,80,80,1,1,47,63,127,247]),(6,[15],[240,125,80,86,126,15,247]),(7,[1],[240,125,80,70,1,247])]:add('PY-request-'+str(kind),92,[kind,*args],[],expected)
for kind,expected in [(0,[240,0,50,69,0,0,0,64,127,247]),(1,ota(17,bytes([1]))),(2,[240,34,36,53,127,247])]:add('PY-loader-command-'+str(kind),95,[kind],[],expected)
read=bytes([3])+words([0xfedcba98])+bytes([0,2,0]);add('PY-loader-request',94,[0],[ota(48,read)],words([3,0xfedcba98,512]))
identity=b'FM-1_7000300'+bytes(15);add('PY-loader-identity',94,[1],[ota(17,identity)],identity)
for seed,frame in enumerate(c['frames']):add(f'C-frame-parse-{seed}',30,[],[frame],words([17,seed,3000])+raw(seed,seed))
add('C-dump-parse',26,[],[c['dump']],words([130,24,*[i*9-100 if i!=23 else 8000 for i in range(24)]]))
for i,b in enumerate(js['chunks']):add(f'JS-chunk-parse-{i}',32,[],[b[1:-1]],words([23,0,3840])+raw([1,2,48,255,256][i]))
for seed,b in enumerate(py['entries']):add(f'PY-entry-parse-{seed}',17,[],[b],words([seed*2053,0x12345678+seed*4096,0x100+seed,seed,255-seed,seed*1993])+raw(16,seed))
(ROOT/'rust/fm1-codecs-wasm/tests/vectors.json').write_text(json.dumps(vectors,separators=(',',':'))+'\n')
print(len(vectors),'independent constructor/C/JS/Python vectors')
