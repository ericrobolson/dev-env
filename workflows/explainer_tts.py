#!/usr/bin/env python3
"""Local CPU speech preflight and cached slide synthesis for explainer.sh."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys


def atomic_write(path, data):
    temporary = path.with_suffix(path.suffix + '.part')
    temporary.write_bytes(data)
    os.replace(temporary, path)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    import kokoro_onnx
    import lameenc
    import numpy as np
    from importlib.metadata import version

    command, root = sys.argv[1], Path(sys.argv[2])
    model = root / 'kokoro-v1.0.onnx'
    voices = root / 'voices-v1.0.bin'
    # Digests of the matched upstream model-files-v1.1 assets, fetched via HTTPS.
    expected = {
        model: 'beb0d1848dee9a49da392cc3df26958d46cfa35d321edf434f52949153f0df3a',
        voices: 'bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d',
    }
    for asset, checksum in expected.items():
        if digest(asset) != checksum:
            raise ValueError(f'model asset checksum mismatch: {asset}')
    engine = kokoro_onnx.Kokoro(str(model), str(voices))
    if command == 'check':
        samples, rate = engine.create('Ready.', voice='af_heart', speed=1.0, lang='en-us')
        if rate <= 0 or len(samples) == 0 or not np.isfinite(samples).all():
            raise ValueError('TTS preflight produced invalid audio')
        encoder = lameenc.Encoder()
        encoder.set_channels(1)
        encoder.set_in_sample_rate(rate)
        encoder.set_bit_rate(64)
        encoded = encoder.encode((np.clip(samples, -1, 1) * 32767).astype('<i2').tobytes())
        if not encoded + encoder.flush():
            raise ValueError('MP3 encoder preflight failed')
        print('Kokoro CPU synthesis and mono MP3 encoder ready.', file=sys.stderr)
        return

    path = Path(sys.argv[3]).resolve()
    deck = json.loads(path.read_text())
    slides = deck['slides']
    voice, speed = deck.get('voice', 'af_heart'), deck.get('speed', 1.0)
    if not isinstance(slides, list) or not slides:
        raise ValueError('slides must be a nonempty list')
    if not isinstance(speed, (int, float)) or not 0.75 <= speed <= 1.5:
        raise ValueError('speed must be between 0.75 and 1.5')
    ids = set()
    for slide in slides:
        identifier = slide['id']
        if not isinstance(identifier, str) or not re.fullmatch(r'[A-Za-z0-9_-]+', identifier) or identifier in ids:
            raise ValueError('slide IDs must be unique safe strings')
        ids.add(identifier)
        if not isinstance(slide['narration'], str) or not slide['narration'].strip():
            raise ValueError(f'empty narration: {identifier}')
        if len(slide['narration'].split()) > 120:
            raise ValueError(f'slide {identifier} exceeds 120 words; split it')
    audio_dir = path.parent / 'audio'
    audio_dir.mkdir(exist_ok=True)
    manifest_path = path.parent / 'audio-manifest.json'
    try:
        old = json.loads(manifest_path.read_text())
    except (FileNotFoundError, ValueError):
        old = {}
    cached = {s['id']: s for s in old.get('slides', [])}
    provenance = {'model_sha256': digest(model), 'voices_sha256': digest(voices),
                  'kokoro_onnx': version('kokoro-onnx'), 'lameenc': version('lameenc'),
                  'voice': voice, 'speed': speed, 'bitrate_kbps': 64, 'channels': 1,
                  'renderer_version': 1, 'onnxruntime': version('onnxruntime'),
                  'phonemizer': version('phonemizer'), 'numpy': version('numpy')}
    result = {'schema_version': 1, **provenance, 'slides': []}
    for slide in slides:
        identifier = slide['id']
        key = hashlib.sha256(json.dumps([slide['narration'], provenance], sort_keys=True).encode()).hexdigest()
        target = audio_dir / f'{identifier}.mp3'
        entry = cached.get(identifier, {})
        if (entry.get('cache_key') == key and target.is_file() and target.stat().st_size
                and entry.get('audio_sha256') == digest(target)):
            print(f'Cached {identifier}', file=sys.stderr)
        else:
            samples, rate = engine.create(slide['narration'], voice=voice, speed=speed, lang='en-us')
            samples = np.asarray(samples)
            if samples.ndim != 1 or not len(samples) or rate <= 0 or not np.isfinite(samples).all():
                raise ValueError(f'invalid speech audio: {identifier}')
            # Brief silence protects sentence boundaries; no persistent PCM/WAV.
            samples = np.concatenate([np.zeros(int(rate * .12)), samples, np.zeros(int(rate * .18))])
            encoder = lameenc.Encoder()
            encoder.set_channels(1)
            encoder.set_in_sample_rate(rate)
            encoder.set_bit_rate(64)
            encoder.set_quality(2)
            encoder.silence()
            data = encoder.encode((np.clip(samples, -1, 1) * 32767).astype('<i2').tobytes()) + encoder.flush()
            if not data:
                raise ValueError(f'empty MP3: {identifier}')
            atomic_write(target, data)
            entry = {'id': identifier, 'file': f'audio/{identifier}.mp3',
                     'duration_seconds': round(len(samples) / rate, 3), 'sample_rate': rate,
                     'cache_key': key, 'audio_sha256': digest(target), 'bytes': len(data)}
            print(f'Generated {identifier}: {entry["duration_seconds"]}s', file=sys.stderr)
        result['slides'].append(entry)
        # Persist completed slides so an interrupted run can resume cheaply.
        atomic_write(manifest_path, json.dumps(result, indent=2).encode() + b'\n')
    result['total_duration_seconds'] = round(sum(s['duration_seconds'] for s in result['slides']), 3)
    result['duration_basis'] = 'PCM including boundary silence; browser MP3 metadata may differ slightly'
    atomic_write(manifest_path, json.dumps(result, indent=2).encode() + b'\n')
    print(f'Total narration: {result["total_duration_seconds"]}s', file=sys.stderr)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(f'explainer TTS: {error}', file=sys.stderr)
        sys.exit(1)
