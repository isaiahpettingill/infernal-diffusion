"""Synthetic OpenGL draw/readback probe; not the production sprite renderer.

Run from the repository root after `python -m pip install --target target/glbench
moderngl==5.12.0 numpy`. This creates only a temporary hidden GL context.
"""
import sys
import time
from pathlib import Path

dependencies = Path(__file__).resolve().parents[1] / "target" / "glbench"
if dependencies.is_dir():
    sys.path.insert(0, str(dependencies))
import moderngl
import numpy as np

started = time.perf_counter()
ctx = moderngl.create_standalone_context(require=330)
program = ctx.program(
    vertex_shader="""#version 330
    in vec3 in_pos;
    in vec3 in_local;
    in float in_mat;
    out vec3 local;
    out float mat;
    uniform float phase;
    void main() {
        float c = cos(phase), s = sin(phase);
        vec2 p = vec2(in_pos.x*c-in_pos.y*s, in_pos.x*s+in_pos.y*c);
        gl_Position = vec4(p, in_pos.z, 1.0);
        local = in_local;
        mat = in_mat;
    }""",
    fragment_shader="""#version 330
    in vec3 local;
    in float mat;
    out vec4 output_color;
    float hash(vec3 p) { return fract(sin(dot(p, vec3(127.1,311.7,74.7)))*43758.5453); }
    float value_noise(vec3 p) {
        vec3 i=floor(p),f=fract(p); f=f*f*(3.0-2.0*f);
        float a=mix(hash(i),hash(i+vec3(1,0,0)),f.x);
        float b=mix(hash(i+vec3(0,1,0)),hash(i+vec3(1,1,0)),f.x);
        float c=mix(hash(i+vec3(0,0,1)),hash(i+vec3(1,0,1)),f.x);
        float d=mix(hash(i+vec3(0,1,1)),hash(i+vec3(1,1,1)),f.x);
        return mix(mix(a,b,f.y),mix(c,d,f.y),f.z);
    }
    void main() {
        float n=(value_noise(local*9.0)-0.5)*0.8+(value_noise(local*24.3)-0.5)*0.35;
        vec3 base=vec3(0.3+mat*0.35,0.24+mat*0.22,0.18+mat*0.15);
        output_color=vec4(base*clamp(0.85+n,0.25,1.35),1.0);
    }""",
)
rng = np.random.default_rng(42)
triangle_count = 1600
centers = rng.uniform(-0.42, 0.42, size=(triangle_count, 2)).astype("f4")
offsets = rng.uniform(-0.05, 0.05, size=(triangle_count, 3, 2)).astype("f4")
vertices = np.zeros((triangle_count, 3, 7), dtype="f4")
vertices[:, :, :2] = centers[:, None, :] + offsets
vertices[:, :, 2] = rng.uniform(-0.8, 0.8, size=(triangle_count, 1))
vertices[:, :, 3:6] = rng.uniform(0, 1, size=(triangle_count, 3, 3))
vertices[:, :, 6] = rng.uniform(0, 1, size=(triangle_count, 1))
buffer = ctx.buffer(vertices.tobytes())
vao = ctx.vertex_array(program, [(buffer, "3f 3f 1f", "in_pos", "in_local", "in_mat")])
targets = {}
for size in (256, 192):
    color = ctx.texture((size, size), 4)
    depth = ctx.depth_renderbuffer((size, size))
    targets[size] = ctx.framebuffer(color_attachments=[color], depth_attachment=depth)
ctx.enable(moderngl.DEPTH_TEST)
print(f"renderer={ctx.info['GL_RENDERER']!r} setup_s={time.perf_counter()-started:.3f}")

def draw(size, phase):
    target = targets[size]
    target.use()
    target.clear(0.0, 0.0, 0.0, 0.0)
    program["phase"].value = phase
    vao.render(moderngl.TRIANGLES)
    return target.read(components=4, alignment=1)

for i in range(10):
    draw(256, i*0.01)
started = time.perf_counter()
bytes_read = 0
for size, frames in ((256, 696), (192, 320)):
    for index in range(frames):
        bytes_read += len(draw(size, index*0.01))
print(f"draw_readback_s={time.perf_counter()-started:.3f} frames=1016 bytes={bytes_read}")
