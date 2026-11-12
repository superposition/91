// SPDX-License-Identifier: GPL-3.0-only
// Source-level WASM integration witness; this does not perform browser/device QA.
import fs from 'node:fs';
import assert from 'node:assert/strict';
let memory;
const wasi=new Proxy({clock_time_get:(_a,_b,p)=>{new DataView(memory.buffer).setBigUint64(p,0n,true);return 0;},fd_write:(_a,_b,_c,p)=>{new DataView(memory.buffer).setUint32(p,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
const{instance}=await WebAssembly.instantiate(fs.readFileSync(process.argv[2]),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
const ex=instance.exports;memory=ex.memory;assert.equal(Object.keys(ex).length,24);if(ex._initialize)ex._initialize();ex.web_seed(0);ex.web_boot();
for(let i=0;i<1200;i++)ex.web_render(256);
assert(ex.web_rust_stat(80)>0,'actual instrument invokes Rust graphics');assert(ex.web_rust_stat(81)>0,'actual instrument writes pixels through Rust');assert.equal(ex.web_rust_stat(83),0,'valid frontend rejects no graphics call');assert.equal(memory.buffer.byteLength,33554432);
console.log(JSON.stringify({graphics_calls:ex.web_rust_stat(80),pixel_writes:ex.web_rust_stat(81),rejections:ex.web_rust_stat(83),exports:Object.keys(ex).length,memory_bytes:memory.buffer.byteLength}));
