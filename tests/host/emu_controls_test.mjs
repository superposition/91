// SPDX-License-Identifier: GPL-3.0-only
// Actual nested frame -> C input -> Rust encoder policy. Source-level WASM QA.
import assert from 'node:assert/strict';
import fs from 'node:fs';
async function boot(path){
 let memory;
 const wasi=new Proxy({clock_time_get:(_a,_b,p)=>{new DataView(memory.buffer).setBigUint64(p,0n,true);return 0;},fd_write:(_a,_b,_c,p)=>{new DataView(memory.buffer).setUint32(p,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
 const{instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
 const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();assert.equal(Object.keys(ex).length,24);
 ex.web_seed(91);ex.web_master(2800);ex.web_boot();return{ex,memory};
}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]),control=await boot(process.argv[3]);
let samples=0;
function input(name,...args){for(const node of[reference,rust,control])node.ex[name](...args);}
function advance(ms){let left=Math.round(ms*44.1);while(left){const n=Math.min(left,256);input('web_render',n);left-=n;
 for(const name of['web_out_l','web_out_r']){
  assert.deepEqual(new Uint32Array(rust.memory.buffer,rust.ex[name](),n),new Uint32Array(reference.memory.buffer,reference.ex[name](),n));
  for(const value of new Float32Array(rust.memory.buffer,rust.ex[name](),n))assert(Number.isFinite(value));
 }samples+=n*2;
 }assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),57600),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),57600));}
advance(6000);input('web_buttons',1<<2);advance(100);input('web_buttons',0);advance(100); // PAD page.
const before=rust.ex.web_rust_stat(82),other=control.ex.web_rust_stat(82);
reference.ex.web_enc(3,4);rust.ex.web_enc(3,4);advance(100);
assert.equal((rust.ex.web_rust_stat(82)-before)-(control.ex.web_rust_stat(82)-other),1,'one actual Rust encoder call beyond the identical frame controller');
for(const role of[3,4,5,6])for(const step of[1,1,1,1,4,-1,-4,-16,16]){
 reference.ex.web_enc(role,step);rust.ex.web_enc(role,step);advance(20);
}
assert.equal(rust.ex.web_rust_stat(83),0);assert.equal(rust.memory.buffer.byteLength,33554432);
// Malformed public-input detents must not reach signed-overflow arithmetic.
rust.ex.web_enc(3,0x7fffffff);for(let i=0;i<20;i++)rust.ex.web_render(256);
assert.equal(rust.ex.web_rust_stat(83),1,'one malformed encoder packet is rejected');
rust.ex.web_enc(3,-1);for(let i=0;i<20;i++)rust.ex.web_render(256);assert.equal(rust.ex.web_rust_stat(83),1,'valid retry remains usable');
console.log(JSON.stringify({scenario:'panel-encoder-acceleration-and-nested-frame-input',compared_samples:samples,actual_encoder_witness:1,malformed_rejections:1,exports:24,memory_bytes:33554432}));
