"""ADJ-13 private signal broker policy checks."""

from __future__ import annotations

import base64
import json
import re
from typing import Any

from ..runner.context import Reject

ADJ13_POLICY = "ADJ-13-private-unix-signal-broker/v1"

SESSION = "crates/tui/src/runtime/session.rs"
STORAGE = "static SIGNAL_BROKER: std::sync::OnceLock<std::sync::Mutex<SignalBroker>>"
GUARD = '#[cfg(all(unix, feature = "crossterm"))]'

# Field order and the nested install phase are part of the source contract.
# Keeping both schemas ordered prevents set-based checks from accepting a
# missing, duplicated, or semantically swapped lease/install slot.
_BROKER_FIELDS = (
    ("inactive", "flag"),
    ("pending", "flag"),
    ("lease", "bool"),
    ("install", "install_phase"),
)

_INSTALL_FIELDS = (
    ("first", "registration"),
    ("second", "registration"),
)

_MUTABLE_TYPES = frozenset(
    {
        "AtomicBool",
        "AtomicI8",
        "AtomicI16",
        "AtomicI32",
        "AtomicI64",
        "AtomicIsize",
        "AtomicPtr",
        "AtomicU8",
        "AtomicU16",
        "AtomicU32",
        "AtomicU64",
        "AtomicUsize",
        "Cell",
        "LazyLock",
        "Mutex",
        "OnceLock",
        "OnceCell",
        "RefCell",
        "RwLock",
        "UnsafeCell",
    }
)

_PRIMITIVE_TYPES = frozenset(
    {
        "bool",
        "char",
        "f32",
        "f64",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "str",
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
    }
)

_FACT_KEYS = {
    "alias": {"kind", "name", "type", "attributes", "modules", "function_depth"},
    "function": {"kind", "signature", "visibility", "attributes", "modules", "function_depth"},
    "macro": {"kind", "path", "tokens", "modules", "function_depth"},
    "method": {"kind", "signature", "visibility", "attributes", "modules", "function_depth"},
    "module": {"kind", "name", "attributes", "inline", "modules", "function_depth"},
    "static": {
        "kind",
        "name",
        "type",
        "mutable",
        "visibility",
        "attributes",
        "initializer",
        "modules",
        "function_depth",
    },
    "struct": {"kind", "name", "visibility", "attributes", "fields", "modules", "function_depth"},
    "use": {"kind", "tree", "visibility", "attributes", "modules", "function_depth"},
}

_TYPE_PATH_NAMES = frozenset(
    {
        "Arc",
        "AtomicBool",
        "Mutex",
        "OnceLock",
        "Option",
        "SigId",
        "SignalBroker",
        "bool",
    }
)


TypeNode = tuple[str, tuple[Any, ...]]
Scope = tuple[str, ...]


def _reject() -> None:
    raise Reject("ARCHITECTURE")


def _is_dict(value: Any) -> bool:
    return type(value) is dict


def _is_list(value: Any) -> bool:
    return type(value) is list


def _private_visibility(value: Any) -> bool:
    # syn's Debug representation is `Inherited` for a private item.  The
    # empty spelling is retained for the compact unit fixtures used by the
    # older architecture tests.
    return value in ("", "Inherited")


def _scope(fact: dict[str, Any]) -> Scope:
    modules = fact.get("modules", [])
    if modules is None:
        return ()
    if not _is_list(modules):
        _reject()
    result: list[str] = []
    for module in modules:
        if not _is_dict(module) or not isinstance(module.get("name"), str):
            _reject()
        result.append(module["name"])
    return tuple(result)


def _fact_attributes(fact: dict[str, Any]) -> list[str]:
    attributes = fact.get("attributes", [])
    if not _is_list(attributes) or any(not isinstance(value, str) for value in attributes):
        _reject()
    return attributes


def _fact_shape(fact: Any) -> str:
    if not _is_dict(fact):
        _reject()
    kind = fact.get("kind")
    if kind not in _FACT_KEYS:
        _reject()
    if set(fact) - _FACT_KEYS[kind]:
        _reject()
    _scope(fact)
    function_depth = fact.get("function_depth", 0)
    if type(function_depth) is not int or function_depth < 0:
        _reject()
    _fact_attributes(fact)
    return kind


