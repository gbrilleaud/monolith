"""Combine a Windows client archive with a verified engine payload for local testing.

This does not authorize public redistribution: perform the source-code/notice
review before publishing an offline variant.
"""
import argparse
import os
from pathlib import Path
import shutil
import stat
import tempfile
import zipfile

REQUIRED_ENGINES = (
    "tools/retroarch/retroarch.exe",
    "tools/retroarch/cores/genesis_plus_gx_libretro.dll",
    "tools/pcsx2/pcsx2-qt.exe",
    "tools/xenia/xenia_canary.exe",
    "engine-lock.json",
)


def _members(archive, prefix, seen):
    entries = archive.infolist()
    if len(entries) > 50000 or sum(item.file_size for item in entries) > 3_000_000_000:
        raise ValueError("archive too large")
    for item in entries:
        raw = item.filename
        if item.is_dir():
            continue
        if not raw or raw.startswith("/") or "\\" in raw or ":" in raw:
            raise ValueError(f"unsafe ZIP member: {raw!r}")
        parts = raw.split("/")
        if any(part in ("", ".", "..") for part in parts):
            raise ValueError(f"unsafe ZIP member: {raw!r}")
        if stat.S_ISLNK(item.external_attr >> 16):
            raise ValueError(f"symlink ZIP member: {raw!r}")
        name = prefix + raw
        normalized = name.casefold()
        if normalized in seen:
            raise ValueError(f"duplicate ZIP member: {name!r}")
        seen.add(normalized)
        yield item, name


def assemble(client_zip, engine_zip, output, client_root):
    client_zip, engine_zip, output = map(Path, (client_zip, engine_zip, output))
    if not client_root or "/" in client_root or "\\" in client_root or client_root in (".", ".."):
        raise ValueError("invalid client root")
    with zipfile.ZipFile(client_zip) as client, zipfile.ZipFile(engine_zip) as engines:
        seen = set()
        client_entries = list(_members(client, "", seen))
        engine_entries = list(_members(engines, client_root + "/", seen))
        client_names = {name for _, name in client_entries}
        engine_names = {item.filename for item, _ in engine_entries}
        if f"{client_root}/monolith.exe" not in client_names:
            raise ValueError("client executable missing")
        missing = set(REQUIRED_ENGINES) - engine_names
        if missing:
            raise ValueError(f"engine payload incomplete: {sorted(missing)}")
        if any(not name.startswith(client_root + "/") for name in client_names):
            raise ValueError("client archive has unexpected root")
        output.parent.mkdir(parents=True, exist_ok=True)
        fd, pending = tempfile.mkstemp(prefix=".offline-windows-", suffix=".zip", dir=output.parent)
        os.close(fd)
        try:
            with zipfile.ZipFile(pending, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6,
                                 allowZip64=True) as result:
                for source, entries in ((client, client_entries), (engines, engine_entries)):
                    for item, name in entries:
                        with source.open(item) as input_file, result.open(name, "w", force_zip64=True) as output_file:
                            shutil.copyfileobj(input_file, output_file, 1024 * 1024)
            os.replace(pending, output)
        finally:
            if os.path.exists(pending):
                os.unlink(pending)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", type=Path, required=True)
    parser.add_argument("--engines", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    print(assemble(args.client, args.engines, args.output, args.root))


if __name__ == "__main__":
    main()
