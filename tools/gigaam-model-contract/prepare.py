#!/usr/bin/env python3
"""Download the pinned contract into an explicit cache; never replace unknown data."""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parent))
from contract import check_path, inspect_existing, load_manifest, verify_cache, verify_file


def prepare(root, manifest, download=True):
    root = Path(root).absolute()
    check_path(root, 'placeholder')
    root.mkdir(parents=True, exist_ok=True)
    # Validate every existing entry before any mutation or network request.
    inspect_existing(root, manifest)
    for item in manifest['files']:
        destination = check_path(root, item['path'])
        if destination.exists():
            verify_file(root, item)
            continue
        if not download:
            raise ValueError('Missing file: ' + item['path'])
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='.gigaam-download-', dir=destination.parent) as stage:
            temporary = Path(stage) / 'artifact'
            with urllib.request.urlopen(item['url'], timeout=180) as response, temporary.open('xb') as output:
                remaining = item['size']
                while remaining:
                    block = response.read(min(1024 * 1024, remaining))
                    if not block:
                        raise ValueError('Truncated download: ' + item['path'])
                    output.write(block)
                    remaining -= len(block)
                if response.read(1):
                    raise ValueError('Oversized download: ' + item['path'])
            verify_file(Path(stage), dict(item, path='artifact'))
            # Same-filesystem hard link publishes verified bytes atomically and
            # fails if another process has created the destination meanwhile.
            os.link(temporary, destination)
            verify_file(root, item)
    verify_cache(root, manifest)
    return {'status': 'pass', 'files_verified': len(manifest['files']), 'cache': str(root)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', required=True, type=Path)
    parser.add_argument('--manifest', type=Path, default=Path(__file__).with_name('manifest.json'))
    parser.add_argument('--check-only', action='store_true', help='No downloads; verify all bytes')
    args = parser.parse_args()
    print(json.dumps(prepare(args.cache, load_manifest(args.manifest), not args.check_only)))


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(json.dumps({'status': 'fail', 'error': str(error)}), file=sys.stderr)
        sys.exit(1)
