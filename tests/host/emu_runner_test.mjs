// SPDX-License-Identifier: GPL-3.0-only
// Actual instrument Game entry/steer/pause/restart/exit through its24 public exports.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path){
 let memory;
 const wasi=new Proxy({clock_time_get:(_a,_b,p)=>{new DataView(memory.buffer).setBigUint64(p,0n,true);return 0;},fd_write:(_a,_b,_c,p)=>{new DataView(memory.buffer).setUint32(p,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
 const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
 const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();
 assert.equal(Object.keys(ex).length,24);ex.web_seed(91);ex.web_master(2800);ex.web_boot();return{ex,memory};
}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);
const sizes=[1,17,31,32,33,127,128,129,255,256,511,1024];let calls=0,samples=0,hash=2166136261,peak=0;
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){
 const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);
 for(const channel of['web_out_l','web_out_r']){
  const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n),b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),n);
  assert.deepEqual(b,a,`Runner PCM call${calls}`);for(const value of b)hash=Math.imul(hash^value,16777619)>>>0;
  for(const value of new Float32Array(rust.memory.buffer,rust.ex[channel](),n)){assert(Number.isFinite(value));peak=Math.max(peak,Math.abs(value));}
 }
 samples+=n*2;remaining-=n;
 }assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));}
function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
advance(6000);tap(10); // The secret entry is available while stopped.
const before=rust.ex.web_rust_stat(82);
input('web_buttons',(1<<1)|(1<<4));advance(150);input('web_buttons',0);advance(13000);
for(let i=0;i<32;i++){
 input('web_enc',0,i&1?-3:2);advance(180);tap(i&1?12:13);
 if(i%5===0){input('web_keys',1<<4);advance(160);input('web_keys',0);advance(1100);}
}
tap(10);advance(1600);tap(10);advance(5000);
const zeroBefore=rust.ex.web_rust_stat(82);input('web_render',0);assert.equal(rust.ex.web_rust_stat(82),zeroBefore);
const applicationCalls=rust.ex.web_rust_stat(82)-before;assert(applicationCalls>2000,'real instrument must execute Rust application policy/drawing');
tap(6);advance(2000); // Exact C0 audio and every pixel above also verify HOME leaves the game.
assert.equal(rust.ex.web_rust_stat(83),0);assert.equal(rust.memory.buffer.byteLength,33554432);assert(peak>0.05&&peak<1);
console.log(JSON.stringify({scenario:'kick-runner-steering-restart-pause-exit',compared_samples:samples,render_calls:calls,peak,pcm_hash:hash.toString(16).padStart(8,'0'),application_calls:applicationCalls,public_exports:24,memory_bytes:rust.memory.buffer.byteLength}));
