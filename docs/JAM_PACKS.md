# Songbook and USB Jam Packs (0.3.0)

Songbook is a backing band for a lead player, with original Harris-informed Citypop compositions, a stable home key, and space above the keyboard voicings for flute. It composes moving sixth-family voicings around authored harmonic destinations. These are original arrangements informed by Barry Harris teaching, not transcriptions or a claim to reproduce everything he would play.

## Three factory songs

| Song | Character | Full form |
|---|---|---|
| Velvet Changes | Mellow tine, sixth-family movement, a contrasting bridge | A-A'-B-A'' |
| Afterhours | Blooming electric piano, minor colors, string swells | A-A'-B-A'' |
| Neon Turnaround | Barking electric piano, dominant chains, brass swells | A-B-A-B |

Each song has major/minor realizations and separate 8/16-bar section charts: four sections make 32/64 bars. Song changes preserve key, mode, tempo and section length. The Authored form is the intended whole song; Vamp, AABA and ABAB are alternatives. Hold repeats the complete current section. New Idea releases Hold and requests a related bass variation at the next boundary. The home key changes only when requested.

The harmony uses major/minor sixth-diminished families, relative-sixth upper structures for sevenths, half-diminished and dominant chords, borrowed notes that return to structural voices, and occasional diminished connectors. Destinations move twice per bar; eight- and sixteen-bar answers have written cadences. Four keyboard notes are voice-led within MIDI 48-72; bass stays within 28-52 and targets the next structural root. Bass motifs recur across sections with controlled answer variations. Factory grooves use straight sixteenths, written velocities and syncopation, without random timing offsets. Activity controls the amount of harmonic movement; Variation controls borrowed colors and motif changes.

## Sound and panel

Keys and bass each have an independent six-operator DX program. Six FM slots provide four keyboard notes, a protected bass slot and one keyboard release slot. Old-voicing overlap fades over about 50 ms, avoiding a fifth sustained keyboard note across chord changes. Programs, touch, brightness, keyboard gate/tail and bass attack/gate can be changed independently. A program change commits at that part's next attack and retires its old tails; it leaves the other part playing. Pitch modulation is disabled for the two accompaniment programs. Two existing pad voices produce two-beat, phrase-timed swells every fourth bar; their texture and amount are adjustable. A short quiet plate supplies space without automatic pitch drift, chorus or shimmer. This preserves the useful PERF-1 synthesis and swell behavior while keeping harmonic movement audible.

| Control | Action |
|---|---|
| PRESETS on Songbook | Three factory songs, followed by four user slots; queue at the section boundary |
| GLO + PRESETS | Choose form |
| Knobs 1-4 | Home key (12 major, then 12 minor), BPM, 8/16 section bars, harmonic activity |
| ALGORITHM | Accompaniment energy |
| F3 / REC | Hold/release the complete section |
| G3 / EDIT | Related new idea at the next section boundary |
| A3 | Cycle bass feel |
| B3 | Fill next bar |
| PLAY | Start/stop |
| SAVE | Save the session while stopped |
| SELECT | Arrangement, DX Keys, DX Bass, Swells, then legacy sound/effects pages |
| HOME | Return to Songbook |
| F4 | Return to the legacy instrument |

The Arrangement page exposes form, section bars, variation and swell amount. DX Keys exposes program, touch, brightness and gate/tail. DX Bass exposes program, attack, gate and energy. Swells exposes texture, amount, variation and activity. Role profiles edited here are part of the pack and survive explicit pack save/readback. Generic legacy effects parameters remain live session edits; author them into `sounds` for portable pack persistence. SAVE saves session settings; use the USB `save SLOT` command to persist edited musical/voice data.

All 176 legacy parameter indices, old pieces, v1 Jam packs and four stored slots remain compatible. Existing saved sessions load their selected old pack until a new song is selected.

## Computer workflow

Run from `C:/Users/ericm/fm1/fw`. Compilation uses Python's standard library. MIDI files need `mido`; USB needs `python-rtmidi`. Preview uses the existing host toolchain (Ubuntu-22.04 WSL on Windows).

```sh
python tools/fm1_jam.py features
python tools/fm1_jam.py pull build/before.json
python tools/fm1_jam.py factory-song build/my-song.json --song 0
python tools/fm1_jam.py validate build/my-song.json
python tools/fm1_jam.py preview build/my-song.json build/my-preview --bars 16 --bpm 100 --phrases 4
python tools/fm1_jam.py audition build/my-song.json
python tools/fm1_jam.py control play 0
python tools/fm1_jam.py save 2
```

Preview writes WAV, MIDI, a screenshot and the MIDI event log from the actual engine. Four phrases render the entire 32/64-bar form. Present playback paused; structural tests and a successful build do not establish musical taste.

