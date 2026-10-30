#!/usr/bin/env python3
"""Standalone Windows x64 / ORT 1.22.0 CPU reference matrix. No GUI or playback."""
import argparse
import hashlib
import importlib.util
import importlib.metadata
import json
from pathlib import Path
import platform
import site
import sysconfig
import sys
import types
import wave

sys.path.insert(0, str(Path(__file__).resolve().parent))
from contract import MODELS, assess_text, load_manifest, matrix_pass, sha256, verify_cache


PINNED_DEPENDENCIES = {
    'onnxruntime': '1.22.0', 'numpy': '2.2.6', 'ml_dtypes': '0.5.1',
    'coloredlogs': '15.0.1', 'flatbuffers': '25.2.10', 'packaging': '25.0',
    'protobuf': '6.31.1', 'sympy': '1.14.0', 'humanfriendly': '10.0',
    'mpmath': '1.3.0', 'pyreadline3': '3.5.6',
}


def inside(path, root):
    return Path(path).resolve().is_relative_to(root)


def validate_environment(report):
    """Reject global package exposure and validate every pinned runtime dependency."""
    if sys.version_info[:2] != (3, 11):
        raise ValueError('Python 3.11 is required')
    prefix = Path(sys.prefix).resolve()
    embedded = Path(sys.executable).parent / 'python311._pth'
    if sys.prefix == sys.base_prefix and not embedded.is_file():
        raise ValueError('Use an isolated venv or task-local embedded Python')
    if not inside(sys.executable, prefix):
        raise ValueError('Python executable is outside isolated prefix')
    config = prefix / 'pyvenv.cfg'
    if config.exists():
        for line in config.read_text(encoding='utf-8').splitlines():
            key, _, value = line.partition('=')
            if key.strip().casefold() == 'include-system-site-packages' and value.strip().casefold() != 'false':
                raise ValueError('system-site-packages must be disabled')
    if site.ENABLE_USER_SITE:
        raise ValueError('User site-packages must be disabled')
    stdlib = Path(sysconfig.get_path('stdlib')).resolve()
    base = Path(sys.base_prefix).resolve()
    allowed = {Path(__file__).resolve().parent, stdlib, stdlib / 'lib-dynload',
               base / 'DLLs', base / 'python311.zip', stdlib.parent / 'python311.zip'}
    for entry in sys.path:
        path = Path(entry or '.').resolve()
        if not inside(path, prefix) and path not in allowed:
            raise ValueError('External Python search path: ' + str(path))
    for packages in [prefix / 'Lib/site-packages', prefix / 'lib/python3.11/site-packages']:
        for pth in packages.glob('*.pth'):
            for line in pth.read_text(encoding='utf-8-sig').splitlines():
                if not line.strip() or line.startswith('#') or line.startswith(('import ', 'import\t')):
                    continue
                if not inside(packages / line.strip(), prefix):
                    raise ValueError('External .pth package origin: ' + str(pth))
    versions = {}
    report['environment']['dependencies'] = versions
    report['environment']['isolated_prefix'] = str(prefix)
    for name, expected in PINNED_DEPENDENCIES.items():
        distribution = importlib.metadata.distribution(name)
        versions[name] = distribution.version
        if distribution.version != expected:
            raise ValueError(f'Expected {name}=={expected}; got {distribution.version}')
        if not inside(distribution.locate_file(''), prefix):
            raise ValueError('Dependency distribution outside isolated prefix: ' + name)
    origins = {}
    report['environment']['module_origins'] = origins
    for name in ('numpy', 'onnxruntime', 'ml_dtypes'):
        spec = importlib.util.find_spec(name)
        if spec is None or spec.origin is None or not inside(spec.origin, prefix):
            raise ValueError('Dependency module outside isolated prefix: ' + name)
        if any(not inside(path, prefix) for path in (spec.submodule_search_locations or [])):
            raise ValueError('External package search location: ' + name)
        module = importlib.import_module(name)
        if not inside(module.__file__, prefix) or any(not inside(path, prefix) for path in getattr(module, '__path__', [])):
            raise ValueError('Imported dependency outside isolated prefix: ' + name)
        if module.__version__ != PINNED_DEPENDENCIES[name]:
            raise ValueError('Imported module version differs from pin: ' + name)
        origins[name] = str(Path(module.__file__).resolve())


