# SPDX-License-Identifier: MPL-2.0
"""Local authoring outputs: external paths or the ignored local/work subtree."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def output_root(path):
    path = Path(path).resolve()
    if path.is_relative_to(ROOT) and not path.is_relative_to(ROOT / 'local' / 'work'):
        raise ValueError('game assets must remain external or under ignored local/work')
    return path
