// SPDX-License-Identifier: GPL-3.0-only
// Metadata and device API/protocol contracts; this does not access hardware.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
import path from 'node:path';
import { Target } from './host/browser_doubles.mjs';

const moduleFrom = async (name) => import('data:text/javascript;base64,' + fs.readFileSync(name).toString('base64'));
const { DEVICE_DATA: data } = await moduleFrom('web/device-data.js');
const { DEVICE_TEXT, deviceText, deviceCapabilities, deviceParseAbout, createDeviceClient, deviceAction,
  deviceMapValid, deviceParseParameters, deviceReadParameters, deviceParseVoice } = await moduleFrom('web/fm1device.js');
const params = JSON.parse(fs.readFileSync('build/params.json', 'utf8'));
assert.deepEqual(data.product, JSON.parse(fs.readFileSync('web/product.json', 'utf8')));
assert.equal(data.parameters.count, params.params.length);
assert.equal(data.parameters.userVoiceBase, params.names.nplain + params.names.nfmfact);
assert.equal(data.parameters.userSlots, params.names.nuser);
assert.deepEqual(data.genres, params.names.genres);
assert.deepEqual(data.objects, params.names.objects);
assert.deepEqual(data.backup, params.names.backup);
assert.deepEqual(data.objects.genres, [0, ...Array.from({ length: 18 }, (_, i) => i + 5)]);
assert.deepEqual([data.objects.settings, data.objects.voices, data.objects.globals, data.objects.extras, data.objects.map, data.objects.bass, data.objects.jamFirst, data.objects.jamLast, data.objects.session], [2,3,4,23,24,25,26,29,30]);

for (const lang of data.product.languages) {
  assert.deepEqual(Object.keys(DEVICE_TEXT[lang]).sort(), Object.keys(DEVICE_TEXT.en).sort());
  for (const key of ['connected','connecting','idle','unsupported','busy','empty','disconnected','badBackup','badMap','badImport']) {
    const text = deviceText(data, lang, key);
    assert(text.includes(data.product.name), `${lang}/${key}: current product name`);
    assert(!/undefined|PERF-1|0\.16\./.test(text));
  }
}
assert.equal(deviceCapabilities(null).backup, false);
for (const ver of [1,2,3]) {
  const about = deviceParseAbout([240,125,80,69,ver,1,48,19,0,247]);
  const caps = deviceCapabilities(about);
  assert.equal(caps.parameters, 176); assert.equal(caps.extras, ver >= 2); assert.equal(caps.bass, ver >= 3);
  assert.equal(caps.mapSettings, null, 'about does not advertise MAP2; do not guess');
}
assert.equal(deviceParseAbout([240,125,80,69,0,0,0,19,0,247]), null);
assert.equal(deviceParseAbout([240,125,80,69,3,128,0,19,0,247]), null);

