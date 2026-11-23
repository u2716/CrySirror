#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Explicit, bounded Vita3K execution of CrySirror's shader compiler only."""
import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import tempfile
import threading
import time
import zipfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('sources', type=Path, help='shader-tools psv export directory')
    parser.add_argument('vpk', type=Path, help='crysirror_vita_shaderc.vpk')
    parser.add_argument('output', type=Path, help='compiled GXP/source/receipt directory')
    parser.add_argument('--vita3k', required=True, type=Path)
    parser.add_argument('--fs-root', required=True, type=Path, help='existing Vita3K fs, used read-only')
    parser.add_argument('--config', required=True, type=Path, help='existing Vita3K config, used read-only')
    parser.add_argument('--log-file', required=True, type=Path, help='Vita3K global log to guard, never truncated by this tool')
    parser.add_argument('--jobs', type=int, choices=range(1, 5), default=1)
    parser.add_argument('--timeout', type=float, default=90, help='seconds per program')
    parser.add_argument('--program', action='append', help='compile only this exact program stem')
    parser.add_argument('--optimization', type=int, choices=range(4))
    args = parser.parse_args()
    module = args.fs_root / 'ur0/data/libshacccg.suprx'
    if not module.is_file():
        parser.error('fs-root must contain ur0/data/libshacccg.suprx')
    if not 1 <= args.timeout <= 600:
        parser.error('timeout must be between 1 and 600 seconds')
    files = (args.sources / 'programs.txt').read_text().splitlines()
    if not files or any(not re.fullmatch(r'[A-Za-z0-9_]+\.cg', f) for f in files):
        parser.error('invalid exported programs.txt')
    names = [Path(f).stem for f in files]
    manifest_file = args.sources / 'manifest.json'
    entries = json.loads(manifest_file.read_text())['entries'] if manifest_file.exists() else []
    levels = {Path(e['file']).stem: e.get('compiler_optimization', 3) for e in entries if str(e.get('file', '')).endswith('.cg')}
    if any(v not in range(4) for v in levels.values()):
        parser.error('invalid compiler optimization policy')
    def optimization(name):
        return args.optimization if args.optimization is not None else levels.get(name, 3)

    if args.program:
        unknown = set(args.program) - set(names)
        if unknown:
            parser.error(f'unknown programs: {sorted(unknown)}')
        names = [n for n in names if n in args.program]
    args.output.mkdir(parents=True, exist_ok=True)
    if shutil.disk_usage(args.output).free < 2 * 1024**3:
        parser.error('at least 2 GiB free space is required before compiler execution')
    module_hash = hashlib.sha256(module.read_bytes()).hexdigest()
    stamp = args.output / 'compiler-module.sha256'
    if stamp.exists() and stamp.read_text().strip() != module_hash:
        parser.error('compiler module changed; use a separate output directory')
    stamp.write_text(module_hash + '\n')
    stop = threading.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda *_: stop.set())
    initial_log = args.log_file.stat().st_size if args.log_file.exists() else 0
    config = args.config.read_text()
    with zipfile.ZipFile(args.vpk) as archive:
        if any(Path(n).is_absolute() or '..' in Path(n).parts for n in archive.namelist()):
            parser.error('unsafe VPK path')
    # Vita3K's SDL frontend can render to a headless surface. Its console flag
    # alone still opens a window on builds using that frontend.
    env = dict(os.environ, SDL_VIDEO_DRIVER='offscreen', SDL_AUDIO_DRIVER='dummy')

    def valid(folder, name):
        try:
            gxp = (folder / (name + '.gxp')).read_bytes()
            receipt = dict(line.split('=', 1) for line in (folder / (name + '.receipt')).read_text().splitlines())
            return ((folder / (name + '.cg')).read_bytes() == (args.sources / (name + '.cg')).read_bytes()
                    and len(gxp) >= 64 and gxp[:4] == b'GXP\0'
                    and len(gxp) == int(receipt['program_bytes'])
                    and receipt['optimization'] == str(optimization(name))
                    and all(receipt[k] == '0' for k in ('fastmath', 'fastprecision', 'fastint')))
        except (OSError, ValueError, KeyError):
            return False

    with tempfile.TemporaryDirectory(prefix='crysirror-psv-compiler-') as temporary:
        base = Path(temporary)

        def worker(index):
            root = base / str(index)
            fs = root / 'fs'
            for path in ['ux0/data', 'ux0/user/00', 'ur0/data']:
                (fs / path).mkdir(parents=True, exist_ok=True)
            shutil.copyfile(module, fs / 'ur0/data/libshacccg.suprx')
            for name in ['vs0', 'sa0', 'os0']:
                source = (args.fs_root / name).resolve()
                if source.exists():
                    (fs / name).symlink_to(source, target_is_directory=True)
            private_config = re.sub(r'(?m)^pref-path:.*$', f'pref-path: {fs}', config)
            private_config = re.sub(r'(?m)^log-level:.*$', 'log-level: 6', private_config)
            cfg = root / 'config.yml'
            cfg.write_text(private_config)
            out = fs / 'ux0/data/crysirror_shaderc'
            report = out / 'report.txt'
            log = root / 'compiler.log'
            missing = []
            for name in names[index::args.jobs]:
                if stop.is_set():
                    missing.append(name)
                    continue
                if valid(args.output, name):
                    continue
                (fs / 'ux0/data/crysirror_shaderc-selection.txt').write_text(name + '\n')
                (fs / 'ux0/data/crysirror_shaderc-optimization.txt').write_text(str(optimization(name)))
                if report.exists():
                    report.unlink()
                with log.open('w') as stderr:
                    process = subprocess.Popen([str(args.vita3k.resolve()), '--console', '--frontend', 'sdl',
                                                '-c', str(cfg), '-f', '-w', '-l', '6', str(args.vpk.resolve())],
                                               stdout=stderr, stderr=subprocess.STDOUT, env=env)
                    deadline = time.monotonic() + args.timeout
                    try:
                        while time.monotonic() < deadline and not stop.is_set():
                            log_size = args.log_file.stat().st_size if args.log_file.exists() else 0
                            if (log_size > initial_log + 16 * 1024**2 or log.stat().st_size > 4 * 1024**2
                                    or shutil.disk_usage(args.output).free < 2 * 1024**3):
                                stop.set()
                                print('Compiler log/disk limit reached', flush=True)
                                break
                            if report.exists() and 'DONE failed=' in report.read_text(errors='replace'):
                                break
                            if process.poll() is not None:
                                break
                            time.sleep(0.1)
                    finally:
                        if process.poll() is None:
                            process.terminate()
                            try:
                                process.wait(timeout=5)
                            except subprocess.TimeoutExpired:
                                process.kill()
                                process.wait()
                if report.exists():
                    shutil.copyfile(report, args.output / (name + '.report'))
                if valid(out, name):
                    for suffix in ['.cg', '.gxp', '.receipt']:
                        shutil.copyfile(out / (name + suffix), args.output / (name + suffix))
                    print(f'Compiled {name}', flush=True)
                else:
                    missing.append(name)
                    if not report.exists():
                        shutil.copyfile(log, args.output / (name + '.log'))
                    print(f'Failed {name}', flush=True)
            return missing

        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            failures = [name for group in pool.map(worker, range(args.jobs)) for name in group]
    (args.output / 'missing.json').write_text(json.dumps(failures, indent=2) + '\n')
    print(f'{len(names) - len(failures)}/{len(names)} programs available')
    return 1 if failures else 0


if __name__ == '__main__':
    raise SystemExit(main())
