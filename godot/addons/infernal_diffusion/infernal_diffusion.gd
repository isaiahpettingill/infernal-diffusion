class_name InfernalDiffusion
extends RefCounted

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

func generate_in_memory_async(prompt: String, seed: int) -> int:
	return _generator.generate_in_memory_async(prompt, seed, _library_dir())

func suggest_prompt_async(seed: int, difficulty: int) -> int:
	return _generator.suggest_prompt_async(seed, difficulty, _library_dir())

func suggest_arena_prompt_async(run_seed: int, round: int) -> int:
	return _generator.suggest_arena_prompt_async(run_seed, round, _library_dir())

func save_in_memory_async(object_job_id: int, output_dir: String) -> int:
	return _generator.save_in_memory_async(object_job_id, ProjectSettings.globalize_path(output_dir))

func release_in_memory(object_job_id: int) -> bool:
	return _generator.release_in_memory(object_job_id)

static func image_from_rgba(atlas: Dictionary) -> Image:
	return Image.create_from_data(atlas.width, atlas.height, false, Image.FORMAT_RGBA8, atlas.rgba)

func poll_result() -> Dictionary:
	return _generator.poll_result()
