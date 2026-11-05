#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Compile the SDK-free console contract. Does not build or launch a console game."""
import argparse
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[1]
TARGETS={'psv':'armv7-sony-vita-newlibeabihf','switch':'aarch64-nintendo-switch-freestanding'}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('platform',choices=TARGETS)
    args=parser.parse_args()
    subprocess.run(['cargo','+nightly','check','--release','-p','crysirror-console',
        '--target',TARGETS[args.platform],'-Z','build-std=core,alloc'],cwd=ROOT,check=True)
    print('Console host contract compiled; native device drivers and packaging are not implemented.')

if __name__=='__main__':main()
