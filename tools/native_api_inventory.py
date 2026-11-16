#!/usr/bin/env python3
"""Generate native opcode/call tables from Rust dispatchers, without running VMs.

Requires tools/requirements-native-api.txt. Handler presence is deliberately
reported separately from a complete semantic audit or execution evidence.
"""
import argparse
import ast
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

try:
    from tree_sitter import Language, Parser
    import tree_sitter_rust
except ImportError:
    sys.exit("Install tools/requirements-native-api.txt before generating native API tables")

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/native-api"
PARSER = Parser(Language(tree_sitter_rust.language()))
MAJIRO = json.loads((ROOT / "tools/native-api/majiro-opcodes.json").read_text())
NSYSTEM = {n: int(v) for n, v in re.findall(
    r"pub const ([A-Z0-9_]+): u16 = (\d+);", (ROOT / "crates/crysirror-nsystem/src/adapter.rs").read_text())}
REALLIVE_NAMES = {(int(t), int(m), int(o)): n for t, m, o, n in re.findall(
    r'\((\d+),\s*(\d+),\s*(\d+),\s*"([^"]+)"\)',
    (ROOT / "crates/crysirror-reallive/src/vm/opcodes.rs").read_text())}
REALLIVE_MODULES = {
    ("jmp.rs", "dispatch"): (0, 1), ("str.rs", "dispatch"): (1, 10),
    ("mem.rs", "dispatch"): (1, 11), ("sel.rs", "dispatch"): (0, 2),
    ("commands.rs", "system"): (1, 4), ("commands.rs", "message"): (0, 3),
    ("extended.rs", "system_extended"): (1, 4), ("extended.rs", "message_extended"): (0, 3),
    ("sys.rs", "dispatch"): (1, 4), ("sys_max.rs", "dispatch"): (1, 4),
}
GENERATED = "<!-- BEGIN GENERATED DISPATCH TABLES -->"
SELECTOR = re.compile(r"(?:opcode|\b(?:op|cmd|sub|code|operation|operator|id|key|mode|hash|name|kind)\b|command|selector|subcommand|method|\ba\[|call\.id|\.tagname|instruction|call\.target|work\.kind)")
TITLES = {
    "circus": "Circus", "system4": "AliceSoft System 4", "reallive": "RealLive",
    "avg32": "AVG32", "catsystem2": "CatSystem2", "majiro": "Majiro",
    "nscripter": "NScripter", "qlie": "QLIE", "system-nnn": "System-NNN",
    "sugarpot": "SugarPot", "willplus": "WillPlus / AdvHD", "krkr": "KiriKiri / TJS2 / KAG",
    "nsystem": "N-System",
}
PURPOSE = {
    "PUSH": "Push an immediate value", "POP": "Pop a stack value",
    "REF": "Read a referenced value", "REFREF": "Read a nested reference",
    "ASSIGN": "Assign a value", "ADD": "Addition", "SUB": "Subtraction",
    "MUL": "Multiplication", "DIV": "Division", "MOD": "Remainder",
    "JUMP": "Unconditional jump", "IFZ": "Jump when zero", "IFNZ": "Jump when nonzero",
    "RETURN": "Return from a function", "CALLFUNC": "Call a script function",
    "CALLMETHOD": "Call an object method", "CALLHLL": "Call a native library export",
    "CALLSYS": "Call a system function", "NEW": "Create an object",
    "DELETE": "Delete an object", "SWITCH": "Integer switch", "STRSWITCH": "String switch",
    "ENDFUNC": "Function-end marker", "EOF": "File-end marker",
    "goto": "Jump within the scenario", "gosub": "Call a scenario subroutine",
    "ret": "Return from a call", "select": "Present a choice", "select_w": "Present a choice window",
    "farcall": "Call another scenario", "jump": "Transfer to another scenario",
    "strcpy": "Copy a string", "strcat": "Append a string", "strlen": "String length",
    "strcmp": "Compare strings", "strclear": "Clear a string", "pause": "Wait for input",
    "nop": "No-operation instruction", "const": "Load a constant", "cp": "Copy a value",
    "call": "Call a function", "calld": "Call a named property", "calli": "Call an indexed property",
    "new": "Construct an object", "throw": "Throw an exception", "debugger": "Debugger instruction",
}
FAMILIES = {
    "text": "Text and message windows", "message": "Text and message windows",
    "audio": "Audio playback/control", "sound": "Audio playback/control", "media": "Audio/video control",
    "movie": "Movie playback", "save": "Save/load", "resource": "Resource loading",
    "sprite": "Sprite state", "visual": "Graphics and composition", "gfx": "Graphics and composition",
    "draw": "Drawing", "effect": "Visual effects", "packed": "Packed image commands",
    "input": "Input", "selection": "Choices", "select": "Choices", "menu": "Menus",
    "value": "Variables and values", "variable": "Variables and values", "array": "Arrays",
    "string": "String operations", "control": "Control flow", "scene": "Scene transfer/state",
    "clock": "Clock and waits", "timer": "Clock and waits", "math": "Arithmetic",
    "font": "Font/text state", "sys": "System commands", "system": "System commands",
    "object": "Object methods", "layer": "Layer methods", "memory": "Memory operations",
    "ud": "UD command selectors", "native": "Native call", "particle": "Particle commands",
}


def text(node):
    return node.text.decode("utf-8", errors="replace") if node else ""


def walk(node):
    yield node
    for child in node.named_children:
        yield from walk(child)


def syntax(path):
    return PARSER.parse(path.read_bytes()).root_node


def owner(node):
    while node:
        if node.type == "impl_item":
            return text(node.child_by_field_name("type"))
        node = node.parent
    return ""


def pattern_base(node):
    if node.type == "match_pattern":
        # Match guards are not additional opcode IDs or API aliases.
        return node.children[0]
    return node


def atoms(node, numeric):
    node = pattern_base(node)
    raw = text(node)
    if numeric:
        if node.type == "integer_literal":
            token = re.sub(r"(?:u|i)(?:8|16|32|64|128|size)$", "", raw).replace("_", "")
            try:
                return [int(token, 0) if token.lower().startswith(("0x", "0b", "0o")) else int(token)]
            except ValueError:
                return []
        if node.type == "range_pattern":
            endpoints = [n for n in node.named_children if n.type == "integer_literal"]
            if len(endpoints) == 2:
                lo, hi = atoms(endpoints[0], True)[0], atoms(endpoints[1], True)[0]
                if "..=" not in raw:
                    hi -= 1
                if 0 <= hi - lo <= 4096:
                    return list(range(lo, hi + 1))
            return []
    elif node.type in ("string_literal", "raw_string_literal"):
        if node.type == "raw_string_literal":
            return [raw[raw.index('"') + 1:raw.rindex('"')]]
        try:
            value = ast.literal_eval(raw)
            if isinstance(value, bytes):
                value = value.decode("ascii")
            return [value] if isinstance(value, str) else []
        except (ValueError, SyntaxError, UnicodeError):
            return []
    if node.type in ("or_pattern", "parenthesized_pattern", "reference_pattern", "match_pattern", "captured_pattern", "tuple_pattern"):
        return [v for c in node.named_children for v in atoms(c, numeric)]
    return []


