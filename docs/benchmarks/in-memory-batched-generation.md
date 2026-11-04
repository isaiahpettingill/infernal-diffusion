> Historical experiment only: these candidate results use the superseded two-direction batch scheduler, not the final dynamic scheduler.

# Historical batched-scheduler measurements

Baseline: `230f3d7e1fdf5edefdcdb9a62409454243481836`. Candidate: renderer scratch reuse, triangle-loop cleanup, and collision-safe compact atlas deduplication in this change. Both are native release builds with `--no-default-features`, seed 42, and `RAYON_NUM_THREADS=2`. No embedded BERT model or PNG encoding is included.

**Timing limitation:** this shared host was noisy. Baseline samples overlapped other local work; candidate samples were collected after that work stopped. Baseline has one cold plus three warm runs, candidate one cold plus two warm runs. The table shows warm medians for transparency, not a controlled speedup result. These samples do not demonstrate a generation speedup.

Each case uses a fresh Python process calling the profile-specific C ABI directly. Peak RSS is the process lifetime high-water mark recorded immediately after generation, before copying/hashing pixels. It includes Python, native runtime, scratch buffers, and allocator retention; it is not peak live allocator bytes. Retained RGBA sums all returned sprite, emission, and projectile buffer lengths, including mounted packages. Demand-zero pages mean RGBA buffer lengths can exceed resident bytes. Indexed PNG disk savings do not reduce these RGBA buffers.

| Case | API | Baseline warm s | Candidate warm s | Baseline peak RSS MiB | Candidate peak RSS MiB | Retained RGBA MiB |
|---|---|---:|---:|---:|---:|---:|
| v1 wolf | full | 1.127 | 1.383 | 45.4 | 44.9 | 45.7 |
| v2 giant spider | full | 2.658 | 2.820 | 64.7 | 63.2 | 81.2 |
| v2 ghost | full | 1.459 | 1.614 | 65.0 | 63.9 | 86.1 |
| v2 orc riding a wolf | compact | 9.854 | 8.942 | 126.7 | 124.1 | 17.6 |

Every paired atlas matches exactly in dimensions, RGBA byte length, SHA-256 of RGBA pixels, SHA-256 of alpha, nonzero-alpha count, and partial-alpha count. This covers sparse spider output, spectral ghost transparency, and compact mounted output. Retained RGBA bytes are unchanged. `in-memory-batched-baseline.jsonl` and `in-memory-batched-candidate.jsonl` contain every sample and hash for this historical variant.

## Reproduce

```sh
cargo build --release --no-default-features --lib
INFERNAL_PROFILE=1 RAYON_NUM_THREADS=2 python tools/bench_in_memory.py /path/to/libinfernal_diffusion.so --prompt wolf --profile v1 --runs 3
INFERNAL_PROFILE=1 RAYON_NUM_THREADS=2 python tools/bench_in_memory.py /path/to/libinfernal_diffusion.so --prompt "orc riding a wolf" --profile v2 --compact --runs 3
```

Use separate preserved libraries for each revision and a quiet machine. `--suite` covers wolf, mounted orc, giant spider, and ghost for V1/V2. Profile stderr reports scene/raster/sheet stages; summed raster worker time is not wall time. CPU time is recorded by the final script as an additional diagnostic. These native measurements do not establish browser/WASM performance.

## Profile stage medians (same noisy samples)

Sum across packages per generation, milliseconds. Raster is summed worker time, not wall time.

| Case | Scene before/after | Raster before/after | Sheet wall before/after |
|---|---:|---:|---:|
| v1 wolf | 90.7/131.1 | 1266.7/1458.2 | 958.8/1171.2 |
| v2 giant spider | 407.3/386.0 | 2382.5/3026.0 | 2100.6/2274.6 |
| v2 ghost | 111.2/98.8 | 1534.8/1866.4 | 1233.3/1438.0 |
| v2 orc riding a wolf compact | 1353.2/1035.1 | 9957.7/9317.8 | 8050.7/7202.2 |

## Bounded interleaved wolf check

A follow-up ran baseline/candidate/baseline/candidate in four fresh processes, each cold + two warm generations, with process CPU time enabled and no profiling log overhead. First pair warm wall medians were 0.956/1.250 seconds and CPU medians 1.451/1.797 seconds. Second pair wall medians were 1.416/1.235 seconds and CPU medians 2.398/1.573 seconds. Thus candidate was not consistently slower across pairs, but pooled warm sample medians were 1.017/1.250 seconds wall (+23%) and 1.622/1.733 seconds CPU (+7%). Host noise remains large; these results cannot rule out a scheduling regression and still do not support a speedup claim. All pixel/alpha hashes match exactly. See `in-memory-wolf-interleaved.jsonl` for raw samples.