const deferred = () => { let resolve; const promise = new Promise((done) => { resolve = done; }); return { promise, resolve }; };
function environment(options = {}) {
  const inp = Object.assign(new Target(), { name: 'FM-1', state: 'connected', connection: 'closed', closes: 0, close() { this.closes++; return Promise.resolve(); } });
  const out = { name: 'FM-1', state: 'connected', connection: 'closed', closes: 0, sent: [], send(bytes) { this.sent.push(bytes); options.reply?.(bytes, inp); }, close() { this.closes++; return Promise.resolve(); } };
  const midi = Object.assign(new Target(), { inputs: new Map([['in', inp]]), outputs: new Map([['out', out]]) });
  let disconnected;
  const client = createDeviceClient(data, { requestMidi: async () => midi, onDisconnect: (_p, error) => { disconnected = error; }, ...options });
  return { client, midi, inp, out, get disconnected() { return disconnected; } };
}
for (const language of ['en','ja']) {
  const env = environment({ language: () => language });
  const session = await env.client.open();
  const absent = await env.client.about(session, 1); assert.equal(absent, null, 'unsupported response is unknown, not a release guess');
  const pending = env.client.about(session, 1000);
  const rejected = assert.rejects(pending, (error) => error.code === 'disconnected' && error.message.includes(data.product.name));
  env.inp.state = 'disconnected'; env.midi.emit('statechange'); await rejected;
  assert.equal(env.inp.listenerCount, 0); assert.equal(env.midi.listenerCount, 0);
  assert.equal(env.inp.closes, 1); assert.equal(env.out.closes, 1);
  assert(env.disconnected.message.includes(data.product.name));
  env.inp.state = 'connected'; const again = await env.client.open(); assert.notEqual(again, session);
  const wait = deferred(); const first = env.client.run(() => wait.promise);
  await assert.rejects(env.client.run(() => {}), (error) => error.code === 'busy' && error.message.includes(data.product.name));
  wait.resolve(); await first; assert.equal(env.client.busy, false);
  await assert.rejects(env.client.run(() => { throw new Error('bad import'); }), /bad import/); assert.equal(env.client.busy, false);
  let report;
  await deviceAction(env.client, () => { throw env.client.error('badImport'); }, (message) => { report = message; })();
  assert(report.includes(data.product.name)); env.client.close(); env.client.close();
}
await assert.rejects(createDeviceClient(data).open(), (error) => error.code === 'browser');
for(const language of ['en','ja']) {
  const client=createDeviceClient(data,{secureContext:false,language:()=>language});
  // Construction stays quiet; only a requested connection reports this condition.
  await assert.rejects(client.open(), (error) => error.code === 'insecure' && error.message.includes('HTTPS') && error.message.includes(data.product.name));
  await assert.rejects(client.access(), (error) => error.code === 'insecure');
}
await assert.rejects(createDeviceClient(data, { requestMidi: async () => { throw new Error('permission'); } }).open(), (error) => error.code === 'denied');
await assert.rejects(createDeviceClient(data, { requestMidi: async () => ({ inputs: new Map(), outputs: new Map() }) }).open(), (error) => error.code === 'notfound');
{
  const pending = deferred(), env = environment();
  const client = createDeviceClient(data, { requestMidi: () => pending.promise });
  const opening = client.open(); const rejected = assert.rejects(opening, (error) => error.code === 'disconnected');
  client.close(); await rejected; pending.resolve(env.midi); await Promise.resolve();
  assert.equal(env.inp.listenerCount, 0); assert.equal(env.midi.listenerCount, 0);
}
{
  const env = environment({ reply(bytes, inp) {
    if (bytes[3] === 66) inp.emit('midimessage', { data: Uint8Array.from([240,125,80,69,3,0,2,19,0,247]) });
    else if (bytes[3] === 80) inp.emit('midimessage', { data: Uint8Array.from([240,125,80,81,0,0,2,64,0,64,7,247]) });
  } });
  const session = await env.client.open(); assert.equal((await env.client.about(session)).npar, 2);
  let received;
  assert.equal(await deviceReadParameters(env.client, session, 2, (values) => { received = values; }), 2);
  assert.deepEqual(received, [[0,0],[1,7]]);
  assert.deepEqual(env.out.sent[1], [240,125,80,80,2,247]);
  env.client.close();
}
assert.equal(deviceParseParameters([240,125,80,81,0,0,2,64,0,247], 176), null);
assert.equal(deviceParseParameters([240,125,80,81,0,0,1,128,0,247], 176), null);
assert.deepEqual(deviceParseParameters([240,125,80,81,0,1,2,64,0,64,1,247], 2), [[1,0]], 'extra peer settings never extend local arrays');
assert.deepEqual(deviceParseVoice([240,125,80,88,0,0,247], 16), { slot:0, loaded:0, data:null });
assert.equal(deviceParseVoice([240,125,80,88,1,0,247],16,0), null, 'reply must match the requested slot');
const full = [240,125,80,88,0,1,...new Array(129).fill(0),247];
assert.equal(deviceParseVoice(full,16).data.length, 128);
full[6] = 1; assert.equal(deviceParseVoice(full,16), null, 'damaged voice checksum');