`audition` first backs up the active binary to `build/jam-backups`, validates and transfers the new pack to RAM, then waits for the section boundary. `--no-wait` returns after queuing. Check `status` rather than repeatedly resending. `rollback previous.jmp` follows the same RAM audition path. Save requires stopped playback and verifies a complete stored readback. Preserve existing user slots unless the user requests replacement. After saving, the pack and session load without a computer. Routine songs, programs and profile edits require no firmware rewrite. New DSP or executable operations still require firmware.

```sh
python tools/fm1_jam.py import-voice bank.syx build/my-song.json build/new-voice.json --voice 3 --role keys
python tools/fm1_jam.py inspect-midi reference.mid
python tools/fm1_jam.py import-midi reference.mid build/my-song.json build/new-bass.json --track 1 --tonic 0 --bar 1 --role bass
```

Voice import reads Yamaha-format 32-voice VMEM banks, retains the selected 128-byte program, appends it to the song's program list and assigns its role. No downloads or paid libraries are required. Record provenance and observe the source license. At most eight programs fit a pack. Bass MIDI import extracts one selected 4/4 bar, quantizes to sixteenths and adds a relative motif; select it with the bass-feel control. Harmony import currently recognizes major/minor sixth voicings for v1 packs only. For v2, author chart destinations explicitly after inspecting the reference. MIDI is a reference to interpret, not proof of a canonical Harris algorithm.

## Songbook v2 data

`packs/velvet-changes.json`, `afterhours.json` and `neon-turnaround.json` are complete editable examples. `factory-song output.json --song 0..2` generates one. Maintainers can run `python tools/songbook.py` to regenerate all three JSON/binary files and the immutable firmware bank. Routine USB edits do not regenerate firmware headers.

Common fields are name, two eight-note scales, nodes, cadence references, four forms, up to eight bass motifs, up to eight drum/comp masks, up to 32 existing sound settings, and UTF-8 source strings. Packs cannot overwrite tempo, key or global preferences. Unlisted sound parameters start from the dry Jam baseline, so prior pieces cannot leave random effects behind.

V2 adds:

- Up to 32 harmonic nodes: `bass` and `sixth` root offsets, `scale`, `label`, `mode`, and `borrow` voice mask. Labels 0-7 are 6, m7, m7b5, 9, m6, dim7, maj9 and 7alt.
- `charts[mode][length][part]`: mode major/minor, length 8/16 bars, part A/B; two node indices per bar. Mode boundaries are validated. Form roles 0/3 select A and 1/2 select B.
- `voice_programs`: one to eight objects with a name and 128 seven-bit DX VMEM bytes. Program names are the embedded ten-character DX names.
- `parts`: keys/bass program indices, swell texture (0-23), keys/bass gain (0-100), swell amount (0-100), variation (0-3), keys/bass gate (1-100), touch/brightness/attack (0-100). `keys_gate` also changes DX release rates; 65 retains the program's original rates.
- `song_id` (0-2) chooses the recurring factory bass contour. Bass feel adds a player-selected motif offset.

Bass motifs use 16 note and velocity entries plus a gate. Notes -12..12 are semitone offsets; -128 rests, -127 approaches the next structural root from below, -126 selects its chord third, and -125 its fifth. Comp/drum masks use sixteen bits per bar. These remain bounded data operations.

| V2 offset | Content |
|---|---|
| 0 | `JMP2`, version 2, common counts |
| 1120 | `SNG2` header (32 bytes): program count, role indices, gains, amount/variation, song id; gate/touch/brightness/attack at +16..20 |
| 1152 | Nodes 16-31 (first sixteen remain at 64) |
| 1280 | Eight padded 32-byte destination charts |
| 1536 | Eight reserved 128-byte DX program records |
| 2560 | UTF-8 source JSON |

V2 keeps musical sixteenths sample-timed while coalescing intermediate outgoing MIDI-clock messages to the next audio chunk (at most 5.8 ms additional delay). Clock count, 24 PPQN and absolute tempo phase are retained. V1 scheduling and externally supplied MIDI clock retain their previous behavior.

Factory packs are 2,792 bytes; the maximum remains 3,840. The legacy DX upload staging buffer doubles as the v2 bass audio ring, so legacy loose-voice upload is disabled while Songbook v2 is active. Import voices into the song pack instead.

## Common layout and v1 compatibility

All multibyte numeric data is little-endian. The binary is 1,120–3,840 bytes.

