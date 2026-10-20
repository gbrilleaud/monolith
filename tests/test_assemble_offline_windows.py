import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile

MODULE = Path(__file__).resolve().parents[1] / "scripts" / "assemble_offline_windows.py"
spec = importlib.util.spec_from_file_location("assemble_offline_windows", MODULE)
assembler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(assembler)


class OfflineWindowsTests(unittest.TestCase):
    def test_combines_client_and_engines_under_client_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            client, engines, output = (root / f"{name}.zip" for name in ("client", "engines", "offline"))
            with zipfile.ZipFile(client, "w") as archive:
                archive.writestr("monolith-windows-x86_64/monolith.exe", b"client")
            with zipfile.ZipFile(engines, "w") as archive:
                archive.writestr("tools/retroarch/retroarch.exe", b"ra")
                archive.writestr("tools/retroarch/cores/genesis_plus_gx_libretro.dll", b"core")
                archive.writestr("tools/pcsx2/pcsx2-qt.exe", b"ps2")
                archive.writestr("tools/xenia/xenia_canary.exe", b"x360")
                archive.writestr("engine-lock.json", b"{}")
            assembler.assemble(client, engines, output, "monolith-windows-x86_64")
            with zipfile.ZipFile(output) as archive:
                names = archive.namelist()
                self.assertIn("monolith-windows-x86_64/monolith.exe", names)
                self.assertIn("monolith-windows-x86_64/tools/retroarch/retroarch.exe", names)
                self.assertIn("monolith-windows-x86_64/engine-lock.json", names)
                self.assertEqual(archive.testzip(), None)

    def test_rejects_unsafe_engine_member_without_replacing_existing_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            client, engines, output = (root / f"{name}.zip" for name in ("client", "engines", "offline"))
            with zipfile.ZipFile(client, "w") as archive:
                archive.writestr("monolith-windows-x86_64/monolith.exe", b"client")
            with zipfile.ZipFile(engines, "w") as archive:
                archive.writestr("../private.txt", b"bad")
            output.write_bytes(b"original")
            with self.assertRaises(ValueError):
                assembler.assemble(client, engines, output, "monolith-windows-x86_64")
            self.assertEqual(output.read_bytes(), b"original")


if __name__ == "__main__":
    unittest.main()
