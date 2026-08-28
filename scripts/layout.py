#!/usr/bin/env python3
"""Check every source file against the naming convention in AGENTS.md.

The convention has one shape: a file and its folder, read together, spell the
type. Either the folder names the kind (`services/input.rs` -> `InputService`)
or the file does (`registry.rs` -> `LaunchRegistry`); when the kind *is* the
subject the two collapse (`capture.rs` -> `Capture`). Written as a predicate:
snake_case(the type) ends with the file's stem.

Run with `--list` to print the classification of every file instead of only
the failures.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()
)

ROLES = {
    "module", "service", "config", "error", "constants", "entity",
    "guard", "strategy", "pipe", "interceptor", "filter", "exception_filter",
    "testing",
}
EDGE_ROLE = {
    "http": "controller", "graphql": "resolver", "ws": "gateway",
    "queue": "processor", "schedule": "tasks", "mcp": "tool", "events": "listener",
}
PROVIDER_FOLDER = {"services": "service", "strategies": "strategy", "pipes": "pipe", "entities": ""}
TRANSFER_FOLDER = {"dtos": "dto", "commands": "command", "events": "event"}

ITEM = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|static\s+|async\s+|unsafe\s+)*"
    r"(struct|enum|trait|type|fn|const|static)\s+([A-Za-z_][A-Za-z_0-9]*)"
)


def snake(name: str) -> str:
    if name.isupper():
        return name.lower()
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def items(path: Path) -> list[tuple[str, str]]:
    """The (kind, name) of every item declared at the top level of a file."""
    return [m.groups() for line in path.read_text().splitlines() if (m := ITEM.match(line))]


def classify(rel: Path) -> tuple[str, str]:
    """Returns (verdict, detail). An empty verdict means the file is fine."""
    parts = rel.parts
    stem, parent = rel.stem, rel.parent.name
    path = ROOT / rel

    if "tests" in parts or stem in ("mod", "lib", "main"):
        return "", "index / entrypoint / test"
    # A backend implements a contract the neutral layer already named; its
    # internals are the OS's vocabulary, not ours.
    if "linux" in parts or "macos" in parts or stem == "unsupported":
        return "", "backend OS"
    if stem in ROLES or EDGE_ROLE.get(parent) == stem:
        return "", f"role: {stem}"

    kinds = items(path)
    if not kinds:
        return "empty", "declares nothing"
    declared = [name for _, name in kinds]
    snakes = [snake(name) for name in declared]

    if parent in PROVIDER_FOLDER:
        want = f"{stem}_{PROVIDER_FOLDER[parent]}".rstrip("_")
        if want in snakes:
            return "", f"{parent}/ -> {declared[snakes.index(want)]}"
        return "provider misnamed", f"{parent}/{stem}.rs wants a `{want}`, declares {declared}"

    if parent in TRANSFER_FOLDER:
        suffix = TRANSFER_FOLDER[parent]
        siblings = [p for p in path.parent.glob("*.rs") if p.name != "mod.rs"]
        if len(siblings) == 1:
            return "plural folder holding one", f"expected {rel.parent.parent.name}/{suffix}.rs"
        if not stem.endswith(f"_{suffix}"):
            return "suffix missing", f"expected *_{suffix}.rs"
        if stem not in snakes:
            return "stem is not the type", f"{stem}.rs declares {declared}"
        return "", f"{parent}/ -> {declared[snakes.index(stem)]}"

    # A file that declares no type is a namespace: its items are reached
    # through it (`frame::encode_webp`, `host::input`), so the stem names the
    # subject they operate on and there is no pairing to check.
    if not any(kind in ("struct", "enum", "trait", "type") for kind, _ in kinds):
        return "", f"{stem}.rs -> namespace ({len(declared)} items)"

    # The pairing: the stem is one of the two words the type is made of, and
    # which one depends on whether the module already supplies the other.
    # `registry.rs` -> LaunchRegistry (the file names the kind);
    # `screen.rs` -> ScreenBackend (the file names the subject).
    for name, sn in zip(declared, snakes):
        if sn == stem or sn.endswith(f"_{stem}") or sn.startswith(f"{stem}_"):
            return "", f"{stem}.rs -> {name}"
    return "stem not in type", (
        f"{stem}.rs declares {declared} — the stem is neither the first nor the last word"
    )


def root_files_are_declared() -> list[str]:
    """A bare `.rs` at a crate's src root owes a reason in `lib.rs`'s `//!`."""
    failures = []
    for lib in ROOT.glob("crates/*/src/lib.rs"):
        header = "\n".join(l for l in lib.read_text().splitlines() if l.startswith("//!"))
        for sibling in sorted(lib.parent.glob("*.rs")):
            named = re.search(rf"\b{re.escape(sibling.stem)}\b", header)
            if sibling.name != "lib.rs" and not named:
                failures.append(
                    f"{sibling.relative_to(ROOT)}: at the crate root, unmentioned in the `//!` of "
                    f"{lib.relative_to(ROOT)} — a closed list owes a reason"
                )
    return failures


def main() -> int:
    listing = "--list" in sys.argv
    files = sorted(
        p for d in ("apps", "crates") for p in (ROOT / d).rglob("*.rs") if "target" not in p.parts
    )
    failures = []
    for path in files:
        rel = path.relative_to(ROOT)
        verdict, detail = classify(rel)
        if verdict:
            failures.append(f"{rel}: {verdict} — {detail}")
        if listing:
            print(f"{'OK ' if not verdict else 'KO '} {rel}  ({detail})")
    failures += root_files_are_declared()

    if failures:
        print("\nlayout: the naming convention is not held\n", file=sys.stderr)
        for f in failures:
            print(f"  {f}", file=sys.stderr)
        return 1
    print(f"layout: {len(files)} files, convention held")
    return 0


if __name__ == "__main__":
    sys.exit(main())
