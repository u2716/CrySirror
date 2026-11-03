# SPDX-License-Identifier: MPL-2.0
"""Reference fetching does not recreate repository lock files or overwrite edits."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("upstreams", ROOT / "tools/upstreams.py")
upstreams = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(upstreams)


class ReferenceFetchTests(unittest.TestCase):
    def test_fetch_and_reuse_leave_repository_and_local_edits_untouched(self):
        config = {
            "schema": 1,
            "console_targets": ["switch", "psv"],
            "sources": {
                name: {"url": url, "ref": "main", "focus_manifests": ["Cargo.toml"]}
                for name, url in upstreams.URLS.items()
            },
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "repository"
            root.mkdir()
            sources = Path(tmp) / "references"

            def git(args, *, cwd=None):
                if args == ["git", "remote", "get-url", "origin"]:
                    return upstreams.URLS[cwd.name]
                return ""

            with patch.object(upstreams, "ROOT", root), patch.object(upstreams, "run", side_effect=git):
                upstreams.fetch(config, sources)
                edited = sources / "siglus_rs" / "local-edit.rs"
                edited.write_text("caller-owned source edits\n")
                upstreams.fetch(config, sources)
                self.assertEqual(edited.read_text(), "caller-owned source edits\n")
                self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