def tuple_part(node, index):
    node = pattern_base(node)
    if node.type != "tuple_pattern":
        return None
    children = [c for c in node.children if c.type not in ("(", ")", ",")]
    return children[index] if index < len(children) else None


def collect(path):
    records = []
    registrations = []
    root = syntax(path)

    def literal_names(node, depth=0):
        if node is None or depth > 5:
            return []
        direct = atoms(node, False)
        if direct:
            return direct
        if node.type == "array_expression":
            names = []
            for entry in node.named_children:
                if entry.type == "tuple_expression" and entry.named_children:
                    entry = entry.named_children[0]
                names.extend(literal_names(entry, depth + 1))
            return names
        if node.type in ("reference_expression", "parenthesized_expression") and node.named_children:
            return literal_names(node.named_children[-1], depth + 1)
        if node.type == "call_expression":
            return [v for part in walk(node) if part.type == "identifier"
                    for v in literal_names(part, depth + 1)]
        if node.type == "identifier":
            name = text(node)
            for declaration in walk(root):
                if declaration.type in ("const_item", "static_item", "let_declaration"):
                    pattern = declaration.child_by_field_name("name") or declaration.child_by_field_name("pattern")
                    if text(pattern) == name:
                        found = literal_names(declaration.child_by_field_name("value"), depth + 1)
                        if found:
                            return found
        return []

    def loop_names(node, variable):
        while node:
            if node.type == "for_expression" and re.search(r"\b" + re.escape(variable) + r"\b", text(node.child_by_field_name("pattern"))):
                found = literal_names(node.child_by_field_name("value"))
                if found:
                    return found
            node = node.parent
        return []

    def visit(node, fn="", contexts=()):
        if node.type == "mod_item" and text(node.child_by_field_name("name")) == "tests":
            return
        if node.type == "function_item":
            fn = text(node.child_by_field_name("name"))
            if fn.startswith("test_"):
                return
        if node.type == "macro_invocation":
            raw = text(node)
            if re.match(r"matches!\s*\(\s*(?:name|method|op|opcode|selector|kind|call\.id|cx\.call\.id)\s*,", raw):
                # `handles()` and ABI allowlists can be pure matches! macros,
                # with no match/if expression for the ordinary walker to see.
                records.append({"file": path, "fn": fn, "value": raw,
                                "line": node.start_point[0] + 1, "contexts": contexts,
                                "conditional": True, "declaration": True,
                                "arms": [{"pattern": node, "body": node, "comments": "",
                                          "line": node.start_point[0] + 1}]})
        if node.type == "if_expression":
            condition = node.child_by_field_name("condition")
            raw = text(condition)
            if SELECTOR.search(raw) and (re.search(r'"|0x[\da-fA-F]+|==\s*\d|\.\.=\s*\d', raw)
                                         or any(n.type == "integer_literal" for n in walk(condition))):
                consequence = node.child_by_field_name("consequence")
                if consequence:
                    records.append({"file": path, "fn": fn, "value": raw,
                                    "line": node.start_point[0] + 1, "contexts": contexts,
                                    "conditional": True,
                                    "arms": [{"pattern": condition, "body": consequence,
                                              "comments": "", "line": node.start_point[0] + 1}]})
            if re.search(r"\b(?:kind|class)(?:\.as_str\(\))?\s*==|matches!\((?:kind|class)\.as_str\(\)", raw):
                branch = node.child_by_field_name("consequence")
                if branch:
                    visit(branch, fn, contexts + (("receiver", raw),))
                alternative = node.child_by_field_name("alternative")
                if alternative:
                    visit(alternative, fn, contexts + (("receiver", "else: " + raw),))
                return
        if node.type == "match_expression":
            value = text(node.child_by_field_name("value"))
            block = node.child_by_field_name("body")
            record = {"file": path, "fn": fn, "value": value, "line": node.start_point[0] + 1,
                      "contexts": contexts, "arms": []}
            for arm in block.named_children if block else []:
                if arm.type != "match_arm":
                    continue
                pattern = arm.child_by_field_name("pattern")
                body = arm.child_by_field_name("value")
                if pattern is None or body is None:
                    continue
                comments = []
                prev = arm.prev_named_sibling
                while prev is not None and prev.type in ("line_comment", "block_comment"):
                    comments.insert(0, text(prev))
                    prev = prev.prev_named_sibling
                record["arms"].append({"pattern": pattern, "body": body, "comments": " ".join(comments),
                                       "line": arm.start_point[0] + 1})
                visit(body, fn, contexts + ((value, text(pattern)),))
            records.append(record)
            return
        if node.type == "call_expression":
            func = text(node.child_by_field_name("function"))
            if re.search(r"register_(?:object|global)_native(?:_property|_with_arg_count)?$", func):
                if fn.startswith("register_"):
                    return  # The generic registration API is not a builtin.
                args = node.child_by_field_name("arguments")
                children = list(args.named_children) if args else []
                global_call = "register_global" in func
                at = 0 if global_call else 1
                if len(children) > at:
                    names = atoms(children[at], False)
                    # Resolve a static loop's method variable rather than omitting it.
                    parent = node.parent
                    while not names and parent:
                        if parent.type == "for_expression":
                            pattern = text(parent.child_by_field_name("pattern"))
                            iterator = parent.child_by_field_name("value")
                            variable = text(children[at]).lstrip("*")
                            if variable == pattern or (re.fullmatch(r"\w+", variable) and re.search(r"\b" + re.escape(variable) + r"\b", pattern)):
                                names = literal_names(iterator)
                        parent = parent.parent
                    if not names:
                        # A local registration helper may receive a static
                        # (name, callback) array from its callers. Read just
                        # tuple names, never strings inside callback bodies.
                        for call in walk(root):
                            if call.type != "call_expression" or text(call.child_by_field_name("function")) != fn:
                                continue
                            arguments = call.child_by_field_name("arguments")
                            for argument in arguments.named_children if arguments else []:
                                if argument.type in ("array_expression", "reference_expression"):
                                    names.extend(literal_names(argument))
                    if not names:
                        # Resolve a wrapper's name argument through constant
                        # arrays at its call sites (windowEx registration).
                        function = node
                        while function and function.type != "function_item":
                            function = function.parent
                        parameters = function.child_by_field_name("parameters") if function else None
                        formal = list(parameters.named_children) if parameters else []
                        position = next((i for i, p in enumerate(formal) if text(p.child_by_field_name("pattern")) == text(children[at])), None)
                        if position is not None:
                            for call in walk(root):
                                if call.type != "call_expression" or text(call.child_by_field_name("function")) != fn:
                                    continue
                                arguments = call.child_by_field_name("arguments")
                                actual = list(arguments.named_children) if arguments else []
                                if position < len(actual):
                                    argument = actual[position]
                                    names.extend(literal_names(argument) or loop_names(call, text(argument)))
                    receiver = "global" if global_call else text(children[0])
                    if not names:
                        names = ["{" + text(children[at]) + "}"]
                    callback = children[-1]
                    if callback.type == "identifier":
                        callback = next((n.child_by_field_name("body") for n in walk(root)
                                         if n.type == "function_item" and text(n.child_by_field_name("name")) == text(callback)), callback)
                    for name in sorted(set(names)):
                        registrations.append((receiver + "." + name, fn, node.start_point[0] + 1, callback))
        for child in node.named_children:
            visit(child, fn, contexts)

    visit(root)
    return records, registrations


