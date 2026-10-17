class_name InfernalDiffusion
extends RefCounted

const FORMAT_PROTOBUF := 0
const FORMAT_CBOR := 1
const PROFILE_V1 := 1
const PROFILE_V2 := 2

var _generator: InfernalGenerator = InfernalGenerator.new()

static func _library_dir() -> String:
	var library_dir: String
	if OS.has_feature("editor"):
		library_dir = ProjectSettings.globalize_path("res://addons/infernal_diffusion/bin")
	elif OS.get_name() == "macOS":
		library_dir = OS.get_executable_path().get_base_dir().path_join("../Frameworks").simplify_path()
	else:
		library_dir = OS.get_executable_path().get_base_dir()
	return library_dir

## Background gameplay must require this handshake, not just compact symbols.
## Both an older shim and a newer shim loading an older core return false.
func supports_live_generation_budget() -> bool:
	return _generator.has_method("supports_live_generation_budget") and bool(_generator.call("supports_live_generation_budget", _library_dir()))

func generate_async(prompt: String, seed: int, output_dir: String) -> int:
	return _generator.generate_async(prompt, seed, ProjectSettings.globalize_path(output_dir), _library_dir())

func generate_async_format(prompt: String, seed: int, output_dir: String, format: int) -> int:
	return _generator.generate_async_format(prompt, seed, ProjectSettings.globalize_path(output_dir), format, _library_dir())

## Explicit profile selection; existing generation methods remain V1.
func generate_async_profile(prompt: String, seed: int, output_dir: String, profile: int, format: int = FORMAT_PROTOBUF) -> int:
	return _generator.generate_async_profile(prompt, seed, ProjectSettings.globalize_path(output_dir), format, profile, _library_dir())

func generate_in_memory_async_profile(prompt: String, seed: int, profile: int) -> int:
	return _generator.generate_in_memory_async_profile(prompt, seed, profile, _library_dir())

## Structured HybridRecipe dictionary; validation happens in the native V2 core.
## Save/release the returned in-memory object with the existing methods.
func generate_hybrid_async(recipe: Dictionary, seed: int) -> int:
	return _generator.generate_hybrid_json_async(JSON.stringify(recipe), seed, _library_dir())

func generate_in_memory_async(prompt: String, seed: int) -> int:
	return _generator.generate_in_memory_async(prompt, seed, _library_dir())

## Bake a static OBJ or prop JSON mesh. Result: {prop: {metadata, sprites}}.
## Returned bytes are owned by Godot; no release_in_memory call is needed.
## In exported games, use user:// or an exported raw file path (not a PCK-only asset).
func bake_prop_async(mesh_path: String, options: Dictionary = {}) -> int:
	return _generator.bake_prop_async(ProjectSettings.globalize_path(mesh_path), JSON.stringify(options), _library_dir())

func suggest_prompt_async(seed: int, difficulty: int) -> int:
	return _generator.suggest_prompt_async(seed, difficulty, _library_dir())

## Summoner vocabulary policy, independent of generation PROFILE_V1/PROFILE_V2.
const PROMPT_DEFAULT := 0
const PROMPT_NECROMANCER := 1
const PROMPT_SHAMAN := 2
const PROMPT_LUNATIC := 3
const PROMPT_BOSS := 4
const PROMPT_OCEAN := 5

func suggest_prompt_async_profile(seed: int, difficulty: int, prompt_profile: int) -> int:
	return _generator.suggest_prompt_async_profile(seed, difficulty, prompt_profile, _library_dir())

func suggest_arena_prompt_async(run_seed: int, round: int) -> int:
	return _generator.suggest_arena_prompt_async(run_seed, round, _library_dir())

func save_in_memory_async(object_job_id: int, output_dir: String) -> int:
	return _generator.save_in_memory_async(object_job_id, ProjectSettings.globalize_path(output_dir))

func save_in_memory_async_format(object_job_id: int, output_dir: String, format: int) -> int:
	return _generator.save_in_memory_async_format(object_job_id, ProjectSettings.globalize_path(output_dir), format)

func release_in_memory(object_job_id: int) -> bool:
	return _generator.release_in_memory(object_job_id)

static func image_from_rgba(atlas: Dictionary) -> Image:
	return Image.create_from_data(atlas.width, atlas.height, false, Image.FORMAT_RGBA8, atlas.rgba)

