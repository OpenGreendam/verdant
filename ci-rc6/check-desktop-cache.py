import hashlib
import json
import sys
import struct
from pathlib import Path

root = Path(sys.argv[1]) / 'render-cache'
entries = []
for folder in sorted(root.iterdir()):
    manifest_path = folder / 'manifest.json'
    if not manifest_path.is_file():
        raise ValueError(f'Incomplete cache entry: {folder.name}')
    data = manifest_path.read_bytes()
    manifest = json.loads(data)
    assert manifest['version'] == 3 and manifest['digest'] == folder.name
    members = {a['file']: a for a in manifest['artifacts']}
    assert 'native.wav' in members and len(members) == len(manifest['artifacts'])
    for name, meta in members.items():
        assert Path(name).name == name
        payload = (folder / name).read_bytes()
        assert len(payload) == meta['bytes'] and hashlib.sha256(payload).hexdigest() == meta['sha256']
    wav = (folder / 'native.wav').read_bytes()
    assert wav[:4] == b'RIFF' and wav[8:12] == b'WAVE'
    offset = 12
    chunks = {}
    while offset + 8 <= len(wav):
        size = struct.unpack_from('<I', wav, offset + 4)[0]
        end = offset + 8 + size
        assert end <= len(wav)
        chunks[wav[offset:offset + 4]] = wav[offset + 8:end]
        offset = end + (size & 1)
    fmt = chunks[b'fmt ']
    rate = struct.unpack_from('<I', fmt, 4)[0]
    alignment = struct.unpack_from('<H', fmt, 12)[0]
    assert rate > 0 and alignment > 0 and len(chunks[b'data']) % alignment == 0
    frames = len(chunks[b'data']) // alignment
    assert frames > 0
    entries.append({'entry': folder.name, 'manifest_sha256': hashlib.sha256(data).hexdigest(), 'route': manifest['route'], 'frames': frames, 'artifacts': list(members.values())})
assert entries, 'Desktop produced no persistent cache entries'
print(json.dumps({'root': str(root), 'entries': entries}, indent=2))
