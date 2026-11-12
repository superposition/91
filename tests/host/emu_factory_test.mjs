// SPDX-License-Identifier: GPL-3.0-only
// Exercise packed factory lines through the actual stored-piece/UI/save path.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const corpus=JSON.parse(fs.readFileSync('rust/fm1-engine/tests/factory/reference.json'));
const symbols=fs.readFileSync('rust/fm1-types/src/params.rs','utf8');
const id=(name)=>+symbols.match(new RegExp(`pub const ${name}: Self = Self\\((\\d+)\\)`))[1];
const bank=new Uint8Array(3836),view=new DataView(bank.buffer);
bank.set([72,89,80,51,2,0,176,0,1,0,120,1,1,1,1,0]);
view.setUint16(16,corpus.presets[0].pool,true);
for(let i=0;i<36;i++)view.setUint16(18+i*2,corpus.presets[0].patterns[i],true);
bank.fill(255,16+74,16+74+176);bank[16+74+id('AC_LOCK')]=bank[16+74+id('CM_LOCK')]=1;
bank[16+74+id('CM_SND')]=42; // first user slot, after12 plain+30 factory DX voices
bank.set(corpus.acid[91],16+256);bank.set(corpus.pluck[13].slice(0,6),16+250);bank.set(corpus.pluck[13].slice(6),16+274);
const voices=new Uint8Array(2056);voices.set([86,79,88,49,1]);
// A canonical packed voice with one carrier, a fixed sustain and a readable name.
for(let i=0;i<6;i++){voices[8+i*17+12]=7<<3;voices[8+i*17+15]=2;}
voices[8+5*17+14]=99;voices.fill(99,8+5*17,8+5*17+7);voices[8+5*17+8]=39;
voices.fill(99,8+102,8+106);voices.fill(50,8+106,8+110);voices[8+110]=31;voices[8+117]=24;
voices.set(new TextEncoder().encode('CODEC TEST'),8+118);
async function boot(path){
 let memory;const wasi=new Proxy({clock_time_get:(_a,_b,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},fd_write:(_a,_b,_c,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;}},{get:(o,k)=>k in o?o[k]:()=>0});
 const{instance}=await WebAssembly.instantiate(fs.readFileSync(path),{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});const ex=instance.exports;memory=ex.memory;if(ex._initialize)ex._initialize();
 assert.equal(Object.keys(ex).length,24);new Uint8Array(memory.buffer,ex.web_store(0),bank.length).set(bank);ex.web_store_set_len(0,bank.length);new Uint8Array(memory.buffer,ex.web_store(3),voices.length).set(voices);ex.web_store_set_len(3,voices.length);ex.web_seed(0);ex.web_master(2800);ex.web_boot();return{ex,memory};
}
const reference=await boot(process.argv[2]),rust=await boot(process.argv[3]);let samples=0,calls=0;
const sizes=[1,127,128,129,255,256,511,1024];
function input(name,...args){reference.ex[name](...args);rust.ex[name](...args);}
function advance(ms){let remaining=Math.round(ms*44.1);while(remaining>0){const n=Math.min(remaining,sizes[calls++%sizes.length]);input('web_render',n);for(const channel of['web_out_l','web_out_r'])assert.deepEqual(new Uint32Array(rust.memory.buffer,rust.ex[channel](),n),new Uint32Array(reference.memory.buffer,reference.ex[channel](),n));remaining-=n;samples+=n*2;}
 assert.deepEqual(new Uint16Array(rust.memory.buffer,rust.ex.web_fb(),240*240),new Uint16Array(reference.memory.buffer,reference.ex.web_fb(),240*240));}
function tap(button){input('web_buttons',1<<button);advance(100);input('web_buttons',0);advance(100);}
advance(6000);tap(6);input('web_keys',1<<12);advance(100);input('web_keys',0);advance(100);advance(1500);
const decoded=rust.ex.web_rust_stat(33);assert(decoded>=2,'stored acid/pluck lines decode through Rust in WASM');
tap(10);advance(1000);tap(5);const writes=rust.ex.web_store_writes();tap(7);advance(1500);
assert(rust.ex.web_store_writes()>writes,'real Global SAVE writes the stored legacy piece');
assert(rust.ex.web_rust_stat(33)>=decoded+2,'saving frozen acid/pluck lines encodes through Rust in WASM');
const len=rust.ex.web_store_len(0);assert.equal(len,reference.ex.web_store_len(0));assert(len>0);
assert.deepEqual(new Uint8Array(rust.memory.buffer,rust.ex.web_store(0),len),new Uint8Array(reference.memory.buffer,reference.ex.web_store(0),len));
assert.equal(rust.ex.web_rust_stat(34),0);assert.equal(rust.memory.buffer.byteLength,33554432);
if(rust.ex.web_rust_stat(66)>0) assert(rust.ex.web_rust_stat(67)>0,'stored record load/save execute through Rust project policy');
else assert(rust.ex.web_rust_stat(36)>0 && rust.ex.web_rust_stat(37)>0,'stored record decode/save execute through Rust formats');
assert.equal(rust.ex.web_rust_stat(38),0,'valid saved record/voice paths reject no format data');
console.log(JSON.stringify({scenario:'stored-frozen-acid-pluck-save',compared_samples:samples,render_calls:calls,saved_bytes:len,
 factory:{accesses:rust.ex.web_rust_stat(32),codecs:rust.ex.web_rust_stat(33),rejections:rust.ex.web_rust_stat(34),copied_bytes:rust.ex.web_rust_stat(35)},formats:{decodes:rust.ex.web_rust_stat(36),encodes:rust.ex.web_rust_stat(37),rejections:rust.ex.web_rust_stat(38),copied_bytes:rust.ex.web_rust_stat(39)},persistence:{project_calls:rust.ex.web_rust_stat(66),object_saves:rust.ex.web_rust_stat(67),bytes:rust.ex.web_rust_stat(71)},memory_bytes:rust.memory.buffer.byteLength}));
