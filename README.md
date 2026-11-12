# NAIWAN 91

**A city-pop band to play along with, in your browser or on the M-VAVE FM-1.**

Choose **Velvet Changes**, **Afterhours** or **Neon Turnaround**, find your key, and play along. Shape the keys, bass and swells as the song unfolds. Hold a phrase you like, ask for a fresh idea, or change the feel while you keep playing.

<p align="center"><img src="docs/img/naiwan91-performance.png" width="320" alt="Velvet Changes on the Songbook performance screen"></p>

*Velvet Changes on the performance screen.*

[Play in your browser](https://superposition.github.io/91/emu/) · [Songbook guide](docs/JAM_PACKS.md) · [Controls and manual](docs/MANUAL.md)

Press **Switch on** to start the browser player. Use your mouse, touch screen or keyboard; an FM-1 is optional. Updates on this branch may not be on the published site yet.

## Find your sound

- **Choose a song.** Turn PRESETS to choose between the three arrangements, then set a comfortable key and tempo.
- **Shape the band.** Give keys and bass their own sounds, adjust their touch and release, and bring swells in around your playing.
- **Stay with a phrase.** REC holds the current phrase; EDIT asks for a related idea. The F3 and G3 piano keys do the same.
- **Change the feel.** Try shorter or longer sections, add movement, or turn ALGORITHM to change the energy.
- **Explore further.** The F4 piano key opens generative pieces, sequencers, effects and visualizers. The [manual](docs/MANUAL.md) explains their controls.

## Shape the song

<p align="center"><img src="docs/img/naiwan91-arrangement.png" width="240" alt="Arrangement: form, section bars, variation and swell amount"> <img src="docs/img/naiwan91-dx-keys.png" width="240" alt="DX Keys: program, touch, brightness and release"></p>
<p align="center"><img src="docs/img/naiwan91-dx-bass.png" width="240" alt="DX Bass: program, attack, gate and energy"> <img src="docs/img/naiwan91-swells.png" width="240" alt="Swells: texture, amount, variation and tail"></p>

SELECT opens Arrangement, DX Keys, DX Bass and Swells. Each page has four matching controls, from top to bottom. HOME returns to the Songbook performance page.

## Make it yours

Keep your songs and sounds in packs. On an FM-1, save up to four packs and come back to them without a computer. Stop playback before saving your session settings. The [Songbook guide](docs/JAM_PACKS.md) covers creating packs, importing sounds and saving your changes.

If you have an FM-1, the [installer](web/amb_installer.html) includes backup and recovery tools, and the [editor](web/editor/) lets you adjust its sounds from your computer. Read the [install instructions](BUILDING.md) before updating your instrument.

## Development

For building and contributing, see the [build instructions](BUILDING.md), [contributor guide](CONTRIBUTING.md) and [development notes](docs/RUST_MIGRATION.md). Technical measurements and screenshot provenance are kept with the [project documentation](docs/WEB_REFACTOR.md).

## Credits and licence

NAIWAN 91 is a fork of **PERF-1** v0.17.0 by deamonSLINKY (GPL-3.0), the lineage and licence staying with it: PERF-1
is a fork of **HYP-1** by [RedtailWorks](https://www.redtail.works), itself a fork of **AMB-1** by Charles Vestal,
built on the platform layer of **Felucca** by Leo Kuroshita (Hügelton Instruments) through X0X and FoMni. The
six-operator FM engine that plays the DX7 voices, its factory voices and the second-core work are from **Melodee** by
Kerem Kilic (Ellic Studio); its synthesis is after **Dexed** by Pascal Gauthier, on MSFA by Raph Levien (Google).
Several generator ideas (bass line modes, pad comping, ghost notes, the pluck's deja vu loop) follow **Tuna** by
delirioux ([github.com/delirioux/tuna](https://github.com/delirioux/tuna), GPL-3.0), written afresh for this engine.
The reverb is Jon Dattorro's plate ("Effect Design, Part 1", JAES 1997). The git history keeps every author's
commits. The idea of a browser emulator for FM-1 firmware comes from Groove OS by Peter Gombos (no code taken). The
community reverse-engineering of the chip, [AL-255/FM-1-RE](https://github.com/AL-255/FM-1-RE), informed the hardware
work, and the rescue route uses FM-1-transporter's protocol and kagaimiq's jl-uboot-tool.

GPL-3.0-only (`LICENSE`); the code's origins and the third-party material (fonts, the JieLi SDK) are in
`LICENSING.md`. "M-VAVE" and "FM-1" are trademarks of their owners; NAIWAN 91 is independent firmware, not affiliated
with or endorsed by them.
