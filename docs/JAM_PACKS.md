# Songbook and USB Jam Packs (0.4.4)

## Current standards and instruments

The factory bank contains **Blue Bossa**, **Autumn Leaves** and **Satin Doll**.
Four user arrangements add Blue Bossa Deep, Leaves Drawbars, Satin Muted Guitar
and Major Minor Lab. Their authored chords, forms, drums and bass patterns stay
intact while 30 custom DX7 instruments provide six keyboard and two bass choices
per pack. [The program manifest](../packs/citypop-manifest.json) and
[voice catalog](../packs/voice-catalog.json) record source banks and octave adaptations.

On v3 packs, turn knob 2 on **DX KEYS** or **DX BASS** to change **OCTAVE**
independently from −2 to +2. New packs start with keys at +1 and bass at +1.
The change applies to the next attack; note-off messages release the actual
previous note. Native program bytes and authored chord voicings are preserved.
Touch and bass Attack remain available through advanced pack options and USB controls.
Earlier v1/v2 packs retain their previous Touch/Attack knob behavior.

The default plate uses Reverb 38, Size 68, Damping 42 and Predelay 80.
Regenerate the reviewed current bank with `python tools/build_citypop.py`.
`python tools/build_standards.py` regenerates the earlier instrument arrangements;
run the city-pop generator afterward to restore the current factory bank.

## Synth Jam: play the keyboard part

Synth Jam requires firmware 0.4.4 or later and a v3 pack.

Turn SELECT to **SYNTH JAM**, then turn its first knob from **BAND** to **SYNTH**.
The automatic keyboard part gives way to your own playing. Drums and bass keep
their place in the song; you can use its current sound straight away, without
loading a different patch or resetting the chart.

| Synth Jam knob | Action |
|---|---|
| MODE | BAND plays the authored keyboard accompaniment; SYNTH lets you play it |
| PROGRAM | Choose a program from the current song's bank |
| OCTAVE | Set the keyboard register from −2 to +2 |
| SPACE | Adjust the existing reverb amount from 0 to 100 |

All **27 piano keys** play chromatic notes in Synth mode, including the keys that
serve as Songbook shortcuts in Band mode. Up to **five keyboard notes** sound at
once, with a separate voice reserved for the bass. Hold a key to sustain its
note and release it to let the program's envelope finish. MIDI notes received
by the FM-1 play their received pitches; they are not snapped to a scale or
shifted by the panel's octave setting. The browser's optional MIDI controller
mapping operates its virtual panel keys, so it retains the panel's note range
and octave control.

**PLAY** starts or stops the drums and bass while the keyboard remains playable.
**HOME** releases the held keyboard notes, returns to Band mode and shows the
Songbook performance page. It does not restart the groove or chart. MODE also
lets you switch back to BAND on the Synth Jam page. The detail-page header shows
the current Songbook key and written chord.

Synth Jam uses the existing keyboard program and octave settings. The new mode
is `parts.synth`, stored as 0 (Band) or 1 (Synth) in v3 header byte 1120+25; an
absent field defaults to Band. It is saved with the pack. To retain edited
mode/program/register settings on the FM-1, use the explicit stopped USB pack
save, as for other part settings. The front-panel SAVE command stores session
settings; it is not a substitute for saving the edited pack.

USB control field 24 is `synth` (0–1), and field 25 is `space` (0–100). SPACE
updates the existing reverb entry in `sounds`; it does not create another
effect parameter or duplicate storage field. For example:

```sh
python tools/fm1_jam.py control synth 1
python tools/fm1_jam.py control space 38
python tools/fm1_jam.py control synth 0
```

The panel's note number is `53 + key_index + 12 * keys_octave`, with key indices
0–26. This is a live keyboard mode, independent of the chart's root/quality
planner. v1/v2 packs retain the earlier Swells page and shortcut behavior.

## Written chord replay (v3)

V3 plays the chord qualities written in the chart. A Cmaj7 contains C, E, G and B;
Cm7 contains C, Eb, G and Bb. The planner voices those notes smoothly and can
spread them with drop-2. It does not substitute a sixth-family approximation or
rewrite a section's final chord. The older v1/v2 musical behavior is retained for
older packs.