const map1 = { 'n:0:60': { t:'k7' }, 'c:0:21': { t:'e2', mode:'rel64' } }, map2 = { 'c:1:8': { t:'p175', mode:'abs' } };
for (const maps of [map1,map2,{}]) assert(deviceMapValid({ perfmap:1,maps },176));
for (const maps of [null, [], {'c:16:1':{t:'k1'}}, {'c:0:1':{t:'p176'}}, {'n:0:1':{t:'p1'}}, {'c:0:1':{t:'e7'}}, {'c:0:1':{t:'b14'}}, {'c:0:1':{t:'k27'}}, {'c:0:1':{t:'e1',mode:'wrong'}}]) assert(!deviceMapValid({perfmap:1,maps},176));
const raw = fs.readFileSync('web/device-data.js','utf8').replace(/^export /gm,'') + fs.readFileSync('web/fm1backup.js','utf8').replace(/^export /gm,'').replace(/^import .*;\r?\n/gm,'');
const backup = new Function(raw + '; return {mmEncode,mmDecode,bkEncode,bkDecode};')();
const hash = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
// Frozen against c1d2249 before metadata extraction, including MAP1/MAP2 headers.
assert.equal(hash(backup.mmEncode(map1).bytes),'2939ee5d81469ef1b5acb760d45d6bae4204f8661de46d6c1b4245fb71932e3e');
assert.equal(hash(backup.mmEncode(map2).bytes),'8c118aa61ed58d2172cff5e34bfaba8bd5502a4f37494fa7135723fcf32da749');
const file = backup.bkEncode([{id:0,data:Uint8Array.from([0,1,255])},{id:24,data:backup.mmEncode({'n:0:60':{t:'k7'}}).bytes}],{id:'FM-1_7000300',created:'2026-11-12'});
assert.equal(hash(file),'708a035c3f022b2d0b3d7b668f256c4e43be029bc0590098ff409ed90e3af78d');
assert.equal(backup.bkDecode(file).objs.length,2);
assert.throws(() => backup.bkDecode(new Uint8Array(0)), /NAIWAN 91/);
const broken = file.slice(); broken[broken.length - 1] ^= 1; assert.throws(() => backup.bkDecode(broken));

const editor = fs.readFileSync('web/editor/index.html','utf8');
const updateNames = /function syncPluckNames\(\) \{[\s\S]*?\n\}/.exec(editor)[0];
const table = structuredClone(params.params), slots = new Array(data.parameters.userSlots).fill(null); slots[0] = new Uint8Array(155);
const factory = table[params.enum.P_CM_SND].t.slice(34,42);
new Function('P','D','slots','nameOf','USER_VOICE_BASE','NSLOTS','renderRows', updateNames + ';syncPluckNames();')(table,params,slots,()=> 'My voice',data.parameters.userVoiceBase,data.parameters.userSlots,()=>{});
assert.deepEqual(table[params.enum.P_CM_SND].t.slice(34,42),factory,'factory entries34–41 survive slot name refresh');
assert.equal(table[params.enum.P_CM_SND].t[data.parameters.userVoiceBase],'DX My voice');
const play = editor.slice(editor.indexOf('$("vplay").addEventListener'),editor.indexOf('/* connecting */'));
const changes=[], values=[]; values[params.enum.P_CM_LVL]=60;
new Function('$','port','D','setParam','val','slot','USER_VOICE_BASE','renderRows','say',play)(()=>({addEventListener:(_name,fn)=>fn()}),{},params,(...args)=>changes.push(args),values,3,data.parameters.userVoiceBase,()=>{},()=>{});
assert.deepEqual(changes[0],[params.enum.P_CM_SND,data.parameters.userVoiceBase+3]);
const parameterCode=editor.slice(editor.indexOf('function clearPendingEdits('),editor.indexOf('const PAGES = '));
const sent=[];
new Function('send','setTimeout','say',`
  const port={}, val=[], pend=new Map(); let flushTimer=null; const parametersReady=true, parameterCount=${data.parameters.count};
  ${parameterCode}
  setParam(${params.enum.P_CM_SND}, ${data.parameters.userVoiceBase+3}); flush();
`)((bytes)=>sent.push(bytes),()=>1,()=>{});
assert.deepEqual(sent[0],[240,125,80,80,1,params.enum.P_CM_SND>>7,params.enum.P_CM_SND&127,64,data.parameters.userVoiceBase+3,247], 'actual parameter packet selects user slot3');
{
  const writes=[], cleared=[];
  const state=new Function('send','setTimeout','clearTimeout','say',`
    let port=null, parametersReady=false, parameterCount=${data.parameters.count}, flushTimer=null;
    const val=[], pend=new Map();
    ${parameterCode}
    setParam(${params.enum.P_CM_SND},42);
    const queuedOffline=pend.size;
    // A historical queued edit/timer survives until reconnect: it must not send.
    pend.set(${params.enum.P_CM_SND},45); flushTimer=7; port={}; flush();
    pend.set(${params.enum.P_CM_SND},46); flushTimer=8; clearPendingEdits();
    parametersReady=true; parameterCount=10; pend.set(${params.enum.P_CM_SND},47); flush();
    return {queuedOffline,remaining:pend.size,timer:flushTimer};
  `)((bytes)=>writes.push(bytes),()=>1,(timer)=>cleared.push(timer),()=>{});
  assert.equal(writes.length,0,'offline/pre-ready/out-of-peer-range edits never reach MIDI');
  assert.equal(state.queuedOffline,0); assert.equal(state.remaining,0); assert.equal(state.timer,null); assert(cleared.includes(8));
  assert(editor.slice(editor.indexOf('async function connect()')).includes('clearPendingEdits(); parametersReady = false'));
}
assert(!editor.includes('12 + 22'));

