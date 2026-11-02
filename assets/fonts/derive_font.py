#!/usr/bin/env python3
"""Derive an OFL static Regular font; input must match the public upstream hash.
Usage: python derive_font.py /path/to/NotoSansSC-Variable.ttf
Requires fontTools==4.61.1. No network, subsetting or upstream source copying.
"""
import hashlib, json, pathlib, sys
import fontTools
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
SOURCE_SHA256 = 'a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da'
ROOT = pathlib.Path(__file__).resolve().parent
source = pathlib.Path(sys.argv[1])
assert fontTools.__version__ == '4.61.1'
assert hashlib.sha256(source.read_bytes()).hexdigest() == SOURCE_SHA256
font = TTFont(source, recalcTimestamp=False)
cmap = dict(font.getBestCmap())
font = instantiateVariableFont(font, {'wght': 400}, inplace=True)
# Rename all existing family/style/full/PostScript/unique/version records,
# including localized records. Preserve copyright, trademark and license entries.
names = {1:'PrivateVP Sans SC', 2:'Regular', 3:'PrivateVP Sans SC Regular; static-400; 1.0',
         4:'PrivateVP Sans SC Regular', 5:'Version 1.000; derived static 400',
         6:'PrivateVPSansSC-Regular', 16:'PrivateVP Sans SC', 17:'Regular',
         18:'PrivateVP Sans SC Regular', 21:'PrivateVP Sans SC', 22:'Regular',
         25:'PrivateVPSansSC'}
for record in font['name'].names:
    if record.nameID in names:
        record.string = names[record.nameID].encode(record.getEncoding())
for name_id, value in names.items():
    font['name'].setName(value, name_id, 3, 1, 0x409)
font['OS/2'].usWeightClass = 400
assert 'fvar' not in font and 'gvar' not in font
assert font.getBestCmap() == cmap
out = ROOT / 'PrivateVPSansSC-Regular.ttf'
font.save(out)
check = TTFont(out, recalcTimestamp=False)
assert 'fvar' not in check and 'gvar' not in check
assert check.getBestCmap() == cmap
samples = {}
for char in '中文导入订阅节点连接測試日本語Aa012':
    name = cmap[ord(char)]
    glyph = check['glyf'][name]
    glyph.recalcBounds(check['glyf'])
    advance, lsb = check['hmtx'][name]
    assert glyph.numberOfContours != 0 and advance > 0
    assert glyph.xMax > glyph.xMin and glyph.yMax > glyph.yMin
    samples[char] = {'glyph':name, 'advance':advance, 'bounds':[glyph.xMin,glyph.yMin,glyph.xMax,glyph.yMax]}
report = {'fontToolsVersion': fontTools.__version__, 'sourceSha256':SOURCE_SHA256,
          'outputSha256':hashlib.sha256(out.read_bytes()).hexdigest(),
          'outputBytes':out.stat().st_size, 'unicodeCodepoints':len(cmap),
          'sameCmapAsSource':True,'staticNoFvarGvar':True,'weight':400,
          'sampleOutlineMetrics':samples,'renderTest':'not performed'}
(ROOT / 'verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k != 'sampleOutlineMetrics'}))
