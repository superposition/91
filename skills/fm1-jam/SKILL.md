---
name: fm1-jam
description: Edit NAIWAN 91 / M-VAVE FM-1 Songbook and Jam packs, import MIDI or DX voices, render engine previews, and audition or save changes over USB without reflashing firmware.
---

# FM-1 Songbook and Jam Packs

Use `C:/Users/ericm/fm1/fw/tools/fm1_jam.py` (or `$FM1_REPO/tools/fm1_jam.py`). Read [the format and workflow](C:/Users/ericm/fm1/fw/docs/JAM_PACKS.md) when editing musical rules or the binary interface. Use the fm1-editor skill for the existing 176 legacy parameters; role profiles are pack data and use this CLI.

The user wants an expressive Harris-informed Citypop band for flute: stable home key, written 32/64-bar songs built from 8/16-bar sections, independent DX electric piano and bass, and PERF-1-style musical swells. Preserve this direction. The first preview had blurred upper partials and excessive reverb/pitch motion; the later approved plan explicitly keeps expressive synthesis and phrase-timed swells. Start with the curated programs, a short quiet plate and stable pitch. Compare tone separately from harmony. Tests establish structure and performance, not musical taste. These are original arrangements informed by Harris teaching, not a universal Barry Harris composition algorithm.

## Edit and preview

1. Run `features`, `status`, and `pull build/before.json` for the connected pack. Preserve the original. Factory v2 examples are `packs/velvet-changes.json`, `afterhours.json` and `neon-turnaround.json`; `factory-song output.json --song 0..2` generates one. The older `packs/harris-citypop.json` remains v1 compatible.
2. V2 edits use explicit charts (major/minor, 8/16 bars, A/B, two destinations per bar), up to 32 sixth-family nodes, four forms, eight bass/drum motifs, 32 legacy sound settings and eight embedded 128-byte DX programs. `parts` selects programs, texture, gains, touch, gate/tail, brightness and attack. Key and tempo stay in the session. Review cadences, borrowed-note resolutions, low register and flute space.
3. Run `validate edited.json`, then `preview edited.json build/preview --bars 16 --bpm 100 --phrases 4`. WAV, MIDI, screenshot and MIDI log come from the actual engine. Four sections render the complete 64-bar song (or 32 bars with `--bars 8`). Present audio paused. Never claim to have listened from waveform statistics alone.
4. `import-voice bank.syx base.json output.json --voice N --role keys|bass` imports one voice from a Yamaha 32-voice VMEM bank and assigns its role. At most eight embedded programs fit a pack. Preserve provenance and source licensing; no paid library is required. Loose legacy DX uploads are disabled while v2 uses that staging memory for its bass ring.
5. `inspect-midi reference.mid` first. `import-midi reference.mid base.json output.json --track N --tonic 0 --bar 1 --role bass` extracts a selected 4/4 bar into a relative motif; select it with bass feel. Timing is quantized to sixteenths. Harmony import recognizes sixth voicings only in v1; author v2 destinations after review. Do not describe arbitrary MIDI as fully transcribed.

## USB audition and save

MIDI commands need `mido` and `python-rtmidi`. `ports` lists devices; `--port SUBSTRING` disambiguates.

```sh
python tools/fm1_jam.py features
python tools/fm1_jam.py pull build/before.json
python tools/fm1_jam.py diff edited.json --against build/before.json
python tools/fm1_jam.py audition edited.json
python tools/fm1_jam.py status
python tools/fm1_jam.py control play 0
python tools/fm1_jam.py save 2
```

Audition backs up the active binary to `build/jam-backups`, transfers to RAM, validates, and waits for the section boundary. It prints the rollback path. `rollback path.jmp` re-auditions the old pack without overwriting saved slots. Check a pending upload with `status`; do not resend it. Explicit stopped `save 1..4` performs complete stored readback. Preserve occupied user slots unless replacement was requested. Panel SAVE saves session settings; edited pack data must be saved to a slot through the CLI. Generic legacy effects edits are live only unless authored into the pack's `sounds` list.

`control` supports `on`, `key` (0..11 major, 12..23 minor), `bars` (8/16), `form` (0 Authored, 1 Vamp, 2 AABA, 3 ABAB), `movement`, `energy`, `hold`, `bass`, `idea 1`, `play`, `song` (0..2 factory; 3..6 saved slots), `swell`, `variation`, `keys_program`, `bass_program`, `touch`, `release` (keyboard gate/tail), `bass_gate`, `swell_voice`, `bass_attack`, and `brightness`. `song-status` reports requested source and active pack version. Program changes commit at the next attack of that role, retiring its old tails without touching the other role.

No port: finish pack and simulator work and report the hardware check unverified. Older engine: explain the one-time Songbook upgrade requirement; ordinary music/voice updates then use packs. Abort an incomplete transfer with `abort`; transfers and read snapshots expire after 30 seconds. Legacy backup and pack transfers share staging and cannot run together.

Packs are bounded data, at most 3,840 bytes in four saved slots. New DSP or executable operations require firmware; scripts cannot be uploaded. Firmware maintainers regenerate factory assets with `python tools/songbook.py`. Optional connected acceptance: `tools/songbook_accept.py --audio-device INDEX` records ten minutes and all three songs without saving a slot.
