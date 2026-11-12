// SPDX-License-Identifier: GPL-3.0-only
// Real27-key DJ panel through the unchanged24-export browser ABI.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path){let memory;const wasi=new Proxy({clock_time_get:(_a,_b,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},fd_write:(_a,_b,_c,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();ex.web_seed(91);ex.web_master(2800);ex.web_boot();assert.equal(Object.keys(ex).length,24);return{ex,memory};}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);
const sizes=[1,17,31,32,33,127,128,129,255,256,511,1024];let calls=0,samples=0,peak=0,hash=2166136261;
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);for(const channel of['web_out_l','web_out_r']){const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n),b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),n);assert.deepEqual(b,a,`DJ PCM call${calls}`);for(const bits of b)hash=Math.imul(hash^bits,16777619)>>>0;for(const x of new Float32Array(rust.memory.buffer,rust.ex[channel](),n)){assert(Number.isFinite(x));peak=Math.max(peak,Math.abs(x));}}samples+=n*2;remaining-=n;}
 if(!process.argv.includes('--audio-only'))assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));}
function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
function hold(keys,ms=180){input('web_keys',keys.reduce((m,k)=>m|1<<k,0));advance(ms);input('web_keys',0);advance(220);}
advance(6000);tap(6);hold([12],400);advance(1000);
tap(0);advance(400);for(let i=0;i<3;i++){input('web_enc',0,1);advance(200);} // Fourth FX bank: DJ SNIPS.
const notesBefore=rust.ex.web_rust_stat(72),framesBefore=rust.ex.web_rust_stat(73);
for(let key=0;key<27;key++)hold([key]);
hold([0,4,17,21],1600);hold([0,1,2,3,4],1300);advance(5500);
const notes=rust.ex.web_rust_stat(72)-notesBefore,voiceSamples=rust.ex.web_rust_stat(73)-framesBefore;
assert(notes>=36,'all27keys and two polyphonic bursts must reach the Rust constructor');assert(voiceSamples>100000,'real active DJ voice samples execute');
const beforeZero=rust.ex.web_rust_stat(73);input('web_render',0);assert.equal(rust.ex.web_rust_stat(73),beforeZero,'zero render consumes no DJ samples');
tap(10);advance(1200);assert.equal(rust.memory.buffer.byteLength,33554432);assert(peak>0.05&&peak<1);
console.log(JSON.stringify({scenario:'all27-dj-keys-polyphony-release-and-stop',compared_samples:samples,render_calls:calls,peak,pcm_hash:hash.toString(16).padStart(8,'0'),dj:{accepted_notes:notes,voice_samples:voiceSamples},memory_bytes:rust.memory.buffer.byteLength,public_exports:24}));
