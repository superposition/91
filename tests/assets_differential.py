#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Generated assets against immutable R01. No app, MIDI or USB is loaded."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import sys
import tempfile
import types
import PIL

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(sys.argv[1]).resolve()
CLI = str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI, FM1_CODEC_CLI=CLI)
for line in (ROOT / 'tests/reference/source.sha256').read_text().splitlines():
    sha, name = line.split(maxsplit=1)
    assert hashlib.sha256((BASE / name.strip()).read_bytes()).hexdigest() == sha, name
sys.path.insert(0, str(ROOT / 'tools'))
import rust_tooling


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


old = load('frozen_font', BASE / 'tools/gen_font.py')
current = load('maintained_font', ROOT / 'tools/gen_font.py')
old.FONTS = current.FONTS  # Same immutable-input directory, same path errors.
cases, records = [], []
paths = [(str(ROOT), '<ROOT>'), (str(BASE), '<R01>')]


def serial(value):
    return value.hex() if isinstance(value, bytes) else str(value)


def stable(value):
    if isinstance(value, (list, tuple)):
        return [stable(item) for item in value]
    if isinstance(value, dict):
        return {key: stable(item) for key, item in value.items()}
    if isinstance(value, Path):
        value = str(value)
    if isinstance(value, str):
        for path, label in paths:
            value = value.replace(path.replace('\\', '\\\\'), label).replace(path, label)
    return value


def capture(fn):
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        try:
            result = ['ok', fn()]
        except BaseException as error:
            result = ['error', type(error).__name__, str(error)]
    return [result, output.getvalue()]


def check(name, want, got):
    if want != got:
        # Preserve complete evidence without overwhelming the command log.
        dest = ROOT / 'build/assets/failure.json'
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(dict(name=name, want=want, got=got), default=serial), encoding='utf-8')
        raise AssertionError((name, str(want)[:300], str(got)[:300], str(dest)))
    cases.append(name)
    records.append([name, hashlib.sha256(json.dumps(stable(want), ensure_ascii=True, default=serial).encode()).hexdigest()])


check('fontsets-data', old.FONTSETS, current.FONTSETS)
check('default-fontset', old.FONTSET, current.FONTSET)
check('sizes', old.SIZES, current.SIZES)
check('fontdirs', old.FONTDIRS, current.FONTDIRS)
check('pad', old.PAD, current.PAD)
check('missing-path', capture(lambda: old.font_path('absent.ttf')),
      capture(lambda: current.font_path('absent.ttf')))
for module in (old, current):
    values = [1, 2, 3]
    assert module.upscale(values, 999, -9, 1) is values
cases.append('upscale-scale-one-identity')
rng = random.Random(0xF017)
for i in range(100):
    w, h, scale = rng.randrange(0, 13), rng.randrange(0, 12), rng.randrange(0, 5)
    pixels = [rng.randrange(16) for _ in range(w*h)]
    check('upscale-%d' % i, capture(lambda: old.upscale(pixels, w, h, scale)),
          capture(lambda: current.upscale(pixels, w, h, scale)))
for w, h, scale, pixels in [(2, 2, 2, [1]), (0, 0, 3, []), (1, 1, 0, [15])]:
    check('upscale-edge-%s' % str((w, h, scale)), capture(lambda: old.upscale(pixels, w, h, scale)),
          capture(lambda: current.upscale(pixels, w, h, scale)))


def fixture(cw=5, ch=5, codes=(63, 65, 85), bx=-1, by=-1, dw=4):
    rows = ['STARTFONT 2.1', 'FONTBOUNDINGBOX %d %d 0 -1' % (cw, ch)]
    for code in codes:
        rows += ['STARTCHAR test', 'ENCODING %d' % code, 'DWIDTH %d 0' % dw,
                 'BBX 7 5 %d %d' % (bx, by), 'BITMAP', 'FE', '82', 'AA', '82', '7C', 'ENDCHAR']
    return '\n'.join(rows + ['ENDFONT'])


