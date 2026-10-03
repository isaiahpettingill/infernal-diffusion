"""Run real in-memory generation, descriptor projection, save/decode and release.

First build the host addon with tools/build_gdextension.py. This uses a disposable
Godot project and isolated user-data directory; no game project is modified.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", required=True, type=Path)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--startup-frames", type=int, default=120,
                        help="allow initial editor extension-doc work to finish before shutdown")
    parser.add_argument("--immediate-import", action="store_true",
                        help="diagnose upstream Godot first-import shutdown race")
    args = parser.parse_args()
    if args.startup_frames < 1 or args.timeout < 1:
        parser.error("startup-frames and timeout must be positive")
    addon = ROOT / "godot" / "addons" / "infernal_diffusion"
    if not (addon / "bin").is_dir():
        parser.error("build the host addon with tools/build_gdextension.py first")
    with tempfile.TemporaryDirectory(prefix="infernal-gdextension-smoke-") as directory:
        project = Path(directory)
        shutil.copytree(addon, project / "addons" / "infernal_diffusion")
        shutil.copy2(ROOT / "godot" / "tests" / "smoke.gd", project / "smoke.gd")
        user_data = (project / "userdata").as_posix()
        (project / "project.godot").write_text(
            'config_version=5\n[application]\nconfig/name="Infernal smoke"\n'
            'config/use_custom_user_dir=true\n'
            f'config/custom_user_dir_name="{user_data}"\n'
            '[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding="utf-8"
        )
        environment = os.environ.copy()
        # Keep both headless editor settings and runtime saves inside this project.
        for name, relative in [("HOME", "home"), ("XDG_DATA_HOME", "data"),
                               ("XDG_CONFIG_HOME", "config"), ("XDG_CACHE_HOME", "cache")]:
            path = project / relative
            path.mkdir()
            environment[name] = str(path)
        environment["GODOT_SILENCE_ROOT_WARNING"] = "1"
        base = [str(args.godot.resolve()), "--headless", "--path", str(project)]
        # Real first discovery with no cached extension registration. Immediate
        # --import exit can race Godot's asynchronous extension-doc generation
        # (godotengine/godot#111645); let the editor finish that work first.
        startup = (["--editor", "--import"] if args.immediate_import else
                   ["--editor", "--quit-after", str(args.startup_frames)])
        for suffix in [startup, ["--script", "res://smoke.gd"]]:
            try:
                result = subprocess.run(base + suffix, capture_output=True, text=True,
                                        timeout=args.timeout, env=environment)
            except subprocess.TimeoutExpired as error:
                for output in [error.stdout, error.stderr]:
                    if output:
                        print(output.decode(errors="replace") if isinstance(output, bytes) else output)
                raise SystemExit(f"GDExtension smoke timed out ({suffix})") from error
            output = result.stdout + result.stderr
            print(output, end="")
            if result.returncode or "SCRIPT ERROR" in output or "ERROR:" in output:
                raise SystemExit(f"GDExtension smoke failed ({suffix}, exit {result.returncode})")
        if "INFERNAL_GDEXTENSION_SMOKE_OK" not in output:
            raise SystemExit("GDExtension smoke did not report completion")


if __name__ == "__main__":
    main()
