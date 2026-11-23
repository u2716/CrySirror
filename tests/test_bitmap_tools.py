# SPDX-License-Identifier: MPL-2.0
"""Filesystem isolation for bitmap comparison output paths."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("compare_bitmap_upstream", ROOT / "tools/compare_bitmap_upstream.py")
bitmap = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bitmap)


class BitmapOutputPathTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="bitmap-tool-test-")
        self.addCleanup(self.temp.cleanup)
        self.source = Path(self.temp.name) / "upstream"
        self.source.mkdir()
        self.files = ["Cargo.toml", "Cargo.lock", "crates/reallive/Cargo.toml",
                      "crates/reallive/src/surface.rs", "crates/avg32/Cargo.toml",
                      "crates/avg32/src/buffer.rs", "crates/avg32/src/pdtmgr.rs"]
        for relative in self.files:
            path = self.source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("synthetic\n")
    def test_complete_source_needs_no_git_metadata_or_revision_lock(self):
        bitmap.validate_source(self.source)
        (self.source / "crates/avg32/src/pdtmgr.rs").unlink()
        with self.assertRaisesRegex(ValueError, "complete siglus_rs source required"):
            bitmap.validate_source(self.source)
    def test_output_symlinks_cannot_redirect_generated_files_into_source(self):
        cache = Path(self.temp.name) / "cache"
        cache.mkdir()
        driver = cache / "bitmap-differential"
        driver.symlink_to(self.source, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "outside both source trees"):
            bitmap.run(self.source, cache)
        driver.unlink()
        driver.mkdir()
        (driver / "src").symlink_to(self.source, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "outside both source trees"):
            bitmap.run(self.source, cache)

    def test_build_outputs_must_stay_outside_sources(self):
        for path in [ROOT / "target/unwanted", self.source / "target"]:
            with self.assertRaisesRegex(ValueError, "outside both source trees"):
                bitmap.external(path, self.source)
        work = Path(self.temp.name) / "build"
        self.assertEqual(bitmap.external(work, self.source), work.resolve())


if __name__ == "__main__":
    unittest.main()