def clean(value, limit=150):
    value = re.sub(r"\s+", " ", value).strip().replace("|", "\\|")
    return value if len(value) <= limit else value[:limit - 1] + "…"


def purpose(name, record, arm):
    if name in PURPOSE:
        return PURPOSE[name]
    comment = re.sub(r"//[/!]?|/\*|\*/", " ", arm.get("comments", ""))
    comment = re.sub(r"\b(?:0x)?[0-9A-Fa-f]{6,16}\b", "", comment)
    comment = re.sub(r"[-=]{3,}", "", comment).strip(" :.-")
    if 5 <= len(comment) <= 160 and not re.fullmatch(r"[\d\s;,.]+", comment) and not re.search(r"[{}]|\b(?:let|unwrap|Some|None)\b", comment):
        return clean(comment, 120)
    if isinstance(name, str) and not name.startswith("0x"):
        label = re.sub(r"(?<=[a-z])(?=[A-Z])", " ", name.replace("_", " ").replace(".", " / "))
        return clean(label, 100)
    operation = re.search(r"\b(?:Event|Operation|Request|Command)::([A-Z][A-Za-z0-9_]+)", text(arm["body"]))
    if operation:
        return re.sub(r"(?<=[a-z])(?=[A-Z])", " ", operation[1]).replace("_", " ")
    for word, label in sorted(FAMILIES.items(), key=lambda item: item[0] in ("native", "sys", "system")):
        if word in record["fn"] or word in record["file"].stem:
            return label + " (" + record["fn"] + ")"
    calls = [n for n in walk(arm["body"]) if n.type == "call_expression"]
    for call in calls:
        function = text(call.child_by_field_name("function"))
        if re.fullmatch(r"self\.[a-z_]+", function):
            method = function[5:]
            if method not in ("byte", "word", "value", "bump", "commit", "advance", "current"):
                return clean(method.replace("_", " ").capitalize(), 100)
    assignments = re.findall(r"(?:self|state|machine|m|sys)\.([a-z_][\w.]*)\s*(?:\+=|-=|=(?!=))", text(arm["body"]))
    if assignments:
        return "Update " + ", ".join(dict.fromkeys(v.replace("_", " ").replace(".", " / ") for v in assignments[:3]))
    return "Execute " + record["fn"].replace("_", " ") + " branch"


def signature(node):
    while node and node.type != "function_item":
        node = node.parent
    if not node:
        return "closure"
    return text(node.child_by_field_name("parameters")) + " -> " + (text(node.child_by_field_name("return_type")) or "()")


def calls(node):
    """Actual call sites, not an inferred claim about a helper's effects."""
    found = []
    for child in walk(node):
        if child.type == "call_expression":
            function = text(child.child_by_field_name("function"))
            if re.match(r"(?:self\.|host\.|h\.|cx\.host\.|(?:\w+::)+)", function) and re.fullmatch(r"[\w.:]+", function):
                if function not in found:
                    found.append(function)
    return ", ".join(found[:5]) or "Local state/value operations"


def status(arm):
    body = text(arm["body"])
    # A branch ending unconditionally in an error is a rejection even when
    # its error message does not use a particular English keyword.
    node = arm["body"]
    if node.type == "closure_expression":
        node = node.child_by_field_name("body") or node
    if node.type == "block":
        children = [c for c in node.named_children if c.type not in ("line_comment", "block_comment")]
        node = children[-1] if children else node
    while node.type in ("expression_statement", "return_expression", "try_expression") and node.named_children:
        node = node.named_children[0]
    direct = bool(re.match(r"\s*(?:bail!|todo!|unimplemented!|(?:[\w:]+::)?Err\s*\()", text(node)))
    if direct:
        errors = re.findall(r'"([^"\n]+)"', text(node))
        return "Unsupported", clean(" / ".join(errors) or "Unconditional rejection")
    errors = re.findall(r'"([^"\n]*(?:unsupported|not implemented|unimplemented|not connected|not audited|not ported|not linked)[^"\n]*)"', body, re.I)
    if errors or re.search(r"\b(?:todo|unimplemented)!", body):
        return "Partial", clean(" / ".join(dict.fromkeys(errors)) or "Unimplemented branch")
    for nested in walk(arm["body"]):
        if nested.type != "match_arm":
            continue
        pattern = text(nested.child_by_field_name("pattern"))
        fallback = text(nested.child_by_field_name("value"))
        if pattern in ("_", "other", "unknown") and re.search(r"\.warn\s*\(", fallback):
            return "Partial", "Unknown subselectors warn and continue"
    if re.fullmatch(r"\s*\{\s*\}\s*", body):
        return "Empty handler", "Original no-op status not established"
    return "Handled", "—"


def short_id(value):
    return f"0x{value:02X}" if value <= 255 else f"0x{value:X}"


