#!/usr/bin/env python3
"""Compile written chord symbols into faithful, transposable JMP3 charts.

This imports harmony, not a melody or an audio/MIDI transcription. Each bar is
4/4 and contains one whole-bar chord or two equal half-bar chords.
"""
import argparse
import copy
import json
import pathlib
import re

QUALITY_NAMES = ('maj7', 'm7', '7', 'm7b5', 'dim7', '6', 'm6', '7sus4',
                 'mMaj7', '+7', '7b9', 'maj9', 'm9', '9', '7alt', '', 'm')
QUALITY_INTERVALS = ((0,4,7,11), (0,3,7,10), (0,4,7,10), (0,3,6,10),
                     (0,3,6,9), (0,4,7,9), (0,3,7,9), (0,5,7,10),
                     (0,3,7,11), (0,4,8,10), (4,7,10,13), (4,7,11,14),
                     (3,7,10,14), (4,7,10,14), (4,8,10,13), (0,4,7,12),
                     (0,3,7,12))
_ALIASES = {
    'maj7':0, 'M7':0, 'Maj7':0, 'major7':0, '\u03947':0, '\u0394':0,
    'm7':1, 'min7':1, '-7':1,
    '7':2, 'dom7':2,
    'm7b5':3, 'min7b5':3, '-7b5':3, '\u00f87':3, '\u00f8':3,
    'dim7':4, 'o7':4, '\u00b07':4,
    '6':5, 'maj6':5, 'M6':5,
    'm6':6, 'min6':6, '-6':6,
    '7sus4':7, 'sus7':7, 'sus4(7)':7,
    'mMaj7':8, 'mM7':8, 'minMaj7':8, 'm(maj7)':8, '-M7':8,
    '+7':9, 'aug7':9, '7#5':9,
    '7b9':10, '7(b9)':10,
    'maj9':11, 'M9':11, 'Maj9':11, '\u03949':11,
    'm9':12, 'min9':12, '-9':12,
    '9':13, 'dom9':13,
    '7alt':14, 'alt7':14,
    '':15, 'maj':15, 'M':15,
    'm':16, 'min':16, '-':16,
}
_PITCHES = {'C':0, 'D':2, 'E':4, 'F':5, 'G':7, 'A':9, 'B':11}


def pitch_class(key):
    """Parse a tonic name or an integer pitch class; do not infer a mode."""
    if type(key) is int and 0 <= key <= 11:
        return key
    if not isinstance(key, str):
        raise ValueError('key must be a note name or integer 0..11')
    key = key.strip().replace('\u266d', 'b').replace('\u266f', '#')
    match = re.fullmatch(r'([A-Ga-g])([b#]?)', key)
    if not match:
        raise ValueError('key must be a pitch name such as C, F# or Bb')
    note, accidental = match.groups()
    return (_PITCHES[note.upper()] + (1 if accidental == '#' else -1 if accidental == 'b' else 0)) % 12


def parse_chord(symbol):
    """Return (absolute root pitch class, exact quality ID)."""
    if not isinstance(symbol, str):
        raise ValueError('each chord must be a symbol string')
    symbol = symbol.strip().replace('\u266d', 'b').replace('\u266f', '#')
    if '/' in symbol:
        raise ValueError('slash bass is not supported; write the chord and arrange its bass separately')
    match = re.fullmatch(r'([A-Ga-g][b#]?)([^\s]*)', symbol)
    if not match or match.group(2) not in _ALIASES:
        raise ValueError('unsupported chord symbol: '+repr(symbol))
    return pitch_class(match.group(1)), _ALIASES[match.group(2)]


def parse_bars(bars):
    """Accept pipe-separated text, one bar per line, or a list of bars."""
    if isinstance(bars, str):
        text = bars.strip()
        if '|' in text:
            bars = text.split('|')
            if not bars[0].strip(): bars.pop(0)
            if bars and not bars[-1].strip(): bars.pop()
        else:
            bars = text.splitlines()
    if not isinstance(bars, (list, tuple)) or len(bars) not in (8,16,32):
        raise ValueError('a chart must contain exactly 8, 16 or 32 bars')
    result = []
    for index, bar in enumerate(bars):
        symbols = bar.split() if isinstance(bar, str) else bar
        if not isinstance(symbols, (list, tuple)) or len(symbols) not in (1,2):
            raise ValueError('bar %d needs one or two chord symbols' % (index+1))
        parsed = [parse_chord(symbol) for symbol in symbols]
        result.append(parsed*2 if len(parsed)==1 else parsed)
    return result


