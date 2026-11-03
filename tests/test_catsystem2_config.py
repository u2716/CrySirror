import copy
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import desktop_project

class CatSystem2Config(unittest.TestCase):
    def test_explicit_word_extremes_null_and_omitted_match_native_defaults(self):
        base = {"script": "kcs.int/main.kcs", "font": "default"}
        for seeds in (None, [0, 2**32 - 1], [17, 19]):
            config = dict(base, random_seeds=seeds)
            self.assertIs(desktop_project.catsystem2_config(config), config)
        self.assertEqual(desktop_project.catsystem2_config(base), base)

    def test_malformed_seed_shapes_paths_and_unknown_fields_are_rejected(self):
        base = {"script": "kcs.int/main.kcs", "font": "default"}
        invalid = [{"random_seeds": x} for x in ([], [1], [1,2,3], [True,1], [1.0,2], [-1,2], [2**32,0], "1,2")]
        invalid += [{"heap_bytes": 3}, {"heap_bytes": True}, {"canvas": [0,720]}, {"script": "../main.kcs"}, {"script": "main.cst"}, {"font": "日" * 86}, {"random_source": "time"}]
        for values in invalid:
            with self.subTest(values=values), self.assertRaises(desktop_project.ProjectError):
                desktop_project.catsystem2_config(dict(base, **values))

    def test_project_validator_uses_native_config_and_never_accepts_data_vm_substitution(self):
        project = {"schema":2,"name":"Cat seed fixture","content_revision":"1","assets":"game","canvas":[1280,720],
                   "clients":[{"name":"game","adapter":"catsystem2","game":"native","config":{"script":"main.kcs","font":"default","random_seeds":[0,4294967295]}}]}
        desktop_project.validate(project)
        malformed = copy.deepcopy(project)
        malformed["clients"][0]["config"]["random_seeds"] = [False, 1]
        with self.assertRaises(desktop_project.ProjectError):
            desktop_project.validate(malformed)
        malformed = copy.deepcopy(project)
        malformed["clients"][0]["program"] = [{"op":"halt"}]
        with self.assertRaises(desktop_project.ProjectError):
            desktop_project.validate(malformed)

    def test_standalone_schema_has_the_real_native_fields_and_u32_bounds(self):
        schema = json.loads((ROOT / "schemas/catsystem2-config.schema.json").read_text())
        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(set(schema["properties"]), {"script", "font", "heap_bytes", "executable", "canvas", "random_seeds"})
        seeds = schema["properties"]["random_seeds"]
        self.assertEqual((seeds["minItems"], seeds["maxItems"]), (2,2))
        self.assertEqual(seeds["items"]["maximum"], 2**32 - 1)

if __name__ == "__main__":
    unittest.main()
