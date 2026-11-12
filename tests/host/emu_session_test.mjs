// SPDX-License-Identifier: GPL-3.0-only
// Exercise the real panel/save/restart flow with irregular browser render sizes.
import fs from 'node:fs';
import assert from 'node:assert/strict';

const pack=fs.readFileSync('rust/fm1-engine/tests/packs/afterhours.jmp');
assert.equal(pack[4],2,'exercise saved v2 restoration');
const session=Uint8Array.from([74,83,77,1,1,11,1,8,2,1,83,0,7,1,190,0,3,83,170,85]);

async function boot(path,objects) {
  let memory;
  const wasi=new Proxy({
    clock_time_get:(_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
    fd_write:(_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}
  },{get:(object,key)=>key in object?object[key]:()=>0});
  const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{
    wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})
  });
  const ex=instance.exports;memory=ex.memory;
  if(ex._initialize)ex._initialize();
  for(const [object,bytes] of Object.entries(objects)) {
    new Uint8Array(memory.buffer,ex.web_store(+object),bytes.length).set(bytes);
    ex.web_store_set_len(+object,bytes.length);
  }
  ex.web_seed(91);ex.web_master(2800);ex.web_boot();
  return {ex,memory};
}

let reference=await boot(process.argv[2],{26:pack,30:session});
let rust=await boot(process.argv[3],{26:pack,30:session});
const memoryBytes=rust.memory.buffer.byteLength;
const sizes=[1,127,17,255,256,511,1024,64];
let calls=0,samples=0,peak=0;
function advance(milliseconds) {
  let remaining=Math.round(milliseconds*44.1);
  while(remaining>0) {
    const frames=Math.min(remaining,sizes[calls++%sizes.length]);
    for(const instrument of [reference,rust])instrument.ex.web_render(frames);
    for(const channel of ['web_out_l','web_out_r']) {
      const a=new Uint32Array(reference.memory.buffer,reference.ex[channel](),frames);
      const b=new Uint32Array(rust.memory.buffer,rust.ex[channel](),frames);
      assert.deepEqual(b,a,`irregular PCM call ${calls}, ${channel}`);
      for(const value of new Float32Array(rust.memory.buffer,rust.ex[channel](),frames)) {
        assert(Number.isFinite(value));peak=Math.max(peak,Math.abs(value));
      }
    }
    samples+=frames*2;remaining-=frames;
  }
  assert.deepEqual(
    new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),240*240),
    new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),240*240),
    'panel framebuffer'
  );
}
function tap(button) {
  for(const instrument of [reference,rust])instrument.ex.web_buttons(1<<button);
  advance(100);
  for(const instrument of [reference,rust])instrument.ex.web_buttons(0);
  advance(100);
}
advance(6000);
tap(6); // HOME: return to the Songbook performance page.
tap(11); // REC: Hold. No new phrase planning while the held phrase repeats.
const heldPlans=rust.ex.web_rust_stat(1);
advance(60000);
assert.equal(rust.ex.web_rust_stat(1),heldPlans,'Hold reuses the phrase without consuming planner RNG');
tap(4); // EDIT: New Idea releases Hold and prepares another phrase.
assert(rust.ex.web_rust_stat(1)>heldPlans,'New Idea invokes the Rust planner');
tap(10); // PLAY: stop before saving, as on the physical panel.
advance(1500);
const writes=rust.ex.web_store_writes();
tap(7); // SAVE: run the shared Rust session encoder through the application.
assert.equal(rust.ex.web_store_writes(),writes+1,'session saved through the panel');
assert.equal(rust.ex.web_store_len(30),20);
const saved=new Uint8Array(rust.memory.buffer,rust.ex.web_store(30),20).slice();
assert.deepEqual(saved,new Uint8Array(reference.memory.buffer,reference.ex.web_store(30),20));
assert.deepEqual(saved.subarray(18),Uint8Array.from([0,0]),'legacy save clears reserved trailer bytes');
assert.equal(rust.ex.web_rust_stat(2),0,'no rejected Rust planning/encoding');
assert.equal(rust.ex.web_rust_stat(3),0,'no C fallback');
assert.equal(rust.memory.buffer.byteLength,memoryBytes);
const counters=Array.from({length:6},(_,index)=>rust.ex.web_rust_stat(index));
// A fresh instance gives power-on RAM semantics while retaining saved objects.
reference=await boot(process.argv[2],{26:pack,30:saved});
rust=await boot(process.argv[3],{26:pack,30:saved});
advance(6000);
assert(rust.ex.web_rust_stat(0)>0,'restart validates the saved v2 pack in Rust');
assert(rust.ex.web_rust_stat(5)>0,'restart validates the saved session in Rust');
assert.equal(rust.ex.web_rust_stat(3),0);
assert(peak>0.05 && peak<1);
console.log(JSON.stringify({scenario:'v2-hold-idea-save-restart',render_calls:calls,
  compared_samples:samples,seconds:samples/2/44100,peak,memory_bytes:memoryBytes,
  saved_session:Array.from(saved),rust:{validations:counters[0],plans:counters[1],
    rejections:counters[2],fallbacks:counters[3],bass_events:counters[4],sessions:counters[5]}}));