// Execute actual editor write code and read/tab bindings against one operation
// lock: a read cannot replace voiceWait during a send; live edits still coalesce.
{
  const env=environment(), session=await env.client.open(), acks=[deferred(),deferred()], writes=[], reports=[];
  let waiting=0,reads=0;
  const voiceSlots=new Array(data.parameters.userSlots).fill(null);
  const sendCode=editor.slice(editor.indexOf('async function sendVoice()'),editor.indexOf('let slotsTimer = '));
  const api=new Function('device','port','slots','packVoice','checksum','send','waitFor','say','renderSlotsSoon',`
    let sending=false,again=false,nloaded=0,slot=0,voice=Uint8Array.from([1]); const parametersReady=true;
    ${sendCode}
    return {sendVoice,edit(value){voice[0]=value;},get sending(){return sending;}};
  `)(env.client,session,voiceSlots,(voice)=>new Uint8Array(128).fill(voice[0]),()=>0,(bytes)=>writes.push(bytes),()=>acks[waiting++].promise,(text)=>reports.push(text),()=>{});
  const readWait=deferred(); let readClick;
  const readBinding=editor.slice(editor.indexOf('$("vread").addEventListener'),editor.indexOf('$("vplay").addEventListener'));
  new Function('$','deviceAction','device','readSlots','say',readBinding)(()=>({addEventListener:(_event,fn)=>{readClick=fn;}}),deviceAction,env.client,async()=>{reads++;await readWait.promise;},(text)=>reports.push(text));
  const first=api.sendVoice(); api.edit(2); await api.sendVoice();
  await readClick(); assert.equal(reads,0); assert(reports.at(-1).includes(data.product.name));
  const tabs=new Map(),tabBinding=editor.slice(editor.indexOf('for (const t of ["sound", "voice", "about"]) $("tab-'));
  new Function('$','deviceAction','device','readSlots','say','tab','parametersReady','slots','nloaded',tabBinding.slice(0,tabBinding.indexOf('\n\nrenderPages')))((id)=>({addEventListener:(_event,fn)=>tabs.set(id,fn)}),deviceAction,env.client,async()=>{reads++;},(text)=>reports.push(text),()=>{},true,voiceSlots,0);
  tabs.get('tab-voice')(); await new Promise(setImmediate); assert.equal(reads,0,'tab cannot start a second read');
  acks[0].resolve({n:1}); await new Promise(setImmediate); assert.equal(writes.length,2);
  acks[1].resolve({n:1}); await first;
  assert.equal(api.sending,false); assert.equal(env.client.busy,false);
  assert.equal(writes[0][6],1); assert.equal(writes[1][6],2,'coalesced send carries the latest live voice');
  const reading=readClick(); assert.equal(reads,1);
  await api.sendVoice(); assert.equal(writes.length,2,'send during read is rejected without a packet');
  readWait.resolve(); await reading; env.client.close();
}
const readSlots = editor.slice(editor.indexOf('async function readSlots()'),editor.indexOf('function syncPluckNames()'));
for(const timeout of [false,true]) {
  const messages=[];
  const read=new Function('NSLOTS','slots','device','say','unpackVoice','syncPluckNames','renderSlots','renderVoice','setTimeout',`
    let voiceWait=null, voiceRequested=-1, nloaded=16, voice=null; const slot=0;
    function send() { ${timeout ? '' : 'queueMicrotask(() => voiceWait({slot:0,loaded:0,data:null}));'} }
    ${readSlots}
    return readSlots().then(() => ({nloaded}));
  `);
  const target=new Array(data.parameters.userSlots).fill(new Uint8Array(155));
  await read(data.parameters.userSlots,target,{text:(key)=>deviceText(data,'en',key)},(text)=>messages.push(text),()=>assert.fail('empty reply cannot unpack a voice'),()=>{},()=>{},()=>{},(fn)=>queueMicrotask(fn));
  if(timeout) { assert(messages.at(-1).includes(data.product.name)); assert(!messages.at(-1).includes('0 voices')); }
  else { assert(messages.at(-1).includes('0 voices')); assert(target.every((entry)=>entry===null)); }
}