Create a text file with one or two symbols per 4/4 bar, separated by `|`, or one
bar per line. A single symbol lasts four beats; two symbols last two beats each.
An eight-bar example is:

```text
| Dm7 | G7 | Cmaj7 | Cmaj7 | Dm7 | G7b9 | C6 | C6 |
```

Import it using an existing pack for the instruments, bass motifs and drums:

```sh
python tools/chord_chart.py my-changes.txt packs/01-blue-bossa.json build/my-changes.json --key C --name "My Changes"
python tools/fm1_jam.py validate build/my-changes.json
python tools/fm1_jam.py audition build/my-changes.json
python tools/fm1_jam.py control bars 8
python tools/fm1_jam.py control form 0
python tools/fm1_jam.py control movement 0
```

The import command reports the original tonic and required section length.
Set the session key to that tonic for the written pitches, or choose another
key to transpose. Imported charts use identical major/minor banks, so changing
the mode selector does not silently reharmonize a written tune. JSON input may
contain `{"name":"My Changes","key":"C","bars":["Dm7","G7",...],"sources":[...]}`;
each bar can also be a list of one or two symbols.

Charts must contain 8, 16 or 32 bars and at most 32 distinct root/quality pairs.
Eight- and sixteen-bar charts loop as one section of that length. A 32-bar chart
uses two 16-bar sections in Authored/Chart form. Selecting the 8-bar view of a
16-bar section plays its first eight bars. Three or more changes per bar, slash
bass chords and unsupported symbols are rejected, not approximated. This is a
chord-chart importer, not an automatic audio or arbitrary MIDI transcription.

| Movement knob on a v3 chart | What changes |
|---|---|
| CHART (0) | Uses the chart's close/drop-2 voicing preference |
| CLOSE (1) | Close-position voice leading |
| DROP2 (2) | Drops the second-highest closed voice by an octave |
| INVERT (3) | Deterministic inversion movement using the written chord tones |

New Idea, bass feel and Variation change the bass accompaniment; INVERT changes
chord inversions. The written roots, qualities and half-bar timing remain intact. `--voicing close`
sets a close-position preference; the importer defaults to drop-2. The input
chart and source provenance remain editable JSON. Additional songs can be
auditioned and explicitly saved to one of four user slots without reflashing
the v3-capable engine.

| Quality ID | Symbol | Four keyboard intervals above the written root |
|---|---|---|
| 0 | maj7 / M7 | 0, 4, 7, 11 |
| 1 | m7 / min7 / -7 | 0, 3, 7, 10 |
| 2 | 7 | 0, 4, 7, 10 |
| 3 | m7b5 / ø7 | 0, 3, 6, 10 |
| 4 | dim7 / o7 | 0, 3, 6, 9 |
| 5 | 6 | 0, 4, 7, 9 |
| 6 | m6 | 0, 3, 7, 9 |
| 7 | 7sus4 | 0, 5, 7, 10 |
| 8 | mMaj7 | 0, 3, 7, 11 |
| 9 | +7 / aug7 | 0, 4, 8, 10 |
| 10 | 7b9 | 4, 7, 10, 13 |
| 11 | maj9 | 4, 7, 11, 14 |
| 12 | m9 | 3, 7, 10, 14 |
| 13 | 9 | 4, 7, 10, 14 |
| 14 | 7alt | 4, 8, 10, 13 |
| 15 | major triad (no suffix) | 0, 4, 7, 12 |
| 16 | minor triad (m) | 0, 3, 7, 12 |

Extended four-note keyboard voicings omit the root where shown; the bass supplies
the written root. `7alt` has the explicitly defined altered fifth and flat ninth
above, rather than an arbitrary choice of every possible altered dominant.

