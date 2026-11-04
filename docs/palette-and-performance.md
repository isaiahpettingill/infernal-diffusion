# Atlas storage and in-game memory

Normal generated sprites, emission maps, projectile atlases, saved in-memory
packages, and imported props use **lossless indexed PNG** when they fit in 256
RGBA colors. The renderer already uses a limited artistic palette; the exporter
stores that palette instead of writing four channels per pixel. It does not do
another nearest-color pass, change alpha, or discard RGB under transparent pixels.
Images exceeding the palette limit fall back to full RGBA, never lossy reduction.
The writer streams one row of indices instead of allocating another full atlas.

`generate-marked` explicitly writes full RGBA PNGs. Do not palette-reduce or
repack a marked sheet afterward; the silly download/upload detector needs the
exported RGB bytes. For Rust callers saving watermarked output, use
`save_generated_with_format_and_encoding(..., atlas_png::Encoding::Rgba)`.
Ordinary `save_generated` and `save_generated_with_format` default to palette
storage. This is an additive API; existing C, Godot, and WASM buffers stay RGBA8.

## What saves memory

Indexed PNGs save **file/transfer storage**, not runtime texture RAM. The current
in-memory C/Rust/Godot/WASM contract still holds four RGBA bytes per pixel, and
loading a palette PNG into an RGBA texture expands it. No runtime palette shader
or changed pixel-pointer representation is implied.

Actual temporary-memory changes are separate:

- The renderer allocates high-resolution color/depth buffers per simultaneous
  worker, not per camera direction. Native rendering normally uses two workers;
  serial WASM uses one. Workers claim the next direction from a shared queue so
  scratch reuse does not require waiting at fixed direction-batch barriers.
- Compact atlas deduplication retains cropped pixels once, rather than also
  copying them into map keys. Hashes select candidates; dimensions and all RGBA
  bytes are compared before reuse, so hash collisions cannot change artwork.
- Triangle scanlines reuse bounded color/depth slices and compute their shading
  multiplier once rather than repeating the work for each channel.

Default in-memory atlas dimensions and retained RGBA sizes do not change. The
existing opt-in compact atlas API still reduces retained textures by repacking
frames; it is not enabled silently for consumers that expect dense frame layout.
See `BENCHMARKS.md` for measured wall time, peak RSS, and retained RGBA bytes.

## Measured PNG storage

Seed-42 normal sprite atlases retained identical decoded RGBA bytes:

| Sample | Previous RGBA PNG | Indexed PNG | Reduction |
|---|---:|---:|---:|
| V1 wolf | 1,772,972 bytes | 251,164 bytes | 85.8% |
| V2 giant spider | 2,615,849 bytes | 319,947 bytes | 87.8% |
| V2 ghost | 1,813,129 bytes | 294,272 bytes | 83.8% |

These measurements compare the previous default fast RGBA encoder with the new
indexed PNG writer using balanced compression and no row filter; they are not an
isolated comparison of compression algorithms. In-memory benchmarks exclude PNG encoding.

Emission and projectile PNGs were also compared pixel-for-pixel. Full details are
in [the storage measurements](benchmarks/palette-storage.json). Preview images
are not atlases and keep their existing encoding. These are **PNG file savings**,
not reductions in retained RGBA texture memory.
