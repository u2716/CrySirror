#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Report Actions cache usage; optionally prune obsolete CrySirror CI entries."""
from __future__ import annotations
import argparse
from collections import defaultdict
import json
import subprocess


def api(path: str, method: str = 'GET'):
    args = ['gh', 'api', path]
    if method == 'GET':
        args += ['--paginate', '--slurp']
    else:
        args += ['--method', method]
    result = subprocess.run(args, check=True, text=True, capture_output=True)
    return json.loads(result.stdout) if result.stdout.strip() else None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True)
    parser.add_argument('--default-branch', default='main')
    parser.add_argument('--prune', action='store_true')
    args = parser.parse_args()
    pages = api(f'repos/{args.repo}/actions/caches?per_page=100')
    caches = [entry for page in pages for entry in page['actions_caches']]
    current = {}
    obsolete = []
    for entry in sorted(caches, key=lambda c: c['created_at'], reverse=True):
        key = entry['key']
        if key.startswith(('cargo-sources-', 'cargo-web-', 'cargo-psv-', 'buildkit-')):
            obsolete.append(entry)
        elif key.startswith('cargo-downloads-v2-') and entry['ref'] == f'refs/heads/{args.default_branch}':
            group = key.rsplit('-', 1)[0]
            if group in current:
                obsolete.append(entry)
            else:
                current[group] = entry
    groups = defaultdict(int)
    for entry in caches:
        group = 'dependency downloads' if entry['key'].startswith('cargo-') else 'compiler/other'
        groups[group] += entry['size_in_bytes']
    print(json.dumps({'cache_count': len(caches), 'bytes': sum(c['size_in_bytes'] for c in caches),
                      'groups': dict(groups), 'obsolete_count': len(obsolete),
                      'obsolete_bytes': sum(c['size_in_bytes'] for c in obsolete)}))
    if args.prune:
        for entry in obsolete:
            api(f'repos/{args.repo}/actions/caches/{entry["id"]}', 'DELETE')
        print(f'Removed {len(obsolete)} obsolete dependency/Docker cache entries; compiler entries retained.')


if __name__ == '__main__':
    main()
