"""Build the Godot shim and fat x64 generator library for one Rust target.

Run from any directory: python tools/build_gdextension.py --target x86_64-pc-windows-msvc
Build on the matching operating system (Windows MSVC, Linux GNU, or macOS).
"""

import argparse
import os
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "godot" / "addons" / "infernal_diffusion" / "bin"
TARGETS = {
    "x86_64-pc-windows-msvc": ("windows", "x86_64", ".dll", ""),
    "i686-pc-windows-msvc": ("windows", "x86_32", ".dll", ""),
    "aarch64-pc-windows-msvc": ("windows", "arm64", ".dll", ""),
    "x86_64-unknown-linux-gnu": ("linux", "x86_64", ".so", "lib"),
    "i686-unknown-linux-gnu": ("linux", "x86_32", ".so", "lib"),
    "aarch64-unknown-linux-gnu": ("linux", "arm64", ".so", "lib"),
    "x86_64-apple-darwin": ("macos", "x86_64", ".dylib", "lib"),
    "aarch64-apple-darwin": ("macos", "arm64", ".dylib", "lib"),
}
TIERS = {
    "x86_64": (("x64", "x86-64"),),
    "x86_32": (("x86", "pentium4"),),
    "arm64": (("arm64", "generic"),),
}


def build(manifest: Path, target: str, tier: str, cpu: str) -> Path:
    environment = os.environ.copy()
    environment["RUSTFLAGS"] = " ".join(
        value for value in (environment.get("RUSTFLAGS", ""), f"-C target-cpu={cpu}") if value
    )
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target" / "godot-build" / target / tier)
    command = [
        "cargo", "build", "--manifest-path", str(manifest), "--target", target,
        "--release", "--lib",
    ]
    print(" ".join(command), f"[{tier}, {cpu}]", flush=True)
    subprocess.run(command, check=True, cwd=ROOT, env=environment)
    return Path(environment["CARGO_TARGET_DIR"]) / target / "release"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--only-shim", action="store_true")
    args = parser.parse_args()
    platform, arch, suffix, prefix = TARGETS[args.target]
    BIN.mkdir(parents=True, exist_ok=True)
    if arch == "x86_64" and not args.only_shim:
        # A prior build may have shipped separate v2/v3 DLLs. Keep the addon
        # directory representative of the current single-binary package.
        for obsolete in ("x64_v2", "x64_v3"):
            (BIN / f"{prefix}infernal_diffusion_{obsolete}{suffix}").unlink(missing_ok=True)

    shim_dir = build(ROOT / "godot" / "extension" / "Cargo.toml", args.target, "shim", "x86-64" if arch == "x86_64" else "generic" if arch == "arm64" else "pentium4")
    shim_source = shim_dir / f"{prefix}infernal_godot{suffix}"
    shim_output = BIN / f"{prefix}infernal_godot.{platform}.{arch}{suffix}"
    shutil.copy2(shim_source, shim_output)
    print(f"Created {shim_output}")
    if args.only_shim:
        return

    for tier, cpu in TIERS[arch]:
        core_dir = build(ROOT / "Cargo.toml", args.target, tier, cpu)
        core_source = core_dir / f"{prefix}infernal_diffusion{suffix}"
        core_output = BIN / f"{prefix}infernal_diffusion_{tier}{suffix}"
        shutil.copy2(core_source, core_output)
        print(f"Created {core_output}")


if __name__ == "__main__":
    main()
