#!/usr/bin/env python3
"""
Prepare and publish a coordinated app + documentation release.

Examples:
    python release.py patch
    python release.py minor
    python release.py major --data-also
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

APP_ROOT = Path(__file__).resolve().parent

# Change this to the location of your documentation repository.
DOCS_ROOT = APP_ROOT.parent / "duplicate-finder-docs"

TAURI_CONFIG = APP_ROOT / "src-tauri" / "tauri.conf.json"
PACKAGE_JSON = APP_ROOT / "package.json"
PACKAGE_LOCK_JSON = APP_ROOT / "package-lock.json"
CARGO_TOML = APP_ROOT / "src-tauri" / "Cargo.toml"

DOCS_RELEASE_JSON = DOCS_ROOT / "src" / "config" / "release.json"

DOCS_CONTENT_ROOT = DOCS_ROOT / "src" / "content" / "docs"
DOCS_RELEASE_NOTES_DIR = "releases"
DOCS_LANGUAGES = ("en", "fr", "pt", "mg")

# Only used with --data-also. Change if your file lives elsewhere.
DOCS_DATA_JSON = DOCS_ROOT / "src" / "config" / "data.json"

APP_BRANCH = "main"
DOCS_BRANCH = "main"
TAG_PREFIX = "app-v"

PREP_RELEASE_DB_SCRIPT = APP_ROOT / "prep_release_db.py"


# =============================================================================
# COMMAND HELPERS
# =============================================================================


def run(
    command: list[str],
    cwd: Path,
    *,
    capture_output: bool = False,
) -> str:
    """Run a command and stop immediately if it fails."""
    print(f"\n> ({cwd.name}) {' '.join(command)}")

    result = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        text=True,
        capture_output=capture_output,
    )

    return result.stdout.strip() if capture_output else ""


def git(repo: Path, *args: str, capture_output: bool = False) -> str:
    """Run a Git command in a repository."""
    return run(["git", *args], repo, capture_output=capture_output)


def require_file(path: Path) -> None:
    if not path.is_file():
        raise RuntimeError(f"Required file not found:\n  {path}")


def read_json(path: Path) -> dict:
    require_file(path)

    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise RuntimeError(f"Invalid JSON in {path}:\n{error}") from error


def write_json(path: Path, data: dict) -> None:
    path.write_text(
        json.dumps(data, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


# =============================================================================
# VERSION HELPERS
# =============================================================================


SEMVER_PATTERN = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


def parse_version(version: str) -> tuple[int, int, int]:
    match = SEMVER_PATTERN.fullmatch(version)

    if not match:
        raise RuntimeError(
            f"Version must use simple semantic versioning (for example 0.6.1), "
            f"but got: {version!r}"
        )

    return tuple(int(part) for part in match.groups())


def bump_version(version: str, bump_type: str) -> str:
    major, minor, patch = parse_version(version)

    if bump_type == "major":
        return f"{major + 1}.0.0"

    if bump_type == "minor":
        return f"{major}.{minor + 1}.0"

    if bump_type == "patch":
        return f"{major}.{minor}.{patch + 1}"

    raise RuntimeError(f"Unknown bump type: {bump_type}")

def get_minor_release_version(version: str) -> str:
    """Return the major.minor part of a semantic version."""
    major, minor, _patch = parse_version(version)
    return f"{major}.{minor}"

def get_release_sidebar_order(version: str) -> int:
    """
    Return a sidebar order that sorts newer semantic versions first.

    Examples:
        0.9.0  -> -9000
        0.9.1  -> -9001
        0.10.0 -> -10000
        1.0.0  -> -1000000
    """
    major, minor, patch = parse_version(version)
    return -(major * 1_000_000 + minor * 1_000 + patch)

# =============================================================================
# VALIDATION
# =============================================================================


def ensure_repo_clean(repo: Path) -> None:
    status = git(repo, "status", "--porcelain", capture_output=True)

    if status:
        raise RuntimeError(
            f"Repository has uncommitted changes:\n  {repo}\n\n"
            "Commit, stash, or discard them before creating a release."
        )


def ensure_on_branch(repo: Path, expected_branch: str) -> None:
    branch = git(repo, "branch", "--show-current", capture_output=True)

    if branch != expected_branch:
        raise RuntimeError(
            f"Repository is not on {expected_branch!r}:\n"
            f"  {repo}\n"
            f"Current branch: {branch!r}"
        )


def ensure_up_to_date(repo: Path, branch: str) -> None:
    git(repo, "fetch", "origin", branch)

    counts = git(
        repo,
        "rev-list",
        "--left-right",
        "--count",
        f"{branch}...origin/{branch}",
        capture_output=True,
    )

    ahead, behind = map(int, counts.split())

    if ahead != 0 or behind != 0:
        raise RuntimeError(
            f"Repository is not fully synchronized with origin/{branch}:\n"
            f"  {repo}\n"
            f"Local commits not pushed: {ahead}\n"
            f"Remote commits not pulled: {behind}\n\n"
            "Synchronize the repository before creating a release."
        )


def get_release_notes_paths(filename: str) -> dict[str, Path]:
    """Return the release-notes path for every documentation language."""
    paths = {}

    for language in DOCS_LANGUAGES:
        language_root = (
            DOCS_CONTENT_ROOT / language
        )
        paths[language] = language_root / DOCS_RELEASE_NOTES_DIR / filename

    return paths


def validate_release_notes(new_version: str) -> None:
    """Ensure every next.md exists and no versioned destination exists yet."""
    source_paths = get_release_notes_paths("next.md")
    destination_paths = get_release_notes_paths(f"v{new_version}.md")

    missing = [path for path in source_paths.values() if not path.is_file()]

    if missing:
        formatted = "\n".join(f"  {path}" for path in missing)
        raise RuntimeError(
            "Release notes are missing for one or more languages:\n"
            f"{formatted}"
        )

    existing = [path for path in destination_paths.values() if path.exists()]

    if existing:
        formatted = "\n".join(f"  {path}" for path in existing)
        raise RuntimeError(
            "Versioned release-note files already exist:\n"
            f"{formatted}"
        )


def tag_exists_locally_or_remotely(repo: Path, tag: str) -> bool:
    local_tag = git(repo, "tag", "--list", tag, capture_output=True)

    if local_tag.strip():
        return True

    remote_tag = git(
        repo,
        "ls-remote",
        "--tags",
        "origin",
        f"refs/tags/{tag}",
        capture_output=True,
    )

    return bool(remote_tag.strip())


# =============================================================================
# FILE UPDATES
# =============================================================================


def update_tauri_config(version: str) -> None:
    data = read_json(TAURI_CONFIG)

    if "version" not in data:
        raise RuntimeError(f"No top-level 'version' field found in {TAURI_CONFIG}")

    data["version"] = version
    write_json(TAURI_CONFIG, data)


def update_package_json(version: str) -> None:
    data = read_json(PACKAGE_JSON)

    if "version" not in data:
        raise RuntimeError(f"No top-level 'version' field found in {PACKAGE_JSON}")

    data["version"] = version
    write_json(PACKAGE_JSON, data)


def update_package_lock(version: str) -> None:
    """
    Keep package-lock.json aligned when it exists.

    npm commonly stores the package version both at the root and under
    packages[""].version.
    """
    if not PACKAGE_LOCK_JSON.exists():
        return

    data = read_json(PACKAGE_LOCK_JSON)

    if "version" in data:
        data["version"] = version

    packages = data.get("packages")

    if isinstance(packages, dict) and isinstance(packages.get(""), dict):
        if "version" in packages[""]:
            packages[""]["version"] = version

    write_json(PACKAGE_LOCK_JSON, data)


def update_cargo_toml(version: str) -> None:
    """
    Update the version inside Cargo.toml's [package] section only.

    This avoids changing a dependency version elsewhere in the file.
    """
    require_file(CARGO_TOML)

    text = CARGO_TOML.read_text(encoding="utf-8")

    package_section = re.compile(
        r"(\[package\]\s.*?)(?=^\[|\Z)",
        flags=re.DOTALL | re.MULTILINE,
    )

    match = package_section.search(text)

    if not match:
        raise RuntimeError(f"Could not find a [package] section in {CARGO_TOML}")

    section = match.group(1)

    updated_section, replacements = re.subn(
        r'(?m)^version\s*=\s*"[^"]*"\s*$',
        f'version = "{version}"',
        section,
        count=1,
    )

    if replacements != 1:
        raise RuntimeError(
            f"Could not find exactly one package version line in {CARGO_TOML}"
        )

    updated_text = text[: match.start(1)] + updated_section + text[match.end(1) :]
    CARGO_TOML.write_text(updated_text, encoding="utf-8")


def update_release_frontmatter(
    text: str,
    title: str,
    sidebar_order: int,
    path: Path,
) -> str:
    """Update the title and make a release page visible in the sidebar."""
    match = re.match(r"\A---\r?\n(.*?)\r?\n---(?=\r?\n|\Z)", text, re.DOTALL)

    if not match:
        raise RuntimeError(f"Missing YAML frontmatter in {path}")

    frontmatter = match.group(1)

    updated_frontmatter, replacements = re.subn(
        r"(?m)^title\s*:\s*.*$",
        f"title: {title}",
        frontmatter,
        count=1,
    )

    if replacements != 1:
        raise RuntimeError(f"Missing title in YAML frontmatter in {path}")

    updated_frontmatter = re.sub(
        r"(?ms)^sidebar:\s*\n(?:^[ \t]+.*\n?)*",
        "",
        updated_frontmatter,
    ).rstrip()

    updated_frontmatter += (
        "\nsidebar:\n"
        f"  order: {sidebar_order}"
    )

    return (
        text[: match.start(1)]
        + updated_frontmatter
        + text[match.end(1) :]
    )


def publish_release_notes(
    release_notes_version: str,
    full_version: str,
) -> None:
    """Version every next.md file and recreate an empty next.md template."""
    source_paths = get_release_notes_paths("next.md")
    destination_paths = get_release_notes_paths(
        f"v{release_notes_version}.md"
    )

    sidebar_order = get_release_sidebar_order(full_version)

    # Read and validate every file before changing any of them.
    updated_contents = {}

    for language, source_path in source_paths.items():
        text = source_path.read_text(encoding="utf-8")
        updated_contents[language] = update_release_frontmatter(
            text,
            f"v{release_notes_version}",
            sidebar_order,
            source_path,
        )

    # we have to keep the indentation like this so that it works in the .md files
    new_next_content = """---
    title: next
    sidebar:
        hidden: true
