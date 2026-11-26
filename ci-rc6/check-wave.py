import hashlib
import json
import math
import struct
import sys
from pathlib import Path

wav = Path(sys.argv[1]).read_bytes()
assert wav[:4] == b"RIFF" and wav[8:12] == b"WAVE" and struct.unpack_from("<I", wav, 4)[0] == len(wav) - 8
chunks = {}
pos = 12
while pos < len(wav):
    tag, length = struct.unpack_from("<4sI", wav, pos)
    start = pos + 8
    end = start + length
    assert end <= len(wav) and tag not in chunks
    chunks[tag] = wav[start:end]
    pos = end + length % 2
fmt, channels, rate, byte_rate, stride, bits = struct.unpack_from("<HHIIHH", chunks[b"fmt "])
assert (fmt, channels, stride, bits) == (3, 2, 8, 32) and byte_rate == rate * stride
pcm = chunks[b"data"]
assert len(pcm) % stride == 0
samples = [sample for (sample,) in struct.iter_unpack("<f", pcm)]
assert samples and all(math.isfinite(sample) for sample in samples)
frames = len(pcm) // stride
nonzero = sum(sample != 0 for sample in samples)
measurements = dict(wav_sha256=hashlib.sha256(wav).hexdigest(), pcm_sha256=hashlib.sha256(pcm).hexdigest(),
                    frames=frames, sample_rate=rate, channels=channels, nonzero_samples=nonzero,
                    peak=max(abs(sample) for sample in samples), archive_render_bytes=len(wav))
Path(sys.argv[2]).write_text(json.dumps(measurements, indent=2) + "\n", encoding="utf-8")
assert frames == 12000 and rate == 48000 and nonzero > 0