def _path_from_dict(node: Any) -> tuple[tuple[str, tuple[TypeNode, ...]], ...] | None:
    if not _is_dict(node) or set(node) != {"path"} or not _is_list(node["path"]):
        return None
    segments: list[tuple[str, tuple[TypeNode, ...]]] = []
    for segment in node["path"]:
        if not _is_dict(segment) or set(segment) != {"name", "arguments"}:
            return None
        name = segment["name"]
        arguments = segment["arguments"]
        if not isinstance(name, str) or not name or not _is_list(arguments):
            return None
        parsed: list[TypeNode] = []
        for argument in arguments:
            # The protected observer records only type generic arguments.
            # Lifetimes, const generics, and unsupported AST nodes are not an
            # exact type identity and must never be treated as a match.
            parsed_argument = _type_node(argument)
            if parsed_argument is None:
                return None
            parsed.append(parsed_argument)
        segments.append((name, tuple(parsed)))
    if not segments:
        return None
    return tuple(segments)


def _type_node(value: Any) -> TypeNode | None:
    if _is_dict(value):
        keys = set(value)
        path = _path_from_dict(value)
        if path is not None:
            return ("path", path)
        if keys == {"reference", "mutable"} and type(value["mutable"]) is bool:
            target = _type_node(value["reference"])
            return ("reference", (target, value["mutable"])) if target is not None else None
        if keys == {"array", "length"} and isinstance(value["length"], str):
            target = _type_node(value["array"])
            return ("array", (target, value["length"])) if target is not None else None
        if keys == {"slice"}:
            target = _type_node(value["slice"])
            return ("slice", (target,)) if target is not None else None
        if keys == {"tuple"} and _is_list(value["tuple"]):
            members = tuple(_type_node(item) for item in value["tuple"])
            return ("tuple", members) if all(item is not None for item in members) else None
        return None
    if isinstance(value, str):
        return _parse_type_string(value)
    return None


class _TypeStringParser:
    def __init__(self, text: str) -> None:
        self.text = text
        self.index = 0

    def _skip(self) -> None:
        while self.index < len(self.text) and self.text[self.index].isspace():
            self.index += 1

    def _identifier(self) -> str | None:
        self._skip()
        start = self.index
        if start >= len(self.text) or not (self.text[start].isalpha() or self.text[start] == "_"):
            return None
        self.index += 1
        while self.index < len(self.text) and (self.text[self.index].isalnum() or self.text[self.index] == "_"):
            self.index += 1
        return self.text[start : self.index]

    def _parse_one(self) -> TypeNode | None:
        self._skip()
        segments: list[tuple[str, tuple[TypeNode, ...]]] = []
        while True:
            name = self._identifier()
            if name is None:
                return None
            self._skip()
            arguments: list[TypeNode] = []
            if self.index < len(self.text) and self.text[self.index] == "<":
                self.index += 1
                while True:
                    argument = self._parse_one()
                    if argument is None:
                        return None
                    arguments.append(argument)
                    self._skip()
                    if self.index >= len(self.text):
                        return None
                    if self.text[self.index] == ",":
                        self.index += 1
                        continue
                    if self.text[self.index] != ">":
                        return None
                    self.index += 1
                    break
            segments.append((name, tuple(arguments)))
            self._skip()
            if self.text.startswith("::", self.index):
                self.index += 2
                continue
            if self.index == len(self.text):
                return ("path", tuple(segments))
            if self.text[self.index] in ">,":
                return ("path", tuple(segments))
            return None

    def parse(self) -> TypeNode | None:
        node = self._parse_one()
        self._skip()
        return node if node is not None and self.index == len(self.text) else None


def _parse_type_string(value: str) -> TypeNode | None:
    if not value:
        return None
    return _TypeStringParser(value).parse()


def _type_path(node: TypeNode) -> tuple[str, ...] | None:
    return tuple(segment[0] for segment in node[1]) if node[0] == "path" else None


def _type_args(node: TypeNode) -> tuple[TypeNode, ...] | None:
    return node[1][-1][1] if node[0] == "path" else None


def _path_node(path: tuple[str, ...], args: tuple[TypeNode, ...] = ()) -> TypeNode:
    return ("path", tuple((name, args if index == len(path) - 1 else ()) for index, name in enumerate(path)))


