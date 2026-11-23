#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Derive Unity 4 managed-serialization schemas from decompiled C# sources.

Unity 4 player data carries no type trees: a MonoBehaviour's script fields
follow the C# declaration order of the class and its bases under Unity's
rules. This tool reads ILSpy project output (one or more directories, e.g.
Assembly-CSharp and Assembly-CSharp-firstpass) and writes the JSON schema
consumed by `crysirror_unity::managed`.

Rules applied (Unity 4.x managed serializer):
- instance fields that are public, or carry [SerializeField], and are not
  static/const/readonly or [NonSerialized];
- bool/byte/sbyte/short/ushort/char fields align to 4 after the value;
- string, T[] and List<T> carry an i32 count and align afterwards;
- enums use their underlying integer type;
- UnityEngine.Object types (and every MonoBehaviour/ScriptableObject
  subclass) are PPtrs;
- [Serializable] non-abstract classes/structs are written inline;
- delegates, interfaces, dictionaries, abstract and non-serializable
  classes are skipped.

The schema records only class/field names and types; it contains no code.
Usage: unity_managed_schema.py OUT.json SRC_DIR [SRC_DIR...]
"""
import json
import pathlib
import re
import sys

PRIMITIVES = {
    "bool": "bool", "Boolean": "bool",
    "byte": "u8", "Byte": "u8", "sbyte": "i8", "SByte": "i8",
    "short": "i16", "Int16": "i16", "ushort": "u16", "UInt16": "u16", "char": "u16", "Char": "u16",
    "int": "i32", "Int32": "i32", "uint": "u32", "UInt32": "u32",
    "long": "i64", "Int64": "i64", "ulong": "u64", "UInt64": "u64",
    "float": "f32", "Single": "f32", "double": "f64", "Double": "f64",
    "string": "string", "String": "string",
}
UNITY_STRUCTS = {
    "Vector2", "Vector3", "Vector4", "Quaternion", "Color", "Color32", "Rect", "Bounds",
    "Matrix4x4", "LayerMask", "AnimationCurve", "Gradient", "RectOffset", "GUIStyle",
}
UNITY_OBJECTS = {
    "Object", "GameObject", "Transform", "Component", "Behaviour", "MonoBehaviour", "ScriptableObject",
    "Texture", "Texture2D", "RenderTexture", "Material", "Shader", "Font", "AudioClip", "AudioSource",
    "Camera", "Light", "Animation", "AnimationClip", "Animator", "Sprite", "Mesh", "MeshFilter",
    "MeshRenderer", "Renderer", "SkinnedMeshRenderer", "Collider", "BoxCollider", "SphereCollider",
    "Rigidbody", "TextAsset", "MovieTexture", "Cubemap", "PhysicMaterial", "ParticleSystem",
    "RuntimeAnimatorController", "AudioListener", "GUISkin", "GUITexture", "Flare", "Texture3D",
    "CharacterController", "LineRenderer", "TrailRenderer", "Projector", "CapsuleCollider",
    "MeshCollider", "Avatar", "WebCamTexture", "RectTransform", "Canvas", "Image", "Text", "Button",
    "CanvasGroup", "Graphic", "Selectable", "TextMesh",
    # MonoBehaviours of Assembly-UnityScript-firstpass (standard image effects).
    "ScreenOverlay", "Bloom", "Blur", "Vignetting", "BloomAndLensFlares", "ColorCorrectionCurves",
}
UNITY_ENUMS = {
    "KeyCode", "FontStyle", "TextAnchor", "FilterMode", "TextureWrapMode", "WrapMode", "TextureFormat",
    "RenderingPath", "CameraClearFlags", "LightType", "AnimationBlendMode", "PlayMode", "QueueMode",
    "SendMessageOptions", "ForceMode", "RuntimePlatform", "ShadowProjection", "LightShadows",
    "DepthTextureMode", "RigidbodyInterpolation", "AudioRolloffMode", "TouchPhase", "ImagePosition",
    "TextClipping", "Space", "CollisionDetectionMode", "HideFlags", "RenderTextureFormat",
}
SKIP_GENERIC = {"Dictionary", "Hashtable", "HashSet", "Queue", "Stack", "LinkedList", "SortedList",
                "SortedDictionary", "Action", "Func", "IEnumerator", "IEnumerable", "ICollection", "IList",
                "IDictionary", "Nullable", "KeyValuePair", "Type", "object", "Object?", "Delegate", "IntPtr"}
MODIFIERS = {"public", "private", "protected", "internal", "static", "readonly", "const", "new",
             "volatile", "unsafe", "extern", "override", "virtual", "abstract", "sealed", "fixed"}

TYPE_DECL = re.compile(
    r"^(?P<indent>\t*)(?P<mods>(?:(?:public|private|protected|internal|static|abstract|sealed|partial|new|unsafe)\s+)*)"
    r"(?P<kind>class|struct|enum|interface)\s+(?P<name>\w+)(?:<[^>]*>)?(?:\s*:\s*(?P<base>[^{]+?))?\s*(?:where .*)?$")
DELEGATE = re.compile(r"^\t*(?:(?:public|private|protected|internal|new)\s+)*delegate\s+[\w<>\[\],\.\s]+?\s+(\w+)\s*[<(]")
FIELD = re.compile(r"^(?P<indent>\t+)(?P<decl>[^(){}=]+?)\s*(?:=.*)?;\s*$")


class Type:
    def __init__(self, kind, name, outer, namespace, base, serializable, abstract, enum_base, usings=()):
        self.kind, self.name, self.outer, self.namespace = kind, name, outer, namespace
        self.usings = list(usings)
        self.base, self.serializable, self.abstract, self.enum_base = base, serializable, abstract, enum_base
        self.fields = []  # (name, csharp type)

    @property
    def full(self):
        if self.outer:
            return f"{self.outer.full}.{self.name}"
        return f"{self.namespace}.{self.name}" if self.namespace else self.name


def parse(path, types, delegates):
    namespace = ""
    usings = []
    stack = []  # (Type or None, brace depth at which its body opened)
    depth = 0
    attrs = []
    pending = None
    for raw in path.read_text(encoding="utf-8-sig", errors="replace").splitlines():
        line = raw.rstrip()
        stripped = line.strip()
        m = re.match(r"^namespace\s+([\w\.]+)", stripped)
        if m:
            namespace = m.group(1)
        u = re.match(r"^using\s+([\w\.]+);$", stripped)
        if u:
            usings.append(u.group(1))
        if stripped.startswith("[") and stripped.endswith("]") and not FIELD.match(line):
            attrs.append(stripped)
        else:
            d = DELEGATE.match(line)
            if d:
                delegates.add(d.group(1))
            t = TYPE_DECL.match(line)
            if t and "(" not in line:
                outer = stack[-1][0] if stack else None
                mods = t.group("mods") or ""
                base = (t.group("base") or "").split(",")[0].strip() or None
                enum_base = None
                if t.group("kind") == "enum":
                    enum_base = base or "int"
                    base = None
                pending = Type(t.group("kind"), t.group("name"), outer, namespace, base,
                               any("Serializable" in a for a in attrs), "abstract" in mods, enum_base, usings)
                types.setdefault(pending.full, []).append(pending)
                attrs = []
            elif stack and stack[-1][0] is not None and depth == stack[-1][1] and stripped:
                owner = stack[-1][0]
                f = FIELD.match(line)
                if f and owner.kind in ("class", "struct") and "=>" not in line and " event " not in f" {f.group('decl')} ":
                    words = f.group("decl").split()
                    mods = set()
                    while words and words[0] in MODIFIERS:
                        mods.add(words.pop(0))
                    if len(words) >= 2:
                        ctype, name = " ".join(words[:-1]), words[-1]
                        serialized = ("public" in mods or any("SerializeField" in a for a in attrs)) \
                            and not mods & {"static", "const", "readonly"} \
                            and not any("NonSerialized" in a for a in attrs)
                        owner.fields.append((name, ctype, serialized))
                attrs = []
            elif stripped and not stripped.startswith("//"):
                attrs = []
        for ch in line:
            if ch == "{":
                depth += 1
                if pending is not None:
                    stack.append((pending, depth))
                    pending = None
                elif not stack or stack[-1][1] != depth:
                    stack.append((None, depth))
            elif ch == "}":
                if stack and stack[-1][1] == depth:
                    stack.pop()
                depth -= 1


class Resolver:
    def __init__(self, types, delegates):
        self.types, self.delegates = types, delegates
        self.by_simple = {}
        for full, decls in types.items():
            self.by_simple.setdefault(full.split(".")[-1], []).append(decls[0])
        self.errors = set()
        self.structs = {}

    def find(self, name, context, qualified_only=False):
        name = re.sub(r"<.*>$", "", name.split("::")[-1].strip())
        parts = name.split(".")
        keys = []
        scope = context
        while scope is not None:
            keys.append(f"{scope.full}.{name}")
            scope = scope.outer
        root = context
        while root is not None and root.outer is not None:
            root = root.outer
        if root is not None:
            ns = root.namespace.split(".") if root.namespace else []
            for i in range(len(ns), 0, -1):
                keys.append(".".join(ns[:i]) + "." + name)
            keys.extend(f"{u}.{name}" for u in root.usings)
        keys.append(name)
        for key in keys:
            hit = self.types.get(key)
            if hit:
                return hit[0]
        if qualified_only and len(parts) == 1:
            return None
        cands = self.by_simple.get(parts[-1], [])
        return cands[0] if len(cands) == 1 else None

    def is_unity_object(self, t, seen=0):
        while t is not None and seen < 32:
            base = t.base
            if base is None:
                return False
            simple = base.split(".")[-1].split("<")[0]
            if simple in UNITY_OBJECTS:
                return True
            t = self.find(base, t.outer)
            seen += 1
        return False

    def resolve(self, ctype, context, depth=0):
        ctype = ctype.strip().replace("global::", "")
        for prefix in ("System.Collections.Generic.", "System.", "UnityEngine.", "UnityEngine.UI."):
            if ctype.startswith(prefix) and "." not in ctype[len(prefix):].split("<")[0]:
                ctype = ctype[len(prefix):]
        if ctype.endswith("[]"):
            inner = self.resolve(ctype[:-2], context, depth)
            return None if inner is None else {"array": inner}
        g = re.match(r"^(\w+)<(.+)>$", ctype)
        if g:
            if g.group(1) == "List":
                inner = self.resolve(g.group(2), context, depth)
                return None if inner is None else {"array": inner}
            return None
        if ctype in PRIMITIVES:
            return PRIMITIVES[ctype]
        simple = ctype.split(".")[-1]
        # Unqualified names resolve through the declaring scopes, then
        # Unity and System types (a nested `Type` enum shadows System.Type).
        t = self.find(ctype, context, qualified_only=True)
        if t is None and (ctype in SKIP_GENERIC or simple in self.delegates):
            return None
        if t is None and "." not in ctype:
            if simple in UNITY_OBJECTS:
                return "pptr"
            if simple in UNITY_STRUCTS:
                return simple
            if simple in UNITY_ENUMS:
                return "i32"
            t = self.find(ctype, context)
        if t is not None:
            if t.kind == "enum":
                return PRIMITIVES.get(t.enum_base.split(".")[-1], "i32")
            if t.kind == "interface":
                return None
            if self.is_unity_object(t):
                return "pptr"
            if t.serializable and not t.abstract and depth < 7:
                self.struct(t, depth + 1)
                return {"struct": t.full}
            return None
        if simple in UNITY_OBJECTS:
            return "pptr"
        if simple in UNITY_STRUCTS:
            return simple
        if simple in UNITY_ENUMS:
            return "i32"
        self.errors.add(f"{context.full if context else '?'}: {ctype}")
        return None

    def fields(self, t, depth=0):
        out = []
        chain = []
        cur = t
        while cur is not None:
            chain.append(cur)
            base = cur.base
            if base is None or base.split(".")[-1].split("<")[0] in UNITY_OBJECTS:
                break
            cur = self.find(base, cur.outer)
        for owner in reversed(chain):
            for name, ctype, serialized in owner.fields:
                if not serialized:
                    continue
                kind = self.resolve(ctype, owner, depth)
                if kind is not None:
                    out.append([name, kind])
        return out

    def struct(self, t, depth):
        if t.full not in self.structs:
            self.structs[t.full] = None
            self.structs[t.full] = self.fields(t, depth)


def main():
    out, sources = sys.argv[1], sys.argv[2:]
    types, delegates = {}, set()
    for src in sources:
        for path in sorted(pathlib.Path(src).rglob("*.cs")):
            parse(path, types, delegates)
    r = Resolver(types, delegates)
    scripts = {}
    for full, decls in sorted(types.items()):
        t = decls[0]
        if t.kind == "class" and r.is_unity_object(t) and not t.abstract:
            scripts[full] = r.fields(t)
    schema = {"schema": 1, "scripts": scripts, "structs": {k: v for k, v in sorted(r.structs.items())}}
    pathlib.Path(out).write_text(json.dumps(schema, ensure_ascii=False, indent=0, sort_keys=True) + "\n", encoding="utf-8")
    for e in sorted(r.errors):
        print("unresolved", e, file=sys.stderr)
    print(f"{len(scripts)} scripts, {len(r.structs)} serializable structs, {len(r.errors)} unresolved types")


if __name__ == "__main__":
    main()
