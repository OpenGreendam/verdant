import argparse
import hashlib
import json
import math
import pathlib
import struct


def read_wave(path):
    data = path.read_bytes()
    tag, size, waveform = struct.unpack_from('<4sI4s', data)
    if tag != b'RIFF' or size + 8 != len(data) or waveform != b'WAVE':
        raise ValueError(f'Invalid RIFF/WAVE in {path}')
    chunks = {}
    position = 12
    while position < len(data):
        tag, size = struct.unpack_from('<4sI', data, position)
        payload = position + 8
        if payload + size > len(data) or tag in chunks:
            raise ValueError(f'Invalid WAVE chunk in {path}')
        chunks[tag] = data[payload:payload + size]
        position = payload + size + (size % 2)
    pcm = chunks[b'data']
    if (struct.unpack_from('<HHIIHH', chunks[b'fmt ']) != (3, 2, 48000, 384000, 8, 32)
            or len(pcm) != 12000 * 8):
        raise ValueError(f'Expected 12,000 frames of 48 kHz stereo float32 samples in {path}')
    samples = struct.unpack('<24000f', pcm)
    if not all(math.isfinite(sample) for sample in samples) or not any(samples):
        raise ValueError(f'Expected finite nonzero audio in {path}')
    receipt = dict(sha256=hashlib.sha256(data).hexdigest(), pcm_sha256=hashlib.sha256(pcm).hexdigest(),
                   channels=2, sample_rate=48000, frames=12000, nonzero=sum(bool(sample) for sample in samples),
                   peak=max(map(abs, samples)))
    return receipt, samples


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('wave', type=pathlib.Path)
    parser.add_argument('baseline_sha256')
    parser.add_argument('--baseline', type=pathlib.Path)
    args = parser.parse_args()
    actual, samples = read_wave(args.wave)
    actual['baseline_sha256'] = args.baseline_sha256
    actual['wav_matches_baseline'] = actual['sha256'] == args.baseline_sha256
    if args.baseline:
        baseline, reference = read_wave(args.baseline)
        if baseline['sha256'] != args.baseline_sha256:
            raise ValueError('Baseline file does not match its pinned digest')
        delta = [sample - original for sample, original in zip(samples, reference, strict=True)]
        actual.update(baseline_pcm_sha256=baseline['pcm_sha256'],
                      pcm_matches_baseline=actual['pcm_sha256'] == baseline['pcm_sha256'],
                      differing_samples=sum(bool(value) for value in delta),
                      max_abs_delta=max(map(abs, delta)),
                      rms_delta=math.sqrt(math.fsum(value * value for value in delta) / len(delta)))
    print(json.dumps(actual, indent=2))
    return 0 if actual['wav_matches_baseline'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
