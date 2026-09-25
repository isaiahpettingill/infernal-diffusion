"""Minimal ctypes consumer: python examples/ffi_smoke.py <shared-library> <output-dir> [2d]."""

import ctypes
import pathlib
import sys


def main() -> None:
    if len(sys.argv) not in (3, 4) or (len(sys.argv) == 4 and sys.argv[3] not in ("2d", "3d")):
        raise SystemExit("usage: ffi_smoke.py <shared-library> <output-dir> [2d]")
    library = ctypes.CDLL(str(pathlib.Path(sys.argv[1]).resolve()))
    library.infernal_abi_version.restype = ctypes.c_uint32
    library.infernal_generate.argtypes = [
        ctypes.c_char_p,
        ctypes.c_uint64,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    library.infernal_generate.restype = ctypes.c_int32
    library.infernal_generate_3d.argtypes = library.infernal_generate.argtypes
    library.infernal_generate_3d.restype = ctypes.c_int32
    library.infernal_generate_2d.argtypes = library.infernal_generate.argtypes
    library.infernal_generate_2d.restype = ctypes.c_int32
    library.infernal_validate_package.argtypes = [
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    library.infernal_validate_package.restype = ctypes.c_int32
    library.infernal_free_string.argtypes = [ctypes.c_void_p]
    assert library.infernal_abi_version() == 1
    directory = str(pathlib.Path(sys.argv[2]).resolve()).encode("utf-8")
    error = ctypes.c_void_p()
    try:
        generate = library.infernal_generate_2d if len(sys.argv) == 4 and sys.argv[3] == "2d" else library.infernal_generate
        if generate(
            b"skeletal jackal with a shotgun", 7, directory, ctypes.byref(error)
        ) != 0:
            raise RuntimeError(ctypes.string_at(error).decode("utf-8"))
        if library.infernal_validate_package(directory, ctypes.byref(error)) != 0:
            raise RuntimeError(ctypes.string_at(error).decode("utf-8"))
    finally:
        if error.value:
            library.infernal_free_string(error)
    print("C ABI generated and validated package")


if __name__ == "__main__":
    main()
