"""Regenerate the Godot 4.6 Godobuf reader from the canonical monster schema."""

import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ADDON = ROOT / "godot" / "addons" / "infernal_diffusion"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", required=True, type=Path, help="Godot 4.6 executable")
    args = parser.parse_args()
    subprocess.run(["python", str(ROOT / "tools" / "sync_godobuf_schema.py")], check=True)
    with tempfile.TemporaryDirectory(prefix="infernal-godobuf-") as directory:
        project = Path(directory)
        (project / "project.godot").write_text("config_version=5\n", encoding="utf-8")
        shutil.copytree(ROOT / "godot" / "addons" / "godobuf", project / "addons" / "godobuf")
        source = project / "monster_godobuf.proto"
        output = project / "monster_pb.gd"
        shutil.copy2(ADDON / source.name, source)
        command = [
            str(args.godot.resolve()), "--headless", "--path", str(project),
            "-s", "res://addons/godobuf/godobuf_cmdln.gd",
            f"--input={source.as_posix()}", f"--output={output.as_posix()}",
            "--prefix=Infernal",
        ]
        result = subprocess.run(command, text=True, capture_output=True, check=True)
        if not output.is_file() or "* Output file was created successfully. *" not in result.stdout:
            raise RuntimeError(result.stdout + result.stderr)
        text = output.read_text(encoding="utf-8")
        if "class InfernalMonster:" not in text:
            raise RuntimeError("generated Godobuf class is missing InfernalMonster")
        text = "\n".join(line.rstrip() for line in text.splitlines()) + "\n"
        (ADDON / output.name).write_text(text, encoding="utf-8")
        print(f"Generated {ADDON / output.name}")


if __name__ == "__main__":
    main()
