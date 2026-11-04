"""Fresh-process in-game C ABI timing/RSS, excluding PNG encoding and pixel analysis.

Each prompt/profile gets its own process. First run is cold; subsequent runs are
warm. RSS is the Linux process high-water mark, not allocator/live bytes. Raw
retained atlas bytes are counted independently. Hash analysis runs AFTER the RSS
snapshot and is excluded from the reported peak. Use identical thread counts.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
import platform
from pathlib import Path
import resource
import statistics
import subprocess
import sys
import time


def worker(args):
    lib = C.CDLL(str(args.library.resolve()))
    ptr = C.c_void_p
    error = ptr()
    generate = lib.infernal_generate_in_memory_profile_compact if args.compact else lib.infernal_generate_in_memory_profile
    generate.argtypes = [C.c_char_p, C.c_uint64, C.c_uint32, C.POINTER(ptr)]
    generate.restype = ptr
    lib.infernal_object_free.argtypes = [ptr]
    lib.infernal_free_string.argtypes = [ptr]
    lib.infernal_object_count.argtypes = [ptr]
    lib.infernal_object_count.restype = C.c_uint32
    lib.infernal_object_pixels.argtypes = [ptr, C.c_uint32, C.c_uint32, C.POINTER(ptr), C.POINTER(C.c_size_t), C.POINTER(C.c_uint32), C.POINTER(C.c_uint32)]
    lib.infernal_cpu_kernel_name.restype = C.c_char_p
    durations = []
    cpu_durations = []
    initial_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
    handle = None
    try:
        for i in range(args.runs):
            if handle:
                lib.infernal_object_free(handle)
                handle = None
            print(f"BENCH profile={args.profile} prompt={args.prompt!r} run={i}", file=sys.stderr, flush=True)
            cpu_start = time.process_time()
            start = time.perf_counter()
            handle = generate(args.prompt.encode(), args.seed, int(args.profile[1:]), C.byref(error))
            durations.append(time.perf_counter() - start)
            cpu_durations.append(time.process_time() - cpu_start)
            if not handle:
                message = C.string_at(error).decode() if error.value else 'unknown error'
                if error.value:
                    lib.infernal_free_string(error)
                raise RuntimeError(message)
        peak_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
        atlases = []
        for index in range(lib.infernal_object_count(handle)):
            for kind in range(3):
                pixels, length, width, height = ptr(), C.c_size_t(), C.c_uint32(), C.c_uint32()
                status = lib.infernal_object_pixels(handle, index, kind, C.byref(pixels), C.byref(length), C.byref(width), C.byref(height))
                if status != 0:
                    continue
                raw = C.string_at(pixels, length.value)
                alpha = raw[3::4]
                atlases.append(dict(package=index, kind=kind, width=width.value, height=height.value,
                    rgba_bytes=length.value, rgba_sha256=hashlib.sha256(raw).hexdigest(),
                    alpha_sha256=hashlib.sha256(alpha).hexdigest(),
                    nonzero_alpha=sum(a != 0 for a in alpha), partial_alpha=sum(0 < a < 255 for a in alpha)))
        result = dict(prompt=args.prompt, profile=args.profile, compact=args.compact, seed=args.seed,
            cpu_seconds=cpu_durations, warm_median_cpu_seconds=statistics.median(cpu_durations[1:]) if len(cpu_durations)>1 else None,
            seconds=durations, warm_median_seconds=statistics.median(durations[1:]) if len(durations)>1 else None,
            initial_rss_bytes=initial_rss, generation_peak_rss_bytes=peak_rss,
            retained_rgba_bytes=sum(a['rgba_bytes'] for a in atlases), atlases=atlases,
            kernel=lib.infernal_cpu_kernel_name().decode(), rayon_threads=os.environ.get('RAYON_NUM_THREADS'),
            library=str(args.library.resolve()), platform=platform.platform(),
            python=sys.version.split()[0], rss_units="bytes",
            rss_note="Process high-water mark before pixel analysis; includes runtime and allocator retention")
        print(json.dumps(result))
    finally:
        if handle:
            lib.infernal_object_free(handle)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('library', type=Path)
    p.add_argument('--prompt', default='wolf')
    p.add_argument('--profile', choices=['v1', 'v2'], default='v1')
    p.add_argument('--runs', type=int, default=4)
    p.add_argument('--seed', type=int, default=42)
    p.add_argument('--compact', action='store_true')
    p.add_argument('--suite', action='store_true', help='Run wolf, mounted orc, giant spider, ghost for both profiles')
    p.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    args = p.parse_args()
    if args.runs < 1:
        p.error('--runs must be positive')
    if args.worker:
        worker(args)
        return
    cases = [(prompt, profile) for profile in ['v1', 'v2'] for prompt in ['wolf', 'orc riding a wolf', 'giant spider', 'ghost']] if args.suite else [(args.prompt, args.profile)]
    for prompt, profile in cases:
        command = [sys.executable, str(Path(__file__).resolve()), str(args.library.resolve()), '--worker', '--prompt', prompt, '--profile', profile, '--runs', str(args.runs), '--seed', str(args.seed)]
        if args.compact:
            command.append('--compact')
        subprocess.run(command, check=True)


if __name__ == '__main__':
    main()