V3 keeps the v2 byte layout and 3,840-byte maximum: magic/version are `JMP3`/3,
the extension magic is `SNG3`, node byte +7 stores the quality ID, and header
1120+21 stores voicing (0 close, 1 drop-2). Node `bass` is the chord root relative
to the session tonic. Other legacy node fields stay bounded but do not determine
v3 chord pitches. Both chart banks may share nodes. Features operation 11 uses
bitmask 7 for v1+v2+v3. Older engines reject v3 rather than misplaying its chart.

V3 header bytes 1120+23 and 1120+24 store signed `keys_octave` and `bass_octave`
values from −2 to +2, encoded as signed bytes. Missing fields default to zero,
so older v3 packs keep their register. Firmware 0.4.3 applies these at playback
to both audio and outgoing MIDI. USB control fields 22 and 23 encode the user
value plus 2 (wire range 0–4); older control fields keep their existing encoding.

V3 `parts.voice_flags` is an optional 0–3 field at header offset 1120+22.
Its default is 0, preserving existing packs byte-for-byte and retaining their
original playback behavior. Firmware with native-voice support interprets:

| Flag bit | Behavior |
|---|---|
| 0 (`1`) | Preserve each keyboard operator's velocity sensitivity at Touch 50. Other Touch values add `((touch - 50) * 7) / 50` to each original sensitivity, clamped to 0–7; signed division truncates toward zero. Bass sensitivities remain unchanged. |
| 1 (`2`) | Preserve the program's pitch-LFO depth for keys and bass. Without this bit, pitch-LFO depth is zero. |

Use `voice_flags: 3`, `touch: 50`, `brightness: 50`, `keys_gate: 65` and
`attack: 0` to retain the imported programs' operator settings and pitch-LFO
depth. Timing, voice allocation, role gain and effects still belong to the
accompaniment engine. Existing v1/v2 packs ignore these flags. Earlier v3
firmware (0.4.1) ignores this previously reserved byte; native-voice profiles
therefore require the native-voice firmware upgrade once, then ordinary pack
uploads can change the programs without reflashing.

## Earlier sixth-family songs (v1/v2)

Songbook is a backing band for a lead player, with original Harris-informed Citypop compositions, a stable home key, and space above the keyboard voicings for flute. It composes moving sixth-family voicings around authored harmonic destinations. These are original arrangements informed by Barry Harris teaching, not transcriptions or a claim to reproduce everything he would play.

## Earlier v1/v2 factory bank

| Song | Character | Full form |
|---|---|---|
| Velvet Changes | Mellow tine, sixth-family movement, a contrasting bridge | A-A'-B-A'' |
| Afterhours | Blooming electric piano, minor colors, string swells | A-A'-B-A'' |
| Neon Turnaround | Barking electric piano, dominant chains, brass swells | A-B-A-B |

Each song has major/minor realizations and separate 8/16-bar section charts: four sections make 32/64 bars. Song changes preserve key, mode, tempo and section length. The Authored form is the intended whole song; Vamp, AABA and ABAB are alternatives. Hold repeats the complete current section. New Idea releases Hold and requests a related bass variation at the next boundary. The home key changes only when requested.

The harmony uses major/minor sixth-diminished families, relative-sixth upper structures for sevenths, half-diminished and dominant chords, borrowed notes that return to structural voices, and occasional diminished connectors. Destinations move twice per bar; eight- and sixteen-bar answers have written cadences. Four keyboard notes are voice-led within MIDI 48-72; bass stays within 28-52 and targets the next structural root. Bass motifs recur across sections with controlled answer variations. Factory grooves use straight sixteenths, written velocities and syncopation, without random timing offsets. Activity controls the amount of harmonic movement; Variation controls borrowed colors and motif changes.

## Earlier v1/v2 sound and panel

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

Run from the repository root. The Python CLI uses the Rust authoring tools; see [build prerequisites](../BUILDING.md). MIDI files need `mido`; USB needs `python-rtmidi`. Preview uses the existing host toolchain (Ubuntu-22.04 WSL on Windows).

```sh
python tools/fm1_jam.py features
python tools/fm1_jam.py pull build/before.json
python tools/fm1_jam.py decode packs/01-blue-bossa.jmp build/my-song.json
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
