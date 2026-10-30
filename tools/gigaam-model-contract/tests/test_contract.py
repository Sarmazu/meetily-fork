import copy
import hashlib
import errno
import json
import struct
import types
from contextlib import ExitStack
from unittest import mock
import wave
import weakref
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contract import MODELS, assess_text, load_manifest, matrix_pass, verify_cache
from prepare import prepare
import verify


class IntegrityTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.item = {'path': 'model/test.bin', 'size': 3, 'sha256': hashlib.sha256(b'abc').hexdigest()}
        self.manifest = {'files': [self.item]}
        (self.root / 'model').mkdir()
        (self.root / self.item['path']).write_bytes(b'abc')

    def test_content_passes(self):
        verify_cache(self.root, self.manifest)

    def test_missing_file_fails(self):
        (self.root / self.item['path']).unlink()
        with self.assertRaises(ValueError):
            verify_cache(self.root, self.manifest)

    def test_same_size_substitution_fails_and_is_preserved(self):
        (self.root / self.item['path']).write_bytes(b'bad')
        with self.assertRaises(ValueError):
            prepare(self.root, self.manifest)
        self.assertEqual((self.root / self.item['path']).read_bytes(), b'bad')

    def test_extra_file_fails(self):
        (self.root / 'unknown').write_bytes(b'preserve')
        with self.assertRaises(ValueError):
            prepare(self.root, self.manifest)
        self.assertEqual((self.root / 'unknown').read_bytes(), b'preserve')

    def test_symlink_fails(self):
        target = self.root / self.item['path']
        target.unlink()
        local_target = self.root / 'symlink-target.bin'
        local_target.write_bytes(b'abc')
        try:
            target.symlink_to(local_target)
        except NotImplementedError as error:
            self.skipTest('Symlink capability unavailable: ' + str(error))
        except OSError as error:
            if error.errno not in (errno.EACCES, errno.EPERM, errno.ENOSYS, errno.ENOTSUP):
                raise
            self.skipTest('Symlink capability unavailable: ' + str(error))
        with self.assertRaises(ValueError):
            verify_cache(self.root, self.manifest)

    def test_unknown_empty_directory_fails(self):
        (self.root / 'unknown').mkdir()
        with self.assertRaises(ValueError):
            verify_cache(self.root, self.manifest)

    def test_verified_download_published_without_overwrite(self):
        source = self.root / 'source.bin'
        source.write_bytes(b'abc')
        item = dict(self.item, url=source.as_uri())
        with tempfile.TemporaryDirectory() as destination:
            result = prepare(Path(destination), {'files': [item]})
            self.assertEqual(result['status'], 'pass')
            self.assertEqual((Path(destination) / item['path']).read_bytes(), b'abc')

    def test_corrupt_download_never_published(self):
        source = self.root / 'corrupt.bin'
        source.write_bytes(b'bad')
        item = dict(self.item, url=source.as_uri())
        with tempfile.TemporaryDirectory() as destination:
            with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                prepare(Path(destination), {'files': [item]})
            self.assertFalse((Path(destination) / item['path']).exists())


class ManifestTests(unittest.TestCase):
    def test_roles_must_belong_to_model_files(self):
        manifest_path = Path(__file__).resolve().parents[1] / 'manifest.json'
        baseline = json.loads(manifest_path.read_text())
        for bad_path in ('../../outside.onnx', 'models/not-declared.onnx', 'source/LICENSE'):
            with self.subTest(path=bad_path), tempfile.TemporaryDirectory() as directory:
                manifest = copy.deepcopy(baseline)
                manifest['models']['v3_ctc']['roles']['model'] = bad_path
                path = Path(directory) / 'manifest.json'
                path.write_text(json.dumps(manifest))
                with self.assertRaisesRegex(ValueError, 'role'):
                    load_manifest(path)