def read_fixture(path, np):
    with wave.open(str(path), 'rb') as wav:
        if (wav.getnchannels(), wav.getsampwidth(), wav.getframerate(), wav.getcomptype()) != (1, 2, 16000, 'NONE'):
            raise ValueError('WAV must be mono PCM16 16000 Hz')
        frames = wav.getnframes()
        if not 320 <= frames <= 400000:
            raise ValueError('Fixture must contain 320..400000 samples')
        data = wav.readframes(frames)
        if len(data) != frames * 2:
            raise ValueError('Truncated WAV: payload does not match declared frame count')
    return np.frombuffer(data, dtype='<i2').astype(np.float32) / 32768


def validate_sessions(sessions):
    providers = [session.get_providers() for session in sessions.values()]
    if not providers or any(p != ['CPUExecutionProvider'] for p in providers):
        raise ValueError('Unexpected active providers: ' + repr(providers))
    for session in sessions.values():
        session.disable_fallback()
    del session


def source_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def reference(cache):
    """Load only verified reference modules without packaging side effects."""
    sys.dont_write_bytecode = True
    source = cache / 'source'
    for name, path in [('onnx_asr', source / 'src/onnx_asr'),
                       ('onnx_asr.models', source / 'src/onnx_asr/models'),
                       ('onnx_asr.preprocessors', source / 'src/onnx_asr/preprocessors')]:
        package = types.ModuleType(name)
        package.__path__ = [str(path)]
        sys.modules[name] = package
    import numpy as np
    from ml_dtypes import bfloat16
    from onnx_asr.models import gigaam
    from onnx_asr.preprocessors.numpy_preprocessor import GigaamPreprocessorNumpy
    fbanks = source_module('contract_fbanks', source / 'preprocessors/fbanks.py')

    class Preprocessor(GigaamPreprocessorNumpy):
        def __init__(self, name):
            if name != 'gigaam_v3':
                raise ValueError('Unexpected reference preprocessor ' + name)
            self._v2 = False
            self._n_fft = self._win_length = 320
            self._window = np.hanning(321)[:-1].astype(bfloat16).astype(np.float32)
            self._melscale_fbanks = fbanks.melscale_fbanks(161, 0, 8000, 64, 16000).astype(bfloat16).astype(np.float32)

    return gigaam, Preprocessor


def preprocessing_reference(preprocessor):
    """Record all features of a deterministic nonzero float32 waveform."""
    import numpy as np
    waveform = (0.25 * np.sin(2 * np.pi * 440 * np.arange(480, dtype=np.float64) / 16000)).astype(np.float32)[None, :]
    features, lengths = preprocessor('gigaam_v3')(waveform, np.array([480], dtype=np.int64))
    if features.shape != (1, 64, 2) or not np.isfinite(features).all() or lengths.tolist() != [2]:
        raise ValueError('Unexpected deterministic reference features')
    return {
        'input': {'formula': '(0.25 * np.sin(2 * np.pi * 440 * np.arange(480, dtype=np.float64) / 16000)).astype(np.float32)[None, :]',
                  'sample_rate': 16000, 'shape': list(waveform.shape), 'dtype': 'float32',
                  'sha256_le_bytes': hashlib.sha256(waveform.astype('<f4').tobytes()).hexdigest()},
        'features': {'shape': list(features.shape), 'dtype': str(features.dtype),
                     'sha256_le_bytes': hashlib.sha256(features.astype('<f4').tobytes()).hexdigest(),
                     'values': features.tolist()},
        'lengths': {'shape': list(lengths.shape), 'dtype': str(lengths.dtype), 'values': lengths.tolist()},
    }


