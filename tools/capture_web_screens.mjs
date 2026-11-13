// SPDX-License-Identifier: GPL-3.0-only
// Deterministic captures from the actual WASM framebuffer, without a browser.
// node tools/capture_web_screens.mjs [build/emu/amb.wasm]
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';

const binary=process.argv[2]||'build/emu/amb.wasm';
let memory;
const wasi=new Proxy({
  clock_time_get:(_id,_p,out)=>{new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
  fd_write:(_fd,_iov,_n,out)=>{new DataView(memory.buffer).setUint32(out,0,true);return 0;},
},{get:(object,key)=>key in object?object[key]:()=>0});
const bytes=fs.readFileSync(binary);
const {instance}=await WebAssembly.instantiate(bytes,{wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})});
const ex=instance.exports;memory=ex.memory;
if(ex._initialize)ex._initialize();
ex.web_seed(91);ex.web_master(2800);ex.web_boot();
function advance(blocks){for(let i=0;i<blocks;i++)ex.web_render(128);}
function crc(bytes){let c=0xffffffff;for(const b of bytes){c^=b;for(let k=0;k<8;k++)c=(c>>>1)^(0xedb88320&-(c&1));}return (c^0xffffffff)>>>0;}
function chunk(kind,data){const name=Buffer.from(kind),size=Buffer.alloc(4),check=Buffer.alloc(4);size.writeUInt32BE(data.length);check.writeUInt32BE(crc(Buffer.concat([name,data])));return Buffer.concat([size,name,data,check]);}
function png(){
  const fb=new Uint16Array(memory.buffer,ex.web_fb(),240*240),rows=Buffer.alloc(240*(1+240*3));
  for(let y=0;y<240;y++)for(let x=0;x<240;x++){
    const raw=fb[y*240+x],c=(raw<<8|raw>>>8)&65535,p=y*721+1+x*3;
    rows[p]=Math.round((c>>>11)*255/31);rows[p+1]=Math.round((c>>>5&63)*255/63);rows[p+2]=Math.round((c&31)*255/31);
  }
  const header=Buffer.alloc(13);header.writeUInt32BE(240,0);header.writeUInt32BE(240,4);header[8]=8;header[9]=2;
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
const out='docs/img';fs.mkdirSync(out,{recursive:true});
const pages=['performance','arrangement','dx-keys','dx-bass','synth-jam'],captures=[];
advance(2800);
for(let i=0;i<pages.length;i++){
  if(i){ex.web_enc(0,1);advance(200);}
  if(pages[i]==='synth-jam'){ex.web_enc(3,1);advance(200);}
  const name='naiwan91-'+pages[i]+'.png',image=png();
  fs.writeFileSync(path.join(out,name),image);
  captures.push({page:pages[i],file:name,width:240,height:240,sha256:crypto.createHash('sha256').update(image).digest('hex'),...(pages[i]==='synth-jam'?{mode:'SYNTH',input:'KNOB1 +1 from BAND'}:{})});
}
fs.writeFileSync(path.join(out,'naiwan91-captures.json'),JSON.stringify({source:'actual WASM RGB565 framebuffer',wasm_sha256:crypto.createHash('sha256').update(bytes).digest('hex'),seed:91,master:2800,captures},null,2)+'\n');
console.log(JSON.stringify(captures));
