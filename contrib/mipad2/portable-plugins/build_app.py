#!/usr/bin/env python3
"""Build a pinned Snapshot commit into a new private package tree, offline/locked."""
import argparse
import io
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import stat
import subprocess
import tarfile
from build_plugins import archive_members, require, sha256, verify_build_abi


def build(source_root, source_commit, output, prefix, cargo_cache, target_cache=None, jobs=2):
    require(re.fullmatch(r'[0-9a-f]{40}', source_commit or ''), 'Explicit source commit SHA required')
    require(isinstance(jobs, int) and not isinstance(jobs, bool) and 1 <= jobs <= 16, 'jobs must be1..16')
    source_root, output = Path(source_root).resolve(), Path(output)
    require(not output.is_symlink(), 'Symlinked output')
    output = output.resolve()
    require(not output.exists() and source_root not in output.parents, 'Output must be new and outside source')
    prefix = Path(prefix)
    require(prefix.parent == Path('/usr/local/lib') and re.fullmatch(r'mipad2-camera-source-[a-z0-9-]+', prefix.name),
            'Explicit versioned private application prefix required')
    cargo_cache = Path(cargo_cache).resolve()
    require((cargo_cache/'registry').is_dir() and cargo_cache not in output.parents, 'Cached Cargo registry required')
    if target_cache is not None:
        target_cache = Path(target_cache).resolve()
        require(target_cache.is_dir() and target_cache not in output.parents, 'Invalid target cache')
    require(shutil.which('patchelf') is not None, 'patchelf required')
    gst = verify_build_abi()
    git = lambda *args: subprocess.check_output(['git', '--no-replace-objects', '-C', str(source_root), *args], text=True).strip()
    require(git('rev-parse', source_commit+'^{commit}') == source_commit, 'Source object is not requested commit')
    cargo = shutil.which('cargo')
    require(cargo is not None, 'Cargo missing')
    archive = subprocess.check_output(['git', '--no-replace-objects', '-C', str(source_root), 'archive', '--format=tar', '--prefix=snapshot/', source_commit])
    output.mkdir(parents=True, exist_ok=False)
    sources = output/'sources'
    sources.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(sources, members=archive_members(tar, 'snapshot'), filter='data')
    src = sources/'snapshot'
    lock_sha = sha256(src/'Cargo.lock')
    work = output/'build'
    work.mkdir()
    home = work/'cargo-home'
    home.mkdir()
    # Copy only source caches, never credentials/configuration from a Cargo home.
    for name in ['registry', 'git']:
        if (cargo_cache/name).is_dir():
            shutil.copytree(cargo_cache/name, home/name, symlinks=True)
    if target_cache is not None:
        shutil.copytree(target_cache, work/'cargo-target', symlinks=True)
    tools = output/'tool-bin'
    tools.mkdir()
    wrapper = tools/'cargo'
    wrapper.write_text('#!/bin/sh\nif [ "$1" = build ]; then shift; exec '+shlex.quote(cargo)+' build --locked "$@"; fi\nexec '+shlex.quote(cargo)+' "$@"\n')
    wrapper.chmod(0o755)
    env = {k:v for k,v in os.environ.items() if not k.startswith('CARGO_PROFILE_') and k not in
           ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']}
    env.update(PATH=str(tools)+':'+env['PATH'], CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS=str(jobs))
    package = output/'package'
    receipt = dict(schema=1, state='building', source_commit=source_commit,
                   source_tree=git('rev-parse', source_commit+'^{tree}'), recipe_sha256=sha256(__file__),
                   prefix=str(prefix), cargo_lock_sha256=lock_sha, gstreamer_build_abi=gst,
                   cargo_locked=True, cargo_offline=True, cargo_profile='release',
                   source_manifest_release_lto=True, system_install=False,
                   canonical_checkout_modified=False, launcher_included=False,
                   device_tested=False, image_qualified=False, commands=[])

    def save():
        (output/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')

    def run(tag, command, timeout=1800):
        with (output/(tag+'.log')).open('xb') as stream:
            result = subprocess.run(command, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
        receipt['commands'].append(dict(tag=tag, args=command, code=result.returncode))
        save()
        require(result.returncode == 0, tag+' failed; inspect retained logs')

    save()
    try:
        run('configure', ['meson','setup',str(work),str(src),'--prefix='+str(prefix),
                          '--buildtype=release','-Dprofile=default','-Dx11=enabled'])
        run('compile', ['ninja','-C',str(work),'-j'+str(jobs)])
        require(sha256(src/'Cargo.lock') == lock_sha, 'Cargo.lock changed')
        run('stage', ['meson','install','-C',str(work),'--destdir',str(package),'--no-rebuild'])
        staged = package/str(prefix).lstrip('/')
        binary = staged/'bin/snapshot'
        require(binary.is_file() and not binary.is_symlink(), 'App binary missing')
        original = sha256(work/'src/snapshot')
        run('remove-build-rpath', ['patchelf','--remove-rpath',str(binary)])
        require(not subprocess.check_output(['patchelf','--print-rpath',str(binary)],text=True).strip(), 'App RPATH remains')
        require(sha256(work/'src/snapshot') == original, 'Build artifact changed')
        run('compile-private-schemas', ['glib-compile-schemas',str(staged/'share/glib-2.0/schemas')])
        files = {}
        for path in sorted(package.rglob('*')):
            require(not path.is_symlink(), 'Unexpected staged symlink')
            if path.is_file():
                files[str(path.relative_to(package))] = dict(sha256=sha256(path), bytes=path.stat().st_size,
                                                            mode=oct(stat.S_IMODE(path.stat().st_mode)))
        require(len(files)>10 and sha256(src/'Cargo.lock') == lock_sha, 'Incomplete package')
        receipt.update(state='source-application-built-staged-host-only', files=files,
                       application_sha256=sha256(binary), build_binary_unchanged=True,
                       rpath_absent=True, no_source_cache_credentials_copied=True)
        save()
        return receipt
    except BaseException as error:
        receipt.update(state='failed',error_type=type(error).__name__)
        save()
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', required=True, type=Path)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--prefix', required=True)
    parser.add_argument('--cargo-cache', required=True, type=Path)
    parser.add_argument('--target-cache', type=Path)
    parser.add_argument('--jobs', default=2, type=int)
    a=parser.parse_args()
    print(json.dumps(build(a.source_root,a.source_commit,a.output,a.prefix,a.cargo_cache,a.target_cache,a.jobs),indent=2))
