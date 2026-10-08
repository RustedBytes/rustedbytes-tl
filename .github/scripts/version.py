"""Validate stable release versions and update the root package without dependencies."""
import argparse
import os
from pathlib import Path
import re
import tomllib

SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")


def components(value):
    match = SEMVER.fullmatch(value)
    if not match:
        raise ValueError("Expected MAJOR.MINOR.PATCH without v, prerelease or build metadata")
    return tuple(int(part) for part in match.groups())


def prepare(text, bump, explicit):
    old = tomllib.loads(text)["package"]["version"]
    major, minor, patch = components(old)
    if explicit:
        new = explicit
    elif bump == "major":
        new = f"{major + 1}.0.0"
    elif bump == "minor":
        new = f"{major}.{minor + 1}.0"
    elif bump == "patch":
        new = f"{major}.{minor}.{patch + 1}"
    else:
        raise ValueError("Unknown version component")
    if components(new) <= components(old):
        raise ValueError("New version must be greater than the current version")
    # Change only the root [package] version; preserve formatting and dependency versions.
    section = re.search(r"(?ms)^\[package\][ \t]*\n.*?(?=^\[|\Z)", text)
    if section is None:
        raise ValueError("Missing [package] section")
    updated, count = re.subn(
        r'(?m)^(version\s*=\s*)"[^"]+"',
        lambda match: f'{match[1]}"{new}"', section[0],
    )
    if count != 1:
        raise ValueError("Expected one quoted package version")
    return text[:section.start()] + updated + text[section.end():], new


def validate_tag(text, tag):
    version = tomllib.loads(text)["package"]["version"]
    components(version)
    if tag != f"v{version}":
        raise ValueError(f"Release tag {tag!r} must equal v{version}")
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bump", choices=("patch", "minor", "major"), default="patch")
    parser.add_argument("--version", default="")
    parser.add_argument("--check-tag")
    args = parser.parse_args()
    manifest = Path("Cargo.toml")
    text = manifest.read_text()
    try:
        if args.check_tag is not None:
            version = validate_tag(text, args.check_tag)
        else:
            updated, version = prepare(text, args.bump, args.version)
            manifest.write_text(updated)
    except (ValueError, KeyError) as error:
        parser.error(str(error))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as stream:
            stream.write(f"version={version}\n")
    print(version)


if __name__ == "__main__":
    main()