class MatrixTests(unittest.TestCase):
    def setUp(self):
        self.results = [{'model': m, 'status': 'pass', 'providers': ['CPUExecutionProvider'],
                         'text': 'Проверка речи.', 'expected_text': 'проверка речи'} for m in MODELS]

    def test_complete_matrix_passes(self):
        self.assertTrue(matrix_pass(self.results))

    def test_empty_text_cannot_pass(self):
        self.results[0]['text'] = ''
        self.assertFalse(matrix_pass(self.results))

    def test_wrong_text_cannot_pass(self):
        self.results[0]['text'] = 'другой текст'
        self.assertFalse(matrix_pass(self.results))

    def test_extra_unicode_word_or_unknown_cannot_pass(self):
        for extra in (' English', ' <unk>', ' 中文', ' δοκιμή'):
            with self.subTest(extra=extra):
                self.results[0]['text'] = 'Проверка речи' + extra
                self.assertFalse(matrix_pass(self.results))

    def test_wrong_provider_cannot_pass(self):
        self.results[0]['providers'] = ['CUDAExecutionProvider', 'CPUExecutionProvider']
        self.assertFalse(matrix_pass(self.results))

    def test_missing_or_duplicate_model_cannot_pass(self):
        self.assertFalse(matrix_pass(self.results[:-1]))
        self.results[-1] = copy.deepcopy(self.results[0])
        self.assertFalse(matrix_pass(self.results))

    def test_not_run_cannot_pass(self):
        self.results[0]['status'] = 'not-run'
        self.assertFalse(matrix_pass(self.results))

    def test_e2e_assessment_does_not_mutate_text(self):
        original = 'Проверка речи!'
        self.assertTrue(assess_text(original, 'проверка речи'))
        self.assertEqual(original, 'Проверка речи!')


class EnvironmentTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.prefix = Path(self.directory.name) / 'isolated'
        self.prefix.mkdir()
        self.report = {'environment': {}}
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        for target, value in [('sys.version_info', (3, 11, 9)), ('sys.prefix', str(self.prefix)),
                              ('sys.base_prefix', str(self.prefix.parent / 'base')),
                              ('sys.executable', str(self.prefix / 'python.exe')),
                              ('sys.path', [str(self.prefix)]), ('site.ENABLE_USER_SITE', False)]:
            self.stack.enter_context(mock.patch(target, value))
        self.stack.enter_context(mock.patch.object(verify.sysconfig, 'get_path',
            return_value=str(self.prefix.parent / 'base/Lib')))
        self.distributions = {name: types.SimpleNamespace(version=version,
                              locate_file=lambda name: self.prefix / 'Lib/site-packages')
                              for name, version in verify.PINNED_DEPENDENCIES.items()}
        self.stack.enter_context(mock.patch.object(verify.importlib.metadata, 'distribution',
                                                  side_effect=lambda name: self.distributions[name]))
        self.origins = {name: self.prefix / 'Lib/site-packages' / name / '__init__.py'
                        for name in ('numpy', 'onnxruntime', 'ml_dtypes')}
        self.stack.enter_context(mock.patch.object(verify.importlib.util, 'find_spec',
            side_effect=lambda name: types.SimpleNamespace(origin=str(self.origins[name]),
                                                           submodule_search_locations=[])))
        self.stack.enter_context(mock.patch.object(verify.importlib, 'import_module',
            side_effect=lambda name: types.SimpleNamespace(__file__=str(self.origins[name]),
                                                           __version__=verify.PINNED_DEPENDENCIES[name])))

    def test_other_python_major_minor_is_rejected(self):
        for version in ((3, 10, 9), (3, 12, 0), (4, 11, 0)):
            with self.subTest(version=version), mock.patch.object(verify.sys, 'version_info', version):
                with self.assertRaisesRegex(ValueError, 'Python 3.11'):
                    verify.validate_environment(self.report)

    def test_pins_and_origins_recorded(self):
        verify.validate_environment(self.report)
        self.assertEqual(self.report['environment']['dependencies'], verify.PINNED_DEPENDENCIES)
        self.assertEqual(set(self.report['environment']['module_origins']), set(self.origins))

    def test_each_dependency_version_is_enforced(self):
        for name, distribution in self.distributions.items():
            with self.subTest(name=name):
                expected = distribution.version
                distribution.version = '999.0'
                with self.assertRaisesRegex(ValueError, 'Expected'):
                    verify.validate_environment(self.report)
                distribution.version = expected

    def test_each_critical_module_origin_is_enforced(self):
        for name in self.origins:
            with self.subTest(name=name):
                expected = self.origins[name]
                self.origins[name] = self.prefix.parent / 'global' / name / '__init__.py'
                with self.assertRaisesRegex(ValueError, 'outside isolated'):
                    verify.validate_environment(self.report)
                self.origins[name] = expected

    def test_global_distribution_is_rejected(self):
        self.distributions['numpy'].locate_file = lambda name: self.prefix.parent / 'global'
        with self.assertRaisesRegex(ValueError, 'distribution outside'):
            verify.validate_environment(self.report)

    def test_system_site_packages_is_rejected(self):
        (self.prefix / 'pyvenv.cfg').write_text('include-system-site-packages = true\n')
        with self.assertRaisesRegex(ValueError, 'system-site-packages'):
            verify.validate_environment(self.report)

    def test_external_search_path_is_rejected(self):
        sys.path.append(str(self.prefix.parent / 'global'))
        with self.assertRaisesRegex(ValueError, 'External Python search path'):
            verify.validate_environment(self.report)

    def test_external_pth_origin_is_rejected(self):
        packages = self.prefix / 'Lib/site-packages'
        packages.mkdir(parents=True)
        (packages / 'external.pth').write_text(str(self.prefix.parent / 'global'))
        with self.assertRaisesRegex(ValueError, 'External .pth'):
            verify.validate_environment(self.report)

    def test_user_site_packages_is_rejected(self):
        with mock.patch.object(verify.site, 'ENABLE_USER_SITE', True):
            with self.assertRaisesRegex(ValueError, 'User site-packages'):
                verify.validate_environment(self.report)


class FixtureTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / 'synthetic.wav'
        with wave.open(str(self.path), 'wb') as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(16000)
            wav.writeframes(bytes(640))

    def test_oversized_header_rejected_before_read(self):
        data = bytearray(self.path.read_bytes())
        struct.pack_into('<I', data, 40, 400001 * 2)
        self.path.write_bytes(data)
        np = mock.Mock()
        with mock.patch.object(wave.Wave_read, 'readframes') as readframes:
            with self.assertRaisesRegex(ValueError, '320..400000'):
                verify.read_fixture(self.path, np)
            readframes.assert_not_called()
        np.frombuffer.assert_not_called()

    def test_truncated_payload_rejected_before_allocation(self):
        data = bytearray(self.path.read_bytes())
        struct.pack_into('<I', data, 40, 321 * 2)
        self.path.write_bytes(data)
        np = mock.Mock()
        with self.assertRaisesRegex(ValueError, 'Truncated WAV'):
            verify.read_fixture(self.path, np)
        np.frombuffer.assert_not_called()


class HarnessProviderTests(unittest.TestCase):
    def test_actual_execute_rejects_mocked_gpu_sessions(self):
        sessions = []
        class Session:
            def __init__(self):
                self.disable_fallback = mock.Mock()
                sessions.append(self)
            def get_providers(self):
                return ['CUDAExecutionProvider', 'CPUExecutionProvider']
        models = []
        class Model:
            def __init__(self, *args):
                self._model = Session()
                self.recognize_batch = mock.Mock()
                models.append(self)
        rt = types.SimpleNamespace(__version__='1.22.0', InferenceSession=Session,
                                   get_available_providers=lambda: ['CPUExecutionProvider'])
        np = types.SimpleNamespace(__version__='2.2.6')
        gigaam = types.SimpleNamespace(**{name: Model for name in
            ('GigaamV2Ctc', 'GigaamV2Rnnt', 'GigaamV3E2eCtc', 'GigaamV3E2eRnnt')})
        report = {'environment': {}, 'results': []}
        args = types.SimpleNamespace(cache=Path('cache'), wav=Path('synthetic.wav'), expected_text='Речь')
        manifest = {'models': {model: {'roles': {}} for model in MODELS}}
        with mock.patch.dict(sys.modules, {'numpy': np, 'onnxruntime': rt}), \
             mock.patch.object(verify.platform, 'system', return_value='Windows'), \
             mock.patch.object(verify.platform, 'machine', return_value='AMD64'), \
             mock.patch.object(verify, 'validate_environment'), \
             mock.patch.object(verify, 'verify_cache'), \
             mock.patch.object(verify, 'read_fixture', return_value=[0] * 320), \
             mock.patch.object(verify, 'sha256', return_value='fixture-hash'), \
             mock.patch.object(verify, 'preprocessing_reference', return_value={}), \
             mock.patch.object(verify, 'reference', return_value=(gigaam, object())):
            verify.execute(args, manifest, report)
        self.assertEqual(report['status'], 'incomplete')
        self.assertEqual(len(report['results']), 4)
        self.assertTrue(all('Unexpected active providers' in result['error'] for result in report['results']))
        for model, session in zip(models, sessions):
            model.recognize_batch.assert_not_called()
            session.disable_fallback.assert_not_called()


