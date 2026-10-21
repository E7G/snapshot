import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import build_app as app


class AppBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = self.base/'source'
        self.source.mkdir()
        self.cache = self.base/'cache'
        (self.cache/'registry').mkdir(parents=True)
        (self.cache/'credentials.toml').write_text('SECRET')
        (self.cache/'config.toml').write_text('SECRET')
        self.out = self.base/'output'
        self.prefix = '/usr/local/lib/mipad2-camera-source-'+'a'*12
        for args in [['init','-q'], ['config','user.name','Fixture'], ['config','user.email','fixture@example.invalid']]:
            subprocess.run(['git','-C',str(self.source),*args],check=True,capture_output=True)
        (self.source/'Cargo.lock').write_text('fixture')
        subprocess.run(['git','-C',str(self.source),'add','Cargo.lock'],check=True,capture_output=True)
        subprocess.run(['git','-C',str(self.source),'commit','-qm','Fixture'],check=True,capture_output=True)
        self.commit = subprocess.check_output(['git','-C',str(self.source),'rev-parse','HEAD'],text=True).strip()
        self.abi = patch.object(app,'verify_build_abi',return_value={'gstreamer-1.0':'1.28.7'})
        self.abi.start()
        self.addCleanup(self.abi.stop)
        self.which = patch.object(app.shutil,'which',side_effect=lambda n:'/usr/bin/'+n)
        self.which.start()
        self.addCleanup(self.which.stop)

    def build(self, **values):
        args = dict(source_root=self.source,source_commit=self.commit,output=self.out,
                    prefix=self.prefix,cargo_cache=self.cache)
        args.update(values)
        return app.build(**args)

    def rejected(self, **values):
        self.assertRaises(ValueError,self.build,**values)
        self.assertFalse(self.out.exists())

    def test_explicit_full_commit_required(self):
        for value in ['HEAD','main',self.commit[:12],'g'*40]:
            self.rejected(source_commit=value)

    def test_jobs_bounds(self):
        for value in [0,17,True,1.5]:
            self.rejected(jobs=value)

    def test_output_not_existing(self):
        self.out.mkdir()
        (self.out/'keep').write_text('keep')
        self.assertRaises(ValueError,self.build)
        self.assertEqual((self.out/'keep').read_text(),'keep')

    def test_output_not_in_checkout(self):
        self.rejected(output=self.source/'output')
        self.assertFalse((self.source/'output').exists())

    def test_output_symlink(self):
        self.out.symlink_to(self.base/'missing')
        self.assertRaises(ValueError,self.build)
        self.assertFalse((self.base/'missing').exists())

    def test_private_prefix_required(self):
        for value in ['/usr','/usr/local/lib/normal','/usr/local/lib/mipad2-camera-source-X']:
            self.rejected(prefix=value)

    def test_cargo_sources_required(self):
        self.rejected(cargo_cache=self.base/'missing')

    def test_optional_target_cache_must_exist(self):
        self.rejected(target_cache=self.base/'missing')

    def test_configure_failure_retained_no_credentials_or_canonical_writes(self):
        old = (self.source/'Cargo.lock').read_bytes()
        real_run = subprocess.run
        def run(args, **kwargs):
            if args[0] == 'git':
                return real_run(args, **kwargs)
            return type('Result',(),{'returncode':1})()
        with patch.object(app.subprocess,'run',side_effect=run):
            self.assertRaises(ValueError,self.build)
        receipt = json.loads((self.out/'receipt.json').read_text())
        self.assertEqual(receipt['state'],'failed')
        self.assertTrue((self.out/'configure.log').is_file())
        self.assertEqual((self.source/'Cargo.lock').read_bytes(),old)
        self.assertFalse((self.source/'src/config.rs').exists())
        self.assertFalse((self.out/'build/cargo-home/credentials.toml').exists())
        self.assertFalse((self.out/'build/cargo-home/config.toml').exists())
        self.assertTrue((self.out/'sources/snapshot/Cargo.lock').is_file())
        self.assertIn('--locked',(self.out/'tool-bin/cargo').read_text())
        self.assertFalse(receipt['device_tested'])
        self.assertFalse(receipt['image_qualified'])


if __name__ == '__main__':
    unittest.main()
