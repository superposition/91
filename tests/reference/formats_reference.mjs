// SPDX-License-Identifier: GPL-3.0-only
// Adapter only: import the immutable R01 JavaScript byte encoders unchanged.
import fs from 'node:fs';
import path from 'node:path';
const root=process.argv[2];
async function load(file,extra='') {const code=fs.readFileSync(path.join(root,file),'utf8')+extra;return import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));}
const piece=await load('web/fm1piece.js'),backup=await load('web/fm1backup.js'),ota=await load('web/fm1ota.js','\nexport {response};');
function record(seed){const b=Uint8Array.from({length:376},(_,i)=>(seed+i*37)&255);b[0]=0xb5;b[1]=10;return b;}
const raw=(n,seed=91)=>Uint8Array.from({length:n},(_,i)=>(seed+i*37)&255);
const pieces=[];
for(let seed=0;seed<4;seed++){
 const p={genre:seed,slot:seed*2,npar:176,rec:record(seed),prg:raw(6,seed),voice:seed&1?raw(128,seed):null,meta:{title:'Friday '+seed,note:'Unknown keys stay — intact',future:{flags:[3,17]}}};
 pieces.push(Array.from(piece.pcEncode(p)));
}
// Unknown header flag bits are part of a valid file and survive a raw roundtrip.
const flagged=Uint8Array.from(pieces[0]);flagged[7]|=0xa0;new DataView(flagged.buffer).setUint32(flagged.length-4,backup.bkCrc32(flagged.subarray(0,-4)),true);pieces.push(Array.from(flagged));
const sets=[];
for(const count of [0,1,16]){
 const ps=Array.from({length:count},(_,i)=>({genre:i%19,slot:i%10,npar:176,vidx:i&1?0:-1,rec:record(i),prg:raw(6,i)}));
 const set={list:Array.from({length:16},(_,i)=>i&1?190:0),on:true,mix:77,pieces:ps,voices:[raw(128)],meta:{title:'Set '+count,future:[0,1,2]}};
 sets.push(Array.from(piece.pcSetEncode(set)));
}
const backups=[];
for(const count of [0,1,4]){const objects=Array.from({length:count},(_,i)=>({id:i===3?127:i,data:raw(i*73)}));backups.push(Array.from(backup.bkEncode(objects,{title:'Backup '+count,future:{text:'preserve'}})));}
const extra=Uint8Array.from([...backups[0].slice(0,-4),0x91,0x27,0,0,0,0]);new DataView(extra.buffer).setUint32(extra.length-4,backup.bkCrc32(extra.subarray(0,-4)),true);backups.push(Array.from(extra));
const packed=[];for(const n of [...Array(65).keys(),127,128,129,255,256,511,512])packed.push({input:Array.from(raw(n)),encoded:ota.pack7(raw(n))});
const chunks=[];for(const n of[1,2,48,255,256])chunks.push(backup.bkChunkMsg(23,0,3840,raw(n)));
const packets=[];for(const n of[0,1,6,7,8,31,32,127,128,511,512])packets.push(ota.response(0xfedcba98,raw(n),3));
console.log(JSON.stringify({pieces,sets,backups,packed,chunks,ota:packets,jam_request:backup.bkJamCapsMsg()}));
