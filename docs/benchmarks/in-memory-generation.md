# Final dynamic-scheduler measurements

Baseline: `230f3d7e1fdf5edefdcdb9a62409454243481836`. Final candidate: a dynamic direction queue across two native scratch lanes, one Rayon dispatch per pose, triangle-loop cleanup, and collision-safe compact atlas deduplication. It does not use the superseded two-direction batch barriers.

Native Linux release builds, `--no-default-features`, seed 42, `RAYON_NUM_THREADS=2`, `INFERNAL_PROFILE=1`. Each case uses a fresh Python process and the in-memory C ABI, with one cold plus two warm runs. Order: wolf baseline/candidate, compact mounted orc baseline/candidate. Local builds/tests were stopped before this bounded comparison. Shared-host noise remains; two warm samples per case do not establish a universal performance result. Embedded BERT and PNG encoding are excluded.

| Case | Warm wall s baseline/final | Warm CPU s baseline/final | Peak RSS MiB baseline/final | Retained RGBA MiB |
|---|---:|---:|---:|---:|
| v1 wolf | 1.341/0.967 | 2.242/1.425 | 45.6/44.9 | 45.7 |
| v2 orc riding a wolf compact | 5.867/5.664 | 8.729/8.131 | 126.8/124.1 | 17.6 |

Every returned atlas is pixel-identical: dimensions, RGBA byte length, SHA-256 of RGBA, SHA-256 of alpha, nonzero-alpha count, and partial-alpha count all match. Retained RGBA bytes are unchanged.

Peak RSS is the process lifetime high-water mark sampled after generation and before copying/hashing pixels. It includes Python, runtime, scratch, and allocator retention, not just peak live allocations. Retained RGBA is the sum of all returned sprite/emission/projectile buffers across packages. Demand-zero pages can make these lengths exceed resident bytes. Indexed PNG disk savings do not reduce these returned RGBA buffers.

Raw final samples are `in-memory-baseline.jsonl` and `in-memory-candidate.jsonl`; stage traces are `in-memory-dynamic-stages.log`. Historical broader four-case and interleaved results for the superseded batched scheduler are in `in-memory-batched-generation.md` and its associated files; do not present them as final candidate measurements.

## Reproduce

```sh
cargo build --release --no-default-features --lib
INFERNAL_PROFILE=1 RAYON_NUM_THREADS=2 python tools/bench_in_memory.py /path/to/libinfernal_diffusion.so --prompt wolf --profile v1 --runs 3
INFERNAL_PROFILE=1 RAYON_NUM_THREADS=2 python tools/bench_in_memory.py /path/to/libinfernal_diffusion.so --prompt "orc riding a wolf" --profile v2 --compact --runs 3
```

Preserve a separate shared library for each revision; use identical features and a quiet host. `--suite` adds wolf/mounted-orc/spider/ghost across V1/V2. Stage raster timing sums worker time, not wall time. These native measurements do not establish browser/WASM performance.