def abi(session):
    return {direction: [{'name': x.name, 'shape': x.shape, 'type': x.type} for x in values]
            for direction, values in [('inputs', session.get_inputs()), ('outputs', session.get_outputs())]}


def execute(args, manifest, report):
    if platform.system() != 'Windows' or platform.machine().upper() not in ('AMD64', 'X86_64'):
        raise ValueError('Native Windows x64 required; Linux does not satisfy acceptance')
    validate_environment(report)
    import numpy as np
    import onnxruntime as rt
    report['environment'].update(onnxruntime=rt.__version__, numpy=np.__version__, available_providers=rt.get_available_providers())
    if rt.__version__ != '1.22.0':
        raise ValueError('Expected ONNX Runtime 1.22.0; upgrading is forbidden')
    forbidden = set(rt.get_available_providers()) - {'CPUExecutionProvider', 'AzureExecutionProvider'}
    if forbidden:
        raise ValueError('GPU/runtime providers present: ' + repr(sorted(forbidden)))
    verify_cache(args.cache, manifest)
    samples = read_fixture(args.wav, np)
    report['fixture'] = {'path': str(args.wav.resolve()), 'sha256': sha256(args.wav), 'samples': len(samples), 'expected_text': args.expected_text}
    gigaam, preprocessor = reference(args.cache)
    report['preprocessing_reference'] = preprocessing_reference(preprocessor)
    classes = [gigaam.GigaamV2Ctc, gigaam.GigaamV2Rnnt, gigaam.GigaamV3E2eCtc, gigaam.GigaamV3E2eRnnt]
    for model_id, cls in zip(MODELS, classes):
        result = {'model': model_id, 'status': 'fail', 'expected_text': args.expected_text}
        report['results'].append(result)
        model = sessions = None
        try:
            package = manifest['models'][model_id]
            files = {role: args.cache / path for role, path in package['roles'].items()}
            model = cls(files, preprocessor, {'providers': ['CPUExecutionProvider']})
            sessions = {key: value for key, value in vars(model).items() if isinstance(value, rt.InferenceSession)}
            validate_sessions(sessions)
            result['providers'] = ['CPUExecutionProvider']
            result['abi'] = {key: abi(session) for key, session in sessions.items()}
            result['text'] = next(model.recognize_batch(samples[None, :], np.array([len(samples)], dtype=np.int64))).text
            result['semantic_match'] = assess_text(result['text'], args.expected_text)
            if not result['semantic_match']:
                raise ValueError('Transcript differs lexically from known synthetic phrase')
            result['status'] = 'pass'
        except Exception as error:
            result['error'] = str(error)
        finally:
            model = sessions = None
    report['status'] = 'pass' if matrix_pass(report['results']) else 'incomplete'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', required=True, type=Path)
    parser.add_argument('--wav', required=True, type=Path, help='Known synthetic Russian mono PCM16 16 kHz WAV')
    parser.add_argument('--expected-text', required=True)
    parser.add_argument('--output', required=True, type=Path, help='New JSON file; existing files preserved')
    parser.add_argument('--manifest', type=Path, default=Path(__file__).with_name('manifest.json'))
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Output exists; choose a new file')
    report = {'status': 'incomplete', 'environment': {'platform': platform.platform(), 'python': sys.version, 'executable': sys.executable}, 'results': []}
    try:
        manifest = load_manifest(args.manifest)
        report['manifest_sha256'] = sha256(args.manifest)
        report['provenance'] = manifest['sources']
        execute(args, manifest, report)
    except Exception as error:
        report['error'] = str(error)
    completed = {r['model'] for r in report['results']}
    for model_id in MODELS:
        if model_id not in completed:
            report['results'].append({'model': model_id, 'status': 'not-run', 'error': report.get('error', 'Environment unavailable')})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as output:
        json.dump(report, output, ensure_ascii=False, indent=2)
    print(json.dumps({'status': report['status'], 'output': str(args.output)}, ensure_ascii=False))
    return 0 if report['status'] == 'pass' and matrix_pass(report['results']) else 1


if __name__ == '__main__':
    sys.exit(main())
