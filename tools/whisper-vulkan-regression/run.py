#!/usr/bin/env python3
"""Build and run against the registry sources selected by the locked Cargo graph."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET


def run(command, cwd):
    print('+ ' + ' '.join(map(str, command)), flush=True)
    subprocess.run(list(map(str, command)), cwd=cwd, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-dir', required=True, type=Path, help='Fresh CMake build directory')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    build_dir = args.build_dir.resolve()
    if build_dir.exists() and any(build_dir.iterdir()):
        raise RuntimeError('Build directory must be fresh/empty; existing files are preserved')
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--format-version', '1'], cwd=root, text=True))
    packages = [p for p in metadata['packages'] if p['name'] == 'whisper-rs-sys']
    if len(packages) != 1 or packages[0]['version'] != '0.15.0':
        raise RuntimeError('Expected exactly one whisper-rs-sys package at 0.15.0')
    package = packages[0]
    if package.get('source') != 'registry+https://github.com/rust-lang/crates.io-index':
        raise RuntimeError('Expected registry whisper-rs-sys, without a path/git override')
    source = Path(package['manifest_path']).parent / 'whisper.cpp'
    if not (source / 'ggml/include/ggml-vulkan.h').is_file():
        raise RuntimeError('Locked sys package does not contain the expected Vulkan sources')
    print(f'Backend: whisper-rs-sys 0.15.0, {source}', flush=True)
    run(['cmake', '-S', Path(__file__).resolve().parent, '-B', build_dir,
         f'-DWHISPER_CPP_DIR={source}', '-DCMAKE_BUILD_TYPE=Release'], root)
    run(['cmake', '--build', build_dir, '--config', 'Release', '--target', 'whisper-vulkan-softmax'], root)
    report = build_dir / 'ctest-results.xml'
    run(['ctest', '--test-dir', build_dir, '-C', 'Release', '--output-on-failure',
         '--output-junit', report], root)
    tests = list(ET.parse(report).getroot().iter('testcase'))
    if len(tests) != 2:
        raise RuntimeError(f'Expected two CTest results, found {len(tests)}')
    skipped = sum(test.find('skipped') is not None for test in tests)
    if skipped:
        print(f'SKIPPED: {skipped}/2 Vulkan cases; native acceptance remains incomplete')
        return 77
    print('PASSED: both native Vulkan softmax cases')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError, ET.ParseError) as error:
        print(f'FAILED: {error}', file=sys.stderr)
        sys.exit(1)