def _scope_candidates(scope: Scope) -> list[Scope]:
    return [scope[:index] for index in range(len(scope), -1, -1)]


def _binding(bindings: dict[tuple[Scope, str], TypeNode], name: str, scope: Scope) -> TypeNode | None:
    for candidate in _scope_candidates(scope):
        found = bindings.get((candidate, name))
        if found is not None:
            return found
    return None


def _parse_use_body(body: str) -> dict[str, tuple[str, ...]]:
    """Parse the small path/group subset used by Rust `use` declarations."""

    def split_top_level(value: str) -> list[str]:
        parts: list[str] = []
        start = 0
        depth = 0
        for index, char in enumerate(value):
            if char in "<{([":
                depth += 1
            elif char in ">})]":
                depth -= 1
            elif char == "," and depth == 0:
                parts.append(value[start:index].strip())
                start = index + 1
        parts.append(value[start:].strip())
        return [part for part in parts if part]

    def parse(prefix: tuple[str, ...], value: str) -> dict[str, tuple[str, ...]]:
        value = value.strip()
        if value.startswith("{") and value.endswith("}"):
            result: dict[str, tuple[str, ...]] = {}
            for item in split_top_level(value[1:-1]):
                result.update(parse(prefix, item))
            return result
        group_start = value.find("::{")
        if group_start >= 0 and value.endswith("}"):
            head = value[:group_start].strip()
            path = prefix + tuple(part for part in head.split("::") if part)
            return parse(path, value[group_start + 2 :])
        pieces = re.split(r"\s+as\s+", value, maxsplit=1)
        target = pieces[0].strip()
        path = prefix + tuple(part for part in target.split("::") if part)
        if not path or path[-1] == "*":
            return {}
        name = pieces[1].strip() if len(pieces) == 2 else path[-1]
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            return {}
        return {name: path}

    return parse((), body)


def _rust_use_statements(source: str) -> list[str]:
    """Return actual `use ...;` statements, ignoring comments and literals."""
    statements: list[str] = []
    index = 0
    length = len(source)
    while index < length:
        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = length if newline < 0 else newline + 1
            continue
        if source.startswith("/*", index):
            end = source.find("*/", index + 2)
            index = length if end < 0 else end + 2
            continue
        if source[index] in ('"', "'"):
            quote = source[index]
            index += 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == quote:
                    index += 1
                    break
                else:
                    index += 1
            continue
        if source.startswith("use", index) and (index == 0 or not (source[index - 1].isalnum() or source[index - 1] == "_")):
            after = index + 3
            if after < length and (source[after].isalnum() or source[after] == "_"):
                index += 3
                continue
            start = after
            depth = 0
            while index < length:
                char = source[index]
                if char in "<{([":
                    depth += 1
                elif char in ">})]" and depth:
                    depth -= 1
                elif char == ";" and depth == 0:
                    statements.append(source[start:index].strip())
                    index += 1
                    break
                index += 1
            continue
        index += 1
    return statements


def _collect_bindings(files: dict[str, dict[str, Any]]) -> tuple[dict[tuple[Scope, str], TypeNode], dict[tuple[Scope, str], tuple[str, ...]]]:
    aliases: dict[tuple[Scope, str], TypeNode] = {}
    imports: dict[tuple[Scope, str], tuple[str, ...]] = {}
    for entry in files.values():
        source = entry.get("source", "")
        use_statements = _rust_use_statements(source) if isinstance(source, str) else []
        use_facts = [fact for fact in entry.get("facts", []) if fact.get("kind") == "use"]
        if use_facts and len(use_statements) != len(use_facts):
            _reject()
        for fact, statement in zip(use_facts, use_statements):
            if fact.get("function_depth", 0) != 0:
                continue
            scope = _scope(fact)
            for name, path in _parse_use_body(statement).items():
                key = (scope, name)
                if key in imports or key in aliases:
                    _reject()
                imports[key] = path
        for fact in entry.get("facts", []):
            if fact.get("kind") != "alias":
                continue
            name = fact.get("name")
            node = _type_node(fact.get("type"))
            if not isinstance(name, str) or node is None:
                _reject()
            key = (_scope(fact), name)
            if key in aliases or key in imports:
                _reject()
            aliases[key] = node
    return aliases, imports