with tempfile.TemporaryDirectory(prefix='fm1-assets-') as directory:
    tmp = Path(directory)
    paths.insert(0,(str(tmp),'<TMP>'))
    # Exact scripts run in scratch trees because gen_pats writes at import.
    pattern_modules = []
    for label, root in [('old', BASE), ('current', ROOT)]:
        tree = tmp / label
        (tree / 'tools/data').mkdir(parents=True)
        (tree / 'firmware/src/dsp').mkdir(parents=True)
        script = tree / 'tools/gen_pats.py'
        shutil.copyfile(root / 'tools/gen_pats.py', script)
        if label == 'current':
            shutil.copyfile(ROOT / 'tools/data/patterns.json', tree / 'tools/data/patterns.json')
        with contextlib.redirect_stdout(io.StringIO()) as printed:
            module = load(label + '_patterns', script)
        pattern_modules.append(module)
        assert printed.getvalue() == 'wrote ' + str(tree / 'firmware/src/dsp/pats.h') + '\n'
    a, b = pattern_modules
    check('pattern-data', a.PIECES, b.PIECES)
    check('pattern-lanes', a.LANES, b.LANES)
    check('pattern-header', Path(a.path).read_bytes(), Path(b.path).read_bytes())
    # Store JSON-safe fingerprint separately for raw generated bytes.
    for i in range(400):
        grid = ''.join(rng.choice('x.') for _ in range(16))
        check('grid-%d' % i, capture(lambda: a.bits(grid)), capture(lambda: b.bits(grid)))
    for grid in ['', '.', 'x'*15, 'x'*17, '?'*16, 'x'*15+'é']:
        check('bad-grid-'+grid, capture(lambda: a.bits(grid)), capture(lambda: b.bits(grid)))
    patterns = subprocess.check_output([CLI, 'tools', 'patterns'])
    check('native-pattern-header', Path(a.path).read_bytes(), patterns)
    for i, (cw, ch, bx, by, dw, codes, scale, first, last) in enumerate([
        (5, 5, -1, -1, 4, (63, 65, 85), 1, 63, 66),
        (5, 5, 1, 1, 7, (63, 65, 85), 3, 63, 85),
        (1, 5, 0, -1, 1, (63, 85), 2, 220, 220),
        (5, 5, -2, -2, 4, (63, 85), 1, 220, 220),
        (5, 5, -1, -1, 4, (65,), 1, 65, 65),
        (5, 5, -1, -1, 4, (63, 85, 85), 0, 63, 85),
        (5, 5, -1, -1, -2, (63, 65), 2, 63, 65),
        (5, 5, -1, -1, 4, (65,), 1, 65, 64),
        (5, 5, -1, -1, 4, (63, 65), 1, 0, 126),
    ]):
        path = tmp / 'edge.bdf'
        path.write_text(fixture(cw, ch, codes, bx, by, dw), encoding='utf-8')
        check('bdf-edge-%d' % i, capture(lambda: old.render_bdf(path, scale, first, last)),
              capture(lambda: current.render_bdf(path, scale, first, last)))
    path.write_text('STARTFONT\nENDFONT', encoding='utf-8')
    check('bdf-no-bounds', capture(lambda: old.render_bdf(path, 1, 63, 63)),
          capture(lambda: current.render_bdf(path, 1, 63, 63)))
    for row in ['-1', '+0xFE', '0x_F_E', '0'*512+'FE', 'g00', 'FE_', '', '--1']:
        path.write_text(fixture(codes=(63,65)).replace('\nFE\n','\n'+row+'\n'), encoding='utf-8')
        check('bdf-bitmap-'+row, capture(lambda: old.render_bdf(path,1,63,65)),
              capture(lambda: current.render_bdf(path,1,63,65)))
    for source in [fixture().replace('BBX 7 5 -1 -1','BBX 7 5'),
                   fixture().replace('DWIDTH 4 0','DWIDTH invalid 0'),
                   fixture().replace('BBX 7 5 -1 -1','BBX 7 5 -1 -1 ignored'),
                   fixture(codes=(63,))[:-12]]:
        path.write_text(source,encoding='utf-8')
        check('bdf-malformed-'+str(len(cases)),capture(lambda: old.render_bdf(path,1,63,65)),
              capture(lambda: current.render_bdf(path,1,63,65)))
    for i in range(75):
        path.write_text(fixture(rng.randrange(1, 12), rng.randrange(2, 12), (63,65),
                                rng.randrange(-7,7), rng.randrange(-7,7), rng.randrange(-2,12)), encoding='utf-8')
        scale = rng.randrange(1, 4)
        check('bdf-random-%d' % i, capture(lambda: old.render_bdf(path, scale, 63, 66)),
              capture(lambda: current.render_bdf(path, scale, 63, 66)))
    for name, sizes in old.FONTSETS.items():
        if not all((old.FONTS / row[1].partition('@')[0]).exists() for row in sizes):
            continue
        for module in (old, current):
            module.FONTSET, module.SIZES = name, sizes
        path = tmp / 'font.h'
        expected = capture(lambda: old.main(path))
        expected_bytes = path.read_bytes()
        actual = capture(lambda: current.main(path))
        check('fontset-'+name, expected, actual)
        check('fontset-bytes-'+name, expected_bytes, path.read_bytes())
    # Controlled Pillow boundary: all levels, half-integer advances, byte/string
    # axis names, default/zero/empty axes, every recipe including absent fonts.
    trace = []
    fault = [None]
    large = [False]
    opened = [0]
    class FakeFont:
        def get_variation_axes(self):
            trace.append(['axes'])
            return [dict(name=b'Weight', default=400), dict(name='Optical size', default=14)]
        def set_variation_by_axes(self, axes):
            trace.append(['set_axes', axes])
        def getmetrics(self):
            trace.append(['metrics'])
            return (1000,3) if large[0] else (8,3)
        def getbbox(self, text, features=None):
            trace.append(['bbox', text, features])
            return 0, 3, 8, 10
        def getlength(self, char, features=None):
            trace.append(['length', char, features])
            if ord(char) == fault[0] and opened[0] >= 2:
                raise OSError('rasterizer deliberately unavailable')
            return (ord(char) % 6) + 0.5
    def truetype(path, px):
        trace.append(['open', path, px])
        opened[0] += 1
        return FakeFont()
    class FakeImage:
        def __init__(self, mode, size, fill):
            self.width, self.height = size
            self.code = 0
            trace.append(['image', mode, size, fill])
        def crop(self, box):
            trace.append(['crop', box])
            self.height = box[3] - box[1]
            return self
        def tobytes(self):
            return bytes((i * 19 + self.code * 7) % 256 for i in range(self.width*self.height))
    class Draw:
        def __init__(self, image): self.image = image
        def text(self, pos, char, font=None, fill=None, features=None):
            trace.append(['draw', pos, char, fill, features])
            self.image.code = ord(char)
    saved = [(m.Image, m.ImageDraw, m.ImageFont, m.FONTS, m.FONTDIRS) for m in (old, current)]
    for sizes in old.FONTSETS.values():
        for row in sizes:
            (tmp / row[1].partition('@')[0]).touch()
    for module in (old, current):
        module.Image = types.SimpleNamespace(new=FakeImage)
        module.ImageDraw = types.SimpleNamespace(Draw=Draw)
        module.ImageFont = types.SimpleNamespace(truetype=truetype)
        module.FONTS, module.FONTDIRS = tmp, []
    def recorded(fn):
        trace.clear()
        opened[0] = 0
        value = capture(fn)
        return value, trace[:]
    for name, sizes in old.FONTSETS.items():
        if name == 'terminus': continue  # Its real BDF source was tested above.
        for module in (old, current):
            module.FONTSET, module.SIZES = name, sizes
        path = tmp / 'fake-font.h'
        expected = recorded(lambda: old.main(path))
        expected_bytes = path.read_bytes()
        check('fake-fontset-'+name, expected, recorded(lambda: current.main(path)))
        check('fake-fontset-bytes-'+name, expected_bytes, path.read_bytes())
    for variant in ['', '@wght=0', '@wght=', '@Weight=550,opti=22', '@wght=100,wght=650']:
        for pixel in (False, True):
            args = ('Inter[opsz,wght].ttf'+variant, 15, 3, pixel, 32, 80)
            check('fake-render-'+variant+str(pixel), recorded(lambda: old.render(*args)),
                  recorded(lambda: current.render(*args)))
    for variant in ['@bad','@wght=1=2','@wght=bad']:
        args=('Inter[opsz,wght].ttf'+variant,15,1,False,32,33)
        check('bad-variation-'+variant,recorded(lambda: old.render(*args)),recorded(lambda: current.render(*args)))
    for first, last in [(100,99), (60,79)]:
        for module in (old, current):
            module.FONTSET, module.SIZES = 'range', [('X','ChakraPetch-Bold.ttf',10,1,False,first,last)]
        path = tmp / 'range.h'
        check('font-range-%d-%d' % (first,last), recorded(lambda: old.main(path)),
              recorded(lambda: current.main(path)))
    fault[0] = 50
    for module in (old, current):
        module.FONTSET, module.SIZES = 'failing', old.FONTSETS['barlow'][:2]
    path = tmp / 'never-written.h'
    check('raster-failure-order', recorded(lambda: old.main(path)), recorded(lambda: current.main(path)))
    assert not path.exists()
    fault[0] = None
    large[0] = True
    for module in (old,current):
        module.FONTSET,module.SIZES='oversized',old.FONTSETS['barlow'][:1]
    check('font-offset-overflow-assert',recorded(lambda:old.main(path)),recorded(lambda:current.main(path)))
    assert not path.exists()
    large[0] = False
    for module, state in zip((old, current), saved):
        module.Image, module.ImageDraw, module.ImageFont, module.FONTS, module.FONTDIRS = state
        module.FONTSET, module.SIZES = 'chakra', module.FONTSETS['chakra']
    # A write failure follows all printed completion rows, just as the old tool.
    missing = tmp / 'nonexistent' / 'font.h'
    check('write-failure-prefix', capture(lambda: old.main(missing)), capture(lambda: current.main(missing)))
    # The old standalone C is compiled independently, with contraction disabled.
    cc = os.environ.get('CC', 'gcc')
    executable = tmp / ('sine.exe' if os.name == 'nt' else 'sine')
    subprocess.run([cc, '-O0', '-ffp-contract=off', '-I', str(BASE / 'firmware/src/dsp'),
                    str(BASE / 'tools/gen_sint.c'), '-o', str(executable)], check=True)
    expected_sine = subprocess.check_output([str(executable)])
    check('native-sine-header', expected_sine, subprocess.check_output([CLI, 'tools', 'sine']))
    check('python-sine-header', expected_sine,
          subprocess.check_output([sys.executable, str(ROOT / 'tools/gen_sint.py')]))
    subprocess.run([cc, str(ROOT / 'tools/gen_sint.c'), '-o', str(executable)], check=True)
    env = dict(os.environ, FM1_PYTHON=sys.executable)
    check('legacy-c-launcher', expected_sine, subprocess.check_output([str(executable)], cwd=ROOT, env=env))
    env['FM1_GEN_SINT_SCRIPT'] = str(ROOT / 'tools/gen_sint.py')
    check('legacy-c-launcher-other-cwd', expected_sine, subprocess.check_output([str(executable)], cwd=tmp, env=env))

rust_tooling.close()
print(json.dumps(dict(cases=len(cases), trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),
                      python=sys.version.split()[0], pillow=PIL.__version__, frozen_files=440)))
