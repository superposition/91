// SPDX-License-Identifier: GPL-3.0-only
// Exact linked WASM section/file accounting, not an MCU flash estimate.
import fs from 'node:fs';
import crypto from 'node:crypto';
function inspect(path){
 const bytes=fs.readFileSync(path);let position=8;const sections={};
 function leb(){let value=0,shift=0,byte;do{byte=bytes[position++];value+=(byte&127)*2**shift;shift+=7;}while(byte&128);return value;}
 while(position<bytes.length){const kind=bytes[position++],length=leb();sections[kind]=(sections[kind]||0)+length;position+=length;}
 return{path,bytes:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),code_bytes:sections[10]||0,data_section_bytes:sections[11]||0,custom_section_bytes:sections[0]||0};
}
const disabled=inspect(process.argv[2]),enabled=inspect(process.argv[3]);
const feature=process.argv[4]||'EN_RUST_FACTORY',label=feature==='EN_RUST_FORMATS'?'formats':'factory';
console.log(JSON.stringify({[label+'_disabled']:disabled,[label+'_enabled']:enabled,delta:{file_bytes:enabled.bytes-disabled.bytes,code_bytes:enabled.code_bytes-disabled.code_bytes,data_section_bytes:enabled.data_section_bytes-disabled.data_section_bytes,custom_section_bytes:enabled.custom_section_bytes-disabled.custom_section_bytes},note:`same Rust engine, toggle only ${feature}; hardware remains C0`},null,2));
