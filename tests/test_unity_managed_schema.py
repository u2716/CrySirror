# SPDX-License-Identifier: MPL-2.0
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

SOURCE = """using System;
using UnityEngine;
public class Base : MonoBehaviour
{
\tpublic enum Type
\t{
\t\tA,
\t\tB
\t}
\t[SerializeField]
\tprotected Type mType;
\tpublic bool flag;
\t[NonSerialized]
\tpublic int skipped;
\tpublic static int shared;
}
[Serializable]
public class Point
{
\tpublic Transform target;
\tpublic float relative;
}
public class Child : Base
{
\tprivate Texture mTexture;
\t[SerializeField]
\tprivate Texture mShown;
\tpublic Point anchor;
\tpublic System.Collections.Generic.List<Point> points;
\tpublic string[] names;
\tpublic Action callback;
\tpublic int Value { get; set; }
}
"""


class UnityManagedSchemaTest(unittest.TestCase):
    def test_serialization_rules(self):
        with tempfile.TemporaryDirectory() as d:
            src = pathlib.Path(d, "src")
            src.mkdir()
            (src / "Types.cs").write_text(SOURCE, encoding="utf-8")
            # ILSpy writes one type per file; a namespaced type must not
            # shadow UnityEngine.Texture outside its namespace.
            (src / "Other.cs").write_text("namespace Other\n{\n\tpublic abstract class Texture\n\t{\n\t}\n}\n", encoding="utf-8")
            out = pathlib.Path(d, "schema.json")
            subprocess.run([sys.executable, "-I", str(ROOT / "tools/unity_managed_schema.py"), str(out), str(src)],
                           check=True, capture_output=True)
            schema = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(schema["scripts"]["Child"], [
            ["mType", "i32"], ["flag", "bool"], ["mShown", "pptr"],
            ["anchor", {"struct": "Point"}], ["points", {"array": {"struct": "Point"}}],
            ["names", {"array": "string"}],
        ])
        self.assertEqual(schema["structs"]["Point"], [["target", "pptr"], ["relative", "f32"]])


if __name__ == "__main__":
    unittest.main()
