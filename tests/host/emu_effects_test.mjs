// SPDX-License-Identifier: GPL-3.0-only
// Real panel requests: held tone/bus/ring FX, release, bank changes and tails.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path){
  let memory;
  const wasi=new Proxy({clock_time_get:(_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},fd_write:(_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
  const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
  const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();ex.web_seed(91);ex.web_master(2800);ex.web_boot();return{ex,memory};
}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);
const sizes=[1,17,127,128,129,255,256,511,1024];let calls=0,samples=0,peak=0,hash=2166136261;
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){
  let remaining=Math.round(ms*44.1);
  while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);
    for(const channel of ['web_out_l','web_out_r']){const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n),b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),n);assert.deepEqual(b,a,`FX PCM call ${calls}`);for(const bits of b)hash=Math.imul(hash^bits,16777619)>>>0;for(const x of new Float32Array(rust.memory.buffer,rust.ex[channel](),n)){assert(Number.isFinite(x));peak=Math.max(peak,Math.abs(x));}}
    samples+=n*2;remaining-=n;
  }
  if(!process.argv.includes('--audio-only'))assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));
}
function tap(b){input('web_buttons',1<<b);advance(100);input('web_buttons',0);advance(100);}
function hold(keys,ms=900){input('web_keys',keys.reduce((m,k)=>m|1<<k,0));advance(ms);input('web_keys',0);advance(300);}
advance(6000);tap(6);hold([12],400);advance(1000); // F4 legacy instrument.
tap(0);advance(400); // FX menu; avoid its double-tap edit shortcut.
hold([7,8,9,10,13]); // Talk/Wah/phaser/flanger/tape.
hold([14,16,18]); // Punch-in filter/delay/phaser share a target bus.
hold([4]);hold([6]);hold([11]);hold([12]); // Freeze, reverse swell, reverse, stop.
input('web_enc',0,1);advance(200); // Beat FX bank.
hold([0,4,6]);hold([9]);hold([10]);hold([11]);hold([12]);hold([13]);
input('web_enc',0,1);advance(200); // Flex Beat bank.
hold([0],1800);hold([25],1800);
tap(0);tap(10);advance(3500); // Stop with reverb/delay tails.
assert.equal(Object.keys(rust.ex).length,24);assert.equal(rust.memory.buffer.byteLength,33554432);assert(peak>0.05&&peak<1);
const counts=Array.from({length:4},(_,i)=>rust.ex.web_rust_stat(28+i));assert(counts.every(x=>x>0),'all four FX render paths execute');assert.equal(counts[0],counts[1],'master and echo cover the same frames');assert.equal(counts[0],counts[2],'tank covers the same frames');assert(counts[3]>=counts[0]*2,'sampler and tone execute each frame');
console.log(JSON.stringify({scenario:'panel-effects-banks-release-tails',compared_samples:samples,render_calls:calls,peak,pcm_hash:hash.toString(16).padStart(8,'0'),effects:{master_samples:counts[0],echo_samples:counts[1],tank_samples:counts[2],colour_sample_calls:counts[3]},memory_bytes:rust.memory.buffer.byteLength}));
