"""Render exported, exact intermediate baker triangles in Blender.
blender -b --python tools/inspect_intermediate_blender.py -- input.json output.png [output.blend]
No generated/reconstructed replacement geometry is used for the monster.
"""
import bpy, json, math, sys
from pathlib import Path
from mathutils import Vector
args=sys.argv[sys.argv.index('--')+1:]
data=json.loads(Path(args[0]).read_text())
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
verts=[]; faces=[]; colors=[]
for tri in data['triangles']:
    start=len(verts)
    # Baker: X forward, Y up, Z lateral. Blender: Z up.
    verts += [(v[0],v[2],v[1]) for v in tri['vertices']]
    faces.append((start,start+1,start+2)); colors.append(tuple(tri['color']))
mesh=bpy.data.meshes.new('Exact baker triangles'); mesh.from_pydata(verts, [], faces); mesh.update()
obj=bpy.data.objects.new(data['affinity'],mesh); bpy.context.collection.objects.link(obj)
palette={}
for color in colors:
    if color in palette: continue
    mat=bpy.data.materials.new('rgba_'+'_'.join(map(str,color)))
    mat.diffuse_color=tuple((c/255)**2.2 for c in color[:3])+(color[3]/255,)
    mat.use_nodes=True
    bsdf=mat.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Base Color'].default_value=mat.diffuse_color; bsdf.inputs['Roughness'].default_value=0.67
    bsdf.inputs['Alpha'].default_value=color[3]/255
    palette[color]=len(mesh.materials); mesh.materials.append(mat)
for polygon,color in zip(mesh.polygons,colors): polygon.material_index=palette[color]
low=min(v[2] for v in verts)
bpy.ops.mesh.primitive_plane_add(size=220,location=(0,0,low-0.15))
floor=bpy.context.object; floor.name='Inspection ground'
mat=bpy.data.materials.new('Neutral ground');mat.diffuse_color=(0.115,0.135,0.16,1);floor.data.materials.append(mat)
target=Vector((0,0,3))
bpy.ops.object.camera_add(location=(42,-70,44))
camera=bpy.context.object;camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=66
bpy.context.scene.camera=camera
for location,power,size in [((20,-35,55),36000,35),((-30,5,35),26000,30),((15,35,45),38000,25)]:
    bpy.ops.object.light_add(type='AREA',location=location)
    light=bpy.context.object;light.data.energy=power;light.data.shape='DISK';light.data.size=size
    light.rotation_euler=(target-light.location).to_track_quat('-Z','Y').to_euler()
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=32;scene.cycles.use_denoising=False
scene.render.resolution_x=640;scene.render.resolution_y=640;scene.render.resolution_percentage=100
scene.world.color=(0.15,0.15,0.15);scene.render.image_settings.file_format='PNG'
scene.view_settings.view_transform='Standard'
scene.render.filepath=str(Path(args[1]).resolve());bpy.ops.render.render(write_still=True)
if len(args)>2: bpy.ops.wm.save_as_mainfile(filepath=str(Path(args[2]).resolve()))
