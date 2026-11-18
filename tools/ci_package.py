#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Package CI build outputs; never collect an ambient game or target directory."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def metadata() -> dict:
    commit = os.environ.get('GITHUB_SHA', '')
    if commit and not re.fullmatch(r'[0-9a-f]{40,64}', commit):
        raise ValueError('Invalid source commit')
    package = (ROOT / 'Cargo.toml').read_text().split('[workspace.package]', 1)[1].split('\n[', 1)[0]
    version = re.search(r'(?m)^version\s*=\s*"([^"]+)"', package)
    if not version:
        raise ValueError('Workspace version missing')
    return {'commit': commit, 'repository': os.environ.get('GITHUB_REPOSITORY', 'xmoezzz/CrySirror'),
            'run_id': os.environ.get('GITHUB_RUN_ID', ''), 'version': version.group(1)}


def notices(destination: Path) -> None:
    for name in ('LICENSE', 'NOTICE.md'):
        shutil.copy2(ROOT / name, destination / name)
    info = metadata()
    (destination / 'build.json').write_text(json.dumps(info, indent=2) + '\n', encoding='utf-8')
    source = f"https://github.com/{info['repository']}/tree/{info['commit'] or 'main'}"
    (destination / 'README.txt').write_text(
        'CrySirror pre-release\n\n'
        'Desktop: ./crysirror play /path/to/game (Windows: crysirror.exe play C:\\path\\to\\game).\n'
        'Linux requires the system ALSA, udev, X11/Wayland and graphics-driver libraries.\n'
        'macOS packages contain a CLI executable, without notarization or an installer.\n'
        'Web: python3 web_host.py serve --output . (http://127.0.0.1:8080).\n'
        'Console builds require the corresponding homebrew environment.\n'
        'Build artifacts do not certify game or physical-device compatibility.\n'
        'No user games, fonts or keys are included.\n\n'
        f'Source and build definitions: {source}\n'
        'See LICENSE and NOTICE.md for component notices.\n', encoding='utf-8')


def archive(source: Path, output: Path, name: str, windows: bool = False) -> Path:
    output.mkdir(parents=True, exist_ok=True)
    if windows:
        result = output / f'{name}.zip'
        with zipfile.ZipFile(result, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as package:
            for path in sorted(source.rglob('*')):
                if path.is_file():
                    package.write(path, str(Path(name) / path.relative_to(source)))
    else:
        result = output / f'{name}.tar.gz'
        with tarfile.open(result, 'w:gz') as package:
            package.add(source, arcname=name)
    return result


def desktop(binary: Path, system: str, output: Path) -> Path:
    arch = {'AMD64': 'x86_64', 'x86_64': 'x86_64', 'arm64': 'arm64', 'aarch64': 'arm64'}[platform.machine()]
    if not binary.is_file():
        raise FileNotFoundError(binary)
    with tempfile.TemporaryDirectory(prefix='crysirror-package-') as work:
        stage = Path(work)
        notices(stage)
        shutil.copy2(binary, stage / ('crysirror.exe' if system == 'windows' else 'crysirror'))
        return archive(stage, output, f'crysirror-{system}-{arch}', windows=system == 'windows')


def web(site: Path, output: Path) -> Path:
    if not (site / 'index.html').is_file() or not list(site.rglob('*.wasm')):
        raise ValueError('Incomplete browser host output')
    with tempfile.TemporaryDirectory(prefix='crysirror-package-') as work:
        stage = Path(work)
        shutil.copytree(site, stage, dirs_exist_ok=True)
        notices(stage)
        # The shipped server supplies the shared-memory worker headers.
        shutil.copy2(ROOT / 'tools/web_host.py', stage / 'web_host.py')
        return archive(stage, output, 'crysirror-web', windows=True)


def console(binary: Path, system: str, output: Path) -> Path:
    extension = 'vpk' if system == 'psv' else 'nro'
    files = list(binary.glob(f'*.{extension}')) if binary.is_dir() else [binary]
    if len(files) != 1 or not files[0].is_file() or files[0].suffix != f'.{extension}':
        raise ValueError(f'Expected one {system} package in {binary}')
    output.mkdir(parents=True, exist_ok=True)
    result = output / f'crysirror-{system}.{extension}'
    shutil.copy2(files[0], result)
    return result


def checksums(directory: Path) -> None:
    files = sorted(path for path in directory.iterdir() if path.is_file() and path.name != 'SHA256SUMS')
    if not files:
        raise ValueError('No release assets')
    lines = []
    for path in files:
        digest = hashlib.sha256()
        with path.open('rb') as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                digest.update(chunk)
        digest = digest.hexdigest()
        lines.append(f'{digest}  {path.name}\n')
    (directory / 'SHA256SUMS').write_text(''.join(lines), encoding='utf-8')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    player = commands.add_parser('desktop')
    player.add_argument('--binary', type=Path, required=True)
    player.add_argument('--platform', choices=['linux', 'windows', 'macos'], required=True)
    player.add_argument('--output', type=Path, default=ROOT / 'dist')
    browser = commands.add_parser('web')
    browser.add_argument('--site', type=Path, default=ROOT / 'target/web')
    browser.add_argument('--output', type=Path, default=ROOT / 'dist')
    homebrew = commands.add_parser('console')
    homebrew.add_argument('--input', type=Path, required=True)
    homebrew.add_argument('--platform', choices=['psv', 'switch'], required=True)
    homebrew.add_argument('--output', type=Path, default=ROOT / 'dist')
    checksum = commands.add_parser('checksums')
    checksum.add_argument('directory', type=Path)
    args = parser.parse_args()
    if args.command == 'desktop':
        print(desktop(args.binary, args.platform, args.output))
    elif args.command == 'web':
        print(web(args.site, args.output))
    elif args.command == 'console':
        print(console(args.input, args.platform, args.output))
    else:
        checksums(args.directory)


if __name__ == '__main__':
    main()
