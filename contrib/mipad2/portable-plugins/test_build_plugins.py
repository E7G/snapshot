"""Offline contract tests; no camera, system install, or GStreamer build."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import shutil
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('build_plugins', Path(__file__).with_name('build_plugins.py'))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class ArchiveTests(unittest.TestCase):
    def archive(self, entries):
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w') as archive:
            for name, kind in entries:
                member = tarfile.TarInfo(name)
                member.type = kind
                if kind in (tarfile.SYMTYPE, tarfile.LNKTYPE):
                    member.linkname = '../../outside'
                archive.addfile(member)
        buffer.seek(0)
        return tarfile.open(fileobj=buffer)

    def test_regular_tree(self):
        with self.archive([('source', tarfile.DIRTYPE), ('source/file', tarfile.REGTYPE)]) as archive:
            self.assertEqual(len(builder.archive_members(archive, 'source')), 2)

    def test_reject_path_traversal_absolute_and_other_root(self):
        for name in ['../source/x', '/source/x', 'source/../x', 'other/x', 'source\\x']:
            with self.subTest(name=name), self.archive([(name, tarfile.REGTYPE)]) as archive:
                self.assertRaises(ValueError, builder.archive_members, archive, 'source')

    def test_reject_links_devices_and_duplicates(self):
        for kind in [tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.CHRTYPE, tarfile.FIFOTYPE]:
            with self.subTest(kind=kind), self.archive([('source/x', kind)]) as archive:
                self.assertRaises(ValueError, builder.archive_members, archive, 'source')
        with self.archive([('source/x', tarfile.REGTYPE)] * 2) as archive:
            self.assertRaises(ValueError, builder.archive_members, archive, 'source')


class PlanAndBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.archives = self.base / 'archives'
        self.source = self.base / 'source'
        self.archives.mkdir()
        self.source.mkdir()
        self.output = self.base / 'output'
        root = 'fixture-' + builder.VERSION
        archive = self.archives / (root + '.tar.xz')
        with tarfile.open(archive, 'w:xz') as tar:
            data = b'old\n'
            member = tarfile.TarInfo(root + '/file.txt')
            member.size = len(data)
            tar.addfile(member, io.BytesIO(data))
        patch_file = self.source / 'fix.patch'
        patch_file.write_text('--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-old\n+new\n')
        self.item = dict(name='fixture', sha256=builder.sha256(archive), patch='fix.patch',
                         patch_sha256=builder.sha256(patch_file), feature='encoding',
                         changed_file='file.txt', patched_sha256=hashlib.sha256(b'new\n').hexdigest(),
                         target='gst/encoding/libgstencoding.so', extra=[])
        self.patcher = patch.object(builder, 'COMPONENTS', (self.item,))
        self.patcher.start()
        self.addCleanup(self.patcher.stop)

    def test_prepare_applies_patch_and_is_not_build_acceptance(self):
        with patch.object(builder, 'verify_build_abi', side_effect=AssertionError('must not run')):
            receipt = builder.build(self.archives, self.source, self.output, prepare_only=True)
        self.assertEqual((self.output / 'work' / ('fixture-' + builder.VERSION) / 'file.txt').read_text(), 'new\n')
        self.assertEqual(receipt['state'], 'sources-prepared')
        self.assertFalse(receipt['device_tested'])
        self.assertFalse(receipt['image_qualified'])
        self.assertFalse(receipt['components'][0]['compiled'])
        self.assertFalse((self.output / 'plugins').exists())
        self.assertEqual(json.loads((self.output / 'receipt.json').read_text()), receipt)

    def test_input_hash_mismatch_before_output_creation(self):
        for key in ['sha256', 'patch_sha256']:
            with self.subTest(key=key), patch.dict(self.item, {key: '0' * 64}):
                self.assertRaises(ValueError, builder.build, self.archives, self.source,
                                  self.output, prepare_only=True)
                self.assertFalse(self.output.exists())

    def test_existing_output_is_not_overwritten(self):
        self.output.mkdir()
        sentinel = self.output / 'keep'
        sentinel.write_text('unchanged')
        self.assertRaises(ValueError, builder.plan, self.archives, self.source, self.output)
        self.assertEqual(sentinel.read_text(), 'unchanged')

    def test_output_must_be_outside_source_and_archive(self):
        for root in [self.source, self.archives]:
            self.assertRaises(ValueError, builder.plan, self.archives, self.source, root / 'out')
            self.assertFalse((root / 'out').exists())

    def test_symlink_archive_or_patch_rejected(self):
        for name in [self.archives / ('fixture-' + builder.VERSION + '.tar.xz'), self.source / 'fix.patch']:
            actual = name.with_suffix('.real')
            name.rename(actual)
            name.symlink_to(actual)
            self.assertRaises(ValueError, builder.plan, self.archives, self.source, self.output)
            name.unlink()
            actual.rename(name)

    def test_abi_mismatch_before_output_creation(self):
        with patch.object(builder.subprocess, 'check_output', return_value='1.28.6\n'):
            self.assertRaises(ValueError, builder.build, self.archives, self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_failed_patch_retains_evidence(self):
        with patch.object(builder.subprocess, 'run', return_value=type('Result', (), {'returncode': 1})()):
            self.assertRaises(ValueError, builder.build, self.archives, self.source,
                              self.output, prepare_only=True)
        self.assertEqual(json.loads((self.output / 'receipt.json').read_text())['state'], 'failed')
        self.assertTrue((self.output / 'fixture-patch-check.log').is_file())

    def test_successful_noop_cannot_count_as_applied_patch(self):
        with patch.object(builder.subprocess, 'run', return_value=type('Result', (), {'returncode': 0})()):
            self.assertRaises(ValueError, builder.build, self.archives, self.source,
                              self.output, prepare_only=True)
        self.assertEqual(json.loads((self.output / 'receipt.json').read_text())['state'], 'failed')

    def test_parent_git_repo_and_host_git_environment_do_not_redirect_patch(self):
        import subprocess
        subprocess.run(['git', 'init', '-q', str(self.base)], check=True)
        with patch.dict(builder.os.environ, {'GIT_DIR': '/nonexistent-git-dir',
                                            'GIT_WORK_TREE': '/nonexistent-work-tree'}):
            receipt = builder.build(self.archives, self.source, self.output, prepare_only=True)
        self.assertEqual(receipt['state'], 'sources-prepared')

    def test_jobs_bounds_and_no_install_or_tests_in_build_commands(self):
        for jobs in [0, 33, True, 1.5]:
            self.assertRaises(ValueError, builder.compile_commands, self.item, self.base, jobs)
        item = dict(self.item, root='fixture-' + builder.VERSION)
        commands = builder.compile_commands(item, self.base, 2)
        self.assertIn('--wrap-mode=nofallback', commands[0])
        self.assertIn('-Dauto_features=disabled', commands[0])
        self.assertEqual(commands[1][-1], self.item['target'])
        self.assertNotIn('install', [word for command in commands for word in command])


@unittest.skipUnless(shutil.which('cc') and shutil.which('patchelf'), 'C compiler and patchelf required')
class ElfExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / 'build.so'
        source = self.root / 'test.c'
        source.write_text('#include <stdio.h>\nint test(void) { return puts("test"); }\n')
        subprocess.run(['cc', '-shared', '-fPIC', str(source), '-o', str(self.binary),
                        '-Wl,-rpath=/private/build-tree/sdk'], check=True, capture_output=True)
        self.destination = self.root / 'export.so'

    def test_real_elf_rpath_removed_without_modifying_build_artifact(self):
        old = builder.sha256(self.binary)
        result = builder.finalize_plugin(self.binary, self.destination)
        self.assertTrue(result['build_rpath_removed'])
        self.assertEqual(builder.sha256(self.binary), old)
        self.assertEqual(result['unmodified_build_artifact_sha256'], old)
        self.assertIn('libc.so.6', result['needed_libraries'])
        self.assertEqual(subprocess.check_output(['patchelf', '--print-rpath', str(self.destination)], text=True).strip(), '')

    def test_existing_plugin_not_overwritten(self):
        self.destination.write_bytes(b'preserve')
        self.assertRaises(ValueError, builder.finalize_plugin, self.binary, self.destination)
        self.assertEqual(self.destination.read_bytes(), b'preserve')

    def test_non_elf_input_rejected_before_copy(self):
        self.binary.write_bytes(b'not ELF')
        self.assertRaises(ValueError, builder.finalize_plugin, self.binary, self.destination)
        self.assertFalse(self.destination.exists())

    def test_remaining_rpath_cannot_be_accepted(self):
        with patch.object(builder.subprocess, 'check_output', return_value='/still/private'):
            self.assertRaises(ValueError, builder.finalize_plugin, self.binary, self.destination)


if __name__ == '__main__':
    unittest.main()
