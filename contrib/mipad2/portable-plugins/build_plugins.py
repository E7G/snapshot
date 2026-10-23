#!/usr/bin/env python3
"""Build only the three functional Mi Pad 2 GStreamer plugins in a new private tree.

No system install, network fetch, app launch, policy changes, diagnostic plugins,
or image qualification. Supply official archives with the pinned names/hashes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile

VERSION = '1.28.7'
COMPONENTS = (
    {'name': 'gst-plugins-base',
     'sha256': 'ed6e5410f496d171818763af2265e7977154bc7f9b827e98acf8c5bed21dd5a7',
     'patch': 'encoding-eos-reset/0001-reset-stale-combiner-eos-on-ready.patch',
     'patch_sha256': 'b7ae185ba088bfbed3c8052afcc4a9f7afeaa19c02e9f4762e62e4049f66a13e',
     'feature': 'encoding', 'target': 'gst/encoding/libgstencoding.so',
     'changed_file': 'gst/encoding/gststreamcombiner.c',
     'patched_sha256': '913fe07df005b41b7ebe0caa8f6451b060a94b3b6ce7ac737a3ab237b643b915',
     'extra': ['-Dtools=disabled', '-Dintrospection=disabled']},
    {'name': 'gst-plugins-good',
     'sha256': '87256969c82cf3bc8574301f3e7044a90de0ac500a5a27d8ba38c4dde894dd8b',
     'patch': 'videoflip-identity/0001-initialize-default-identity-passthrough.patch',
     'patch_sha256': 'cd2bf5abbd0c40bfa30c4dc72322702705268c3a507c1382697f3487eeeed8f2',
     'feature': 'videofilter', 'target': 'gst/videofilter/libgstvideofilter.so',
     'changed_file': 'gst/videofilter/gstvideoflip.c',
     'patched_sha256': '5f8ea46ea3915d568a633986f228f770cb05f2a4b435f59125f29e4aaff7f096',
     'extra': []},
    {'name': 'gst-plugins-bad',
     'sha256': 'dc525383c18b2c265bbe6a43d498656cd918aaa130aa4e3abeabcdaa741c3ffe',
     'patch': 'va-async-depth/0001-proposed-opt-in-live-two-frame-output.patch',
     'patch_sha256': '3ce3a569feb997396c0f69cdcf5dec1e71a7ffb2c51a7a2582dd0a222ef8476c',
     'extra_patches': [{'patch': 'va-frame-rate/0001-pack-bounded-rational-frame-rate.patch',
                       'patch_sha256': 'ca5618cfe40717d59a08e3856b86ad4314d1e609252ed681ebf4e7f941695192',
                       'changed_file': 'sys/va/gstvabaseenc.c',
                       'patched_sha256': 'cd330e257d494c51eb6a10aa8def9cd939f82d8ee234df4dfcb60c2afeedfd8f'}],
     'feature': 'va', 'target': 'sys/va/libgstva.so',
     'changed_file': 'sys/va/gstvah264enc.c',
     'patched_sha256': '44b328c0c66bd4cf9dfa10e56ee0cadb444d2a0ac642737dc7ef7669832c7ecb',
     'extra': ['-Ddrm=enabled', '-Dtools=disabled', '-Dintrospection=disabled']},
)


def require(value, message):
    if not value:
        raise ValueError(message)


def sha256(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def checked_file(path, expected, maximum):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), f'Not a regular input: {path}')
    require(0 < path.stat().st_size <= maximum, f'Input size outside bound: {path}')
    require(sha256(path) == expected, f'Input hash mismatch: {path}')
    return path


def archive_members(archive, root):
    """Fail before extraction on traversal, links, devices, duplicates or bombs."""
    members = archive.getmembers()
    require(0 < len(members) <= 30000, 'Archive entry count outside bound')
    seen = set()
    total = 0
    for member in members:
        path = PurePosixPath(member.name)
        require(not path.is_absolute() and '..' not in path.parts and '\\' not in member.name,
                'Invalid archive path')
        require(path.parts and path.parts[0] == root, 'Unexpected archive root')
        require(member.isdir() or member.isfile(), 'Archive links/special entries forbidden')
        normalized = str(path)
        require(normalized not in seen, 'Duplicate archive entry')
        seen.add(normalized)
        require(0 <= member.size <= 64 * 1024**2, 'Archive member size outside bound')
        total += member.size
    require(total <= 256 * 1024**2, 'Archive unpacked size outside bound')
    return members


def plan(archive_dir, source_root, output):
    archive_dir, source_root = Path(archive_dir).resolve(), Path(source_root).resolve()
    raw_output = Path(output)
    require(not raw_output.is_symlink(), 'Symlinked output forbidden')
    output = raw_output.resolve()
    require(not output.exists(), 'Output must not already exist')
    require(output not in (source_root, archive_dir) and source_root not in output.parents
            and archive_dir not in output.parents, 'Output must be outside input trees')
    prepared = []
    for component in COMPONENTS:
        root = component['name'] + '-' + VERSION
        filename = root + '.tar.xz'
        path = checked_file(archive_dir / filename, component['sha256'], 32 * 1024**2)
        patch = checked_file(source_root / component['patch'], component['patch_sha256'], 1024**2)
        additional = []
        for extra in component.get('extra_patches', []):
            extra_file = checked_file(source_root / extra['patch'],
                                      extra['patch_sha256'], 1024**2)
            additional.append(dict(extra, patch_file=str(extra_file)))
        with tarfile.open(path, 'r:xz') as archive:
            archive_members(archive, root)
        prepared.append(dict(component, archive=str(path), patch_file=str(patch),
                             additional_patches=additional, root=root, url=f'https://gstreamer.freedesktop.org/src/{component["name"]}/{filename}'))
    return output, prepared


def compile_commands(item, work, jobs):
    require(isinstance(jobs, int) and not isinstance(jobs, bool) and 1 <= jobs <= 32,
            'jobs must be in 1..32')
    work = Path(work)
    build = work / ('build-' + item['name'])
    flags = ['--wrap-mode=nofallback', '--buildtype=release', '-Dauto_features=disabled',
             '-D' + item['feature'] + '=enabled', '-Dtests=disabled', '-Dexamples=disabled',
             '-Ddoc=disabled', '-Dnls=disabled', *item['extra']]
    return [['meson', 'setup', str(build), str(work / item['root']), *flags],
            ['ninja', '-C', str(build), '-j' + str(jobs), item['target']]]


def verify_build_abi():
    versions = {}
    for name in ['gstreamer-1.0', 'gstreamer-base-1.0', 'gstreamer-video-1.0',
                 'gstreamer-plugins-bad-1.0']:
        version = subprocess.check_output(['pkg-config', '--modversion', name], text=True).strip()
        require(version == VERSION, f'Build ABI mismatch: {name}={version}, need {VERSION}')
        versions[name] = version
    return versions


def finalize_plugin(binary, destination):
    """Never ship Meson's build-tree RPATH or mutate the original build artifact."""
    binary, destination = Path(binary), Path(destination)
    require(binary.is_file() and not binary.is_symlink(), 'Missing compiled plugin')
    require(not destination.exists() and not destination.is_symlink(), 'Plugin output already exists')
    with binary.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'Expected ELF plugin')
    before = sha256(binary)
    shutil.copyfile(binary, destination)
    destination.chmod(0o755)
    subprocess.run(['patchelf', '--remove-rpath', str(destination)], check=True,
                   capture_output=True, timeout=30)
    rpath = subprocess.check_output(['patchelf', '--print-rpath', str(destination)],
                                    text=True, timeout=30).strip()
    require(not rpath, 'Build RPATH remains in exported plugin')
    needed = subprocess.check_output(['patchelf', '--print-needed', str(destination)],
                                     text=True, timeout=30).splitlines()
    require(all(name and '/' not in name and '\\' not in name for name in needed),
            'Nonportable dependency filename')
    require(sha256(binary) == before, 'Original build artifact changed during export')
    return dict(binary_sha256=sha256(destination), bytes=destination.stat().st_size,
                build_rpath_removed=True, needed_libraries=needed,
                unmodified_build_artifact_sha256=before)