class Inventory:
    def __init__(self, engine):
        self.engine = engine
        self.rows = {}
        self.sites = []
        self.rules = []

    def rules_for(self, record, detailed=True):
        if record.get("conditional"):
            kind = "Conditional route"
        elif not SELECTOR.search(record["value"]):
            return
        else:
            kind = "Selector rules"
        if not record["arms"]:
            return
        if not record.get("conditional") and not any(
                n.type in ("integer_literal", "string_literal", "raw_string_literal")
                for a in record["arms"] for n in walk(a["pattern"])):
            return  # Enum-only Rust plumbing is not a numeric/name native ABI.
        self.sites.append(record)
        for arm in record["arms"]:
            if not detailed and " if " not in text(arm["pattern"]) and any(
                    n.type in ("integer_literal", "string_literal", "raw_string_literal")
                    for n in walk(arm["pattern"])):
                continue
            # Preserve the whole pattern, including wildcard/default and
            # dynamic guards. No native module/class identity is flattened.
            self.rules.append({"record": record, "arm": arm, "kind": kind})

    def add(self, section, key, name, meaning, state, gap, path, line, condition=""):
        key = str(key)
        identity = section, key
        row = {"id": key, "name": name, "purpose": meaning, "status": state, "gap": gap,
               "path": str(path.relative_to(ROOT)), "line": line, "condition": clean(condition, 100000)}
        previous = self.rows.get(identity)
        if previous is None:
            self.rows[identity] = row
        elif previous["status"] in ("Decode only", "Unreviewed") and state not in ("Decode only", "Unreviewed"):
            self.rows[identity] = row
        elif state in ("Partial", "Unsupported") and previous["status"] == "Handled":
            self.rows[identity] = row

    def route(self, section, record, numeric=True, tuple_index=None):
        # Preserve dispatch context: identical IDs in different object classes,
        # parent opcodes or RealLive modules are different native APIs.
        contexts = [(v, p) for v, p in record["contexts"]
                    if "opcode" in v or "c.op.module" in v or v in ("kind", "kind.as_str()", "class", "class.as_str()", "a[0]", "a[1]", "receiver")]
        if contexts:
            section += " / " + " / ".join(clean(v + "=" + p, 75) for v, p in contexts)
        receiver = owner(record["arms"][0]["body"]) if record["arms"] else ""
        if self.engine == "willplus" and section.startswith("Lua") and receiver:
            section += " / " + receiver
        if self.engine == "reallive" and record["value"].startswith("(c.op.module,"):
            for arm in record["arms"]:
                modules = tuple_part(arm["pattern"], 0)
                opcodes = tuple_part(arm["pattern"], 1)
                if modules is None or opcodes is None:
                    continue
                for module in atoms(modules, True):
                    for opcode in atoms(opcodes, True):
                        key = "module=" + str(module) + ", opcode=" + str(opcode)
                        state, gap = status(arm)
                        self.add(section, key, key, purpose(short_id(opcode), record, arm), state, gap,
                                 record["file"], arm["line"])
            return
        for arm in record["arms"]:
            node = arm["pattern"]
            if tuple_index is not None:
                node = tuple_part(node, tuple_index)
                if node is None:
                    continue
            keys = atoms(node, numeric)
            if self.engine == "majiro" and record["fn"] == "execute" and not keys:
                # The VM routes typed arithmetic/store families through a
                # guarded opcode variable. Expand only catalogue members.
                guard = text(arm["pattern"])
                bounds = re.search(r"\((0x[\da-f]+)\.\.=(0x[\da-f]+)\)\.contains\(op\)", guard)
                if not bounds:
                    bounds = re.search(r"\*op>=(0x[\da-f]+)\s*&&\s*\*op<=(0x[\da-f]+)", guard)
                if bounds:
                    lo, hi = (int(v, 16) for v in bounds.groups())
                    keys = [int(v, 16) for v in MAJIRO["opcodes"] if lo <= int(v, 16) <= hi]
            named = {}
            if self.engine == "nsystem" and not keys:
                # MSD command arms name the opcodes module constants.
                for ident in re.findall(r"\b[A-Z][A-Z0-9_]+\b", text(arm["pattern"]).split(" if ", 1)[0]):
                    if ident in NSYSTEM:
                        keys.append(NSYSTEM[ident])
                        named[NSYSTEM[ident]] = ident
            for key in keys:
                name = short_id(key) if numeric else key
                native_name = named.get(key)
                if self.engine == "reallive" and numeric:
                    module = REALLIVE_MODULES.get((record["file"].name, record["fn"]))
                    if module:
                        native_name = REALLIVE_NAMES.get((*module, key))
                state, gap = status(arm)
                guard = text(arm["pattern"])
                guard = guard.split(" if ", 1)[1] if " if " in guard else ""
                identity = name
                if tuple_index is not None:
                    parts = [text(c) for c in pattern_base(arm["pattern"]).children if c.type not in ("(", ")", ",")]
                    parts[tuple_index] = name
                    identity = "(" + ", ".join(parts) + ")"
                    # Majiro's operand pattern is not another API namespace.
                    if self.engine == "majiro":
                        identity = name
                self.add(section, identity, (identity + " " + native_name) if native_name else identity,
                         purpose(native_name or name, record, arm), state, gap,
                         record["file"], arm["line"], guard)


def choose(engine, r):
    """Return (section, numeric IDs, tuple component) for real dispatch sites."""
    file, fn, value = r["file"].name, r["fn"], r["value"]
    if r.get("conditional"):
        return None
    rel = r["file"].relative_to(ROOT / "crates" / ("crysirror-" + engine) / "src").as_posix()
    if engine == "circus":
        if rel.startswith("machine") and value == "i.opcode" and fn.startswith("execute"):
            return "DC4PH 0xABB6 opcodes", True, None
        if rel.startswith("dc2") and value == "instruction.opcode" and fn.startswith("execute"):
            return "DC2PC 0x5769 opcodes", True, None
        if rel.startswith("dc3") and value in ("instruction.opcode", "i.opcode") and fn.startswith(("execute", "prepare_value")):
            return "DC3WY 0xA09F shared handlers", True, None
        if file == "dc3_original.rs" and fn == "map" and value == "opcode":
            return "DC3 0x8A71 opcode remapping", True, None
        if file == "dc1_story.rs" and fn == "execute" and value == "record.opcode":
            return "DC1 ADV_TXT instructions", True, None
        if fn.startswith("execute") and value in ("a[0]", "a[1]", "selector", "subcommand"):
            return "Selectors / " + rel + " / " + fn + " / " + value, True, None
    elif engine == "system4":
        if rel.startswith("vm/") and fn.startswith("execute") and value in ("i.name", "instruction.name", "name"):
            return "AIN VM opcodes", False, None
        if not rel.startswith("vm/") and ("name" in value) and "query" not in value and fn not in ("fmt", "normalize_name"):
            if not value.startswith("("):
                return "Native exports / " + rel.removesuffix(".rs"), False, None
            parts = value.strip("()").split(",")
            at = next((i for i, p in enumerate(parts) if "name" in p), None)
            if at is not None:
                return "Native exports / " + rel.removesuffix(".rs"), False, at
        if file == "adapter.rs" and value in ("number", "code", "id", "syscall"):
            return "System calls", True, None
    elif engine == "reallive":
        if value in ("(command.op.modtype, command.op.module)", "(c.op.modtype, c.op.module)"):
            return "Module routing / " + rel.removesuffix(".rs"), True, 1
        if value.startswith("(op.opcode,"):
            return "Module commands / " + rel.removesuffix(".rs") + " / " + fn, True, 0
        if "op.opcode" in value or value in ("opcode", "op"):
            if rel.startswith(("adapter/", "vm/modules/")):
                return "Module commands / " + rel.removesuffix(".rs") + " / " + fn, True, None
        if value.startswith("(c.op.module,"):
            return "Adapter module/opcode pairs / " + fn, True, 1
    elif engine == "avg32":
        if file == "scenario.rs" and value == "self.cmd" and fn == "decode":
            return "Scenario opcodes", True, None
        if file == "scenario.rs" and value in ("subcommand", "subcommand >> 4", "self.byte()") and fn not in ("decode", "value", "conditions"):
            return "Subcommands / " + fn + " / " + value, True, None
    elif engine == "catsystem2":
        if file == "kcs.rs" and fn == "instruction" and value == "op":
            return "KCS v2 opcodes", True, None
        if value == "call.id" and "native_classes/" not in rel:
            return "Native gcalls", True, None
        if value in ("call.id", "cx.call.id") and "native_classes/" in rel:
            return "Class calls / " + rel.removesuffix(".rs"), True, None
        if file == "fes_runtime.rs" and "op.name.as_slice()" in value:
            return "FES command names", False, 1
    elif engine == "majiro":
        if file == "machine.rs" and fn == "execute" and value == "(&i.opcode,&i.operand)":
            return "MJO VM opcodes", True, 0
        if value in ("hash", "*hash") and (rel.startswith("adapter") or rel == "machine/events.rs"):
            return "Native syscall hashes", True, None
    elif engine == "nscripter":
        if fn in ("extended_command", "native_command", "execute") and value in ("command", "command.as_str()"):
            return "Script commands", False, None
    elif engine == "qlie":
        if file == "adapter.rs" and fn == "native" and value == "ins.op.as_str()":
            return "Script commands", False, None
        if rel.startswith("adapter/") and value in ("op", "method"):
            return "Object and system methods / " + rel.removesuffix(".rs") + " / " + fn, False, None
    elif engine == "system-nnn":
        if file == "machine.rs" and value == "i.code":
            return "Legacy1000 VM instructions", True, None
        if value == "call.code":
            return "Legacy1000 native commands", True, None
        if value.startswith("(") and "call.code" in value:
            at = value.strip("()").split(",").index(" call.code") if " call.code" in value.strip("()").split(",") else None
            if at is None:
                at = next((i for i, p in enumerate(value.strip("()").split(",")) if "call.code" in p), None)
            return "Legacy1000 native commands", True, at
        if file == "native_picture_effect.rs" and fn == "parse" and value == "command":
            return "Picture effect selectors", True, None
    elif engine == "sugarpot":
        if value in ("name", "name.as_str()") and fn in ("command", "event", "value_command"):
            return "Tsukumo no Tsuki script commands", False, None
        if fn == "command" and value == "selector":
            return "Command selectors / " + rel + " / " + value, True, None
    elif engine == "willplus":
        if file == "vm.rs" and fn == "execute" and value == "i.opcode":
            return "WS2 V3 opcodes", True, None
        if value in ("name", "method") and fn in ("cfunc", "call"):
            return "Lua host API / " + rel.removesuffix(".rs") + " / " + fn, False, None
        if file == "vm.rs" and fn == "method" and value == "(class, name)":
            return "Lua class dispatch", False, 1
    elif engine == "nsystem":
        if file == "adapter.rs" and fn == "execute" and value == "c.id":
            return "MSD commands", True, None
        if value in ("picked", "picked.or(cancel.then_some(0))", "picked.or(cancel.then_some(18))", "i") and rel.startswith("adapter"):
            return "Native screen controls / " + rel.removesuffix(".rs") + " / " + fn, True, None
    elif engine == "tjs2":
        if fn == "execute_instruction" and value == "inst.opcode":
            return "TJS2 VM opcodes", True, None
    elif engine == "kag":
        if fn in ("handle_special_tag", "handle_condition_tag") and value == "tag.tagname.as_str()":
            return "KAG parser control tags", False, None
    return None


