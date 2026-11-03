# SPDX-License-Identifier: MPL-2.0
from __future__ import annotations
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "tools" / f"{name}.py")
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result

validator = module("validate_project")

class ProjectTests(unittest.TestCase):
    def setUp(self):
        self.project = json.loads((ROOT / "examples/fusion.json").read_text())
    def test_valid_fixture(self):
        validator.validate(self.project)
    def test_schema_boolean_rejected(self):
        self.project["schema"] = True
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_wrong_schema(self):
        self.project["schema"] = 2
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_other_console_rejected(self):
        self.project["console_targets"] = ["switch", "another-console"]
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_duplicate_console_rejected(self):
        self.project["console_targets"] = ["switch", "switch"]
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_duplicate_client(self):
        self.project["clients"][1]["name"] = self.project["clients"][0]["name"]
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_unknown_engine(self):
        self.project["clients"][0]["engine"] = "pretend"
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_unknown_op(self):
        self.project["clients"][0]["commands"][0]["op"] = "draw.3d"
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_extra_property(self):
        self.project["shell"] = "do-not-execute"
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_path_traversal(self):
        for path in ("../escape", "..\\escape", "/etc/passwd", "C:\\data", "a/../../b", "a\0b", "."):
            with self.subTest(path=path), self.assertRaises(validator.ValidationError): validator.asset_path(path)
    def test_windows_paths_at_adapter_boundary(self):
        validator.asset_path("image\\A.png")
    def test_zero_extent(self):
        self.project["clients"][0]["commands"][0]["width"] = 0
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_invalid_color(self):
        self.project["clients"][0]["commands"][0]["rgba"][0] = 256
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_invalid_gain(self):
        self.project["clients"][0]["commands"][2]["gain_milli"] = 1001
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_invalid_boolean_as_id(self):
        self.project["clients"][0]["commands"][0]["id"] = True
        with self.assertRaises(validator.ValidationError): validator.validate(self.project)
    def test_schema_file_matches_positive_fixture(self):
        try:
            import jsonschema
        except ImportError:
            self.skipTest("optional JSON Schema cross-check requires jsonschema")
        schema = json.loads((ROOT / "schemas/project-v1.schema.json").read_text())
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.Draft202012Validator(schema).validate(self.project)



if __name__=="__main__":
    unittest.main()
