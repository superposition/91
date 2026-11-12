// SPDX-License-Identifier: GPL-3.0-only
import assert from 'node:assert/strict';
import fs from 'node:fs';
const source=fs.readFileSync('web/emu/worklet.js');
const {Amb}=await import('data:text/javascript;base64,'+source.toString('base64'));
const flavors=process.argv.slice(2);
assert(flavors.length>=2,'reference and Rust WASM required');
const processors=[];
for(const file of flavors){
  const port={postMessage(){},close(){}};
  const processor=new Amb({port,audioTime:()=>0});
  await processor.onMessage({type:'load',wasm:fs.readFileSync(file),store:{},seed:91,master:2800});
  assert.equal(processor.mem.buffer.byteLength,33554432);
  assert.throws(()=>processor.ex.memory.grow(1),RangeError,'fixed memory protects cached views');
  processors.push(processor);
}
const lengths=[0,1,3,31,127,128,129,255,256,257,1023,1024,1025,2048,4097];
const outputs=processors.map(()=>lengths.map(n=>[[new Float32Array(n),new Float32Array(n)]]));
const NativeFloat32Array=globalThis.Float32Array,NativeDataView=globalThis.DataView;
let allocations=0,values=0;
const count=(target)=>new Proxy(target,{construct(t,args){allocations++;return Reflect.construct(t,args,t);}});
try{
  globalThis.Float32Array=count(NativeFloat32Array);globalThis.DataView=count(NativeDataView);
  for(let pass=0;pass<32;pass++)for(let k=0;k<lengths.length;k++){
    for(let f=0;f<processors.length;f++)processors[f].process([],outputs[f][k]);
    for(let f=1;f<processors.length;f++)assert.deepEqual(outputs[f][k],outputs[0][k]);
    values+=lengths[k]*2;
  }
}finally{globalThis.Float32Array=NativeFloat32Array;globalThis.DataView=NativeDataView;}
assert.equal(allocations,0,'no temporary float or WASI DataView construction in the audio callback');
for(const p of processors)p.dispose();
console.log(JSON.stringify({variable_render_sizes:lengths,compared_values:values,audio_view_allocations:allocations,memory_growth:'rejected',scope:'Audio copying/WASI views; frame and store message serialization remains browser binding work'}));
