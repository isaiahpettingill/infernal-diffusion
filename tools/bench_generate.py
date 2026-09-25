"""Time repeated 3D generation inside one process, including a warm model run."""

import argparse
import ctypes
import json
import os
from pathlib import Path
import tempfile
import time


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library", type=Path)
    parser.add_argument("--prompt", default="wolf")
    parser.add_argument("--runs", type=int, default=2)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")

    lib = ctypes.CDLL(str(args.library.resolve()))
    lib.infernal_generate_3d.argtypes = [ctypes.c_char_p, ctypes.c_uint64, ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
    lib.infernal_generate_3d.restype = ctypes.c_int
    lib.infernal_cpu_kernel_name.restype = ctypes.c_char_p
    lib.infernal_free_string.argtypes = [ctypes.c_void_p]
    durations = []
    with tempfile.TemporaryDirectory(prefix="infernal-bench-") as root:
        for index in range(args.runs):
            output = Path(root) / str(index)
            error = ctypes.c_void_p()
            begin = time.perf_counter()
            status = lib.infernal_generate_3d(args.prompt.encode(), 42, str(output).encode(), ctypes.byref(error))
            durations.append(round(time.perf_counter() - begin, 3))
            if status != 0:
                message = ctypes.string_at(error).decode() if error.value else "unknown error"
                if error.value:
                    lib.infernal_free_string(error)
                raise RuntimeError(message)
            if not (output / "monster.pb").is_file() or not (output / "sprites.png").is_file():
                raise RuntimeError("generation returned without a complete package")
    print(json.dumps({"library": str(args.library), "prompt": args.prompt,
                      "kernel": lib.infernal_cpu_kernel_name().decode(),
                      "requested_kernel": os.environ.get("INFERNAL_CPU_KERNEL"),
                      "seconds": durations}))


if __name__ == "__main__":
    main()
