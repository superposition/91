#!/usr/bin/env python3
"""Literal chart import/codec regressions; no MIDI hardware required."""
import copy
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import fm1_jam as pack
import songbook
from chord_chart import from_chords, parse_chord, parse_bars, pitch_class, QUALITY_NAMES
from jam_midi import Device


class ChordCharts(unittest.TestCase):
    def setUp(self):
        self.base = songbook.factory()
        self.bars = ['Dm7','G7','Cmaj7','Cmaj7','Dm7','G7b9','C6','C6']

    def test_symbol_aliases_and_all_qualities(self):
        for quality,suffix in enumerate(QUALITY_NAMES):
            self.assertEqual(parse_chord('C'+suffix),(0,quality))
        for text,want in [('BbM7',(10,0)), ('F#min7',(6,1)), ('Eb-7',(3,1)),
                          ('B\u00f87',(11,3)), ('C\u00b07',(0,4)), ('DbmMaj7',(1,8)),
                          ('G7(b9)',(7,10)), ('F\u266fmaj9',(6,11)), ('E\u266dm9',(3,12))]:
            self.assertEqual(parse_chord(text),want)
        for text in ('C/E','C13','Cadd9','C7sus2','N.C.','H7','C major seventh','Cm/maj7'):
            with self.assertRaises(ValueError,msg=text): parse_chord(text)
        self.assertEqual(pitch_class('Cb'),11)
        for key in (12,-1,True,'C minor'):
            with self.assertRaises(ValueError):pitch_class(key)

    def test_replay_roundtrip_shares_mode_nodes(self):
        d = from_chords(self.base,self.bars,key='C',name='Written Changes')
        self.assertEqual(len(d['nodes']),5)
        self.assertEqual(d['charts'][0],d['charts'][1])
        self.assertTrue(all(n['mode']==0 for n in d['nodes']))
        self.assertEqual(d['parts']['voicing'],1)
        self.assertEqual(d['parts']['variation'],0)
        self.assertEqual(d['forms'][0]['roles'],[0])
        blob = pack.compile_pack(d)
        self.assertEqual(blob[:5],b'JMP3\3')
        self.assertEqual(blob[songbook.H:songbook.H+4],b'SNG3')
        self.assertEqual(pack.compile_pack(pack.decompile(blob)),blob)
        self.assertLessEqual(len(blob),3840)
        self.assertEqual(self.base['version'],2)  # caller's template is untouched

    def test_exact_bars_halves_transposition_and_second_section(self):
        bars = ['Dm7 G7','Cmaj7','Am7 D7','Gmaj7']*8
        d = from_chords(self.base,bars,key='G',voicing=0)
        self.assertEqual(d['forms'][0]['roles'],[0,1])
        actual = d['charts'][0][1][0]+d['charts'][0][1][1]
        expected = [value for bar in parse_bars(bars) for value in bar]
        for index,(root,quality) in zip(actual,expected):
            self.assertEqual((d['nodes'][index]['bass'],d['nodes'][index]['quality']),((root-7)%12,quality))
        self.assertEqual(d['charts'][0][0][0],actual[:16])
        self.assertEqual(d['charts'][0][0][1],actual[32:48])
        self.assertEqual(parse_bars('| '+' | '.join(self.bars)+' |'),parse_bars(self.bars))
        self.assertEqual(parse_bars('\n'.join(self.bars)),parse_bars(self.bars))

    def test_invalid_chart_does_not_approximate(self):
        for bars in (self.bars[:7], self.bars+['C7'], ['C D E']+self.bars[1:],
                     ['']+self.bars[1:], ['C/E']+self.bars[1:]):
            with self.assertRaises(ValueError):from_chords(self.base,bars)
        # 64 half bars can exceed the hardware's bounded 32-node dictionary.
        roots = ['C','Db','D','Eb','E','F','Gb','G','Ab','A','Bb','B']
        changes = [root+quality for quality in ('maj7','m7','7') for root in roots]
        changes += ['Cmaj7']*(64-len(changes))
        with self.assertRaisesRegex(ValueError,'32 distinct'):
            from_chords(self.base,[changes[i:i+2] for i in range(0,64,2)])

    def test_codec_rejects_bad_version_quality_voicing_and_header(self):
        d = from_chords(self.base,self.bars)
        for update in (lambda x:x.update(version=4),
                       lambda x:x['nodes'][0].update(quality=17),
                       lambda x:x['nodes'][0].pop('quality'),
                       lambda x:x['parts'].update(voicing=2),
                       lambda x:x.update(cadence=[[0,0,0]])):
            bad=copy.deepcopy(d);update(bad)
            with self.assertRaises((ValueError,KeyError)):pack.compile_pack(bad)
        blob=pack.compile_pack(d)
        for offset,value in ((3,ord('2')),(4,4),(songbook.H+3,ord('2')),
                             (64+7,17),(songbook.H+21,2),(songbook.CHART+31,32)):
            bad=bytearray(blob);bad[offset]=value
            with self.assertRaises(ValueError,msg=str(offset)):pack.decompile(bytes(bad))

    def test_features_report_v3_without_hardware(self):
        with patch.object(Device,'request',return_value=bytes([7,8,3,6])):
            self.assertEqual(Device.features(object.__new__(Device))['pack_versions'],[1,2,3])
        with patch.object(Device,'request',return_value=bytes([3,8,3,6])):
            self.assertEqual(Device.features(object.__new__(Device))['pack_versions'],[1,2])

    def test_cli_json_and_text_inputs(self):
        with tempfile.TemporaryDirectory() as td:
            td=pathlib.Path(td)
            base=td/'base.json';base.write_text(json.dumps(self.base),encoding='utf-8')
            for filename,content in [('chart.txt',' | '.join(self.bars)),
                                     ('chart.json',json.dumps(dict(name='My Chart',key='Bb',bars=self.bars,sources=['User chart'])) )]:
                path=td/filename;path.write_text(content,encoding='utf-8')
                output=td/'output.json'
                result=subprocess.run([sys.executable,str(ROOT/'tools/chord_chart.py'),str(path),str(base),str(output)],
                                      capture_output=True,text=True,check=True)
                report=json.loads(result.stdout)
                self.assertEqual(report['section_bars'],8)
                self.assertEqual(pack.decompile(pack.load(output))['version'],3)


if __name__=='__main__':unittest.main()
