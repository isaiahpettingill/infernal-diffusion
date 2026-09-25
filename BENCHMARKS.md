# Renderer benchmarks

The production generator uses a CPU rasterizer and performs expensive work on
the Godot extension's background worker. Its x64 library selects baseline,
v2, or v3 raster kernels at runtime. The render pool uses two threads on
systems with at least four logical cores, and one otherwise.

To benchmark generation, build the release library and run:

```text
python tools/bench_generate.py target/release/infernal_diffusion.dll --prompt "wolf" --runs 3
```

Set `INFERNAL_CPU_KERNEL` to `x64`, `x64_v2`, or `x64_v3` to compare tiers. The
override only lowers the selected tier and never enables unsupported CPU
instructions. On an Intel Core Ultra 9 285, a warmed wolf took 0.26 seconds;
a warmed mounted orc and giant spider took about 1.0–1.2 seconds. These are
local results, not estimates for a 2018 laptop. The baseline, v2, and v3
builds produced identical sprite hashes for both prompts at seed 42.

`tools/bench_gl_probe.py` is an optional synthetic OpenGL comparison. Install
its isolated development dependencies and run it with:

```text
python -m pip install --target target/glbench moderngl==5.12.0 numpy
python tools/bench_gl_probe.py
```

The probe draws 1,016 textured 3D frames and reads back 230 MB of RGBA pixels.
On this machine's Intel graphics, its draw and readback took 0.62–0.67 seconds;
setup took 0.19 seconds with warm driver caches and 3.1 seconds on the first
run. It omits anatomy, simulation, palette reduction, and package work, so
the numbers are not a direct end-to-end GPU versus CPU comparison. The CPU
path remains the default because the probe does not show a reliable complete
generation win, while a GPU path would require another renderer and context
management inside the Godot process.
