// SPDX-License-Identifier: GPL-3.0-only
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const expected=readFileSync(process.argv[2]);
for(const path of process.argv.slice(3)){
 let instance;const chunks=[];
 ({instance}=await WebAssembly.instantiate(readFileSync(path),{env:new Proxy({emit(p,n){chunks.push(Buffer.from(new Uint8Array(instance.exports.memory.buffer,p,n)));}},{get:(o,k)=>k in o?o[k]:()=>0})}));
 const cases=instance.exports.run();
 assert(cases>=0,'production witness/guard checks pass');
 const actual=Buffer.concat(chunks);assert.deepEqual(actual,expected,path);
 console.log(JSON.stringify({path,cases,bytes:actual.length,sha256:createHash('sha256').update(actual).digest('hex')}));
}
