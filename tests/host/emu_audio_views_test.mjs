// SPDX-License-Identifier: GPL-3.0-only
import assert from 'node:assert/strict';
import fs from 'node:fs';
const source=fs.readFileSync('web/emu/worklet.js');
const {Amb}=await import('data:text/javascript;base64,'+source.toString('base64'));
const flavors=process.argv.slice(2);
assert(flavors.length>=2,'reference and Rust WASM required');
const processors=[],clocks=[],messages=[];
let processing=false;
for(const file of flavors){
  const time={now:0},sent=[];clocks.push(time);messages.push(sent);
  const port={postMessage(message){assert(!processing,'no IPC or serialization during process');sent.push(message);},close(){}};
  const processor=new Amb({port,audioTime:()=>time.now});
  await processor.onMessage({type:'load',wasm:fs.readFileSync(file),store:{4:Uint8Array.from([0,127,128,254,255])},seed:91,master:2800});
  // Force an outstanding observation, as after a firmware save. Rendering
  // must leave even dirty store/frame serialization to the MessagePort task.
  processor.sentWrites=-1;
  assert.equal(processor.mem.buffer.byteLength,33554432);
  assert.throws(()=>processor.ex.memory.grow(1),RangeError,'fixed memory protects cached views');
  processors.push(processor);
}
const lengths=[0,1,3,31,127,128,129,255,256,257,1023,1024,1025,2048,4097];
const outputs=processors.map(()=>lengths.map(n=>[[new Float32Array(n),new Float32Array(n)]]));
const natives=Object.fromEntries(['Float32Array','Uint8Array','Uint16Array','DataView','Array','BigInt'].map(name=>[name,globalThis[name]]));
let allocations=0,values=0;
const count=(target)=>new Proxy(target,{
 construct(t,args){if(processing)allocations++;return Reflect.construct(t,args,t);},
 apply(t,self,args){if(processing)allocations++;return Reflect.apply(t,self,args);},
});
try{
  for(const [name,native]of Object.entries(natives))globalThis[name]=count(native);
  for(let pass=0;pass<32;pass++)for(let k=0;k<lengths.length;k++){
    for(let f=0;f<processors.length;f++){
      clocks[f].now+=lengths[k]/44100;
      processing=true;
      try{processors[f].process([],outputs[f][k]);}finally{processing=false;}
      await processors[f].onMessage({type:'poll'});
    }
    for(let f=1;f<processors.length;f++)assert.deepEqual(outputs[f][k],outputs[0][k]);
    values+=lengths[k]*2;
  }
}finally{for(const [name,native]of Object.entries(natives))globalThis[name]=native;}
assert.equal(allocations,0,'no owned array/view/BigInt construction in the audio callback');
for(const sent of messages){
 assert(sent.some(message=>message.type==='frame'&&message.fb?.length===240*240),'display snapshots publish outside rendering');
 assert(sent.some(message=>message.store&&Buffer.from(message.store[4]).equals(Buffer.from([0,127,128,254,255]))),'persisted object bytes publish outside rendering');
}
for(const p of processors)p.dispose();
console.log(JSON.stringify({variable_render_sizes:lengths,compared_values:values,audio_constructor_allocations:allocations,render_ipc_calls:0,memory_growth:'rejected',scope:'Owned audio copying and WASI clock; frame/store serialization and IPC run in polled MessagePort tasks'}));