def catalogues(inventories):
    inv = inventories["krkr"]
    path = ROOT / "crates/crysirror-krkr/src/surfaces.rs"
    root = syntax(path)
    declaration = next(n for n in walk(root) if n.type == "const_item" and text(n.child_by_field_name("name")) == "SURFACES")
    array = next(n for n in walk(declaration.child_by_field_name("value")) if n.type == "array_expression")
    for entry in array.named_children:
        if entry.type != "tuple_expression":
            continue
        parts = entry.named_children
        plugin, receiver = atoms(parts[0], False)[0], atoms(parts[1], False)[0]
        kind = text(parts[2])
        for member in (v for n in walk(parts[3]) for v in atoms(n, False)):
            identity = plugin + " / " + receiver.removeprefix("new:") + "." + member
            inv.add("Declared plug-in surfaces", identity, identity, "Known " + kind.lower() + " facade",
                    "Unsupported", "Explicit unmapped member; call/read/write fails", path, entry.start_point[0] + 1)
    path = ROOT / "crates/crysirror-text/src/markup.rs"
    declaration = next(n for n in walk(syntax(path)) if n.type == "const_item" and text(n.child_by_field_name("name")) == "FIELDS")
    array = next(n for n in walk(declaration.child_by_field_name("value")) if n.type == "array_expression")
    target = ROOT / "crates/crysirror-krkr/src/text_render_plugin.rs"
    src = target.read_text()
    anchor = src.index('format!("default{suffix}")')
    for entry in array.named_children:
        if entry.type == "tuple_expression" and entry.named_children:
            suffix = atoms(entry.named_children[0], False)
            if suffix:
                name = "TextRenderBase.default" + suffix[0]
                inv.add("TextRenderBase default properties", name, name, "Default " + suffix[0] + " markup property",
                        "Handled", "Field types/defaults are defined in crysirror_text::markup::FIELDS", target, src[:anchor].count("\n") + 1)
    inv = inventories["sugarpot"]
    path = ROOT / "crates/crysirror-sugarpot/src/adapter/update.rs"
    src = path.read_text()
    effects = {1: "Crossfade", 2: "Stored-mask transition", 19: "Zoom old stage in",
               20: "Zoom target stage toward native size", 21: "Shrink old stage", 22: "Expand target stage",
               23: "Rotate/zoom old stage", 24: "Rotate/zoom target stage",
               25: "Circle iris", 26: "Inverse circle iris", 27: "Angular transition",
               28: "Angular transition", 29: "Band transition", 30: "Band transition",
               31: "Mosaic transition", 32: "Truncated row blur", 33: "4x4 cell reveal",
               34: "Wave transition", 35: "Squeeze transition", 36: "Squeeze transition",
               37: "Random runs / middle colour", 38: "Random runs / mean colour", 39: "Moving-disc copies"}
    # Numeric families are admitted in the command's range guard; do not
    # mistake absent individual match arms for absent UD implementations.
    for code in range(1, 40):
        if code in (*range(3, 10), 14):
            route = ROOT / "crates/crysirror-sugarpot/src/adapter.rs"
            source = route.read_text()
            anchor = source.index('//449870\'s reviewed default')
            meaning = "Instant target copy; hide native windows"
        else:
            route, source = path, src
            anchor = source.index("fn mask_update_step")
            meaning = effects.get(code, "Native line/column scan transition")
        inv.add("UD effect kinds", str(code), str(code), meaning,
                "Partial" if code == 2 else "Handled",
                "Scalar stored-mask policy; no MMX bit-parity claim" if code == 2 else "—",
                route, source[:anchor].count("\n") + 1)
    path = ROOT / "crates/crysirror-sugarpot/src/values.rs"
    src = path.read_text()
    dummy = re.search(r'if \[(.*?)\]\.contains\(&name\)', src)
    for name in re.findall(r'"([^"]+)"', dummy[1]):
        inv.add("Tsukumo no Tsuki script commands", name, name, "Native named dummy command",
                "Original no-op", "Explicit named slot; not an unknown-command fallback", path,
                src[:dummy.start()].count("\n") + 1)
    path = ROOT / "crates/crysirror-sugarpot/src/machine.rs"
    src = path.read_text()
    anchor = src.index("if line.starts_with('!')")
    inv.add("Source control prefixes", "!", "!", "Native case 0x21; advance source line",
            "Original no-op", "—", path, src[:anchor].count("\n") + 1)
    inv = inventories["circus"]
    path = ROOT / "crates/crysirror-circus/src/opcodes.rs"
    src = path.read_text()
    for label, state in (("NATIVE_NO_OPS", "Original no-op"), ("NATIVE_OPERATIONS", "Unreviewed")):
        block = re.search(label + r"\s*:\s*\[[^;]+;\s*\d+\]\s*=\s*\[([^]]+)\]", src)
        if not block:
            raise ValueError("Circus native opcode catalogue changed")
        ids = [int(v, 16) for v in re.findall(r"0x[0-9a-fA-F]+", block[1])]
        for value in ids:
            if state == "Original no-op":
                inv.rows.pop(("DC4PH 0xABB6 opcodes", short_id(value)), None)
            inv.add("DC4PH 0xABB6 opcodes", short_id(value), short_id(value),
                    "Original empty dispatcher entry" if state == "Original no-op" else "Native operation",
                    state, "—" if state == "Original no-op" else "No extracted execution branch",
                    path, src[:block.start()].count("\n") + 1)
    inv.add("DC4PH 0xABB6 opcodes", "0x77", "Continuation", "Continuation operand record", "Continuation", "Not an independent operation", path, 14)
    # These handlers use if-guards before the main match rather than match
    # arms. Keep their routing anchors explicit and fail on source drift.
    guarded = {
        0x0C: ("text_ops.rs", r"if i\.opcode == 0x0c", "Commit UI builder to the message panel"),
        0x0F: ("effect_ops.rs", r"if i\.opcode == 0x0f", "Framebuffer operation dispatcher"),
        0x38: ("sprite_ops.rs", r"if i\.opcode == 0x38", "Decoration/sprite operation dispatcher"),
        0x3E: ("text_ops.rs", r"if i\.opcode == 0x3e", "Enter a native menu"),
        0x52: ("movie_ops.rs", r"if i\.opcode != 0x52", "Movie playback and control"),
        0xB0: ("macro_ops.rs", r"let handled = if i\.opcode == 0xb0", "Record native command macros and select a bank"),
    }
    for code, (file, anchor, meaning) in guarded.items():
        path = ROOT / "crates/crysirror-circus/src/machine" / file
        src = path.read_text()
        found = re.search(anchor, src)
        if not found:
            raise ValueError("Circus guarded route changed: " + short_id(code))
        inv.add("DC4PH 0xABB6 opcodes", short_id(code), short_id(code), meaning,
                "Handled", "—", path, src[:found.start()].count("\n") + 1)
    path = ROOT / "crates/crysirror-system4/src/opcodes.rs"
    src = path.read_text()
    inv = inventories["system4"]
    for match in re.finditer(r'(0x[\da-fA-F]+)\s*=>\s*\("([^"]+)",\s*(\d+)\)', src):
        code, name, words = match.groups()
        key = ("AIN VM opcodes", name)
        row = inv.rows.pop(key, None)
        if row:
            inv.add(key[0], code, name, PURPOSE.get(name, row["purpose"]), row["status"], row["gap"], ROOT / row["path"], row["line"], row["condition"])
        else:
            inv.add(key[0], code, name, PURPOSE.get(name, name.replace("_", " ")), "Decode only", "No extracted executor route", path, src[:match.start()].count("\n") + 1)
    path = ROOT / "crates/crysirror-willplus/src/tables.rs"
    src = path.read_text()
    for profile, contents in re.findall(r'const (V[123]):[^=]+=\s*&?\s*\[(.*?)\];', src, re.S):
        for match in re.finditer(r'\((0x[\da-fA-F]+),\s*"([^"]*)"\)', contents):
            code, schema = match.groups()
            inventories["willplus"].add("WS2 " + profile + " opcodes", short_id(int(code, 16)), code,
                "Operand layout: " + (schema or "none"), "Decode only", "No executor for this profile" if profile != "V3" else "No extracted executor route", path,
                src[:src.index(contents) + match.start()].count("\n") + 1)
    meanings = {
        0x01: "Conditional branch", 0x02: "Jump", 0x06: "Jump", 0x04: "Call a script", 0x05: "Return",
        0x07: "Transfer to another script", 0xFF: "End script", 0x09: "Variable arithmetic/assignment",
        0x0B: "Set a flag", 0x0D: "Fill variable range", 0x0E: "Fill flag range", 0x0F: "Present choices",
        0x11: "Start named timer", 0x12: "Wait for named timer", 0x14: "Display message", 0x15: "Set speaker",
        0x16: "Close message window", 0x1A: "Call Lua", 0x1C: "Call Lua", 0x1B: "Open system screen",
        0x1E: "Play BGM", 0x1F: "Stop BGM", 0x20: "Set volume", 0x2A: "Set volume",
        0x28: "Play sound/voice", 0x29: "Stop sound", 0x32: "Set face graphic", 0x33: "Load layer image",
        0x34: "Load PNA", 0x35: "Movie control", 0x3A: "Movie control", 0x37: "Erase layers",
        0x39: "Select PNA parts", 0x3F: "Set stacking order", 0x40: "Set layer mask", 0x42: "Record layer attributes",
        0x45: "Read layer property group", 0x46: "Set layer property group", 0x47: "Queue layer tween",
        0x48: "Start tween queue", 0x56: "Streak weather", 0x57: "Wait for layer tweens", 0x5C: "Streak weather",
        0x64: "Prepare screen transition", 0x65: "Run screen transition", 0x66: "Set transition rule",
    }
    inv = inventories["willplus"]
    for code, meaning in meanings.items():
        row = inv.rows.get(("WS2 V3 opcodes", short_id(code)))
        if row and row["status"] != "Decode only":
            row["purpose"] = meaning
            if code in (0x42, 0x46, 0x56, 0x5C):
                row["status"] = "Partial"
                row["gap"] = {0x42: "Attributes retained; visible semantics not connected",
                              0x46: "Groups 10/11 retained without visible semantics",
                              0x56: "Weather semantics inferred from script usage",
                              0x5C: "Weather semantics inferred from script usage"}[code]
    inv = inventories["majiro"]
    descriptions = {
        "mul": "Multiply", "div": "Divide", "rem": "Remainder", "add": "Add", "sub": "Subtract",
        "shr": "Shift right", "shl": "Shift left", "cle": "Compare less than or equal", "clt": "Compare less than",
        "cge": "Compare greater than or equal", "cgt": "Compare greater than", "ceq": "Compare equal",
        "cne": "Compare unequal", "xor": "Bitwise XOR", "andl": "Logical AND", "orl": "Logical OR",
        "and": "Bitwise AND", "or": "Bitwise OR", "notl": "Logical NOT", "not": "Bitwise NOT", "neg": "Negate",
        "st": "Store variable and retain result", "stp": "Store variable and pop result",
        "stelem": "Store array element and retain result", "stelemp": "Store array element and pop result",
        "ldc": "Load constant", "ld": "Load variable", "alloca": "Allocate local variables", "argcheck": "Check argument types",
        "br": "Branch", "brtrue": "Branch if true", "brfalse": "Branch if false", "bne": "Branch if unequal",
        "bge": "Branch if greater than or equal", "ble": "Branch if less than or equal",
        "blt": "Branch if less than", "bgt": "Branch if greater than", "syscall": "Call native syscall and retain result",
        "syscallp": "Call native syscall and discard result", "ldelem": "Load array element", "line": "Record source/read-history line",
        "text": "Emit text", "proc": "Process message", "ctrl": "Process text control", "conv": "Convert scalar type",
        "bsel": "Button-selection control", "switch": "Select branch from jump table", "callp": "Call function and discard result",
        "nop": "No operation (reference mnemonic)",
    }
    for code, metadata in MAJIRO["opcodes"].items():
        identity = ("MJO VM opcodes", code)
        row = inv.rows.get(identity)
        name = metadata["name"]
        meaning = PURPOSE.get(name, descriptions.get(name.split(".")[0], name))
        if metadata["operator"]:
            meaning += " (" + metadata["operator"] + ")"
        if row:
            row["name"], row["purpose"] = name, meaning
        else:
            path = ROOT / "crates/crysirror-majiro/src/program.rs"
            inv.add(identity[0], code, name, meaning, "Unreviewed", "Reference opcode; no indexed executor route", path, 1)
    inv = inventories["avg32"]
    meanings = {
        0x02: "Insert message line break", 0x05: "Set message font size", 0x08: "Consume control operands",
        0x15: "Branch when condition is false", 0x16: "Call or transfer to another Seen scenario",
        0x17: "Start/wait for screen shake", 0x18: "Set font colour", 0x1B: "Call local subroutine",
        0x1C: "Jump", 0x1D: "Call selected jump-table entry", 0x1E: "Jump to selected jump-table entry",
        0x20: "Return or change call stack", 0x58: "Present choices", 0x59: "String operations",
        0x5B: "Write variable/flag sequence", 0x5C: "Fill variable/flag range", 0x5D: "Copy variable/flag range",
        0x5E: "Query date/time or current Seen", 0x5F: "Add to variable ranges", 0x60: "System command dispatcher",
        0x61: "Name-entry control", 0x63: "Copy graphics regions", 0x66: "Write graphics pixel",
    }
    for code, meaning in meanings.items():
        row = inv.rows.get(("Scenario opcodes", short_id(code)))
        if row:
            row["purpose"] = meaning
            if code == 0x08:
                row["status"], row["gap"] = "Partial", "Operands consumed without a connected runtime effect"
    for code in [0x06, 0x1A, *range(0x22, 0x2A), 0x65]:
        row = inv.rows.get(("Scenario opcodes", short_id(code)))
        if row:
            row["purpose"] = "End command and yield"
    # TJS2 has a complete 0..127 catalogue, including four property forms of
    # each arithmetic/logical family. Read its names directly from the source.
    path = ROOT / "crates/crysirror-tjs2/src/bytecode/instruction.rs"
    src = path.read_text()
    mnemonic = src[src.index("pub(super) fn opcode_mnemonic"):]
    names = {int(code): name for code, name in re.findall(r'(\d+)\s*=>\s*"([^"]+)"', mnemonic)}
    bases_block = re.search(r'let bases\s*=\s*\[(.*?)\];', mnemonic, re.S)
    bases = re.findall(r'"([^"]*)"', bases_block[1])
    for code in range(26, 82):
        names[code] = bases[(code - 26) // 4] + ["", "pd", "pi", "p"][(code - 26) % 4]
    inv = inventories["krkr"]
    for code, name in sorted(names.items()):
        identity = ("TJS2 VM opcodes", short_id(code))
        existing = inv.rows.get(identity)
        if existing:
            existing["name"] = name
            existing["purpose"] = PURPOSE.get(name, name)
        else:
            inv.add(identity[0], identity[1], name, PURPOSE.get(name, name), "Unreviewed",
                    "Catalogue entry; no indexed execution branch", path, 223)


def render(inv):
    groups = defaultdict(list)
    for (section, key), row in inv.rows.items():
        groups[section].append(row)
    counts = Counter(row["status"] for row in inv.rows.values())
    path = OUT / (inv.engine + ".md")
    existing = path.read_text() if path.exists() else ""
    introduction = existing.split(GENERATED, 1)[0].rstrip() if GENERATED in existing else f"# {TITLES[inv.engine]} developer reference\n\n[All engines](README.md)"
    lines = [introduction, "", GENERATED, "", "## Opcode and API implementation tables", "",
             "`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.", "",
             "| Status | Entries |", "| --- | ---: |"]
    lines += [f"| {state} | {count} |" for state, count in sorted(counts.items())]
    if inv.engine == "circus":
        lines += ["", "DC4 0xAAB6 is recognized by the parser but has no execution dispatcher. DC3 0x8A71 remaps are listed separately from shared DC3WY handlers."]
    lines += ["", "These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows."]
    if inv.engine == "krkr":
        lines += ["", "Native API names use the Rust registration receiver when the native class is selected dynamically. The section names identify the registration module and function."]
    if inv.engine == "reallive":
        lines += ["", "Module routing uses `(modtype, module)` IDs. Command tables keep opcode/overload pairs and parent module conditions separate; a bare opcode is scoped by its linked dispatcher."]
    if inv.engine == "majiro":
        lines += ["", "Reference mnemonics: [MajiroTools Opcode.cs](" + MAJIRO["source"] + ") at the revision recorded in [SOURCE.json](../../crates/crysirror-majiro/SOURCE.json). Execution status comes from CrySirror's handlers, not that reference catalogue."]
    for section, rows in sorted(groups.items()):
        def key(row):
            try:
                return 0, int(row["id"], 16)
            except ValueError:
                return 1, row["id"]
        lines += ["", "## " + section, "", "| ID / API | Purpose | Status | Gap / condition | Source |", "| --- | --- | --- | --- | --- |"]
        for row in sorted(rows, key=key):
            identity = row["id"] + (" " + row["name"].removeprefix(row["id"] + " ") if row["name"] != row["id"] else "")
            gap = row["gap"]
            if row["condition"]:
                gap = ("" if gap == "—" else gap + "; ") + "Guard: " + row["condition"]
            source = f"[{Path(row['path']).name}:{row['line']}](../../{row['path']}#L{row['line']})"
            lines.append(f"| `{clean(identity, 100000)}` | {clean(row['purpose'])} | {row['status']} | {clean(gap, 100000)} | {source} |")
    if inv.rules:
        lines += ["", "## Subselectors and conditional dispatch", "",
                  "Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls."]
        groups = defaultdict(list)
        for rule in inv.rules:
            r = rule["record"]
            groups[(str(r["file"]), owner(rule["arm"]["body"]), r["fn"])].append(rule)
        for (file, receiver, fn), rules in sorted(groups.items()):
            r = rules[0]["record"]
            rel = Path(file).relative_to(ROOT).as_posix()
            qualified = (receiver + "::" if receiver else "") + fn
            lines += ["", "### " + clean(rel.split("/src/", 1)[-1] + " / " + qualified, 180), "",
                      "```rust", "fn " + fn + signature(r["arms"][0]["body"]), "```", "",
                      "| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |",
                      "| --- | --- | --- | --- | --- |"]
            for rule in rules:
                r = rule["record"]
                arm = rule["arm"]
                pattern = text(arm["pattern"])
                state, gap = status(arm)
                if r.get("declaration"):
                    state, gap = "Guard / allowlist", "Predicate only; execution is defined by its caller"
                if re.fullmatch(r"\s*(?:return\s+)?(?:Ok\()?(?:None|false)\)?[;\s]*", text(arm["body"])):
                    state, gap = "Return only", "Boolean/optional result interpreted by the caller"
                selector = "condition" if r.get("conditional") else r["value"]
                if r["contexts"]:
                    selector += "; under " + " / ".join(v + "=" + p for v, p in r["contexts"])
                restriction = state + (": " + gap if gap != "—" else "")
                source = "[" + str(arm["line"]) + "](../../" + rel + "#L" + str(arm["line"]) + ")"
                lines.append("| `" + clean(selector, 100000) + "` | `" + clean(pattern, 100000) + "` | " + clean(calls(arm["body"]), 200) + " | " + clean(restriction, 100000) + " | " + source + " |")
    return "\n".join(lines) + "\n", counts


def generate():
    inventories = {engine: Inventory(engine) for engine in TITLES}
    for engine in list(TITLES) + ["tjs2", "kag"]:
        inv = inventories["krkr" if engine in ("tjs2", "kag") else engine]
        for path in sorted((ROOT / "crates" / ("crysirror-" + engine) / "src").rglob("*.rs")):
            if "test" in path.name:
                continue
            matches, regs = collect(path)
            for record in matches:
                selection = choose(engine, record)
                # Catalogue rows are emitted separately. Serialized value
                # kinds and decompiler formatting are data, not native APIs.
                rel = path.relative_to(ROOT / "crates" / ("crysirror-" + engine) / "src").as_posix()
                metadata = (rel == "opcodes.rs" or rel.startswith(("decompile/", "native_save/"))
                            or rel in ("bytecode/disasm.rs", "bytecode/instruction.rs", "bytecode/verify.rs", "settings_policy.rs")
                            or (rel in ("vm/global_save.rs", "native_storage.rs") and "kind" in record["value"]))
                if not metadata:
                    inv.rules_for(record, detailed=not bool(selection))
                if selection:
                    section, numeric, at = selection
                    if numeric and at is None and not any(atoms(a["pattern"], True) for a in record["arms"]) and any(atoms(a["pattern"], False) for a in record["arms"]):
                        numeric = False
                    inv.route(section, record, numeric, at)
            if engine in ("krkr", "tjs2"):
                for name, fn, line, node in regs:
                    if "{" in name and path.name in ("surfaces.rs", "text_render_plugin.rs"):
                        continue  # Expanded from the declarative tables above.
                    state, gap = status({"body": node})
                    if re.search(r"\{[^}]+\}", name):
                        state, gap = "Parameterized", "Registration name supplied by the caller; inspect the linked registration function"
                    inv.add("Registered TJS native APIs / " + path.stem + " / " + fn, name, name,
                            PURPOSE.get(name.split(".")[-1], name.split(".")[-1]), state, gap, path, line)
    catalogues(inventories)
    outputs = {}
    index = ["# Native engine developer references", "",
             "Each reference contains the native ABI, dispatch/return and wait contracts, shared-service mappings, implementation tables, subselectors and dynamic routing rules.", "",
             "| Engine | Reference | Opcode / API entries | Selector / conditional rules |", "| --- | --- | ---: | ---: |"]
    for engine, inv in inventories.items():
        if not inv.rows:
            raise ValueError("No native API entries extracted for " + engine)
        output, counts = render(inv)
        outputs[OUT / (engine + ".md")] = output
        index.append(f"| {TITLES[engine]} | [{engine}]({engine}.md) | {len(inv.rows)} | {len(inv.rules)} |")
    index += ["", "## Status", "", "| Label | Meaning |", "| --- | --- |",
              "| Handled | Executable source branch present. Parameter restrictions remain in the row and linked handler. |",
              "| Partial | The branch includes an explicit unsupported/unimplemented path. |",
              "| Unsupported | The branch explicitly rejects execution. |",
              "| Original no-op | Confirmed empty entry in the recorded native dispatch catalogue. |",
              "| Empty handler | Empty source body without original no-op evidence. |",
              "| Decode only | An operand/name catalogue entry with no extracted execution route. |",
              "| Unreviewed | Known native catalogue entry whose execution route needs review. |",
              "| Continuation | Bytecode continuation record, not a separate VM operation. |",
              "| Parameterized | Native registration accepts a caller-supplied name; this is an open name pattern, not one fixed builtin. |",
              "| Guard / allowlist | A native dispatch predicate; execution belongs to the caller. |",
              "| Return only | The branch returns a boolean/optional result; its caller defines pending/forward semantics. |", "",
              "Counts are table entries, not an upstream completeness percentage. Modules, class receivers, profiles and subcommands have separate namespaces. Unknown native totals remain unknown.", "",
              "Siglus, BGI, SoftPAL and RFVP use the [upstream-version and local-change table](../INTEGRATED_ENGINES.md).", "",
              "Regenerate after dispatch changes:", "", "```sh", "python3 -m pip install -r tools/requirements-native-api.txt",
              "python3 tools/native_api_inventory.py", "python3 tools/native_api_inventory.py --check", "```", "",
              "## Maintenance and coverage", "",
              "The text above the generated marker in each reference is maintained by developers. The generator rebuilds explicit dispatch tables and selector/conditional rules from production Rust source, including default branches. Update the ABI and service contract when changing a dispatcher.", "",
              "Coverage includes matched opcode/module/export tables, native selector expressions and conditional name/opcode routes. Serialized save-value kinds, opcode layout catalogues already listed in the opcode tables, decompilers and test modules are excluded from selector counts. Rust pattern counts and aliases are not native operation totals. Parameterized registrations retain their receiver expression; game-defined macros/functions have no fixed built-in denominator.", "",
              "`Handled` reports a source implementation, not a semantic-parity certificate. `—` means no explicit rejection was found in that branch. Validation is recorded separately in each reference; neither table generation nor source inspection is a game or platform execution result."]
    outputs[OUT / "README.md"] = "\n".join(index) + "\n"
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check generated documents without changing them")
    args = parser.parse_args()
    changed = []
    for path, content in generate().items():
        if not path.exists() or path.read_text() != content:
            changed.append(str(path.relative_to(ROOT)))
            if not args.check:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
    if args.check and changed:
        sys.exit("Outdated native API tables: " + ", ".join(changed))
    print(("Checked" if args.check else "Generated") + " native API tables" + (": " + ", ".join(changed) if changed else " (up to date)"))


if __name__ == "__main__":
    main()
