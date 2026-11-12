#!/usr/bin/env python3
"""Validate a Cargo-generated lockfile update preserves existing package locks.

This is a CI-only audit, not a dependency resolver. Cargo generates both lockfiles;
this script checks that adding symthaea-core 0.5.1 did not move unrelated packages.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys
import tomllib

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
ALLOWED_UPGRADES = {
    # symthaea-core 0.5.1 declares libc = "0.2.186"; the old 0.2.184 pin
    # is below that minimum. Do not float it to a later compatible release.
    ("libc", REGISTRY, "0.2.184"): "0.2.186",
}
REQUIRED = {
    ("symthaea-core", REGISTRY, "0.5.1"),
    ("rayon", REGISTRY, "1.12.0"),
}


def package_set(path: pathlib.Path) -> set[tuple[str, str | None, str]]:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    packages = data.get("package")
    if not isinstance(packages, list):
        raise ValueError(f"{path}: no [[package]] array")
    result: set[tuple[str, str | None, str]] = set()
    for package in packages:
        name = package.get("name")
        version = package.get("version")
        source = package.get("source")
        if not isinstance(name, str) or not isinstance(version, str):
            raise ValueError(f"{path}: malformed package entry: {package!r}")
        result.add((name, source, version))
    return result


def audit(base: set[tuple[str, str | None, str]],
          candidate: set[tuple[str, str | None, str]]) -> dict[str, object]:
    missing = []
    allowed = []
    for package in sorted(base):
        if package in candidate:
            continue
        name, source, old_version = package
        allowed_version = ALLOWED_UPGRADES.get((name, source, old_version))
        replacement = (name, source, allowed_version) if allowed_version else None
        if replacement and replacement in candidate:
            allowed.append({
                "name": name,
                "source": source,
                "from": old_version,
                "to": allowed_version,
                "reason": "symthaea-core 0.5.1 minimum requirement",
            })
        else:
            missing.append({
                "name": name,
                "source": source,
                "version": old_version,
            })

    additions = sorted(candidate - base)
    missing_required = sorted(REQUIRED - candidate)
    return {
        "schema": "symtropy.cargo-lock-preservation.v1",
        "base_package_count": len(base),
        "candidate_package_count": len(candidate),
        "added_package_count": len(additions),
        "approved_existing_package_updates": allowed,
        "unapproved_removed_or_changed_locks": missing,
        "required_packages_missing": [
            {"name": name, "source": source, "version": version}
            for name, source, version in missing_required
        ],
        "added_packages": [
            {"name": name, "source": source, "version": version}
            for name, source, version in additions
        ],
        "disposition": "PASS" if not missing and not missing_required else "FAIL",
    }


def self_test() -> None:
    base = {("serde", REGISTRY, "1.0.228"),
            ("libc", REGISTRY, "0.2.184")}
    good = base - {("libc", REGISTRY, "0.2.184")}
    good |= {
        ("libc", REGISTRY, "0.2.186"),
        ("symthaea-core", REGISTRY, "0.5.1"),
        ("rayon", REGISTRY, "1.12.0"),
        ("new-transitive", REGISTRY, "0.1.0"),
    }
    result = audit(base, good)
    assert result["disposition"] == "PASS", result
    assert result["approved_existing_package_updates"]

    bad = good - {("serde", REGISTRY, "1.0.228")}
    bad.add(("serde", REGISTRY, "1.0.229"))
    result = audit(base, bad)
    assert result["disposition"] == "FAIL", result
    assert result["unapproved_removed_or_changed_locks"]

    print("cargo_lock_preservation_self_test=PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", type=pathlib.Path)
    parser.add_argument("--candidate", type=pathlib.Path)
    parser.add_argument("--report", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return 0
    if not args.base or not args.candidate:
        parser.error("--base and --candidate are required unless --self-test is used")

    try:
        result = audit(package_set(args.base), package_set(args.candidate))
    except (OSError, ValueError, tomllib.TOMLDecodeError) as exc:
        print(f"cargo_lock_preservation_error={exc}", file=sys.stderr)
        return 2

    serialized = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.report:
        args.report.write_text(serialized, encoding="utf-8")
    print(serialized, end="")
    return 0 if result["disposition"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