## Logical IDs remain unchanged in compact atlases. Margins restore the original
## virtual canvas so Sprite2D/AnimatedSprite2D centering, flipping and anchors agree.
static func frame_texture(monster: Dictionary, atlas: Texture2D, frame_id: int, direction_index: int = 0) -> AtlasTexture:
	if atlas == null or frame_id < 0 or direction_index < 0: return null
	var data: Dictionary = monster.get("sprites", {})
	var tile := Vector2i(int(data.get("frame_width", 0)), int(data.get("frame_height", 0)))
	if tile.x <= 0 or tile.y <= 0: return null
	var id := frame_id + direction_index * int(data.get("direction_stride", 0))
	var columns := maxi(1, int(data.get("columns", 1)))
	var region := Rect2i((id % columns) * tile.x, (id / columns as int) * tile.y, tile.x, tile.y)
	var offset := Vector2i.ZERO
	var regions: Array = data.get("frame_regions", [])
	if not regions.is_empty():
		if id >= regions.size(): return null
		var entry: Dictionary = regions[id]
		region = Rect2i(int(entry.get("x", -1)), int(entry.get("y", -1)), int(entry.get("width", 0)), int(entry.get("height", 0)))
		offset = Vector2i(int(entry.get("offset_x", -1)), int(entry.get("offset_y", -1)))
	elif int(monster.get("format_version", 0)) >= 8:
		return null
	if not region.has_area() or offset.x < 0 or offset.y < 0 \
			or not Rect2i(Vector2i.ZERO, Vector2i(atlas.get_width(), atlas.get_height())).encloses(region) \
			or not Rect2i(Vector2i.ZERO, tile).encloses(Rect2i(offset, region.size)):
		return null
	var texture := AtlasTexture.new()
	texture.atlas = atlas
	texture.region = Rect2(region)
	texture.margin = Rect2(Vector2(offset), Vector2(tile - region.size))
	texture.filter_clip = true
	return texture

func poll_result() -> Dictionary:
	var result := _generator.poll_result()
	for package: Dictionary in result.get("packages", []):
		# Default engine contract. Full legacy metadata remains under `monster`.
		package["locomotion"] = locomotion_profile(package.monster)
	return result

## Descriptor-first integration surface for engine-owned movement controllers.
## No targets, approach/escape policy, attack recipes, or trajectory commands.
## Speeds and root deltas are advisory generator-length units, not Godot meters.
static func locomotion_profile(monster: Dictionary) -> Dictionary:
	var modes: Array[Dictionary] = []
	var movement_clip_ids: Array[String] = []
	for source: Dictionary in monster.get("movement_modes", []):
		var category: String = source.get("category", "")
		modes.append({
			"id": source.get("id", ""),
			"category": category if !category.is_empty() else "UNKNOWN",
			"modifiers": source.get("modifiers", []).duplicate(),
			"animation_id": source.get("animation_id", ""),
			"reference_speed": source.get("speed", 0.0),
		})
		movement_clip_ids.append(source.get("animation_id", ""))
	var clips: Array[Dictionary] = []
	var movement_states := ["IDLE", "MOVE", "FAST_MOVE", "TURN", "TAKEOFF", "FLY", "FLY_TURN", "LAND_FROM_FLIGHT", "JUMP_START", "JUMP_AIR", "LAND", "CLIMB", "BURROW", "BURROWED_MOVE", "EMERGE"]
	for source: Dictionary in monster.get("animations", []):
		if source.get("id", "") not in movement_clip_ids and source.get("semantic_state", "") not in movement_states:
			continue
		var frames: Array[Dictionary] = []
		for frame: Dictionary in source.get("frames", []):
			frames.append({
				"sprite_frame_id": frame.get("sprite_frame_id", 0),
				"duration_ms": frame.get("duration_ms", 0),
				"reference_root_dx": frame.get("root_dx", 0.0),
				"reference_root_dy": frame.get("root_dy", 0.0),
			})
		var contacts: Array[Dictionary] = []
		for event: Dictionary in source.get("events", []):
			if event.get("kind", "") == "FOOT_CONTACT":
				contacts.append({"time_ms": event.get("time_ms", 0), "kind": "FOOT_CONTACT"})
		clips.append({
			"id": source.get("id", ""),
			"semantic_state": source.get("semantic_state", ""),
			"looped": source.get("looped", false),
			"frames": frames,
			"events": contacts,
		})
	var size: Dictionary = monster.get("size") if monster.get("size") is Dictionary else {}
	return {
		"contract_version": 1,
		"distance_unit": "GENERATOR_UNIT",
		"speed_unit": "GENERATOR_UNIT_PER_SECOND",
		"frame_duration_unit": "MILLISECOND",
		"pixels_per_unit": size.get("pixels_per_unit", 0.0),
		"modes": modes,
		"clips": clips,
	}