class HarnessCleanupTests(unittest.TestCase):
    def test_failed_graphs_released_before_next_model_and_return(self):
        references = []
        class Session:
            def get_providers(self):
                return ['CPUExecutionProvider']
            def disable_fallback(self):
                pass
            def get_inputs(self):
                return []
            def get_outputs(self):
                return []
        testcase = self
        class Model:
            def __init__(self, *args):
                testcase.assertTrue(all(reference() is None for reference in references))
                self._model = Session()
                self.index = len(references) // 2
                references.extend([weakref.ref(self), weakref.ref(self._model)])
            def recognize_batch(self, *args):
                if self.index % 2 == 0:
                    raise RuntimeError('synthetic inference error')
                return iter([types.SimpleNamespace(text='wrong')])
        class Samples:
            def __len__(self):
                return 320
            def __getitem__(self, index):
                return self
        rt = types.SimpleNamespace(__version__='1.22.0', InferenceSession=Session,
                                   get_available_providers=lambda: ['CPUExecutionProvider'])
        np = types.SimpleNamespace(__version__='2.2.6', array=lambda *args, **kwargs: None, int64='int64')
        gigaam = types.SimpleNamespace(**{name: Model for name in
            ('GigaamV2Ctc', 'GigaamV2Rnnt', 'GigaamV3E2eCtc', 'GigaamV3E2eRnnt')})
        report = {'environment': {}, 'results': []}
        args = types.SimpleNamespace(cache=Path('cache'), wav=Path('synthetic.wav'), expected_text='Речь')
        manifest = {'models': {model: {'roles': {}} for model in MODELS}}
        with mock.patch.dict(sys.modules, {'numpy': np, 'onnxruntime': rt}), \
             mock.patch.object(verify.platform, 'system', return_value='Windows'), \
             mock.patch.object(verify.platform, 'machine', return_value='AMD64'), \
             mock.patch.object(verify, 'validate_environment'), \
             mock.patch.object(verify, 'verify_cache'), \
             mock.patch.object(verify, 'read_fixture', return_value=Samples()), \
             mock.patch.object(verify, 'sha256', return_value='fixture-hash'), \
             mock.patch.object(verify, 'preprocessing_reference', return_value={}), \
             mock.patch.object(verify, 'reference', return_value=(gigaam, object())):
            verify.execute(args, manifest, report)
        self.assertEqual(report['status'], 'incomplete')
        self.assertEqual(len(references), 8)
        self.assertTrue(all(reference() is None for reference in references))
        self.assertEqual([result['error'] for result in report['results']],
                         ['synthetic inference error', 'Transcript differs lexically from known synthetic phrase'] * 2)


if __name__ == '__main__':
    unittest.main()
