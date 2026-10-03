class_name InfernalDiffusion
extends RefCounted

const FORMAT_PROTOBUF := 0
const FORMAT_CBOR := 1

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

func generate_async(prompt: String, seed: int, output_dir: String) -> int:
	return _generator.generate_async(prompt, seed, ProjectSettings.globalize_path(output_dir), _library_dir())

func generate_async_format(prompt: String, seed: int, output_dir: String, format: int) -> int:
	return _generator.generate_async_format(prompt, seed, ProjectSettings.globalize_path(output_dir), format, _library_dir())

func generate_in_memory_async(prompt: String, seed: int) -> int:
	return _generator.generate_in_memory_async(prompt, seed, _library_dir())

## Bake a static OBJ or prop JSON mesh. Result: {prop: {metadata, sprites}}.
## Returned bytes are owned by Godot; no release_in_memory call is needed.
## In exported games, use user:// or an exported raw file path (not a PCK-only asset).
func bake_prop_async(mesh_path: String, options: Dictionary = {}) -> int:
	return _generator.bake_prop_async(ProjectSettings.globalize_path(mesh_path), JSON.stringify(options), _library_dir())

func suggest_prompt_async(seed: int, difficulty: int) -> int:
	return _generator.suggest_prompt_async(seed, difficulty, _library_dir())

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
