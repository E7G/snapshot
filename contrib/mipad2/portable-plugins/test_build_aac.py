"""AAC source-build contracts, not target audio/device acceptance."""
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import build_aac as builder


class AacContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.archive = self.root / builder.ARCHIVE
        with tarfile.open(self.archive, 'w:xz') as tar:
            data = b'fixture source only\n'
            member = tarfile.TarInfo('gst-libav-1.28.7/file.txt')
            member.size = len(data)
            tar.addfile(member, io.BytesIO(data))
        self.output = self.root / 'output'
        p = patch.object(builder, 'ARCHIVE_SHA256', builder.sha256(self.archive))
        p.start()
        self.addCleanup(p.stop)

    def test_exact_ffmpeg_versions(self):
        with patch.object(builder.subprocess, 'check_output', side_effect=[v+'\n' for v in builder.FFMPEG_ABI.values()]):
            self.assertEqual(builder.ffmpeg_versions(), builder.FFMPEG_ABI)

    def test_wrong_ffmpeg_major_is_not_accepted(self):
        with patch.object(builder.subprocess, 'check_output', return_value='62.28.102\n'):
            self.assertRaises(ValueError, builder.ffmpeg_versions)

    def test_commands_are_release_nofallback_named_target_without_install(self):
        commands = builder.commands(self.root / 'src', self.output, 2)
        self.assertIn('--wrap-mode=nofallback', commands[0])
        self.assertIn('--buildtype=release', commands[0])
        self.assertEqual(commands[1][-1], 'ext/libav/libgstlibav.so')
        self.assertNotIn('install', [word for command in commands for word in command])

    def test_prepare_only_does_not_probe_or_claim_build_abi(self):
        with patch.object(builder, 'verify_build_abi', side_effect=AssertionError('must not run')), \
             patch.object(builder, 'ffmpeg_versions', side_effect=AssertionError('must not run')):
            receipt = builder.build(self.archive, self.output, prepare_only=True)
        self.assertEqual(receipt['state'], 'aac-sources-prepared')
        self.assertEqual(receipt['gstreamer_build_abi'], {})
        self.assertEqual(receipt['ffmpeg_build_abi'], {})
        self.assertFalse(receipt['device_tested'])
        self.assertFalse(receipt['sdk_libraries_deployed'])
        self.assertFalse((self.output/'plugins').exists())
        self.assertTrue((self.output/'source/gst-libav-1.28.7/file.txt').is_file())

    def test_hash_mismatch_before_output_creation(self):
        with patch.object(builder, 'ARCHIVE_SHA256', '0'*64):
            self.assertRaises(ValueError, builder.build, self.archive, self.output, prepare_only=True)
        self.assertFalse(self.output.exists())

    def test_existing_output_unchanged(self):
        self.output.mkdir()
        keep = self.output / 'keep'
        keep.write_text('unchanged')
        self.assertRaises(ValueError, builder.build, self.archive, self.output, prepare_only=True)
        self.assertEqual(keep.read_text(), 'unchanged')

    def test_symlinked_archive_rejected(self):
        actual = self.archive.with_suffix('.actual')
        self.archive.rename(actual)
        self.archive.symlink_to(actual)
        self.assertRaises(ValueError, builder.build, self.archive, self.output, prepare_only=True)
        self.assertFalse(self.output.exists())

    def test_invalid_jobs_rejected(self):
        for jobs in [0, 33, True, 1.5]:
            self.assertRaises(ValueError, builder.commands, self.root, self.output, jobs)
            self.assertRaises(ValueError, builder.build, self.archive, self.output, jobs)
        self.assertFalse(self.output.exists())

    def test_changed_helper_rejected_before_output(self):
        with patch.object(builder, 'HELPER_SHA256', '0'*64):
            self.assertRaises(ValueError, builder.build, self.archive, self.output, prepare_only=True)
        self.assertFalse(self.output.exists())

    def test_wrong_gstreamer_abi_rejected_before_output(self):
        with patch.object(builder, 'verify_build_abi', side_effect=ValueError('wrong ABI')):
            self.assertRaises(ValueError, builder.build, self.archive, self.output)
        self.assertFalse(self.output.exists())

    def test_failed_command_preserves_receipt_and_logs(self):
        with patch.object(builder, 'verify_build_abi', return_value={}), \
             patch.object(builder, 'ffmpeg_versions', return_value=builder.FFMPEG_ABI), \
             patch.object(builder.subprocess, 'run', return_value=type('Result', (), {'returncode': 1})()):
            self.assertRaises(ValueError, builder.build, self.archive, self.output)
        self.assertEqual(json.loads((self.output/'receipt.json').read_text())['state'], 'failed')
        self.assertTrue((self.output/'build-0.log').is_file())


if __name__ == '__main__':
    unittest.main()
