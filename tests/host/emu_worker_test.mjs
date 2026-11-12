// SPDX-License-Identifier: GPL-3.0-only
// Actual serialized WASM dispatch must exercise the same generation/stage API.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path){let memory;const wasi=new Proxy({clock_time_get:(_a,_b,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},fd_write:(_a,_b,_c,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});const{instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();ex.web_seed(91);ex.web_master(2800);ex.web_boot();return{ex,memory};}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);let calls=0,samples=0,peak=0,hash=2166136261;
const sizes=[1,17,127,128,129,255,256,511,1024];
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);for(const channel of['web_out_l','web_out_r']){const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n),b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),n);assert.deepEqual(b,a);for(const bits of b)hash=Math.imul(hash^bits,16777619)>>>0;for(const v of new Float32Array(rust.memory.buffer,rust.ex[channel](),n)){assert(Number.isFinite(v));peak=Math.max(peak,Math.abs(v));}}samples+=n*2;remaining-=n;}if(!process.argv.includes('--audio-only'))assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));}
function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
advance(8000);tap(6);input('web_keys',1<<12);advance(100);input('web_keys',0);advance(4000);tap(10);advance(2000);
const counts=Array.from({length:8},(_,i)=>rust.ex.web_rust_stat(56+i));
assert(counts[0]>1000);assert.equal(counts[0],counts[1]);assert.equal(counts[1],counts[3]);assert(counts[2]>=counts[0]*2&&counts[2]<=counts[0]*3);assert.equal(counts[0],counts[4]);assert.deepEqual(counts.slice(5),[0,0,0]);assert.equal(Object.keys(rust.ex).length,24);assert.equal(rust.memory.buffer.byteLength,33554432);assert(peak>0.05&&peak<1);
console.log(JSON.stringify({scenario:'serialized-generation-stage-ownership',compared_samples:samples,render_calls:calls,peak,pcm_hash:hash.toString(16).padStart(8,'0'),worker:counts,memory_bytes:rust.memory.buffer.byteLength}));
