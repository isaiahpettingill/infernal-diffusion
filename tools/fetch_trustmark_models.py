#!/usr/bin/env python3
"""Explicit, checksum-verified download of Adobe's MIT-licensed Q models only."""
import argparse
import hashlib
from pathlib import Path
import tempfile
import urllib.request

ROOT = "https://cai-watermark.adobe.net/watermarking/trustmark-models/"
MODELS = {
    "encoder_Q.onnx": "19b3d1b25836130ffd78775a8f61539f993375d1823ef0e59ba5b8dffb4f892d",
    "decoder_Q.onnx": "ee3268f057c9dabef680e169302f5973d0589feea86189ed229a896cc3aa88df",
    "resizer.onnx": "e7d6014def382e20d3748fc793761bfddce62578a5905fa6ef3d348c69a787c3",
}


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    for name, expected in MODELS.items():
        destination = args.directory / name
        if destination.exists():
            if digest(destination) != expected:
                raise SystemExit(f"{destination}: existing file checksum differs; not overwriting")
            print(f"Verified {destination}")
            continue
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=args.directory, delete=False) as output:
                temporary = Path(output.name)
                with urllib.request.urlopen(ROOT + name, timeout=120) as source:
                    total = 0
                    while block := source.read(1024 * 1024):
                        total += len(block)
                        if total > 100_000_000:
                            raise ValueError("Model exceeds download limit")
                        output.write(block)
            if digest(temporary) != expected:
                raise ValueError(f"{name}: upstream bytes changed; expected SHA-256 {expected}")
            temporary.replace(destination)
            print(f"Downloaded and verified {destination}")
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    print("Models: Adobe TrustMark, MIT license. Retain upstream LICENSE when redistributing.")


if __name__ == "__main__":
    main()
