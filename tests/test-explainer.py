#!/usr/bin/env python3
"""Integration checks; first run auto-installs TTS under ~/local-models.
Run: python3 tests/test-explainer.py
"""
import json
from pathlib import Path
import subprocess
import tempfile

SCRIPT = Path(__file__).resolve().parents[1] / 'workflows' / 'explainer.sh'


def run(*args, cwd, success=True):
    result = subprocess.run([str(SCRIPT), *args], cwd=cwd, text=True, capture_output=True)
    assert (result.returncode == 0) == success, result.stderr
    return result


with tempfile.TemporaryDirectory(prefix='explainer-test-') as temporary:
    root = Path(temporary)
    subprocess.run(['git', 'init', '-q', str(root)], check=True)
    prompt = run('Review architecture', cwd=root)
    outputs = list((root / 'explainer').iterdir())
    assert len(outputs) == 1
    output = outputs[0]
    assert str(output) in prompt.stdout
    assert 'file://' in prompt.stdout and 'prefers-reduced-motion' in prompt.stdout
    assert (output / '.gitattributes').exists() or (output / '.gitignore').exists()
    deck = {'title': 'Test', 'slides': [
        {'id': '01', 'narration': 'This service turns incoming orders into confirmed deliveries.'},
        {'id': '02', 'narration': 'A queue allows workers to retry failed requests safely.'},
    ]}
    path = output / 'deck.json'
    path.write_text(json.dumps(deck))
    run('--render', str(path), cwd=root)
    manifest = json.loads((output / 'audio-manifest.json').read_text())
    assert len(manifest['slides']) == 2
    assert manifest['total_duration_seconds'] > 0
    assert manifest['channels'] == 1 and manifest['bitrate_kbps'] == 64
    audio = output / 'audio' / '01.mp3'
    assert audio.stat().st_size > 100
    first_modified = audio.stat().st_mtime_ns
    cached = run('--render', str(path), cwd=root)
    assert 'Cached 01' in cached.stderr and 'Cached 02' in cached.stderr
    assert audio.stat().st_mtime_ns == first_modified
    # Changing a single slide preserves the other MP3.
    deck['slides'][1]['narration'] = 'The queue also absorbs sudden peaks in demand.'
    path.write_text(json.dumps(deck))
    changed = run('--render', str(path), cwd=root)
    assert 'Cached 01' in changed.stderr and 'Generated 02' in changed.stderr
    assert audio.stat().st_mtime_ns == first_modified
    # A corrupt cached MP3 must be regenerated.
    audio.write_bytes(b'broken audio')
    repaired = run('--render', str(path), cwd=root)
    assert 'Generated 01' in repaired.stderr and audio.stat().st_size > 100
    deck['slides'][0]['id'] = '../escape'
    path.write_text(json.dumps(deck))
    invalid = run('--render', str(path), cwd=root, success=False)
    assert 'unique safe strings' in invalid.stderr
    assert not (output / 'escape.mp3').exists()
    second = run('Review architecture', cwd=root)
    assert len(list((root / 'explainer').iterdir())) == 2
    print('PASS: startup, scoped Git rules, output paths, real MP3 synthesis, cache, repair, and invalid IDs')