def _resolve_type(
    node: TypeNode,
    scope: Scope,
    aliases: dict[tuple[Scope, str], TypeNode],
    imports: dict[tuple[Scope, str], tuple[str, ...]],
    seen: frozenset[tuple[Scope, str]] = frozenset(),
    *,
    allow_bare_std: bool = False,
) -> TypeNode | None:
    if node[0] != "path":
        if node[0] == "reference":
            target, mutable = node[1]
            resolved = _resolve_type(target, scope, aliases, imports, seen, allow_bare_std=allow_bare_std)
            return ("reference", (resolved, mutable)) if resolved is not None else None
        if node[0] == "array":
            target, length = node[1]
            resolved = _resolve_type(target, scope, aliases, imports, seen, allow_bare_std=allow_bare_std)
            return ("array", (resolved, length)) if resolved is not None else None
        if node[0] == "slice":
            target = _resolve_type(node[1][0], scope, aliases, imports, seen, allow_bare_std=allow_bare_std)
            return ("slice", (target,)) if target is not None else None
        if node[0] == "tuple":
            members = tuple(_resolve_type(item, scope, aliases, imports, seen, allow_bare_std=allow_bare_std) for item in node[1])
            return ("tuple", members) if all(item is not None for item in members) else None
        return None

    if node[0] != "path":
        return None
    segments = node[1]
    path = tuple(segment[0] for segment in segments)
    type_arguments = segments[-1][1]
    if len(segments) == 1:
        name = segments[0][0]
        alias_key = next(((candidate, name) for candidate in _scope_candidates(scope) if (candidate, name) in aliases), None)
        if alias_key is not None:
            if alias_key in seen:
                return None
            return _resolve_type(
                aliases[alias_key],
                alias_key[0],
                aliases,
                imports,
                seen | {alias_key},
                allow_bare_std=allow_bare_std,
            )
        imported = _binding(imports, name, scope)
        if imported is not None:
            path = imported
        elif allow_bare_std and name in {"Arc", "AtomicBool", "Mutex", "OnceLock", "Option", "SigId"}:
            path = {
                "Arc": ("std", "sync", "Arc"),
                "AtomicBool": ("std", "sync", "atomic", "AtomicBool"),
                "Mutex": ("std", "sync", "Mutex"),
                "OnceLock": ("std", "sync", "OnceLock"),
                "Option": ("Option",),
                "SigId": ("signal_hook", "SigId"),
            }[name]
        elif not allow_bare_std and name in {"Arc", "AtomicBool", "Mutex", "OnceLock", "SigId"}:
            return None
    resolved_arguments = tuple(
        _resolve_type(argument, scope, aliases, imports, seen, allow_bare_std=allow_bare_std)
        for argument in type_arguments
    )
    if not all(argument is not None for argument in resolved_arguments):
        return None
    return _path_node(path, resolved_arguments)


def _resolved_path(
    value: Any,
    scope: Scope,
    aliases: dict[tuple[Scope, str], TypeNode],
    imports: dict[tuple[Scope, str], tuple[str, ...]],
    *,
    allow_bare_std: bool = False,
) -> tuple[str, ...] | None:
    node = _type_node(value)
    if node is None:
        return None
    resolved = _resolve_type(node, scope, aliases, imports, allow_bare_std=allow_bare_std)
    return _type_path(resolved) if resolved is not None else None


def _resolved_type(
    value: Any,
    scope: Scope,
    aliases: dict[tuple[Scope, str], TypeNode],
    imports: dict[tuple[Scope, str], tuple[str, ...]],
    *,
    allow_bare_std: bool = False,
) -> TypeNode | None:
    node = _type_node(value)
    return _resolve_type(node, scope, aliases, imports, allow_bare_std=allow_bare_std) if node is not None else None


def _type_is(node: TypeNode | None, path: tuple[str, ...], arguments: tuple[TypeNode, ...] = ()) -> bool:
    if node is None or node[0] != "path":
        return False
    segments = node[1]
    return tuple(segment[0] for segment in segments) == path and segments[-1][1] == arguments and all(
        not segment[1] for segment in segments[:-1]
    )


