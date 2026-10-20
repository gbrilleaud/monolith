"""Build an offline Windows engine payload from checksum-pinned vendor 7z files.

The output is an engine-only ZIP, not a public Monolith Release. Redistributing
it requires the corresponding-source and third-party-notice review described
in docs/ENGINE_LOCK_RESEARCH.md. No ROM, firmware, key or BIOS is included.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

DESTINATIONS = {"retroarch": "retroarch", "cores": "retroarch", "pcsx2": "pcsx2", "xenia": "xenia"}


def _safe_path(value):
    if not value or "\\" in value or ":" in value or value.startswith("/"):
        raise ValueError(f"unsafe archive path: {value!r}")
    parts = value.rstrip("/").split("/")
    if any(part in ("", ".", "..") for part in parts):
        raise ValueError(f"unsafe archive path: {value!r}")
    return Path(*parts)


def _digest(path):
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(block)
    return sha.hexdigest()


def build(lock_path, archive_dir, output, sevenzip):
    lock_path, archive_dir, output = map(Path, (lock_path, archive_dir, output))
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    components = lock["components"]
    if {item["id"] for item in components} != set(DESTINATIONS) or len(components) != len(DESTINATIONS):
        raise ValueError("engine lock must contain exactly four components")
    # Verify every input before extracting or touching an existing output.
    for item in components:
        if item["platform"] != "windows-x86_64" or not item["source_url"].startswith("https://"):
            raise ValueError("unsupported platform or source")
        archive = archive_dir / _safe_path(item["archive"])
        if archive.parent != archive_dir:
            raise ValueError("archives must be direct children of archive directory")
        expected = item["sha256"].lower()
        if len(expected) != 64 or _digest(archive) != expected:
            raise ValueError(f"SHA-256 mismatch: {item['id']}")
    if not Path(sevenzip).is_file():
        raise ValueError("7zz executable not found")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="monolith-engines-") as scratch:
        scratch = Path(scratch)
        staged = scratch / "stage"
        staged.mkdir()
        for item in components:
            component = item["id"]
            extract = scratch / ("extract-" + component)
            extract.mkdir()
            archive = archive_dir / item["archive"]
            command = [str(sevenzip), "x", "-y", f"-o{extract}", str(archive)]
            if item.get("select"):
                command += item["select"]
            subprocess.run(command, check=True, capture_output=True, text=True)
            prefix = _safe_path(item["source_prefix"]) if item["source_prefix"] else Path()
            source = extract / prefix
            if not source.is_dir():
                raise ValueError(f"missing source root: {component}")
            for required in item["expected"]:
                selected = item.get("select")
                if selected and item["source_prefix"] + required not in selected:
                    continue
                required_path = extract / _safe_path(item["source_prefix"] + required)
                if not required_path.is_file():
                    raise ValueError(f"missing expected file: {component}/{required}")
            destination = staged / "tools" / DESTINATIONS[component]
            destination.mkdir(parents=True, exist_ok=True)
            for path in source.rglob("*"):
                if path.is_symlink():
                    raise ValueError(f"symlink in engine: {path}")
                if not path.is_file():
                    continue
                relative = path.relative_to(source)
                target = destination / relative
                if target.exists():
                    raise ValueError(f"duplicate engine file: {target}")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(path.read_bytes())
        (staged / "engine-lock.json").write_text(json.dumps(lock, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        notices = Path(__file__).resolve().parents[1] / "config" / "licenses"
        for name in ("retroarch-GPL-3.0.txt", "genesis-plus-gx-LICENSE.txt"):
            target = staged / "licenses" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((notices / name).read_bytes())
        fd, pending = tempfile.mkstemp(prefix=".engine-bundle-", suffix=".zip", dir=output.parent)
        os.close(fd)
        try:
            with zipfile.ZipFile(pending, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
                for path in sorted(staged.rglob("*")):
                    if path.is_file():
                        bundle.write(path, path.relative_to(staged).as_posix())
            os.replace(pending, output)
        finally:
            if os.path.exists(pending):
                os.unlink(pending)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, required=True)
    parser.add_argument("--archive-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sevenzip", required=True)
    args = parser.parse_args()
    print(build(args.lock, args.archive_dir, args.output, args.sevenzip))


if __name__ == "__main__":
    main()
