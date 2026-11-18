# Mounted generation, monster meshes and shade attacks

These measurements compare the native compact C ABI at seed 42 against commit `1aa6bbe1a529cd7413b0d543c6c70fcb28a77024`. Timing includes all rider, mount and minion packages. The host had nine available logical CPUs on an AMD EPYC 9V74, running Linux x86-64 and Rust 1.100.0. Each build used its normal render pool. The baseline used two render workers. The candidate used four. Both had `RAYON_NUM_THREADS=2`.

Each case ran in a fresh process, alternating which build ran first across cases. Wall and process CPU times are medians of three warm generations after one cold generation. Peak RSS is the process lifetime high-water mark before reading the output for hash checks. It includes the host, scratch buffers, allocator, returned textures and the embedded model where enabled. Atlas SHA-256 covers dimensions and raw RGBA for every sprite, emission and projectile atlas in every package.

| Case | Packages | Wall s, before → after | Time saved | CPU s, before → after | Peak MiB, before → after |
|---|---:|---:|---:|---:|---:|
| Orc riding a giant spider | 3 | 5.45 → 2.50 | 54% | 7.71 → 4.93 | 167 → 122 |
| Goblin riding a wolf riding a giant spider | 5 | 8.14 → 3.53 | 57% | 11.51 → 7.37 | 227 → 137 |
| Skeleton | 1 | 2.03 → 0.74 | 64% | 2.68 → 1.53 | 58 → 59 |
| Giant fire-breathing dragon | 1 | 1.85 → 1.21 | 35% | 2.66 → 2.16 | 124 → 100 |
| Mushroom that summons hornets | 2 | 1.16 → 0.63 | 46% | 1.70 → 1.21 | 63 → 61 |
| Four-armed, eight-legged hybrid | 1 | 2.07 → 0.99 | 52% | 2.85 → 1.90 | 84 → 72 |
| Wolf (V1) | 1 | 0.61 → 0.38 | 38% | 0.93 → 0.82 | 52 → 52 |
| Shade humanoid | 1 | 1.31 → 0.61 | 53% | 1.72 → 1.11 | 54 → 63 |
| Nested mount, 3,999-byte BERT input | 5 | 7.51 → 3.39 | 55% | 10.87 → 7.11 | 360 → 211 |
| Armored rider, 2,514-byte BERT input | 3 | 7.37 → 3.48 | 53% | 10.63 → 6.63 | 343 → 230 |

The first eight cases used the vocabulary build. The last two used the default embedded BERT model. The 3,999-byte case extends the same five-package prompt with repeated scene description to isolate input length. The [2,514-byte armored rider](../../tools/benchmark-fixtures/armored-rider.txt) uses distinct instructions for anatomy, equipment, attack movement and dismount behavior. All eight cases outside the revised skeleton and shade artwork matched the baseline atlas bytes.

With both builds restricted to the same four logical CPUs, each used two render workers. The five-package nested mount took 8.13 → 5.01 s. This measures the improvements with the same worker count.

The production portable WebAssembly bundle used its embedded BERT model on one thread in Node.js 24.22.3. Generating the five-package nested mount took 12.25 → 9.08 s, with matching RGBA hashes across all five packages. Model initialization and hashing were outside the timed interval.

## CPU profile

GNU gprofng sampled the release renderer at 2 ms intervals. On the original build, rasterization and its callees accounted for 91–93% of sampled CPU time on the mount, nested mount, skeleton, dragon and summoner. The skeleton spent 86% in the raster loop itself. Texture noise alone accounted for 19% on the mount and 22% on the summoner. Rasterization still accounted for 88% on the 3,999-byte prompt and 85% on the detailed armored rider.

| CPU profile case | Raster, exclusive | Raster and callees | Texture noise, exclusive |
|---|---:|---:|---:|
| Orc riding a giant spider | 62.23% | 91.79% | 19.12% |
| Goblin riding a wolf riding a giant spider | 61.00% | 92.48% | 21.19% |
| Skeleton | 85.99% | 92.51% | below sample resolution |
| Giant fire-breathing dragon | 60.77% | 91.15% | 16.75% |
| Mushroom that summons hornets | 59.62% | 91.92% | 22.31% |
| Four-armed, eight-legged hybrid | 74.22% | 93.25% | 12.53% |
| Wolf (V1) | 61.54% | 90.38% | 22.44% |
| Shade humanoid | 68.66% | 92.16% | 12.31% |
| Nested mount, 3,999-byte BERT input | 59.20% | 87.66% | 18.36% |
| Armored rider, 2,514-byte BERT input | 61.20% | 85.36% | 16.13% |

The V2 depth pass now finds the winning face before evaluating texture noise. Each pose computes face lighting once across all four directions. Body parts reuse their scaled meshes and materials. A bounding box can prove that a pose fits the current frame before checking every vertex. Duplicate frame reuse requires a full bitwise scene match. The mounted generator reserves its full budget before recursion and allocates the parent sheet after the child packages finish. Empty tile rows can stay on shared zero pages on systems that support them.

A second CPU profile after these changes found 92% of skeleton samples and 88% of dragon samples still in the V2 raster kernel. Reducing the skeleton's fine geometry then cut its warm time from 0.88 to 0.74 s with the same renderer and worker count.

## Mesh and attack changes

| Skeleton part | Original triangles | Revised triangles |
|---|---:|---:|
| Ribs | 4,372 | 2,476 |
| Skull | 3,947 | 2,431 |
| Long bone | 496 | 416 |
| Hand | 1,268 | 740 |
| Foot | 940 | 580 |
| Pelvis | 568 | 504 |

Small teeth and joints now use one vertex per pole. Thin bones use fewer sides. The rib paths still have a separate curve for each rib. Across the full seed-42 sprite sheet, the fraction of occupied pixels shared by both silhouettes (intersection over union) was 99.17%. About 24.8% of occupied pixels changed RGBA, including small shifts in bone shading. The mean absolute RGB change was 7.4 out of 255 over the silhouette union. [Side-by-side idle frame](../evidence/v2-skeleton-detail.png)

The three shade tendrils use 480 triangles, down from 1,860. Shared rings keep them connected during the coil, overhead lash and low thrust. Their needle tips follow the same centerlines as the attack contacts. The full extension still occurs at 650 ms. The longer strike needs a larger dense frame on this specimen. [Shade animation](../evidence/v2-shade-tendrils.gif)

## Reproduce

Build each revision with the same toolchain, copy its shared library to a separate directory, and run the candidate benchmark script against each library. For the native table, use the vocabulary build for the eight short cases and the default build for the two long cases. The suite can also run all cases with either parser.

```sh
cargo build --release --locked --no-default-features --lib
RAYON_NUM_THREADS=2 python3 tools/bench_in_memory.py target/release/libinfernal_diffusion.so --stress-suite --compact --runs 4
```

To collect the baseline CPU samples, run the script's single-case worker directly under gprofng. This keeps the native renderer in the profiled process.

```sh
RAYON_NUM_THREADS=2 gprofng collect app -p 2 -o skeleton.er python3 tools/bench_in_memory.py target/release/libinfernal_diffusion.so --worker --timing-only --prompt skeleton --profile v2 --compact --runs 1
gprofng display text -functions skeleton.er
```

[Raw timings, peak memory and atlas digests](mounted-generation-samples.jsonl)
