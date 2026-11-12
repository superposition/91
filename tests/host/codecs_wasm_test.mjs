// SPDX-License-Identifier: GPL-3.0-only
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { getCodecs, codecWords } from '../../web/codec-runtime.mjs';
const codecs=getCodecs(),vectors=JSON.parse(fs.readFileSync('rust/fm1-codecs-wasm/tests/vectors.json'));
const originalMemory=codecs.memoryBytes();let attempts=0;
for(const v of vectors){const actual=codecs.call(v.op,v.args,v.blobs);assert.deepEqual(Array.from(actual),v.expected,v.name);assert(codecs.guards(),v.name);attempts++;}
const python=JSON.parse(fs.readFileSync('rust/fm1-formats/tests/formats/python-reference.json'));
const pack=Uint8Array.from(python.package),logical=codecs.call(15,[],[pack]);
assert.deepEqual(codecs.call(99,[],[python.flash,python.ota,new TextEncoder().encode('FM-1_7000300')]),pack);
assert.deepEqual(codecs.call(16,[],[logical,new TextEncoder().encode('FM-1_7000300')]),pack);
const directory=codecs.call(19,[],[pack]);assert.equal(codecWords(directory.slice(0,8))[0],2);
let keyHash=2166136261;
for(let key=0;key<65536;key++){const blob=codecs.call(11,[key]);assert.equal(codecWords(codecs.call(10,[],[blob.slice(0,32)]))[0],key);for(const byte of blob)keyHash=Math.imul(keyHash^byte,16777619)>>>0;}
assert.equal(keyHash,python.key_blob_hash);
const frozen=JSON.parse(fs.readFileSync('rust/fm1-formats/tests/formats/c-reference.json'));
for(let field=0;field<283;field++){
 let hash=2166136261;
 for(let value=0;value<256;value++){
  const op=field<128?20:21,n=field<128?128:155,mul=field<128?37:53;
  const data=Uint8Array.from({length:n},(_,i)=>(91+i*mul)&255);data[field<128?field:field-128]=value;
  for(const byte of codecs.call(op,[],[data]))hash=Math.imul(hash^byte,16777619)>>>0;
 }
 assert.equal(hash,frozen.voice_mutations[field],'frozen C DX7 field '+field);
}
let seed=91;
for(const v of vectors.filter(v=>[20,21,41,50,51,52,55,56,59,70,71,72,93].includes(v.op)).filter((_,i)=>i%11===0)){
 for(let i=0;i<64;i++){seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;const blobs=v.blobs.map(b=>b.slice());if(blobs.length){const b=blobs[(seed>>>0)%blobs.length];if(b.length)b[(seed>>>0)%b.length]^=seed&255;}
  try{codecs.call(v.op,v.args,blobs);}catch(e){assert.equal(e.name,'CodecError');}assert(codecs.guards());attempts++;
 }
}
assert.equal(codecs.memoryBytes(),originalMemory,'memory is fixed; no growth');
for(const op of [20,21,25,27,29,41,48,50,51,52,53,54,55,56,57,58,59,70,71,72,73,93])assert(codecs.counter(op)>0,'executed opcode'+op);
assert.throws(()=>codecs.call(1,[],[new Uint8Array(1_100_000)]),{name:'CodecError'});
assert.throws(()=>codecs.call(127),{name:'CodecError'});
for(const [op,args]of [[104,[2,7,65536]],[104,[1,256]],[60,[256]]])assert.throws(()=>codecs.call(op,args),{name:'CodecError'});
const schema=codecWords(codecs.call(0));assert.equal(schema[2],376);
console.log(JSON.stringify({vectors:vectors.length,attempts,dxFieldMutations:72448,keyBlobs:65536,memoryBytes:originalMemory,guards:codecs.guards()}));
