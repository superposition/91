// SPDX-License-Identifier: GPL-3.0-only
// Enter the legacy instrument through its real F4 key and compare the WASM
// scheduler during irregular audio callbacks, piece/genre changes and transport.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path) {
  let memory;
  const binary=fs.readFileSync(path);
  assert.equal(WebAssembly.Module.exports(new WebAssembly.Module(binary)).length,24,'public WASM export count');
  const wasi=new Proxy({
    clock_time_get:(_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
    fd_write:(_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}
  },{get:(object,key)=>key in object?object[key]:()=>0});
  const {instance}=await WebAssembly.instantiate(binary,{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
  const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();
  ex.web_seed(91);ex.web_master(2800);ex.web_boot();return {ex,memory};
}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);
let calls=0,samples=0,peak=0;
const sizes=[1,127,128,129,255,256,511,1024];
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){
  let remaining=Math.round(ms*44.1);
  while(remaining>0){
    const frames=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',frames);
    for(const channel of ['web_out_l','web_out_r']){
      const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),frames);
      const b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),frames);
      assert.deepEqual(b,a,`legacy irregular PCM call ${calls}`);
      for(const value of new Float32Array(rust.memory.buffer,rust.ex[channel](),frames)){assert(Number.isFinite(value));peak=Math.max(peak,Math.abs(value));}
    }
    remaining-=frames;samples+=frames*2;
  }
  assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),240*240),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),240*240));
  assert.equal(rust.ex.web_lit_keys(),reference.ex.web_lit_keys());assert.equal(rust.ex.web_lit_buttons(),reference.ex.web_lit_buttons());
}
function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
advance(6000);tap(6);input('web_keys',1<<12);advance(100);input('web_keys',0);advance(100);advance(1000);
const boundary=rust.ex.web_rust_stat(21),lanes=rust.ex.web_rust_stat(18);
const copiedBefore=rust.ex.web_rust_stat(35);
advance(5000);input('web_enc',2,1);advance(5000);tap(10);advance(1000);tap(10);advance(4000);
input('web_buttons',1<<5);advance(100);input('web_enc',2,1);advance(100);input('web_buttons',0);advance(6000);
assert.equal(rust.ex.web_rust_stat(21),boundary,'F4 leaves Songbook boundary scheduling');
assert(rust.ex.web_rust_stat(18)>lanes,'legacy drum decisions execute in Rust');
assert(rust.ex.web_rust_stat(20)>0,'legacy delayed-hit deadlines execute in Rust');
const melodic=Array.from({length:6},(_,index)=>rust.ex.web_rust_stat(22+index));
const voices=Array.from({length:2},(_,index)=>rust.ex.web_rust_stat(40+index));
assert(voices.every(value=>value>0),'percussion and plain pluck render blocks execute in Rust');
const tonal=[rust.ex.web_rust_stat(43),rust.ex.web_rust_stat(44)];
assert(tonal.every(value=>value>0),'drone and acid render blocks execute in Rust');
const pad=[rust.ex.web_rust_stat(42),rust.ex.web_rust_stat(46)];
assert(pad.every(value=>value>0),'pad worker and bus render blocks execute in Rust');
const controls=rust.ex.web_rust_stat(45);
assert(controls>0,'chunk-rate voice motion executes in Rust');
assert(melodic.every(value=>value>0),'acid, mutation, pluck, bass, bar and MIDI/transport paths execute in Rust');
assert.equal(rust.memory.buffer.byteLength,33554432);assert(peak>0.05 && peak<1);
assert(rust.ex.web_rust_stat(35)>copiedBefore,'real legacy piece/genre changes copy Rust factory data');
assert.equal(rust.ex.web_rust_stat(34),0,'legacy factory calls stay in bounds');
console.log(JSON.stringify({scenario:'legacy-f4-piece-genre-transport',compared_samples:samples,render_calls:calls,peak,
  sequencer:{lanes:rust.ex.web_rust_stat(18)-lanes,waits:rust.ex.web_rust_stat(20),boundaries:boundary},
  melodic:{acid:melodic[0],mutations:melodic[1],pluck:melodic[2],bass:melodic[3],bar:melodic[4],midi_transport:melodic[5]},
  voices:{drums:voices[0],pluck:voices[1],drone:tonal[0],acid:tonal[1],pad:pad[0],pad_bus:pad[1],controls},
  factory:{accesses:rust.ex.web_rust_stat(32),codecs:rust.ex.web_rust_stat(33),rejections:rust.ex.web_rust_stat(34),copied_bytes:rust.ex.web_rust_stat(35)},memory_bytes:rust.memory.buffer.byteLength}));