---
"""

    for language in DOCS_LANGUAGES:
        source_path = source_paths[language]
        destination_path = destination_paths[language]

        destination_path.parent.mkdir(parents=True, exist_ok=True)

        # Write the completed release page.
        destination_path.write_text(
            updated_contents[language],
            encoding="utf-8",
        )

        # Replace next.md with a fresh template for every language.
        source_path.write_text(
            new_next_content,
            encoding="utf-8",
        )


def update_docs_release_json(version: str) -> None:
    data = read_json(DOCS_RELEASE_JSON)

    if "version" not in data:
        raise RuntimeError(
            f"No top-level 'version' field found in {DOCS_RELEASE_JSON}"
        )

    data["version"] = version

    # Keep this aligned if your release.json already contains a tag field.
    if "tag" in data:
        data["tag"] = f"{TAG_PREFIX}{version}"

    write_json(DOCS_RELEASE_JSON, data)


def update_docs_data_json(version: str) -> None:
    data = read_json(DOCS_DATA_JSON)

    if "release" not in data:
        raise RuntimeError(
            f"No top-level 'release' field found in {DOCS_DATA_JSON}"
        )

    data["release"] = version
    write_json(DOCS_DATA_JSON, data)


def get_app_version() -> str:
    data = read_json(TAURI_CONFIG)

    version = data.get("version")

    if not isinstance(version, str):
        raise RuntimeError(f"Invalid or missing version in {TAURI_CONFIG}")

    parse_version(version)
    return version


def get_docs_version() -> str:
    data = read_json(DOCS_RELEASE_JSON)

    version = data.get("version")

    if not isinstance(version, str):
        raise RuntimeError(f"Invalid or missing version in {DOCS_RELEASE_JSON}")

    parse_version(version)
    return version


# =============================================================================
# RELEASE STEPS
# =============================================================================


def prepare_database() -> None:
    require_file(PREP_RELEASE_DB_SCRIPT)

    print("\nPreparing clean release database...")
    run([sys.executable, str(PREP_RELEASE_DB_SCRIPT)], APP_ROOT)


def commit_and_tag_app(version: str, tag: str) -> None:
    commit_message = f"release v{version}"

    git(APP_ROOT, "add", "-A")
    git(APP_ROOT, "commit", "-m", commit_message)
    git(APP_ROOT, "tag", "-a", tag, "-m", f"Release {tag}")


def commit_and_tag_docs(version: str, tag: str) -> None:
    commit_message = f"release v{version}"

    git(DOCS_ROOT, "add", "-A")
    git(DOCS_ROOT, "commit", "-m", commit_message)
    git(DOCS_ROOT, "tag", "-a", tag, "-m", f"Release {tag}")


def push_release(tag: str) -> None:
    """
    Push docs first, then app branch, then app tag last.

    The final app-tag push triggers the Tauri workflow. By that time the docs
    repository already contains the matching tag for the deployment workflow.
    """
    print("\nPushing documentation repository first...")
    git(DOCS_ROOT, "push", "origin", DOCS_BRANCH)
    git(DOCS_ROOT, "push", "origin", tag)

    print("\nPushing application release commit...")
    git(APP_ROOT, "push", "origin", APP_BRANCH)

    print("\nPushing application tag last — this triggers the release workflow...")
    git(APP_ROOT, "push", "origin", tag)


# =============================================================================
# MAIN
# =============================================================================


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Prepare and publish an app + documentation release."
    )

    parser.add_argument(
        "bump",
        choices=["major", "minor", "patch"],
        help="Semantic-version component to bump.",
    )

    parser.add_argument(
        "--data-also",
        action="store_true",
        help="Also update the 'release' field in the docs data.json file.",
    )

    args = parser.parse_args()

    print("Validating repository state...")

    require_file(TAURI_CONFIG)
    require_file(PACKAGE_JSON)
    require_file(CARGO_TOML)
    require_file(DOCS_RELEASE_JSON)

    if args.data_also:
        require_file(DOCS_DATA_JSON)

    ensure_repo_clean(APP_ROOT)
    ensure_repo_clean(DOCS_ROOT)

    ensure_on_branch(APP_ROOT, APP_BRANCH)
    ensure_on_branch(DOCS_ROOT, DOCS_BRANCH)

    ensure_up_to_date(APP_ROOT, APP_BRANCH)
    ensure_up_to_date(DOCS_ROOT, DOCS_BRANCH)

    app_version = get_app_version()
    docs_version = get_docs_version()

    if app_version != docs_version:
        raise RuntimeError(
            "App and docs versions do not match before release:\n"
            f"  App:  {app_version}\n"
            f"  Docs: {docs_version}\n\n"
            "Resolve this manually before running the release script."
        )

    new_version = bump_version(app_version, args.bump)
    tag = f"{TAG_PREFIX}{new_version}"

    if tag_exists_locally_or_remotely(APP_ROOT, tag):
        raise RuntimeError(f"Tag already exists in app repository: {tag}")

    if tag_exists_locally_or_remotely(DOCS_ROOT, tag):
        raise RuntimeError(f"Tag already exists in docs repository: {tag}")

    docs_release_version = get_minor_release_version(new_version)

    if args.bump == "minor":
        validate_release_notes(docs_release_version)

    print("\nRelease summary")
    print("=" * 60)
    print(f"Current version: {app_version}")
    print(f"New version:     {new_version}")
    print(f"Tag:             {tag}")
    print(f"Data update:     {'yes' if args.data_also else 'no'}")
    print("=" * 60)

    confirmation = input(
        f"\nAre you sure you want to bump the version from {app_version} "
        f"to {new_version} and deploy the application and updated documentation? "
        "[y/N]: "
    ).strip().lower()

    if confirmation not in {"y", "yes"}:
        print("\nRelease cancelled. No files were changed.")
        return

    # Application updates.
    prepare_database()
    update_tauri_config(new_version)
    update_package_json(new_version)
    update_package_lock(new_version)
    update_cargo_toml(new_version)

    # Documentation updates.
    if args.bump == "minor":
        publish_release_notes(
            docs_release_version,
            new_version,
        )
    
    update_docs_release_json(new_version)

    if args.data_also:
        update_docs_data_json(new_version)

    # Commit and tag both repositories locally before pushing either one.
    print("\nCommitting and tagging documentation repository...")
    commit_and_tag_docs(new_version, tag)

    print("\nCommitting and tagging application repository...")
    commit_and_tag_app(new_version, tag)

    # Push docs tag before app tag, because app-tag push starts deployment.
    push_release(tag)

    print("\nRelease preparation complete.")
    print(f"Application and documentation version: {new_version}")
    print(f"Release tag: {tag}")

    if args.data_also:
        print(
            "\nReminder: upload the updated data files to the GitHub Release "
            "after the release workflow has finished."
        )


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        print(
            f"\nERROR: Command failed with exit code {error.returncode}.",
            file=sys.stderr,
        )
        sys.exit(error.returncode)
    except RuntimeError as error:
        print(f"\nERROR: {error}", file=sys.stderr)
        sys.exit(1)