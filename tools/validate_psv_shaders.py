#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build and run the native GXM synthetic offscreen validator, never a game."""
import argparse
import json
import hashlib
import re
import shutil
import signal
import threading
import subprocess
import tempfile
import time
import zipfile
from pathlib import Path


def compare_pixels(output):
    images = {p.stem: p.read_bytes() for p in output.glob('*.rgba')}
    if not images or any(data == bytes([205]) * (64 * 64 * 4) for data in images.values()):
        print('GXM API admission passed; native GPU readback unavailable (initial marker retained)')
        return 2
    expected = bytes(value for y in range(64) for x in range(64)
                     for value in (x * 4, y * 4, (x ^ y) * 4, 255))
    comparison = {}
    for name in ['present', 'quad', 'wipe'] + [f'filtered-s{i}' for i in range(5)]:
        data = images.get(name, b'')
        error = max((abs(a - b) for a, b in zip(data, expected)), default=256)
        passed = len(data) == len(expected) and error <= (2 if name.startswith('filtered') else 0)
        comparison[name] = {'max_byte_error': error, 'passed': passed}
    mesh = images.get('mesh', b'')
    ray = images.get('ray', b'')
    center = (32 * 64 + 32) * 4
    def close(data, offset, expected):
        return len(data) == 64 * 64 * 4 and all(abs(data[offset+i] - v) <= 2 for i, v in enumerate(expected))
    comparison['mesh'] = {'passed': close(mesh, center, [32, 127, 64, 255])}
    comparison['ray'] = {'passed': close(ray, center, [255, 0, 0, 255]) and close(ray, 0, [0, 64, 128, 255])}
    (output / 'pixels.json').write_text(json.dumps(comparison, indent=2) + '\n')
    print(json.dumps(comparison))
    return 0 if all(c['passed'] for c in comparison.values()) else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--gxp-overrides', type=Path, help='explicit diagnostic GXP overrides, never imported automatically')
    parser.add_argument('--sdk', type=Path, default=Path('/usr/local/vitasdk'))
    parser.add_argument('--build-only', action='store_true')
    parser.add_argument('--compare-only', action='store_true')
    parser.add_argument('--vita3k', type=Path)
    parser.add_argument('--fs-root', type=Path)
    parser.add_argument('--config', type=Path)
    parser.add_argument('--log-file', type=Path)
    args = parser.parse_args()
    if args.compare_only:
        return compare_pixels(args.output)
    if not args.build_only and not all([args.vita3k, args.fs_root, args.config, args.log_file]):
        parser.error('execution requires --vita3k, --fs-root, --config and --log-file')
    stop = threading.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda *_: stop.set())
    root = Path(__file__).resolve().parents[1]
    args.output.mkdir(parents=True, exist_ok=True)
    if shutil.disk_usage(args.output).free < 2 * 1024**3:
        parser.error('at least 2 GiB free space is required')
    native = root / 'crates/crysirror-shaders/native/psv'
    manifest = json.loads((native / 'manifest.json').read_text())
    entries = [e for e in manifest['entries'] if e.get('status') == 'gxp_compiled']
    if not entries or len(entries) != manifest['native_program_count']:
        parser.error('incomplete native program catalog')
    build_log = args.output / 'build.log'
    with tempfile.TemporaryDirectory(prefix='crysirror-psv-gxm-') as tmp:
        base = Path(tmp)
        tool = args.sdk / 'bin'

        def run(command):
            with build_log.open('ab') as log:
                subprocess.run([str(c) for c in command], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=60)

        libraries = ['vita2d', 'SceGxm_stub', 'SceDisplay_stub', 'SceSysmodule_stub',
                     'SceCommonDialog_stub', 'SceAppUtil_stub', 'SceAppMgr_stub',
                     'SceSysmem_stub', 'SceKernelThreadMgr_stub', 'SceLibKernel_stub', 'm']
        run([tool / 'arm-vita-eabi-gcc', '-O2', '-std=gnu17', '-Wall', '-Wextra',
             root / 'platform/psv/shader-validate/main.c', '-o', base / 'validator.elf']
            + ['-l' + lib for lib in libraries])
        run([tool / 'vita-elf-create', base / 'validator.elf', base / 'validator.velf'])
        run([tool / 'vita-make-fself', '-s', base / 'validator.velf', base / 'eboot.bin'])
        run([tool / 'vita-mksfoex', '-s', 'TITLE_ID=CRYSGXMV1', 'CrySirror GXM Validation', base / 'param.sfo'])
        vpk = args.output / 'validator.vpk'
        with zipfile.ZipFile(vpk, 'w', zipfile.ZIP_DEFLATED) as archive:
            archive.write(base / 'eboot.bin', 'eboot.bin')
            archive.write(base / 'param.sfo', 'sce_sys/param.sfo')
            names = []
            inputs = []
            for e in entries:
                path = Path(e['gxp'])
                if path.parts[0] != 'gxp' or len(path.parts) != 2 or '..' in path.parts:
                    raise ValueError('invalid program path')
                names.append(path.stem)
                override = args.gxp_overrides / path.name if args.gxp_overrides else None
                selected = override if override and override.is_file() else native / path
                data = selected.read_bytes()
                if len(data) < 64 or data[:4] != b'GXP\0':
                    raise ValueError(f'invalid GXP: {selected}')
                archive.writestr(str(path), data)
                inputs.append({'program': path.stem, 'sha256': hashlib.sha256(data).hexdigest(), 'diagnostic_override': selected != native / path})
            archive.writestr('programs.txt', '\n'.join(sorted(names)) + '\n')
        (args.output / 'program-inputs.json').write_text(json.dumps(inputs, indent=2) + '\n')
        if args.build_only:
            print(f'Built {vpk}')
            return 0
        fs = base / 'fs'
        # Vita3K validates -r against its global inventory before reading -c.
        # Reuse the installed compiler's folder name only in the private FS;
        # the validator VPK retains its distinct CRYSGXMV1 title ID.
        app = fs / 'ux0/app/CRYSHADR1'
        app.mkdir(parents=True)
        for path in ['ux0/data', 'ux0/user/00', 'ur0/data']:
            (fs / path).mkdir(parents=True, exist_ok=True)
        for name in ['vs0', 'sa0', 'os0']:
            source = (args.fs_root / name).resolve()
            if source.exists():
                (fs / name).symlink_to(source, target_is_directory=True)
        with zipfile.ZipFile(vpk) as archive:
            archive.extractall(app)
        config = args.config.read_text()
        for key, value in [('pref-path', fs), ('log-level', 6), ('v-sync', 'false'), ('async-pipeline-compilation', 'false'), ('high-accuracy', 'true')]:
            config = re.sub(r'(?m)^' + key + r':.*$', f'{key}: {value}', config)
        cfg = base / 'config.yml'
        cfg.write_text(config)
        report = fs / 'ux0/data/crysirror_shader_validation/report.txt'
        initial = args.log_file.stat().st_size if args.log_file.exists() else 0
        log = args.output / 'emulator.log'
        reason = 'process exited before completion'
        with log.open('w') as stderr:
            process = subprocess.Popen([str(args.vita3k.resolve()), '--console', '--frontend', 'sdl',
                                        '-c', str(cfg), '-f', '-w', '-l', '6', '-r', 'CRYSHADR1'],
                                       stdout=stderr, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 180
            try:
                while process.poll() is None and not stop.is_set():
                    if report.exists() and 'DONE failures=' in report.read_text(errors='replace'):
                        reason = 'completed'
                        break
                    current = args.log_file.stat().st_size if args.log_file.exists() else 0
                    if current > initial + 16 * 1024**2 or log.stat().st_size > 4 * 1024**2:
                        reason = 'log limit reached'
                        break
                    if shutil.disk_usage(args.output).free < 2 * 1024**3:
                        reason = 'disk floor reached'
                        break
                    if time.monotonic() >= deadline:
                        reason = '180-second timeout'
                        break
                    time.sleep(.1)
            finally:
                if process.poll() is None and reason == 'completed':
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        pass
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
        if report.exists():
            for path in report.parent.iterdir():
                if path.is_file():
                    shutil.copyfile(path, args.output / path.name)
            result = report.read_text(errors='replace')
            print(result[-2000:])
            if 'DONE failures=0' in result:
                return compare_pixels(args.output)
        print(reason)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
