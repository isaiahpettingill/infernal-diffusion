"""Compare V1 with pinned pre-V2 output on the same host and compiler.

Frozen hashes remain diagnostics: platform-dependent math can change rasterized
edge pixels across hosts. This reports baseline drift without presuming its exact
CPU/libm cause. The acceptance check has no tolerance: every PNG and protobuf
byte, and the complete file set, must match the original implementation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import tempfile

REFERENCE_COMMIT = "6f0c9b31eabfdc9e1b593cf1c3621f75d63dd9f3"
SUBJECTS = [
    "wolf", "beetle", "spider", "orc", "dragon", "mammoth", "serpent", "slime",
    "snail", "stormtrooper", "humanoid",
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--report", type=Path, default=Path("v1-compatibility-report.json"))
    args = parser.parse_args()
    binaries = {"baseline": args.baseline.resolve(), "candidate": args.candidate.resolve()}
    frozen_path = Path(__file__).resolve().parents[1] / "tests/v1_terrestrial_file_goldens.json"
    frozen = json.loads(frozen_path.read_text())
    report = {
        "reference_commit": REFERENCE_COMMIT,
        "platform": platform.platform(),
        "libc": platform.libc_ver(),
        "machine": platform.machine(),
        "compiler": subprocess.run(["rustc", "-Vv"], check=True, capture_output=True, text=True).stdout,
        "specimens": [],
    }
    failed = []
    with tempfile.TemporaryDirectory(prefix="infernal-v1-compat-") as temporary:
        root = Path(temporary)
        for subject in SUBJECTS:
            dirs = {name: root / name / subject for name in binaries}
            for name, binary in binaries.items():
                subprocess.run(
                    [str(binary), subject, "42", str(dirs[name])],
                    check=True, capture_output=True, text=True,
                )
            files = {
                name: sorted(p.relative_to(directory) for p in directory.rglob("*") if p.is_file())
                for name, directory in dirs.items()
            }
            if files["baseline"] != files["candidate"]:
                failed.append(f"{subject}: file set")
            entry = {"prompt": subject, "seed": 42, "files": []}
            for file in files["baseline"]:
                before = (dirs["baseline"] / file).read_bytes()
                after_path = dirs["candidate"] / file
                after = after_path.read_bytes() if after_path.exists() else b""
                equal = before == after and after_path.exists()
                baseline_hash = hashlib.sha256(before).hexdigest()
                frozen_hash = frozen.get(subject, {}).get(str(file))
                entry["files"].append({
                    "path": str(file), "equal": equal,
                    "baseline_sha256": baseline_hash,
                    "candidate_sha256": hashlib.sha256(after).hexdigest(),
                    "original_host_sha256": frozen_hash,
                    "baseline_matches_original_host": baseline_hash == frozen_hash if frozen_hash else None,
                })
                if not equal:
                    failed.append(f"{subject}: {file}")
            report["specimens"].append(entry)
    report.update(passed=not failed, differences=failed)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    if failed:
        raise SystemExit("V1 regression: " + ", ".join(failed))
    print(f"V1 compatibility: {len(SUBJECTS)} specimens; every PNG/protobuf byte identical to pinned baseline")
    print(f"Host: {report['platform']}; libc: {report['libc']}")


if __name__ == "__main__":
    main()
