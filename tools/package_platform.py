#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Local native Release packages. Uses the existing Cargo target directory."""
from __future__ import annotations
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target' / 'platform'
APPLE = ROOT / 'platform' / 'apple'

def run(*args: str, cwd: Path = ROOT, env: dict | None = None) -> None:
    subprocess.run(list(map(str, args)), cwd=cwd, env=env, check=True)

def rust(target: str | None, kind: str) -> Path:
    args = ['cargo', 'rustc', '--release', '-p', 'crysirror-mobile', '--lib', '--crate-type', kind]
    if target:
        args += ['--target', target]
    env = os.environ.copy()
    if target and target.startswith('aarch64-apple-ios'):
        sdk = 'iphonesimulator' if target.endswith('-sim') else 'iphoneos'
        env['SDKROOT'] = subprocess.check_output(['xcrun', '--sdk', sdk, '--show-sdk-path'], text=True).strip()
        env['IPHONEOS_DEPLOYMENT_TARGET'] = '16.0'
    run(*args, env=env)
    return ROOT / 'target' / (target or '') / 'release'

def apple_project() -> None:
    run('xcodegen', 'generate', '--spec', APPLE / 'project.yml')

def xcode(scheme: str, sdk: str, destination: str) -> Path:
    apple_project()
    derived = OUT / 'apple'
    run('xcodebuild', '-project', APPLE / 'CrySirror.xcodeproj', '-scheme', scheme,
        '-configuration', 'Release', '-sdk', sdk, '-destination', destination,
        '-derivedDataPath', derived, 'CODE_SIGNING_ALLOWED=NO',
        'ARCHS=' + (platform.machine() if sdk == 'macosx' else 'arm64'), 'build')
    suffix = '' if sdk == 'macosx' else '-' + sdk
    return derived / 'Build' / 'Products' / ('Release' + suffix) / 'CrySirror.app'

def macos(skip: bool) -> Path:
    source = ROOT / 'target/release' if skip else rust(None, 'cdylib')
    vendor = APPLE / 'Vendor'; vendor.mkdir(exist_ok=True)
    library = vendor / 'libcrysirror_mobile.dylib'
    shutil.copy2(source / library.name, library)
    run('install_name_tool', '-id', '@rpath/' + library.name, library)
    app = xcode('CrySirror-macOS', 'macosx', 'generic/platform=macOS')
    run('codesign', '--force', '--sign', '-', app / 'Contents/Frameworks' / library.name)
    run('codesign', '--force', '--sign', '-', app)
    run('codesign', '--verify', '--strict', app)
    return app

def framework(skip: bool) -> Path:
    libs = []
    for target in ('aarch64-apple-ios', 'aarch64-apple-ios-sim'):
        folder = ROOT / 'target' / target / 'release' if skip else rust(target, 'staticlib')
        libs += ['-library', str(folder / 'libcrysirror_mobile.a'), '-headers', str(ROOT / 'platform/include')]
    vendor = APPLE / 'Vendor'; vendor.mkdir(exist_ok=True)
    output = vendor / 'CrySirror.xcframework'
    # xcodebuild refuses an existing framework. Replace only this tool's output.
    if output.exists():
        shutil.rmtree(output)
    run('xcodebuild', '-create-xcframework', *libs, '-output', output)
    return output

def android(skip: bool, abis: list[str]) -> Path:
    sdk = Path(os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT') or Path.home() / 'Library/Android/sdk')
    env = os.environ.copy(); env['ANDROID_HOME'] = str(sdk)
    if platform.system() == 'Darwin' and not env.get('JAVA_HOME'):
        env['JAVA_HOME'] = subprocess.check_output(['/usr/libexec/java_home', '-v', '17'], text=True).strip()
    if not env.get('ANDROID_NDK_HOME'):
        ndks = sorted((sdk / 'ndk').glob('*'), key=lambda p: tuple(int(s) for s in p.name.split('.')))
        if not ndks:
            raise RuntimeError('Android NDK is required (ANDROID_NDK_HOME)')
        env['ANDROID_NDK_HOME'] = str(ndks[-1])
    output = OUT / 'android/jniLibs'
    targets = {'arm64-v8a':'aarch64-linux-android', 'x86_64':'x86_64-linux-android'}
    if not skip:
        args = ['cargo', 'ndk']
        for abi in abis:
            args += ['-t', abi]
        run(*args, '-P', '28', '-o', output, 'rustc', '--release', '-p', 'crysirror-mobile', '--lib', '--crate-type', 'cdylib', env=env)
    for abi in abis:
        if not (output / abi / 'libcrysirror_mobile.so').is_file():
            raise RuntimeError(f'Missing Release library for {abi} ({targets[abi]})')
    tools = list((Path(env['ANDROID_NDK_HOME']) / 'toolchains/llvm/prebuilt').glob('*/bin/llvm-nm'))
    if not tools:
        raise RuntimeError('NDK llvm-nm is required for JNI export validation')
    for abi in abis:
        symbols = subprocess.check_output([str(tools[0]), '-D', '--defined-only', str(output / abi / 'libcrysirror_mobile.so')], text=True)
        for method in ('initialize', 'probe', 'start', 'attach', 'detach', 'command', 'destroy'):
            if 'Java_org_crysirror_player_Native_' + method not in symbols:
                raise RuntimeError(f'Native library is missing JNI {method}; rebuild Rust for {abi}')
    run('gradle', '--no-daemon', ':app:assembleRelease', '-PcrysirrorAbis=' + ','.join(abis), cwd=ROOT / 'platform/android', env=env)
    apk = ROOT / 'platform/android/app/build/outputs/apk/release/app-release-unsigned.apk'
    with zipfile.ZipFile(apk) as package:
        for abi in abis:
            if f'lib/{abi}/libcrysirror_mobile.so' not in package.namelist():
                raise RuntimeError(f'APK is missing its native library for {abi}')
    return apk

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('platform', choices=['macos', 'ios', 'ios-sim', 'xcframework', 'android'])
    parser.add_argument('--skip-rust', action='store_true', help='Package previously built Release libraries')
    parser.add_argument('--abi', action='append', choices=['arm64-v8a','x86_64'], help='Android ABI; default arm64-v8a')
    args = parser.parse_args()
    if args.platform == 'android':
        result = android(args.skip_rust, args.abi or ['arm64-v8a'])
    else:
        if platform.system() != 'Darwin':
            parser.error('Apple packages require macOS and Xcode')
        if args.platform == 'macos':
            result = macos(args.skip_rust)
        else:
            result = framework(args.skip_rust)
            if args.platform != 'xcframework':
                simulator = args.platform == 'ios-sim'
                result = xcode('CrySirror-iOS', 'iphonesimulator' if simulator else 'iphoneos',
                    'generic/platform=iOS Simulator' if simulator else 'generic/platform=iOS')
    print(result)

if __name__ == '__main__':
    main()