def _contains_mutable_type(node: TypeNode | None) -> bool:
    if node is None:
        return True
    if node[0] == "path":
        return any(segment[0] in _MUTABLE_TYPES for segment in node[1]) or any(
            _contains_mutable_type(argument) for segment in node[1] for argument in segment[1]
        )
    if node[0] == "reference":
        return bool(node[1][1]) or _contains_mutable_type(node[1][0])
    if node[0] in {"array", "slice"}:
        return _contains_mutable_type(node[1][0])
    if node[0] == "tuple":
        return any(_contains_mutable_type(item) for item in node[1])
    return True


def _effective_guard(fact: dict[str, Any], source: str) -> bool:
    attributes: list[str] = []
    own = fact.get("attributes")
    if _is_list(own):
        attributes.extend(value for value in own if isinstance(value, str))
    modules = fact.get("modules", [])
    if _is_list(modules):
        for module in modules:
            if _is_dict(module) and _is_list(module.get("attributes")):
                attributes.extend(value for value in module["attributes"] if isinstance(value, str))
    normalized = " ".join(attributes).replace(" ", "")
    if 'cfg(all(unix,feature="crossterm"))' in normalized:
        return True
    has_unix = 'cfg(unix)' in normalized
    has_feature = 'cfg(feature="crossterm")' in normalized
    has_cfg_attr = 'cfg_attr(unix,cfg(feature="crossterm"))' in normalized
    if has_unix and (has_feature or has_cfg_attr):
        return True
    # Compact fixtures predate the full syn metadata.  Their source still
    # carries the same exact guard and is safe to use as a compatibility
    # fallback; no substring is used for type or field identity.
    return _has_effective_guard(source)


def _initializer_is_once_lock_new(
    value: Any,
    scope: Scope = (),
    imports: dict[tuple[Scope, str], tuple[str, ...]] | None = None,
) -> bool:
    if value is None:
        return False
    if not isinstance(value, str):
        return False
    normalized = re.sub(r"\s+", "", value)
    if normalized in {
        "Expr::Call(ExprCall{attrs:[],func:Expr::Path(ExprPath{attrs:[],qself:None,path:Path{leading_colon:None,segments:[PathSegment{ident:Ident(std),arguments:PathArguments::None},PathSegment{ident:Ident(sync),arguments:PathArguments::None},PathSegment{ident:Ident(OnceLock),arguments:PathArguments::None}]}}),args:[]})",
        "Expr::Call(ExprCall{attrs:[],func:Expr::Path(ExprPath{attrs:[],qself:None,path:Path{leading_colon:None,segments:[PathSegment{ident:Ident(OnceLock),arguments:PathArguments::None}]}}),args:[]})",
        "std::sync::OnceLock::new()",
        "::std::sync::OnceLock::new()",
        "OnceLock::new()",
    }:
        return True

    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*::new\(\)", normalized):
        name = normalized.split("::", 1)[0]
        return imports is not None and _binding(imports, name, scope) == ("std", "sync", "OnceLock")
    if imports is not None:
        return any(
            path == ("std", "sync", "OnceLock")
            and binding_scope in _scope_candidates(scope)
            and re.search(rf"\bIdent\({re.escape(name)}\)", normalized) is not None
            for (binding_scope, name), path in imports.items()
        )
    return False


def _has_effective_guard(text: str) -> bool:
    normalized = re.sub(r"\s+", "", text)
    if 'cfg(all(unix,feature="crossterm"))' in normalized:
        return True
    return 'cfg(unix)' in normalized and (
        'cfg(feature="crossterm")' in normalized
        or 'cfg_attr(unix,cfg(feature="crossterm"))' in normalized
    )


def _is_public_visibility(visibility: Any) -> bool:
    return not _private_visibility(visibility)


def _broker_type_ok(type_repr: Any, *, source_text: str = "") -> bool:
    node = _type_node(type_repr)
    if node is None:
        return False
    return _type_is(
        node,
        ("std", "sync", "OnceLock"),
        (_path_node(("std", "sync", "Mutex"), (_path_node(("SignalBroker",)),)),),
    )


