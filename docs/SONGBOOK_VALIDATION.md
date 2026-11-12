# Songbook 0.3.0 local validation

Validated on 2026-11-10, on the connected M-VAVE FM-1. The installed identity is `FM-1_7000300`. This release remains local; public hosting was not changed.

## Result

- Three original songs, major/minor realizations, 8/16-bar sections and 32/64-bar forms.
- Independent DX keys/bass, protected bass allocation, bounded keyboard overlap, phrase-timed swells and persistent role profiles.
- Routine songs and up to eight embedded DX programs update through USB packs and the installed `fm1-jam` skill.
- Velvet Changes is saved in slot 2, with F-sharp minor, 97 BPM and eight-bar sections. Readback and restart restoration match the factory binary. Original slot 1 and legacy stored objects remain byte-identical. Playback is stopped for the player.

## Physical acceptance

The final uninterrupted run recorded 600.07 seconds of stereo 44.1 kHz USB audio at a fixed recording level. It exercised all three songs, 16-bar sections, song handoffs, role edits, Hold/New Idea, telemetry/status polling and a live 2,792-byte pack transfer. Analog master volume was not changed.

| Check | Result |
|---|---|
| Added audio underruns | 0 |
| Added late deadlines | 0 |
| Added crashes | 0 |
| Maximum load shedding | 0 |
| Maximum main / second-core load | 81% / 70% |
| Worst measured render half | 11393 us / 11,609 us budget |
| Audio capture callback errors | 0 |
| Captured PCM16 peak / RMS | 5083 / 951.1 |

Evidence: `build/hardware/songbook-final/report.json`, `telemetry.json`, `extra.json`, `status.json`, `device-songbook.wav`, `saved-result.json` and `reboot-check.json`. Earlier failed/interrupted runs are retained in separate build folders. The first full run had no lost buffers but failed the deadline criterion; subsequent fixes bounded old-voicing overlap, reduced planner and patch-loading work, deferred following-section planning past the handoff, coalesced intermediate MIDI clocks, and removed random timing offsets from the factory grooves. The final result above is from the finished build.

## Software checks

- 144 song/key/mode/length combinations: scale membership, voicing register, cadence destinations, bass register, half-bar bass changes and altered-dominant chord tones.
- Reference comparison: 18,432 optimized harmonic phrases matched the original planner byte-for-byte.
- Voice-role isolation, protected bass under stealing, independent LFO resets, live profiles, swells and Stop; AddressSanitizer passed the DSP and both new UI scenarios.
- Eight pack/tool tests, including v1/v2 roundtrips, invalid data, MIDI selection, DX-bank import and the older-engine upgrade message.
- Two C/Python transport integration cases cover factory switching, v1 audition/rollback, queued updates, stopped save, complete readback, source/profile/session restoration after reboot and the temporary USB recording flag.
- Serial/deferred/threaded/mixed outputs agree: v1 `2151ff10`, v2 `2ad545b4`. The maximum-tempo clock test retains 24 PPQN and every musical sixteenth with irregular render sizes.
- Browser/WASM boot, sound and all four detail pages passed; peak 0.231, 239 blits. Backup, OTA handshake and idle-port tests passed with Node 22.
- Full-song WAV/MIDI previews include all three 32-bar forms, Velvet Changes at 64 bars and a dry comparison. No clipped samples or stuck keys/bass/swell MIDI notes were found. Playback controls start paused.

The broad legacy suite is not entirely green: its three existing musical-distribution failures and 19 scenario failures remain; all 19 scenario failure sets match the untouched baseline. The initial Linux run also used Node 12, which cannot run several newer JS tests; the relevant web tests were rerun successfully with Windows Node 22. Actual package installation and post-install identity were checked on the device. Structural/audio/performance checks do not substitute for the player's musical judgment.

## Build and recovery

Package: `build/naiwan91-0.3.0.fwsc`, 611,251 bytes. SHA-256:

`c4f6c00b4a5558101333893f7702d5ce7dd98c297b3391a3f847e7a1b962f61e`

Firmware image: 482,440 bytes. Regular data/BSS: 97,196 / 98,304 bytes. Audio pool: 339,404 / 344,064 bytes, retaining the required 4 KiB reserve. Both hardware and WASM release builds passed.

Before-upgrade stored-object backup: `build/hardware/songbook-before/before-songbook.p1bk`; active patch and parameter snapshots are alongside the hardware evidence. The 0.2.0 firmware package and original slot 1 remain available for rollback. Generic legacy effects edits stay live unless authored into a pack's `sounds`; profile data requires an explicit stopped pack save. See [Songbook and Jam Packs](JAM_PACKS.md) for the panel and update workflow.
