// SPDX-License-Identifier: GPL-3.0-only
// Exercise the real C/Rust/browser input path, without a browser or hardware.
import assert from 'node:assert/strict';
import fs from 'node:fs';

const bytes=fs.readFileSync(process.argv[2]||'build/emu/amb.wasm');
async function instrument() {
  let memory;
  const wasi=new Proxy({
    clock_time_get:(_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
    fd_write:(_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}
  },{get:(object,key)=>key in object?object[key]:()=>0});
  const {instance}=await WebAssembly.instantiate(bytes,{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
  const ex=instance.exports;memory=ex.memory;
  if(ex._initialize)ex._initialize();
  ex.web_seed(91);ex.web_master(2800);ex.web_boot();
  function render(blocks,measure=false) {
    let peak=0,power=0,samples=0;
    for(let block=0;block<blocks;block++) {
      ex.web_render(128);
      if(measure)for(const name of ['web_out_l','web_out_r']) {
        for(const sample of new Float32Array(memory.buffer,ex[name](),128)) {
          assert(Number.isFinite(sample),'finite Synth Jam output');
          peak=Math.max(peak,Math.abs(sample));power+=sample*sample;samples++;
        }
      }
    }
    return {peak,rms:samples?Math.sqrt(power/samples):0};
  }
  function tap(button) {ex.web_buttons(1<<button);render(24);ex.web_buttons(0);render(24);}
  render(1800); // Complete boot before touching the panel.
  for(let page=0;page<4;page++){ex.web_enc(0,1);render(48);}
  ex.web_enc(3,1);render(48); // Page42: MODE BAND -> SYNTH.
  ex.web_enc(6,-100);render(48); // SPACE0 separates note release from reverb tails.
  tap(10); // Stop backing; Synth keyboard must remain playable.
  render(1400);
  return {ex,render,tap,memory};
}

const held=await instrument(),released=await instrument();
const quiet=held.render(100,true);
released.render(100);
assert(quiet.rms<0.0001,`stopped backing settles: ${quiet.rms}`);
for(const synth of [held,released]){synth.ex.web_keys((1<<0)|(1<<4)|(1<<7));synth.render(160);}
const audible=held.render(140,true);released.render(140);
assert(audible.rms>0.0001&&audible.peak<1,`manual chord is audible and bounded: ${JSON.stringify(audible)}`);
released.tap(6); // HOME must release notes even while physical keys remain down.
held.render(48); // Match the time spent in the HOME press/release.
held.render(300);released.render(300);
const continuing=held.render(200,true),afterHome=released.render(200,true);
assert(continuing.rms>0.00001,'control chord remains held');
assert(afterHome.rms<continuing.rms*0.1,`HOME releases held synth notes: ${afterHome.rms} vs ${continuing.rms}`);
for(const synth of [held,released])synth.ex.web_keys(0);
console.log(JSON.stringify({synth_jam_wasm:'pass',stopped_backing_rms:quiet.rms,manual_chord:audible,
  held_control_rms:continuing.rms,home_release_rms:afterHome.rms,memory_bytes:held.memory.buffer.byteLength,
  physical_hardware_qa:'not run'}));
