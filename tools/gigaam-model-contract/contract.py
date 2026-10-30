"""Integrity and acceptance rules shared by preparation and Windows verification."""
import hashlib
import json
import re
from pathlib import Path

MODELS = ('v3_ctc', 'v3_rnnt', 'v3_e2e_ctc', 'v3_e2e_rnnt')


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def load_manifest(path):
    manifest = json.loads(Path(path).read_text(encoding='utf-8'))
    if set(manifest['models']) != set(MODELS):
        raise ValueError('Manifest must declare exactly the four FP32 models')
    seen = set()
    for item in manifest['files']:
        name = item['path']
        if (Path(name).is_absolute() or '\\' in name or ':' in name or
                any(part in ('', '.', '..') for part in name.split('/')) or name in seen):
            raise ValueError('Unsafe or duplicate manifest path: ' + name)
        seen.add(name)
        if not re.fullmatch('[0-9a-f]{64}', item['sha256']) or item['size'] <= 0:
            raise ValueError('Invalid file identity: ' + name)
    for model in manifest['models'].values():
        if not set(model['files']).issubset(seen):
            raise ValueError('Model refers to undeclared files')
        if not set(model['roles'].values()).issubset(set(model['files'])):
            raise ValueError('Model role refers outside its declared verified files')
    return manifest


def check_path(root, relative):
    root = Path(root)
    for path in [root, *list((root / relative).parents)[:-1], root / relative]:
        if path.is_symlink() or (hasattr(path, 'is_junction') and path.is_junction()):
            raise ValueError('Linked paths are forbidden: ' + str(path))
        if path.exists() and getattr(path.lstat(), 'st_file_attributes', 0) & 0x400:
            raise ValueError('Reparse points are forbidden: ' + str(path))
    return root / relative


def verify_file(root, item):
    path = check_path(root, item['path'])
    if not path.is_file() or path.stat().st_size != item['size']:
        raise ValueError('Missing file or size mismatch: ' + item['path'])
    if sha256(path) != item['sha256']:
        raise ValueError('SHA-256 mismatch: ' + item['path'])


def inspect_existing(root, manifest):
    expected = {item['path']: item for item in manifest['files']}
    directories = {str(parent).replace('\\', '/') for name in expected
                   for parent in Path(name).parents if str(parent) != '.'}
    for path in Path(root).rglob('*'):
        relative = path.relative_to(root).as_posix()
        check_path(root, relative)
        if path.is_dir():
            if relative not in directories:
                raise ValueError('Undeclared directory: ' + relative)
        elif not path.is_file() or relative not in expected:
            raise ValueError('Undeclared or special file: ' + relative)
        else:
            verify_file(root, expected[relative])


def verify_cache(root, manifest):
    inspect_existing(root, manifest)
    for item in manifest['files']:
        verify_file(root, item)


def words(text):
    return re.findall(r'\w+', text.casefold().replace('ё', 'е'))


def assess_text(text, expected):
    """Exact lexical agreement; punctuation/case ignored only for assessment."""
    return isinstance(text, str) and bool(text.strip()) and bool(words(expected)) and words(text) == words(expected)


def matrix_pass(results):
    return (len(results) == 4 and {r.get('model') for r in results} == set(MODELS)
            and all(r.get('status') == 'pass' and r.get('providers') == ['CPUExecutionProvider']
                    and assess_text(r.get('text'), r.get('expected_text', '')) for r in results))
