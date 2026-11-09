# The chord movements, and how they play in NAIWAN 91

Three loops carry the CITYPOP pieces. They are not transcriptions: they are the harmonic moves those records are
built on, in the engine's own voicings (open, drop-2, 9ths and 11ths), so they can be played over and remixed live.

| # | movement | where it comes from |
|---|---|---|
| 0 | **IV – V – iii – vi** | 真夜中のドア / Stay With Me (Miki Matsubara); the same turn is the chorus lift in Ride On Time (Tatsuro Yamashita) |
| 1 | **i – ♭VII – ♭VI – V** | the descending minor loop of Anri's 悲しみがとまらない |
| 2 | **I – iii – IV – V** | the summer lift of Anri's Remember Summer Days |

Each chord holds **two bars**; pieces 01–10 cycle the loops (01 uses 0, 02 uses 1, …) so a set walks through all
three without you touching anything.

## How the firmware stores it

A progression is 48 bits, posted as one command (`en_prog`, `firmware/src/dsp/amb.c`):

```
bits  0..23   8 slots x 3 bits: the chord degree + 1 (0 = empty)
bits 24..39   8 slots x 2 bits: bars a chord as log2 (1 = two bars, 2 = four)
bit  40       Play
bits 41..42   the clock's unit (0 = bars)
```

Degrees are 0-based in the mode being played (0 = I, 3 = IV, 4 = V …), so the same loop re-harmonises when the
piece changes key or mode. Chords are voiced by the engine from the chord kind (`P_CHORD`, upgraded a step per genre
to 9ths and 11ths) and `P_VOICING` (open = drop 2).

## Playing with it live

- **PRESETS** (turn) picks another piece: the loop changes with it (0 → 1 → 2 → 0 …).
- **GLO + PRESETS** changes the genre; CITYPOP keeps this progression system, the other eighteen run their own walk.
- Hold **EDIT** for the Progression page: the eight slots are the degrees, the black keys fill a run, and the white
  keys 9/10 put the bass and the pad on a **pedal** so the chords move over a held note — the easiest way to
  reharmonise a loop on the fly.
- **SAVE** is the breakdown, again for the build, and the drop: the progression keeps its place through all of it.
- Scenes store the FX and mix, so a loop can be recalled with its reverb and filter settings intact.

## Reharmonising by hand

The Progression page's slots are stored with the piece. Set a slot to any degree 1–7 (a chord per slot), or leave
slots empty to stop the loop short; the chord kind of each slot can be its own (`en_prog_kinds`) instead of the
piece's. The three loops above are the defaults for CITYPOP pieces 01–10.

## The sound at boot

As the shine crosses the 内湾 wordmark the firmware plays a **turnaround**: two diminished-7th chords a semitone
apart, drop-2 voicing (the second voice from the top dropped an octave), on the pluck's bell — the chord the piece is
in, then the same shape a semitone above leaning back into it. It is fired from `draw_splash()` with `en_note_v`,
so it is the same voice the piece plays, not a separate chime.

## Bass, by genre

- **CITYPOP**: eighths on the chord roots, fifths and octaves (`P_BL_MODE` 3, short notes, little movement) — a pop
  bass, not a breakbeat riff; the acid sits far back so nothing wanders outside the key.
- **NEON**: off-beat, short, octave lifts, and the harmony back to two bars a chord — it pushes instead of dragging.
- Every genre's modes are limited to major, dorian, mixolydian and aeolian, so no piece lands in phrygian or lydian
  under a IV–V–iii–vi loop.
