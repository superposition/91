// SPDX-License-Identifier: GPL-3.0-only
// Four loaded DX programs through the unchanged app/AudioWorklet ABI.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const corpus=JSON.parse(fs.readFileSync('rust/fm1-engine/tests/factory/reference.json'));
const symbols=fs.readFileSync('rust/fm1-types/src/params.rs','utf8');
const id=name=>+symbols.match(new RegExp(`pub const ${name}: Self = Self\\((\\d+)\\)`))[1];
const sizes=[1,3,63,64,127,128,129,255,256,511,1024];
const reports=[];
// A saved legacy/off session is essential: default Songbook boot deliberately
// refuses pending legacy voice commits while that union carries the bass FIFO.
for(const algorithm of [0,3,5,31]){
 const bank=new Uint8Array(3836),view=new DataView(bank.buffer);
 bank.set([72,89,80,51,2,0,176,0,1,0,120,1,1,1,1,0]);view.setUint16(16,corpus.presets[0].pool,true);
 for(let i=0;i<36;i++)view.setUint16(18+i*2,corpus.presets[0].patterns[i],true);
 bank.fill(255,16+74,16+74+176);
 for(const [name,value] of Object.entries({CM_SND:42,CM_LVL:100,CM_DENS:100,CM_LOCK:1,AC_LOCK:1,PAD:0,ACID:0,BASS:0,PULSE:0,NOISE:0,ECHO:0,REVERB:0}))bank[16+74+id(name)]=value;
 bank.set(corpus.acid[91],16+256);bank.set(corpus.pluck[13].slice(0,6),16+250);bank.set(corpus.pluck[13].slice(6),16+274);
 const voices=new Uint8Array(2056);voices.set([86,79,88,49,1]);
 for(let i=0;i<6;i++){const p=8+i*17;voices.fill(99,p,p+7);voices[p+8]=39;voices[p+12]=7<<3;voices[p+14]=99;voices[p+15]=2;}
 voices.fill(99,8+102,8+106);voices.fill(50,8+106,8+110);voices[8+110]=algorithm;voices[8+111]=15;voices[8+117]=24;voices.set(new TextEncoder().encode(`FM6 ALG ${algorithm}`),8+118);
 async function boot(path){let memory;const wasi=new Proxy({clock_time_get:(_a,_b,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},fd_write:(_a,_b,_c,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();assert.equal(Object.keys(ex).length,24);new Uint8Array(memory.buffer,ex.web_store(0),bank.length).set(bank);ex.web_store_set_len(0,bank.length);new Uint8Array(memory.buffer,ex.web_store(3),voices.length).set(voices);ex.web_store_set_len(3,voices.length);const session=Uint8Array.from([74,83,77,1,0,0,0,16,0,3,60,0,0,0,120,0,0,83,0,0]);new Uint8Array(memory.buffer,ex.web_store(30),20).set(session);ex.web_store_set_len(30,20);ex.web_seed(0);ex.web_master(2800);ex.web_boot();return{ex,memory};}
 const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);let samples=0,calls=0,peak=0,hash=2166136261;
 function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
 function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);for(const channel of['web_out_l','web_out_r']){const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n),b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),n);assert.deepEqual(b,a,`algorithm${algorithm} call${calls}`);for(const bits of b)hash=Math.imul(hash^bits,16777619)>>>0;for(const v of new Float32Array(rust.memory.buffer,rust.ex[channel](),n)){assert(Number.isFinite(v));peak=Math.max(peak,Math.abs(v));}}remaining-=n;samples+=n*2;}
  if(!process.argv.includes('--audio-only'))assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));}
 function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
 advance(6000);tap(6);input('web_keys',1<<12);advance(100);input('web_keys',0);advance(1500);
 for(const key of[0,4,7,12,19,24,0]){input('web_keys',1<<key);advance(120);input('web_keys',0);advance(90);}
 tap(10);advance(4000);
 const counters=Array.from({length:8},(_,i)=>rust.ex.web_rust_stat(48+i));
 for(const i of[0,1,3,4,5,6,7])assert(counters[i]>0,`FM counter${48+i} activates`);
 assert(peak>0.01&&peak<1);assert.equal(rust.memory.buffer.byteLength,33554432);
 reports.push({algorithm,compared_samples:samples,render_calls:calls,peak,pcm_hash:hash.toString(16).padStart(8,'0'),fm6:counters});
}
assert.equal(new Set(reports.map(x=>x.pcm_hash)).size,4,'all four loaded algorithms change audible PCM');
console.log(JSON.stringify({scenario:'loaded-user-FM6-algorithms-key-retrigger-stop-tails',cases:reports}));
