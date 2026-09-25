"""Create Godobuf's package-free view of the canonical wire schema."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
source = (ROOT / "proto" / "monster.proto").read_text(encoding="utf-8")
header = "// Generated from proto/monster.proto for Godobuf 0.7.0.\n// Only the unsupported package declaration is removed; field numbers stay identical.\n"
without_package = source.replace("package infernal.monster;\n", "", 1)
if without_package == source:
    raise RuntimeError("canonical package declaration changed")
(ROOT / "godot" / "addons" / "infernal_diffusion" / "monster_godobuf.proto").write_text(
    header + without_package, encoding="utf-8"
)
