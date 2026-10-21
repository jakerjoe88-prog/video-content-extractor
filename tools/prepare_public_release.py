"""Populate redistributable runtime files only; never copy an app profile."""
from pathlib import Path
import argparse
import hashlib
import json
import shutil

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--runtime-source', required=True, type=Path)
args = parser.parse_args()
source = args.runtime_source.resolve()
target = root / 'src-tauri/resources'
assert source != target.resolve()

def ignored(directory, names):
    return [n for n in names if n in {'__pycache__', '.git', '.cache'} or
            n.endswith(('.pyc', '.pyo', '.log', '.tmp')) or n in {'direct_url.json'}]

for name in ('bin', 'ffmpeg'):
    shutil.copytree(source/name, target/name, dirs_exist_ok=True, ignore=ignored)

python_source=source/'python-embed'
python_target=target/'python-embed'
python_target.mkdir(exist_ok=True)
for file in python_source.iterdir():
    if file.is_file() and (file.suffix in {'.exe','.dll','.pyd','.zip','.cat','._pth'} or file.name=='LICENSE.txt'):
        shutil.copy2(file, python_target/file.name)
shutil.copytree(python_source/'Lib',python_target/'Lib',dirs_exist_ok=True,ignore=ignored)
manifest={'pythonEmbed':'python-embed','ffmpeg':'ffmpeg','dyMcp':'dy-mcp'}
(target/'runtime-manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
files=[p for folder in ('bin','ffmpeg','python-embed') for p in (target/folder).rglob('*') if p.is_file()]
print(json.dumps({'runtime_files':len(files),'runtime_bytes':sum(p.stat().st_size for p in files)},ensure_ascii=False))
