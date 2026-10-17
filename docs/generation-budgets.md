# Live generation admission budgets

Compact runtime generation now uses `GenerationBudget::default()`. This applies to
Rust `generate_in_memory_compact_with_profile` / `generate_hybrid_in_memory_compact`,
the existing compact C ABI exports, the native Godot bridge using those exports,
and the corresponding compact WASM exports. No exported symbol or argument changes.
Legacy dense/offline entry points retain their previous behavior.

The default admission limits are deliberately generous:

- 4,096 UTF-8 prompt bytes; at most three riding/mount links (four creatures)
- Eight generated packages, including root, detached riders/mounts and minion
- Five heads, twelve legs and eight arms per creature; 128 declared parts across a composite
- 384 assembled anatomy nodes and 512 poses per package
- 512 px logical frame side; 16,384 px dense atlas side; 64 Mi dense pixels
- 2 GiB cumulative estimated generator working storage across the whole request

The estimate reserves eight dense RGBA equivalents for raster/compaction pixel
storage, supersampled RGBA/depth/downsample buffers for all four directions,
256 KiB per anatomy node for mesh/pose/metadata work, and 8 MiB per package.
Reservations are cumulative: an already-rendered parent remains counted while its
companions are generated. This intentionally overestimates retained compact
textures instead of assuming compaction will always be effective.

These are admission guards, **not an OS-level memory or CPU quota**. Parser/model
memory, allocator overhead, native engine/JS copies and unrelated application
memory are not covered. The game should still run one background generation at a
time with a bounded pending queue. Guarded native BERT parsing uses a dedicated
maximum-two-worker Rayon pool (one worker on hosts with fewer than four logical
cores). Native rasterization has its existing separate two-worker pool and runs
after parsing, or renders serially on smaller hosts. Portable WASM runs serially
inside its Web Worker; the optional threaded build uses the host-initialized Rayon
pool (the provided worker initializes 2–4 threads). Ordinary memory spikes remain possible.

Input depth is checked before invoking BERT. Every nested spec is checked before
recursive anatomy construction, anatomy nodes before physics/animation, and a
lower-bound canvas before fitting all posed meshes. Final dense dimensions,
pixels and cumulative reservation are checked before sprite rasterization and its
large allocations. A failure returns `Error::ResourceLimit` with the resource,
requested amount and limit; C returns its normal null/error string and WASM throws
its normal generation error. The in-memory call returns no partial result.

No prompt rewriting, anatomy removal, dropped special attacks, frame thinning,
rescaling, or texture degradation is used to fit a budget. In particular, overly
long riding chains are rejected instead of inheriting the compatibility parser's
four-clause truncation. Prebaked packages can still be loaded independently of
these live-generation limits.

Rust callers can select explicit limits through
`generate_in_memory_compact_with_budget` and
`generate_hybrid_in_memory_compact_with_budget`. Start from
`GenerationBudget::default()` and change the relevant fields. Raising a budget
does not remove existing geometry, parser or package-format constraints. There
is no deadline/cancellation claim: these bounds limit admitted work, while the
host owns queueing and worker lifetime.

## Detecting installed runtime support

Compact storage predates these guards. Do not infer live-generation safety from
ABI version 1, format version 8, a compact export, or a copied build manifest.
Check the actual runtime before allowing automatic background generation:

- C: optional `infernal_live_budget_version()` returns `1`; a missing symbol is unsupported
- Godot wrapper: `supports_live_generation_budget()` checks both the shim method and
  the selected core's version marker plus prompt/hybrid compact entry points
- Raw `InfernalGenerator`: `supports_live_generation_budget(library_dir)` takes the
  same absolute library directory as generation
- Web client: `await generator.ready()` returns `liveGenerationBudgetVersion: 1`
  from actual WASM `generator_info()`; the binary also exports
  `live_generation_budget_version()`. Missing/zero means unsupported

`build-info.json` includes the marker for provenance, but it is not a substitute
for the runtime handshake. Older cores retain their manual/legacy arena API
compatibility; a background gameplay scheduler should use prebaked fallbacks
when the capability is absent.
