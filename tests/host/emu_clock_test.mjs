// SPDX-License-Identifier: GPL-3.0-only
// Compare the actual WASI clock binding against the previous u64 wire encoding.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const {Amb}=await import('data:text/javascript;base64,'+fs.readFileSync('web/emu/worklet.js').toString('base64'));
let seconds=0,imports;
const original=WebAssembly.instantiate;
WebAssembly.instantiate=async function(bytes,bindings){imports=bindings;return original(bytes,bindings);};
const processor=new Amb({port:{postMessage(){},close(){}},audioTime:()=>seconds});
try{await processor.onMessage({type:'load',wasm:fs.readFileSync(process.argv[2]),seed:91});}
finally{WebAssembly.instantiate=original;}
const offset=processor.mem.buffer.byteLength-8;
const before=new Uint8Array(processor.mem.buffer,offset,8).slice();
const values=[0,1e-9,1/44100,0.1,4.294967295,4.294967296,4.294967297,60,3600,86400,4294967.295,10000000];
for(seconds of values){
 assert.equal(imports.wasi_snapshot_preview1.clock_time_get(1,0,offset),0);
 assert.equal(processor.dataView.getBigUint64(offset,true),BigInt(Math.round(seconds*1e9)));
}
new Uint8Array(processor.mem.buffer,offset,8).set(before);
processor.dispose();
console.log(JSON.stringify({wasi_clock_cases:values.length,legacy_u64_bytes:'exact',clock_bigint_construction:0}));
