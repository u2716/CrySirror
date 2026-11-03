# SPDX-License-Identifier: MPL-2.0
"""Source/output capability guards only; these tests never compile Rust."""
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("compare_bgi_cpu", ROOT / "tools/compare_bgi_cpu.py")
bgi = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bgi)


class BgiCpuToolGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="bgi-cpu-guard-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.source = self.base / "source"
        self.source.mkdir()
        path = self.source / bgi.FILES[0]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("synthetic reference\n")

    def test_all_generated_source_and_lock_symlinks_are_rejected_without_cargo(self):
        cache = self.base / "cache"
        driver = cache / "bgi-cpu-differential"
        (driver / "src").mkdir(parents=True)
        for name in ["src/main.rs", "src/original_core.rs", "src/original_bitmap.rs",
                     "src/original_snapshot.rs", "Cargo.toml", "Cargo.lock"]:
            with self.subTest(name=name):
                path = driver / name
                path.symlink_to(self.source / bgi.FILES[0])
                with patch.object(bgi.subprocess, "run", side_effect=AssertionError("Cargo forbidden")):
                    with self.assertRaisesRegex(ValueError, "outside both source trees"):
                        bgi.output_paths(self.source, cache, None)
                path.unlink()
        escaped = self.base / "unrelated.rs"
        escaped.write_text("keep")
        (driver / "src/original_core.rs").symlink_to(escaped)
        with self.assertRaisesRegex(ValueError, "escapes the external work directory"):
            bgi.output_paths(self.source, cache, None)
        self.assertEqual(escaped.read_text(), "keep")

    def test_unrelated_cargo_manifest_is_preserved(self):
        cache = self.base / "cache"
        driver = cache / "bgi-cpu-differential"
        driver.mkdir(parents=True)
        manifest = driver / "Cargo.toml"
        original = '[package]\nname="unrelated-project"\n'
        manifest.write_text(original)
        with self.assertRaisesRegex(ValueError, "unrelated Cargo project"):
            bgi.output_paths(self.source, cache, None)
        self.assertEqual(manifest.read_text(), original)
        manifest.write_text('[package]\nname = "crysirror-bgi-cpu-differential"\n')
        self.assertEqual(bgi.output_paths(self.source, cache, None)[0], driver)

    def test_help_without_font_or_cargo_and_missing_font_argument(self):
        env = {**os.environ, "PATH": ""}
        script = str(ROOT / "tools/compare_bgi_cpu.py")
        help_result = subprocess.run([sys.executable, script, "--help"], env=env,
                                     text=True, capture_output=True)
        self.assertEqual(help_result.returncode, 0, help_result.stderr)
        self.assertIn("--font", help_result.stdout)
        missing = subprocess.run([sys.executable, script, "--source-root", str(self.source),
                                  "--work-dir", str(self.base / "unused")], env=env,
                                 text=True, capture_output=True)
        self.assertEqual(missing.returncode, 2)
        self.assertIn("--font", missing.stderr)
        self.assertFalse((self.base / "unused").exists())


if __name__ == "__main__":
    unittest.main()