| Offset | Content |
|---|---|
| 0 | `JMP1`, version 1, node/bass/drum/sound counts |
| 9 | Six cadence node indices |
| 16 | Name, 24-byte zero-terminated ASCII field |
| 40 | Two eight-note scales |
| 64 | Sixteen 8-byte node records: bass, sixth, scale, label, mode, emphasis, reserved ×2 |
| 192 | 16×16 transition matrix |
| 448 | Four 32-byte forms: name[16], section count, up to eight roles, reserved |
| 576 | Eight 36-byte bass records: int8 notes[16], velocities[16], gate, reserved |
| 864 | Eight 16-byte drum records: six uint16 lane masks, uint16 comp mask, uint8 comp gate (0 means 80), reserved |
| 992 | Thirty-two 4-byte sound records: uint16 parameter, int16 value |
| 1120 | Optional source JSON in v1; v2 extension follows |

Objects 26–29 are the four packs at A/B bases `0xC5000`, `0xC7000`, `0xC9000`, `0xCB000`. Object 30 is the separate session at `0xCD000/0xCE000`. Existing IDs and flash areas are unchanged. The existing A/B storage layer writes commit headers last and validates CRCs. Backups probe the new capability before including these objects; old backups still restore.

The active buffer is in regular BSS, preserving the audio pool's 4 KiB reserve. The implementation budgets at most 5.5 KiB of added static state, including two compact phrase plans and protocol overhead, and reuses the legacy backup buffer for staging. No heap allocation or flash write occurs in the audio render.

## USB protocol v1

Requests are `F0 7D 50 60 op token payload checksum F7`. Replies are `F0 7D 50 61 op token status [raw bytes as high/low nibbles] checksum F7`. The checksum is the seven-bit two's complement of bytes from `op` through the end of the payload. Token is a caller-selected 7-bit transaction identifier.

| Operation | Request payload | Reply raw payload |
|---|---|---|
| 1 capabilities | none | version, max bytes LE16, slots, first pack object, session object |
| 2 read | slot (0 active, 1–4 saved), 64-byte chunk index | total length LE16, offset LE16, bytes |
| 3 begin | length as high/low 7-bit pair, CRC32 as eight most-significant-first nibbles | none |
| 4 chunk | index, count as high/low 7-bit pair, data nibbles (up to 256 raw bytes) | index |
| 5 validate | none | none |
| 6 audition | none | none; status remains pending until activated |
| 7 save active | slot 1–4 | saved CRC32 LE32 |
| 8 status | none | on, active bars, bar index, beat index, hold, pending, slot, CRC32 LE32, revision LE32, xruns LE32, requested key/minor/form/movement/energy, playing |
| 9 abort | none | none; already queued activation cannot be aborted |
| 10 control | field, value as high/low 7-bit pair | none |

Status codes: 0 success, 1 malformed/invalid, 2 checksum, 3 busy, 4 stop required, 5 storage/readback failure, 6 unsupported pack version. Control fields 0–10 are on, key, bars, form, movement, energy, hold, bass, idea, saved slot, transport. Production clients should use the CLI rather than reconstructing frames.

Incomplete transfers/read snapshots expire after thirty seconds of inactivity. Identical duplicate chunks are accepted; conflicting duplicates and incomplete validation are rejected. A client must wait for activation before another upload. The immutable factory pack remains available as a fallback. New executable musical operations or synthesis engines require firmware; ordinary rules and musical content do not.


## Additional 0.3.0 USB operations

The original capability response and protocol envelope remain unchanged. Operation 11 reports pack-version bits (3 means v1+v2), eight programs, three factory songs and six FM slots. Operation 12 reports requested song source, active pack version and loading flag. Operation 13 accepts a temporary fixed-level USB recording flag (0/1), returns it, and never alters analog master volume or saves preferences.

Control fields 11-21 are song source, swell amount, variation, keys program, bass program, touch, keyboard gate/tail, bass gate, swell texture, bass attack and brightness. Song sources 0-2 are factory songs and 3-6 are saved slots 1-4. `features` probes v2 support before auditioning a new pack. `song-status` distinguishes a pending source from the active pack version.

The 20-byte JSM1 session retains its existing fields; reserved bytes 16-17 record a factory source plus the marker `S`. Older sessions remain valid. Role edits live in the pack, not new legacy parameter indices.

## Validation and references

Run `tests/run_tests.sh` for host/USB/UI/WASM checks. `tools/songbook_accept.py --audio-device INDEX` runs the optional ten-minute physical test and fixed-level recording; it writes evidence under `build/hardware/songbook` and never saves a slot. Select the FM-1 WASAPI input index from `sounddevice.query_devices()`.

Primary harmonic reference: Howard Rees / Barry Harris, [Evolutionary Voicings, Part 1](https://jazzworkshops.com/workshop-item/evolutionary-voicings-part-1/). Open exercises: [A. Aelony's Barry Harris exercises](https://github.com/aaelony/barry-harris-exercises). FM engine reference: the locally pinned [PERF-1](https://github.com/deamonSLINKY/PERF-1) source. Embedded voices are adaptations of its public-domain DY bank; see `tools/fm6_dy.txt` and `LICENSING.md`. No third-party MIDI performance is bundled.