const installer=fs.readFileSync('web/amb_installer.html','utf8');
{
  const env=environment(), transfer=deferred(), reports=[]; let starts=0;
  const gate=installer.slice(installer.indexOf('async function run(img'),installer.indexOf('async function runFirmware('));
  const run=new Function('device','say','runFirmware',`const writing=false; ${gate}; return run;`)(env.client,(message)=>reports.push(message),async()=>{starts++;return transfer.promise;});
  const first=run({},'FM-1_7000300',false);
  assert.equal(await run({},'FM-1_7000300',false),false);
  assert.equal(starts,1,'repeated install cannot bypass the lock during MIDI preflight');
  assert(reports[0].includes(data.product.name)); transfer.resolve(true); assert.equal(await first,true);
}
const translation=installer.slice(installer.indexOf('const TEXT = '),installer.indexOf('let lang = '));
const text=new Function('DEVICE_TEXT',translation+';return TEXT;')(DEVICE_TEXT);
assert.deepEqual(Object.keys(text.en).sort(),Object.keys(text.ja).sort(),'installer supported languages have the same keys');
assert(!text.en.mmold.includes('0.16.18')); assert.equal(text.ja.mmold,DEVICE_TEXT.ja.mapSettings);
for (const marker of installer.matchAll(/data-t="([^"]+)"/g)) assert(marker[1] in text.en,marker[1]);

if(process.argv[2]) {
  for(const route of ['install','editor','pieces']) {
    const html=fs.readFileSync(path.join(process.argv[2],route,'index.html'),'utf8');
    assert(!html.includes('/*DEVICE*/'));
    const release=JSON.parse(/const DEVICE_RELEASE = (\{[^\n]+\});/.exec(html)[1]);
    const version=process.argv[3] || data.product.releaseVersion;
    assert.equal(release.version,version);
    assert.match(release.product,/^FM-1_7\d{6}$/); assert.equal(release.pkg,`../firmware/${data.product.slug}-${version}.fwsc`);
    assert(html.includes('const DEVICE_DATA = ')); assert(html.includes('function createDeviceClient('));
  }
}
console.log(JSON.stringify({metadata_drift:'pass',locales:['en','ja'],capability_connection_busy_import_tests:'pass',legacy_backup_map_bytes:'identical',dx_user_offset:data.parameters.userVoiceBase,hardware_qa:'not run'}));