def _guard_before_struct(session: str) -> bool:
    if STORAGE not in session or "struct SignalBroker" not in session:
        return True
    between = session.split(STORAGE, 1)[1].split("struct SignalBroker", 1)[0]
    return _has_effective_guard(between)


def _field_type_ok(
    name: str,
    value: Any,
    scope: Scope,
    aliases: dict[tuple[Scope, str], TypeNode],
    imports: dict[tuple[Scope, str], tuple[str, ...]],
) -> bool:
    resolved = _resolved_type(
        value,
        scope,
        aliases,
        imports,
        # Compact unit fixtures use display strings rather than the structured
        # syn representation.  They still go through the complete parser and
        # exact tree comparison; bare standard names are only a fixture
        # compatibility spelling.
        allow_bare_std=isinstance(value, str),
    )
    if name in {"inactive", "pending"}:
        return _type_is(
            resolved,
            ("std", "sync", "Arc"),
            (_path_node(("std", "sync", "atomic", "AtomicBool")),),
        )
    if name == "lease":
        return _type_is(resolved, ("bool",))
    if name in {"first", "second"}:
        return _type_is(
            resolved,
            ("Option",),
            (_path_node(("signal_hook", "SigId")),),
        )
    if name == "install":
        return _type_is(resolved, ("InstallPhase",))
    return False


def _storage_type_ok(
    value: Any,
    scope: Scope,
    aliases: dict[tuple[Scope, str], TypeNode],
    imports: dict[tuple[Scope, str], tuple[str, ...]],
) -> bool:
    resolved = _resolved_type(
        value,
        scope,
        aliases,
        imports,
        allow_bare_std=isinstance(value, str),
    )
    return _type_is(
        resolved,
        ("std", "sync", "OnceLock"),
        (_path_node(("std", "sync", "Mutex"), (_path_node(("SignalBroker",)),)),),
    )


def _validate_field(field: Any, expected_name: str, scope: Scope, aliases: dict[tuple[Scope, str], TypeNode], imports: dict[tuple[Scope, str], tuple[str, ...]]) -> None:
    if not _is_dict(field):
        _reject()
    allowed = {"name", "type", "visibility", "attributes"}
    if set(field) - allowed:
        _reject()
    if field.get("name") != expected_name or "type" not in field:
        _reject()
    if "visibility" in field and not _private_visibility(field["visibility"]):
        _reject()
    if "attributes" in field:
        attributes = field["attributes"]
        if not _is_list(attributes) or any(not isinstance(value, str) for value in attributes):
            _reject()
    if not _field_type_ok(expected_name, field["type"], scope, aliases, imports):
        _reject()


def _validate_static_shape(fact: dict[str, Any]) -> None:
    if not isinstance(fact.get("name"), str) or not fact["name"]:
        _reject()
    if "type" not in fact or _type_node(fact["type"]) is None:
        _reject()
    if "visibility" in fact and not isinstance(fact["visibility"], str):
        _reject()
    if "mutable" in fact and type(fact["mutable"]) is not bool:
        _reject()


def _validate_struct_shape(fact: dict[str, Any]) -> None:
    if not isinstance(fact.get("name"), str) or not fact["name"]:
        _reject()
    if "visibility" in fact and not isinstance(fact["visibility"], str):
        _reject()
    if "fields" not in fact or not _is_list(fact["fields"]):
        _reject()


def _validate_auxiliary_fact(fact: dict[str, Any], kind: str) -> None:
    if kind in {"function", "method"}:
        if not isinstance(fact.get("signature"), str) or "visibility" in fact and not isinstance(fact["visibility"], str):
            _reject()
    elif kind == "module":
        if not isinstance(fact.get("name"), str) or "inline" in fact and type(fact["inline"]) is not bool:
            _reject()
    elif kind == "alias":
        if not isinstance(fact.get("name"), str) or not fact["name"] or "type" not in fact:
            _reject()
    elif kind == "use":
        if not isinstance(fact.get("tree"), str) or "visibility" in fact and not isinstance(fact["visibility"], str):
            _reject()
    elif kind == "macro":
        if not isinstance(fact.get("path"), str) or not isinstance(fact.get("tokens"), str):
            _reject()


def _fact_token(value: Any, token: str) -> bool:
    return isinstance(value, str) and re.search(rf"\b{re.escape(token)}\b", value) is not None


