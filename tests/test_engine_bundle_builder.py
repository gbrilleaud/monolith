"""Offline engine-bundle builder tests; no proprietary media or BIOS."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("builder", ROOT / "scripts/build_engine_bundle.py")
SEVEN_ZIP = Path(os.environ.get("MONOLITH_SEVENZIP", shutil.which("7z") or "/tmp/monolith-7zip/7zz"))


class BundleBuilderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.archives = self.root / "archives"
        self.archives.mkdir()
        self.sevenzip = str(SEVEN_ZIP)
        self.assertTrue(SEVEN_ZIP.is_file(), "local 7zz required for integration fixture")

    def archive(self, name, files):
        src = self.root / f"src-{name}"
        for path, contents in files.items():
            dest = src / path
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(contents)
        archive = self.archives / f"{name}.7z"
        subprocess.run([self.sevenzip, "a", str(archive), str(src / "*")],
                       check=True, capture_output=True)
        return archive, hashlib.sha256(archive.read_bytes()).hexdigest()

    def test_rejects_checksum_mismatch_before_writing_bundle(self):
        archive, _ = self.archive("xenia", {"xenia.exe": b"fixture"})
        lock = {"components": [{"id": name, "archive": archive.name,
                                 "sha256": "0" * 64, "source_url": "https://example.test/xenia.7z",
                                 "version": "test", "platform": "windows-x86_64",
                                 "license": "BSD-3-Clause", "expected": ["xenia.exe"],
                                 "source_prefix": ""}
                                for name in ("retroarch", "cores", "pcsx2", "xenia")]}
        lock_path = self.root / "lock.json"
        lock_path.write_text(json.dumps(lock))
        output = self.root / "bundle.zip"
        spec = importlib.util.spec_from_file_location("builder", ROOT / "scripts/build_engine_bundle.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            module.build(lock_path, self.archives, output, self.sevenzip)
        self.assertFalse(output.exists())

    def test_builds_portable_layout_with_only_selected_core(self):
        assets = [
            ("retroarch", {"RetroArch-Win64/retroarch.exe": b"exe",
                           "RetroArch-Win64/assets/menu.txt": b"asset"}, "RetroArch-Win64/"),
            ("cores", {"RetroArch-Win64/cores/genesis_plus_gx_libretro.dll": b"core",
                       "RetroArch-Win64/cores/other_libretro.dll": b"excluded"},
             "RetroArch-Win64/"),
            ("pcsx2", {"pcsx2-qt.exe": b"ps2", "docs/GPL.html": b"license"}, ""),
            ("xenia", {"xenia_canary.exe": b"360", "LICENSE": b"license"}, ""),
        ]
        components = []
        for name, files, prefix in assets:
            archive, digest = self.archive(name, files)
            components.append({"id": name, "archive": archive.name, "sha256": digest,
                               "source_url": f"https://example.test/{name}.7z",
                               "version": "test", "platform": "windows-x86_64",
                               "license": "test", "expected": [p.removeprefix(prefix) for p in files],
                               "source_prefix": prefix,
                               "select": ["RetroArch-Win64/cores/genesis_plus_gx_libretro.dll"] if name == "cores" else None})
        lock_path = self.root / "lock.json"
        lock_path.write_text(json.dumps({"components": components}))
        spec = importlib.util.spec_from_file_location("builder", ROOT / "scripts/build_engine_bundle.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        output = self.root / "bundle.zip"
        module.build(lock_path, self.archives, output, self.sevenzip)
        with zipfile.ZipFile(output) as bundle:
            names = bundle.namelist()
            self.assertIn("tools/retroarch/retroarch.exe", names)
            self.assertIn("tools/retroarch/assets/menu.txt", names)
            self.assertIn("tools/retroarch/cores/genesis_plus_gx_libretro.dll", names)
            self.assertNotIn("tools/retroarch/cores/other_libretro.dll", names)
            self.assertIn("tools/pcsx2/pcsx2-qt.exe", names)
            self.assertIn("tools/xenia/xenia_canary.exe", names)
            self.assertIn("engine-lock.json", names)
            self.assertIn("licenses/retroarch-GPL-3.0.txt", names)
            self.assertIn("licenses/genesis-plus-gx-LICENSE.txt", names)


if __name__ == "__main__":
    unittest.main()
