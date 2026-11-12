// SPDX-License-Identifier: GPL-3.0-only
// An OFF saved session enters the legacy engine before any Jam source starts.
// This catches recipe engine effects that a pure record/state corpus cannot see.
import fs from 'node:fs';
import assert from 'node:assert/strict';
const session=Uint8Array.from([74,83,77,1,0,0,0,16,0,3,60,0,0,0,100,0,0,0,0,0]);
const sizes=[1,127,128,129,255,256,511,1024];
async function boot(path,bank){
 let memory;const wasi=new Proxy({clock_time_get:(_a,_b,p)=>{new DataView(memory.buffer).setBigUint64(p,0n,true);return 0;},fd_write:(_a,_b,_c,p)=>{new DataView(memory.buffer).setUint32(p,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
 const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});const ex=instance.exports;memory=ex.memory;assert.equal(Object.keys(ex).length,24);if(ex._initialize)ex._initialize();
 for(const [object,bytes]of [[30,session],...(bank?[[0,bank]]:[])]){new Uint8Array(memory.buffer,ex.web_store(object),bytes.length).set(bytes);ex.web_store_set_len(object,bytes.length);}
 ex.web_seed(0);ex.web_master(2800);ex.web_boot();return{ex,memory};
}
const reports=[];
for(const kind of ['recipes','corrupt-record','saved-factory-reset']){
 let bank;
 if(kind!=='recipes'){bank=new Uint8Array(3836);bank.set([72,89,80,51,2,0,176,0,1,0,120,1,1,1,1,0]);bank.fill(255,16+74,16+74+176);new DataView(bank.buffer).setUint16(16,kind==='corrupt-record'?0:0xab5,true);}
 const instruments=await Promise.all(process.argv.slice(2).map(path=>boot(path,bank)));const [reference,rust]=instruments;
 let calls=0,samples=0,hash=2166136261>>>0;
 function input(name,...args){for(const p of instruments)p.ex[name](...args);}
 function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);for(const channel of ['web_out_l','web_out_r']){const expected=new Uint32Array(reference.memory.buffer,reference.ex[channel](),n);for(const p of instruments.slice(1))assert.deepEqual(new Uint32Array(p.memory.buffer,p.ex[channel](),n),expected,`${kind} call${calls}`);for(const value of expected)hash=Math.imul((hash^value)>>>0,16777619)>>>0;}remaining-=n;samples+=n*2;}for(const p of instruments.slice(1))assert.deepEqual(new Uint16Array(p.memory.buffer,p.ex.web_fb(),240*240),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),240*240));}
 advance(6000);input('web_keys',1<<12);advance(250);input('web_keys',0);advance(1500);
 if(kind==='saved-factory-reset'){input('web_buttons',1<<5);advance(100);input('web_buttons',0);advance(100);input('web_buttons',1<<7);advance(1100);input('web_buttons',0);advance(1800);assert(rust.ex.web_store_writes()>0,'factory reset writes the real saved bank');}
 advance(2500);
 for(const p of instruments.slice(1)){assert.equal(p.ex.web_store_len(0),reference.ex.web_store_len(0));const n=Math.max(0,p.ex.web_store_len(0));assert.deepEqual(new Uint8Array(p.memory.buffer,p.ex.web_store(0),n),new Uint8Array(reference.memory.buffer,reference.ex.web_store(0),n));assert.equal(p.memory.buffer.byteLength,33554432);}
 assert(rust.ex.web_rust_stat(66)>0,'project state executes in Rust');
 reports.push({scenario:kind,compared_samples:samples,render_calls:calls,pcm_hash:hash.toString(16).padStart(8,'0'),project_calls:rust.ex.web_rust_stat(66),object_saves:rust.ex.web_rust_stat(67)});
}
console.log(JSON.stringify({legacy_project:reports,frozen_reference:process.argv[4]||null}));