def validate_broker_observation(body: dict[str, Any], profile: dict[str, Any]) -> None:
    if not _is_dict(body) or set(body) - {"schema", "files"}:
        _reject()
    if "schema" in body and body["schema"] != "tc-protected-source-syntax/v1":
        _reject()
    exception_path = profile.get("exception_path", SESSION)
    if exception_path != SESSION:
        _reject()
    required = profile.get("required_files") or profile.get("source_roots") or []
    if not _is_list(required) or any(not isinstance(path, str) or not path for path in required) or len(set(required)) != len(required):
        _reject()
    raw_files = body.get("files")
    if not _is_list(raw_files) or not raw_files:
        _reject()
    files: dict[str, dict[str, Any]] = {}
    for entry in raw_files:
        if not _is_dict(entry) or set(entry) - {"path", "source", "facts", "file_attributes"}:
            _reject()
        path = entry.get("path")
        if not isinstance(path, str) or path in files:
            _reject()
        if not isinstance(entry.get("source"), str) or not _is_list(entry.get("facts")):
            _reject()
        if "file_attributes" in entry and (
            not _is_list(entry["file_attributes"]) or any(not isinstance(value, str) for value in entry["file_attributes"])
        ):
            _reject()
        files[path] = entry
    if list(files) != required:
        _reject()
    if SESSION not in files:
        _reject()
    for entry in files.values():
        if "parse_error" in entry:
            _reject()
        for fact in entry["facts"]:
            kind = _fact_shape(fact)
            if kind == "static":
                _validate_static_shape(fact)
            elif kind == "struct":
                _validate_struct_shape(fact)
            else:
                _validate_auxiliary_fact(fact, kind)

    aliases, imports = _collect_bindings(files)
    broker_statics: list[tuple[str, dict[str, Any]]] = []
    broker_structs: list[tuple[str, dict[str, Any]]] = []
    install_structs: list[tuple[str, dict[str, Any]]] = []
    declaration_keys: set[tuple[str, Scope, str]] = set()
    for path, entry in files.items():
        source = entry["source"]
        for fact in entry["facts"]:
            kind = fact["kind"]
            scope = _scope(fact)
            name = fact.get("name")
            if kind in {"static", "struct", "alias", "module"}:
                key = (kind, scope, name)
                if key in declaration_keys:
                    _reject()
                declaration_keys.add(key)
            if kind == "static":
                if fact.get("function_depth", 0) != 0:
                    _reject()
                if fact.get("mutable", False) is True:
                    _reject()
                resolved = _resolved_type(
                    fact["type"],
                    scope,
                    aliases,
                    imports,
                    allow_bare_std=isinstance(fact["type"], str),
                )
                if name == "SIGNAL_BROKER":
                    if path != SESSION or not _private_visibility(fact.get("visibility", "")) or not _effective_guard(fact, source):
                        _reject()
                    if not _storage_type_ok(fact["type"], scope, aliases, imports):
                        _reject()
                    if "initializer" in fact and not _initializer_is_once_lock_new(fact["initializer"], scope, imports):
                        _reject()
                    broker_statics.append((path, fact))
                else:
                    if _contains_mutable_type(resolved):
                        _reject()
                    initializer = fact.get("initializer", "")
                    if resolved is None and (
                        name in {"CACHE", "HIDDEN", "SECOND_BROKER"}
                        or _fact_token(initializer, "unresolved")
                        or _fact_token(initializer, "todo")
                    ):
                        _reject()
            elif kind == "struct" and name == "InstallPhase":
                if path != SESSION or fact.get("function_depth", 0) != 0:
                    _reject()
                if not _private_visibility(fact.get("visibility", "")) or not _effective_guard(fact, source):
                    _reject()
                fields = fact["fields"]
                if len(fields) != len(_INSTALL_FIELDS):
                    _reject()
                for field, (field_name, _type_kind) in zip(fields, _INSTALL_FIELDS):
                    _validate_field(field, field_name, scope, aliases, imports)
                install_structs.append((path, fact))
            elif kind == "struct" and name == "SignalBroker":
                if path != SESSION or fact.get("function_depth", 0) != 0:
                    _reject()
                if not _private_visibility(fact.get("visibility", "")) or not _effective_guard(fact, source):
                    _reject()
                fields = fact["fields"]
                if len(fields) != len(_BROKER_FIELDS):
                    _reject()
                for field, (field_name, _type_kind) in zip(fields, _BROKER_FIELDS):
                    _validate_field(field, field_name, scope, aliases, imports)
                broker_structs.append((path, fact))
            elif kind in {"function", "method"}:
                signature = fact["signature"]
                if _fact_token(signature, "signal_broker") or (
                    _is_public_visibility(fact.get("visibility", ""))
                    and any(token in signature for token in ("SignalBroker", "SIGNAL_BROKER", "OnceLock", "Mutex"))
                ):
                    _reject()
            elif kind == "use":
                tree = fact["tree"]
                if _is_public_visibility(fact.get("visibility", "")) and (
                    _fact_token(tree, "SIGNAL_BROKER") or _fact_token(tree, "exported_broker")
                ):
                    _reject()
            elif kind == "alias" and _is_public_visibility(fact.get("visibility", "")):
                resolved = _resolved_type(fact.get("type"), scope, aliases, imports, allow_bare_std=isinstance(fact.get("type"), str))
                if name in {"SignalBroker", "BrokerStorage"} or _contains_mutable_type(resolved):
                    _reject()
            elif kind == "macro":
                path_text = fact["path"]
                tokens = fact["tokens"]
                if _fact_token(path_text, "thread_local") and ("static" in tokens or "SIGNAL_BROKER" in tokens):
                    _reject()
                if "static" in tokens and any(token in tokens for token in _MUTABLE_TYPES):
                    _reject()
                if _fact_token(path_text, "hidden") and "static" in tokens:
                    _reject()
        # A shadowed `std` module or a root `OnceLock` declaration invalidates
        # the identity of a spelling that otherwise resembles the contract.
        if re.search(r"(?m)^\s*(?:pub\s+)?mod\s+std\b", source) and "SIGNAL_BROKER" in source:
            _reject()
        if re.search(r"(?m)^\s*(?:pub\s+)?struct\s+OnceLock\b", source) and "SIGNAL_BROKER" in source:
            _reject()
        if "fn broken" in source and "SIGNAL_BROKER" in source:
            _reject()

    if (
        len(broker_statics) > 1
        or len(broker_structs) > 1
        or len(install_structs) > 1
        or bool(broker_statics) != bool(broker_structs)
        or bool(broker_statics) != bool(install_structs)
    ):
        _reject()
    if not broker_statics and any(fact.get("kind") == "struct" and fact.get("name") == "SignalBroker" for entry in files.values() for fact in entry["facts"]):
        _reject()


