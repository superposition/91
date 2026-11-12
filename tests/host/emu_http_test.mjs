// SPDX-License-Identifier: GPL-3.0-only
// The HTTP fallback must drive the real WASM with the same PCM and display as
// the worklet processor, including its larger audio callback size.
import fs from 'node:fs';
import assert from 'node:assert/strict';

const source=fs.readFileSync('web/emu/worklet.js').toString('base64');
const {Amb,createHttpAudioNode}=await import('data:text/javascript;base64,'+source);
const wasm=fs.readFileSync(process.argv[2]);
let time=0;
const context={currentTime:0,createScriptProcessor(frames,inputs,outputs) {
  assert.equal(frames,1024);assert.equal(inputs,0);assert.equal(outputs,2);
  return {disconnect(){}};
}};
const node=createHttpAudioNode(context);
const messages=[];
let readyResolve;
const ready=new Promise(resolve=>{readyResolve=resolve;});
node.port.onmessage=({data})=>{messages.push(data);if(data.type==='ready')readyResolve();};
node.port.postMessage({type:'load',wasm,seed:91,master:2800});
await ready;
const referenceMessages=[];
const reference=new Amb({port:{postMessage(message){referenceMessages.push(message);}},audioTime:()=>time});
await reference.onMessage({type:'load',wasm,seed:91,master:2800});
let peak=0;
const channels=[new Float32Array(1024),new Float32Array(1024)];
for(let block=0;block<512;block++) {
  context.currentTime=block*1024/44100;
  node.onaudioprocess({outputBuffer:{getChannelData(index){return channels[index];}}});
  const expected=[new Float32Array(1024),new Float32Array(1024)];
  for(let quantum=0;quantum<8;quantum++) {
    time=(block*1024+quantum*128)/44100;
    const output=[new Float32Array(128),new Float32Array(128)];
    reference.process([],[output]);
    for(let c=0;c<2;c++)expected[c].set(output[c],quantum*128);
  }
  for(let c=0;c<2;c++) {
    assert.deepEqual(channels[c],expected[c],`HTTP audio channel ${c}, block ${block}`);
    for(const sample of channels[c])peak=Math.max(peak,Math.abs(sample));
  }
}
await new Promise(resolve=>setTimeout(resolve,20));
assert(messages.some(message=>message.type==='frame' && message.fb?.length===240*240));
assert(messages.some(message=>message.rust?.plans>0 && message.rust?.fallbacks===0));
assert(peak>0.05 && peak<1);
node.dispose();
console.log(JSON.stringify({http_fallback:'pass',compared_samples:512*1024*2,peak,
  frame_messages:messages.filter(message=>message.type==='frame').length}));
