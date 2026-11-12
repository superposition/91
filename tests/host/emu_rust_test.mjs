// SPDX-License-Identifier: GPL-3.0-only
// Compare complete instrument output, not just the extracted planner fixtures.
import fs from 'node:fs';
import assert from 'node:assert/strict';

async function boot(path,packPath) {
  let memory;
  const wasi = new Proxy({
    clock_time_get: (_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
    fd_write: (_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}
  },{get:(object,key)=>key in object?object[key]:()=>0});
  const {instance}=await WebAssembly.instantiate(fs.readFileSync(path),{
    wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})
  });
  const ex=instance.exports;memory=ex.memory;
  if(ex._initialize)ex._initialize();
  if(packPath) {
    const bytes=fs.readFileSync(packPath);
    new Uint8Array(memory.buffer,ex.web_store(26),bytes.length).set(bytes);
    ex.web_store_set_len(26,bytes.length);
    const session=Uint8Array.from([74,83,77,1,1,6,1,8,0,3,60,0,0,1,97,0,3,83,0,0]);
    new Uint8Array(memory.buffer,ex.web_store(30),session.length).set(session);
    ex.web_store_set_len(30,session.length);
  }
  ex.web_seed(91);ex.web_master(2800);ex.web_boot();
  return {ex,memory};
}

const reference=await boot(process.argv[2],process.argv[4]);
const rust=await boot(process.argv[3],process.argv[4]);
assert.equal(typeof rust.ex.web_rust_stat,'function','migration diagnostics export');
const memoryBytes=rust.memory.buffer.byteLength;
assert.equal(Object.keys(rust.ex).length,24,'bounded browser API and memory exports');
assert(!Object.keys(rust.ex).some(name=>name.startsWith('_RN')),'Rust internals stay private');
for(const name of Object.keys(reference.ex).filter(name=>name.startsWith('web_'))) {
  assert.equal(typeof rust.ex[name],typeof reference.ex[name],`preserved browser export ${name}`);
}
const events=new Map([
  [1000,[['web_keys',1]]],[1050,[['web_keys',0]]],
  [3000,[['web_enc',0,1]]],[3060,[['web_enc',4,1]]],
  [8000,[['web_enc',2,1]]],[16000,[['web_enc',2,1]]],
  [24000,[['web_enc',0,1]]],[24500,[['web_enc',4,1]]],
  [32000,[['web_enc',0,-1]]],
  [35000,[['web_buttons',1<<10]]],[35005,[['web_buttons',0]]],
  [35100,[['web_buttons',1<<10]]],[35105,[['web_buttons',0]]],
  [40000,[['web_enc',0,1]]],[40100,[['web_enc',4,-1]]]
]);
const blocks=55000,frames=128;
let peak=0,hash=2166136261;
for(let block=0;block<blocks;block++) {
  for(const [name,...args] of events.get(block)||[]) {
    reference.ex[name](...args);rust.ex[name](...args);
  }
  reference.ex.web_render(frames);rust.ex.web_render(frames);
  for(const name of ['web_out_l','web_out_r']) {
    const a=new Uint32Array(reference.memory.buffer,reference.ex[name](),frames);
    const b=new Uint32Array(rust.memory.buffer,rust.ex[name](),frames);
    const sound=new Float32Array(rust.memory.buffer,rust.ex[name](),frames);
    for(let sample=0;sample<frames;sample++) {
      assert.equal(b[sample],a[sample],`PCM ${name} block ${block} sample ${sample}`);
      assert(Number.isFinite(sound[sample]));
      peak=Math.max(peak,Math.abs(sound[sample]));
      hash=Math.imul((hash^b[sample])>>>0,16777619)>>>0;
    }
  }
  if(block%1000===0) {
    const a=new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),240*240);
    const b=new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),240*240);
    assert.deepEqual(b,a,`framebuffer block ${block}`);
    assert.equal(rust.ex.web_lit_buttons(),reference.ex.web_lit_buttons());
    assert.equal(rust.ex.web_lit_keys(),reference.ex.web_lit_keys());
  }
}
const counts=Array.from({length:10},(_,index)=>rust.ex.web_rust_stat(index));
assert(counts[1]>=6,`Rust planner executed: ${counts}`);
assert(counts[4]>=500,`Rust bass decisions executed: ${counts}`);
assert.equal(counts[2],0,'no planner rejection');
assert.equal(counts[3],0,'no C planner fallback');
assert.equal(rust.memory.buffer.byteLength,memoryBytes,'no memory growth during playback');
if(process.argv[4])assert(counts[0]>0,'saved-pack startup calls Rust validation');
assert(counts[5]>0,'startup calls Rust session validation');
assert(counts[6]>176 && counts[7]>176,'Rust command producer and consumer execute');
assert.equal(counts[8],0,'normal playback drops no commands');
assert(counts[9]>0 && counts[9]<=1024,'bounded command queue high-water mark');
assert(peak>0.05 && peak<1);
const pcmHash=hash.toString(16).padStart(8,'0');
const golden=JSON.parse(fs.readFileSync(new URL('./emu_rust_golden.json',import.meta.url)));
const scenario=process.argv[4]?.replaceAll('\\','/').split('/').at(-1)||'factory';
if(golden[scenario]) {
  assert.equal(pcmHash,golden[scenario].pcm_hash,'frozen R01 audio fingerprint');
  assert.equal(rust.ex.web_blits(),golden[scenario].blits,'frozen R01 display count');
}
console.log(JSON.stringify({initial_pack:process.argv[4]||'factory',blocks,seconds:blocks*frames/44100,compared_samples:blocks*frames*2,
  pcm_hash:pcmHash,peak,blits:rust.ex.web_blits(),memory_bytes:memoryBytes,
  rust:{validations:counts[0],plans:counts[1],rejections:counts[2],fallbacks:counts[3],bass_events:counts[4],sessions:counts[5],
    commands:{accepted:counts[6],consumed:counts[7],dropped:counts[8],high_water:counts[9]}}}));