def from_chords(base, bars, key=0, name=None, voicing=1, sources=None):
    """Return a complete v3 pack using the base's sounds and accompaniment.

    `key` is the original written tonic. Set the FM-1 session key to it for
    original pitch, or another tonic to transpose. A 32-bar chart uses Authored
    A/B with 16-bar sections. 8/16-bar charts use a single looping section of
    that length. The 8-bar view of a 16-bar section is its opening eight bars.
    `sources`, when supplied, replaces chart provenance while retaining any
    explicit base voice-provenance string containing "DX" or "voice".
    """
    from fm1_jam import compile_pack, decompile, load, integer
    if isinstance(base, (str, pathlib.Path)):
        base = decompile(load(base))
    if not isinstance(base, dict) or base.get('version') not in (2,3):
        raise ValueError('base must be a Songbook v2/v3 pack with voice programs')
    tonic = pitch_class(key)
    parsed = parse_bars(bars)
    integer(voicing, 0, 1, 'voicing')
    d = copy.deepcopy(base)
    d['version'] = 3
    d['name'] = name or 'Chord Replay'
    d.pop('transitions', None)
    d['nodes'] = []
    node_map = {}
    chart = []
    # Legacy labels are only a display fallback; quality is authoritative.
    labels = (6,1,3,2,5,0,4,3,4,7,7,6,1,3,7,0,4)
    for bar in parsed:
        for root, quality in bar:
            item = ((root-tonic)%12, quality)
            if item not in node_map:
                if len(node_map) == 32:
                    raise ValueError('chart exceeds 32 distinct root/quality pairs')
                node_map[item] = len(d['nodes'])
                d['nodes'].append(dict(bass=item[0], sixth=item[0], scale=0,
                                       label=labels[quality], mode=0, emphasis=0,
                                       borrow=0, quality=quality))
            chart.append(node_map[item])
    # V3 follows the chart even at cadences. These fields remain valid for the
    # common layout but never cause an inserted ii-V or an ending rewrite.
    d['cadence'] = [[0,0,0], [0,0,0]]
    a = chart[:32]
    if len(parsed)==8: a = chart*2
    b = chart[32:] if len(parsed)==32 else list(a)
    versions = [[a[:16], b[:16]], [a, b]]
    d['charts'] = [copy.deepcopy(versions), copy.deepcopy(versions)]
    d['forms'] = [dict(name='Chart', roles=[0,1] if len(parsed)==32 else [0]),
                  dict(name='Vamp', roles=[0]),
                  dict(name='AABA', roles=[0,0,2,3]),
                  dict(name='ABAB', roles=[0,1,0,1])]
    d['parts']['voicing'] = voicing
    d['parts']['variation'] = 0
    d['song_id'] = 0
    if sources is not None:
        if not isinstance(sources, list) or any(not isinstance(s,str) for s in sources):
            raise ValueError('sources must be a list of provenance strings')
        retained = [s for s in d.get('sources',[]) if isinstance(s,str) and ('DX' in s or 'voice' in s.lower())]
        d['sources'] = list(sources) + [s for s in retained if s not in sources]
    d.setdefault('sources',[]).append('Written chord replay: %d bars; source tonic pitch class %d; one or two changes per 4/4 bar.' % (len(parsed), tonic))
    compile_pack(d)
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('input', help='text chart or JSON {bars, key, name, sources}')
    ap.add_argument('base', help='sound/groove template JSON or JMP')
    ap.add_argument('output', help='output editable JSON pack')
    ap.add_argument('--key', help='written tonic, e.g. C or Bb')
    ap.add_argument('--name')
    ap.add_argument('--voicing', choices=['close','drop2'], default='drop2')
    a = ap.parse_args()
    path = pathlib.Path(a.input)
    raw = path.read_text(encoding='utf-8-sig')
    source = json.loads(raw) if path.suffix.lower()=='.json' else dict(bars=raw)
    if isinstance(source,list): source = dict(bars=source)
    if not isinstance(source,dict) or 'bars' not in source:
        raise ValueError('chart JSON needs a bars list')
    key = a.key if a.key is not None else source.get('key',0)
    d = from_chords(a.base, source['bars'], key=key,
                    name=a.name or source.get('name'),
                    voicing=0 if a.voicing=='close' else 1,
                    sources=source.get('sources', ['User-written chord chart: '+path.name]))
    from fm1_jam import write_json, compile_pack
    write_json(a.output,d)
    print(json.dumps(dict(output=a.output, bytes=len(compile_pack(d)),
                          bars=len(parse_bars(source['bars'])), key=pitch_class(key),
                          section_bars=8 if len(parse_bars(source['bars']))==8 else 16,
                          form=0, movement=0)))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        raise SystemExit(str(error))