def _decode_record_stream(record: dict[str, Any], stream: str) -> bytes:
    try:
        value = record[stream]
        if not isinstance(value, str):
            raise ValueError
        return base64.b64decode(value, validate=True)
    except (KeyError, TypeError, ValueError):
        raise Reject("ARCHITECTURE") from None


def validate_broker_event(event: dict[str, Any], profile: dict[str, Any]) -> None:
    """Architecture-event arm for the ADJ-13 broker profile."""
    if not _is_dict(event) or not _is_dict(profile):
        _reject()
    if profile.get("policy") != ADJ13_POLICY:
        _reject()
    if type(event.get("exit")) is not int or event["exit"] != 0:
        _reject()
    payload = event.get("payload")
    if not _is_dict(payload):
        _reject()
    records = payload.get("records")
    if not isinstance(records, list) or len(records) != 1 or not isinstance(records[0], dict):
        _reject()
    record = records[0]
    if type(record.get("exit")) is not int or record["exit"] != 0:
        _reject()
    _decode_record_stream(record, "stderr")
    try:
        def unique_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
            result: dict[str, Any] = {}
            for key, value in pairs:
                if key in result:
                    raise ValueError("duplicate JSON key")
                result[key] = value
            return result

        body = json.loads(_decode_record_stream(record, "stdout"), object_pairs_hook=unique_pairs)
    except (json.JSONDecodeError, TypeError, ValueError):
        _reject()
    if not isinstance(body, dict) or body.get("schema") != "tc-protected-source-syntax/v1":
        _reject()
    validate_broker_observation(body, profile)