def build(archive_dir, source_root, output, jobs=2, prepare_only=False):
    require(isinstance(jobs, int) and not isinstance(jobs, bool) and 1 <= jobs <= 32,
            'jobs must be in 1..32')
    output, items = plan(archive_dir, source_root, output)
    abi = {} if prepare_only else verify_build_abi()
    if not prepare_only:
        require(shutil.which('patchelf') is not None, 'patchelf required for portable export')
    output.mkdir(parents=True, exist_ok=False)
    work = output / 'work'
    work.mkdir()
    receipt = dict(schema=1, state='preparing', upstream_version=VERSION,
                   recipe_sha256=sha256(__file__), build_abi=abi, components=[],
                   system_install=False, diagnostic_plugins_included=False,
                   app_included=False, launcher_changed=False,
                   device_tested=False, image_qualified=False)

    def save():
        (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')

    def run(command, log, cwd=None, env=None):
        with log.open('xb') as stream:
            result = subprocess.run(command, cwd=cwd, stdout=stream,
                                    stderr=subprocess.STDOUT, timeout=900, env=env)
        require(result.returncode == 0, f'Command failed; retained log: {log}')

    save()
    try:
        for item in items:
            with tarfile.open(item['archive'], 'r:xz') as archive:
                members = archive_members(archive, item['root'])
                archive.extractall(work, members=members, filter='data')
            src = work / item['root']
            # No .git dependency, fuzzy patch application, or dirty checkout input.
            git_env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
            git_env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
                           GIT_CEILING_DIRECTORIES=str(work))
            for mode in ['--check', '--apply']:
                command = ['git', '-c', 'core.autocrlf=false', 'apply']
                if mode == '--check':
                    command.append('--check')
                command.append(item['patch_file'])
                run(command, output / (item['name'] + '-patch-' + mode[2:] + '.log'), src, git_env)
            for index, extra in enumerate(item['additional_patches']):
                for mode in ['--check', '--apply']:
                    command = ['git', '-c', 'core.autocrlf=false', 'apply']
                    if mode == '--check':
                        command.append('--check')
                    command.append(extra['patch_file'])
                    run(command, output / (item['name'] + f'-extra{index}-patch-' + mode[2:] + '.log'), src, git_env)
                require(sha256(src / extra['changed_file']) == extra['patched_sha256'],
                        'Additional patched source hash mismatch: ' + item['name'])
            require(sha256(src / item['changed_file']) == item['patched_sha256'],
                    'Patched source hash mismatch: ' + item['name'])
            entry = dict(name=item['name'], url=item['url'], archive_sha256=item['sha256'],
                         patch=item['patch'], patch_sha256=item['patch_sha256'],
                         changed_file=item['changed_file'], patched_sha256=item['patched_sha256'],
                         target=item['target'], compiled=False)
            entry['additional_patches'] = [{k: v for k, v in extra.items() if k != 'patch_file'}
                                           for extra in item['additional_patches']]
            receipt['components'].append(entry)
            save()
        if not prepare_only:
            plugins = output / 'plugins'
            plugins.mkdir()
            for item, entry in zip(items, receipt['components']):
                commands = compile_commands(item, work, jobs)
                for index, command in enumerate(commands):
                    run(command, output / (item['name'] + f'-build-{index}.log'))
                binary = work / ('build-' + item['name']) / item['target']
                destination = plugins / binary.name
                exported = finalize_plugin(binary, destination)
                entry.update(compiled=True, plugin='plugins/' + binary.name, **exported)
                save()
        receipt['state'] = 'sources-prepared' if prepare_only else 'plugins-built-host-only'
        save()
        return receipt
    except BaseException as error:
        receipt.update(state='failed', error_type=type(error).__name__)
        save()
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive-dir', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--jobs', default=2, type=int)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    print(json.dumps(build(args.archive_dir, Path(__file__).resolve().parent.parent,
                           args.output, args.jobs, args.prepare_only), indent=2))
