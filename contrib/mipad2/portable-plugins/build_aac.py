#!/usr/bin/env python3
"""Source-build the matching AAC/libav plugin privately; never deploy SDK libraries."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
from build_plugins import require, checked_file, archive_members, sha256, verify_build_abi, finalize_plugin
import tarfile

ARCHIVE = 'gst-libav-1.28.7.tar.xz'
ARCHIVE_SHA256 = '58da51dd39ecf1cf6faade34cc6412001be2e2e145bca8ae0f45336f60a36ab2'
HELPER_SHA256 = '854e33239c0e358e708efc334d103cd4da31c743cc8feff461f0f7e325c3915e'
FFMPEG_ABI = {'libavcodec': '63.1.101', 'libavformat': '63.1.101',
              'libavfilter': '12.1.101', 'libavutil': '61.1.101'}


def ffmpeg_versions():
    versions = {}
    for name, expected in FFMPEG_ABI.items():
        actual = subprocess.check_output(['pkg-config', '--modversion', name], text=True).strip()
        require(actual == expected, f'FFmpeg ABI mismatch: {name}={actual}, need {expected}')
        versions[name] = actual
    return versions


def commands(source, output, jobs):
    require(isinstance(jobs, int) and not isinstance(jobs, bool) and 1 <= jobs <= 32,
            'jobs must be in 1..32')
    build = Path(output) / 'build'
    return [['meson', 'setup', str(build), str(source), '--wrap-mode=nofallback',
             '--buildtype=release', '-Dauto_features=disabled', '-Ddoc=disabled', '-Dtests=disabled'],
            ['ninja', '-C', str(build), '-j' + str(jobs), 'ext/libav/libgstlibav.so']]


def build(archive, output, jobs=2, prepare_only=False):
    require(isinstance(jobs, int) and not isinstance(jobs, bool) and 1 <= jobs <= 32,
            'jobs must be in 1..32')
    archive, output = Path(archive), Path(output)
    require(not output.is_symlink(), 'Symlinked output')
    output = output.resolve()
    require(not output.exists(), 'Output must be new')
    helper = Path(__file__).with_name('build_plugins.py')
    checked_file(helper, HELPER_SHA256, 1024**2)
    checked_file(archive, ARCHIVE_SHA256, 8 * 1024**2)
    with tarfile.open(archive, 'r:xz') as tar:
        archive_members(tar, 'gst-libav-1.28.7')
    gst = {} if prepare_only else verify_build_abi()
    ffmpeg = {} if prepare_only else ffmpeg_versions()
    if not prepare_only:
        require(shutil.which('patchelf') is not None, 'patchelf required')
    output.mkdir(parents=True, exist_ok=False)
    source = output / 'source'
    source.mkdir()
    receipt = dict(schema=1, state='preparing', upstream_version='1.28.7',
                   archive_url='https://gstreamer.freedesktop.org/src/gst-libav/' + ARCHIVE,
                   archive_sha256=ARCHIVE_SHA256, recipe_sha256=sha256(__file__),
                   helper_sha256=HELPER_SHA256, gstreamer_build_abi=gst,
                   ffmpeg_build_abi=ffmpeg, system_install=False, sdk_libraries_deployed=False,
                   device_tested=False, image_qualified=False)

    def save():
        (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')

    save()
    try:
        with tarfile.open(archive, 'r:xz') as tar:
            tar.extractall(source, members=archive_members(tar, 'gst-libav-1.28.7'), filter='data')
        if prepare_only:
            receipt['state'] = 'aac-sources-prepared'
        else:
            for index, command in enumerate(commands(source / 'gst-libav-1.28.7', output, jobs)):
                with (output / f'build-{index}.log').open('xb') as stream:
                    result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, timeout=900)
                require(result.returncode == 0, f'AAC build command {index} failed; logs retained')
            plugin_dir = output / 'plugins'
            plugin_dir.mkdir()
            exported = finalize_plugin(output / 'build/ext/libav/libgstlibav.so', plugin_dir / 'libgstlibav.so')
            require({'libavcodec.so.63', 'libavutil.so.61'} <= set(exported['needed_libraries']),
                    'Wrong AAC codec/util SONAME dependencies')
            receipt.update(state='aac-plugin-built-host-only', plugin='plugins/libgstlibav.so', **exported)
        save()
        return receipt
    except BaseException as error:
        receipt.update(state='failed', error_type=type(error).__name__)
        save()
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--jobs', default=2, type=int)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    print(json.dumps(build(args.archive, args.output, args.jobs, args.prepare_only), indent=2))
