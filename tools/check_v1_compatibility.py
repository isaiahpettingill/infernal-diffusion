"""Compare V1 with pinned pre-V2 output on the same host and compiler.

Frozen hashes remain diagnostics: platform-dependent math can change rasterized
edge pixels across hosts. This reports baseline drift without presuming its exact
CPU/libm cause. The acceptance check has no tolerance for artwork or existing contracts. Every
PNG byte and the complete file set must match; only the additive world-scale
fields and their declared recipe/identity revision are normalized for protobuf.
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



def varint(value):
    result = bytearray()
    while value >= 128:
        result.append((value & 127) | 128)
        value >>= 7
    result.append(value)
    return bytes(result)


def fields(data):
    def read(at):
        value, shift = 0, 0
        while True:
            byte = data[at]
            at += 1
            value |= (byte & 127) << shift
            if byte < 128:
                return value, at
            shift += 7
    at = 0
    while at < len(data):
        key, at = read(at)
        number, wire = key >> 3, key & 7
        start = at
        if wire == 0:
            _, at = read(at)
        elif wire in (1, 5):
            at += 8 if wire == 1 else 4
        elif wire == 2:
            length, at = read(at)
            start = at
            at += length
        else:
            raise ValueError(f"unsupported protobuf wire type {wire}")
        yield number, wire, data[start:at]


def compatible_metadata(data, scope="monster"):
    output = bytearray()
    for number, wire, value in fields(data):
        if scope == "monster" and number == 2:
            continue  # Package ID intentionally includes the new scale policy.
        if scope == "size" and number in (9, 10, 11, 12):
            continue  # Only the additive player-relative scale contract.
        if scope == "generation" and number == 2:
            value = value.removesuffix(b"-scale.player_relative@1")
        if scope == "generation" and number == 8 and value == b"scale.player_relative@1":
            continue
        if scope == "monster" and number in (4, 16):
            value = compatible_metadata(value, "generation" if number == 4 else "size")
        output.extend(varint((number << 3) | wire))
        if wire == 2:
            output.extend(varint(len(value)))
        output.extend(value)
    return bytes(output)

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
                compatible = (compatible_metadata(before) == compatible_metadata(after)
                              if file.name == "monster.pb" and after_path.exists() else equal)
                baseline_hash = hashlib.sha256(before).hexdigest()
                frozen_hash = frozen.get(subject, {}).get(str(file))
                entry["files"].append({
                    "path": str(file), "equal": equal, "compatible": compatible,
                    "baseline_sha256": baseline_hash,
                    "candidate_sha256": hashlib.sha256(after).hexdigest(),
                    "original_host_sha256": frozen_hash,
                    "baseline_matches_original_host": baseline_hash == frozen_hash if frozen_hash else None,
                })
                if not compatible:
                    failed.append(f"{subject}: {file}")
            report["specimens"].append(entry)
    report.update(passed=not failed, differences=failed)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    if failed:
        raise SystemExit("V1 regression: " + ", ".join(failed))
    print(f"V1 compatibility: {len(SUBJECTS)} specimens; every PNG byte and existing metadata contract identical; additive scale/identity revision only")
    print(f"Host: {report['platform']}; libc: {report['libc']}")


if __name__ == "__main__":
    main()
