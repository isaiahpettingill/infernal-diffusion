extends SceneTree

const Infernal = preload("res://addons/infernal_diffusion/infernal_diffusion.gd")
const MonsterPb = preload("res://addons/infernal_diffusion/monster_pb.gd")

var generator: InfernalDiffusion

func _initialize() -> void:
	# GDScript assertions stop their coroutine, not SceneTree; bound failure runs.
	create_timer(120.0).timeout.connect(func() -> void:
		push_error("native smoke did not complete before deadline")
		quit(1)
	)
	_run.call_deferred()

func _await_result(job: int) -> Dictionary:
	assert(job > 0, "request was rejected")
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		var result := generator.poll_result()
		if !result.is_empty():
			assert(result.job_id == job, "wrong async job ID")
			return result
		await process_frame
	assert(false, "timed out waiting for native generation")
	return {}

func _test_props() -> void:
	# Actual OS file IO, off-thread native import/render, independent prop metadata.
	var path := "user://smoke-prop.obj"
	var source := FileAccess.open(path, FileAccess.WRITE)
	assert(source != null, "cannot write prop fixture")
	source.store_string("v -1 0 -1\nv 1 0 -1\nv 0 0 1\nv 0 2 0\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n")
	source.close()
	var options := {"tile_width": 96, "tile_height": 96, "pixels_per_unit": 24.0,
		"angles_deg": [0, 45, 90, 135, 180, 225, 270, 315], "columns": 4}
	var first: Dictionary = {}
	# More than four props work without touching retained monster handles.
	for index in range(6):
		var result := await _await_result(generator.bake_prop_async(path, options))
		assert(result.ok, str(result.get("error", "prop bake failed")))
		assert(result.has("prop") and !result.has("packages"))
		assert(!result.has("monster_seed") and !result.has("difficulty"))
		var prop: Dictionary = result.prop
		assert(!prop.has("monster") and !prop.has("locomotion"))
		assert(!prop.metadata.has("behavior") and !prop.metadata.has("attacks"))
		assert(prop.metadata.frames.size() == 8)
		assert(prop.metadata.pixels_per_unit == 24.0)
		assert(prop.metadata.atlas_width == prop.sprites.width)
		assert(prop.metadata.atlas_height == prop.sprites.height)
		assert(prop.sprites.rgba.size() == prop.sprites.width * prop.sprites.height * 4)
		var image := Infernal.image_from_rgba(prop.sprites)
		assert(!image.is_empty() and image.get_used_rect().has_area())
		for frame: Dictionary in prop.metadata.frames:
			assert(frame.content_bounds[2] > 0 and frame.content_bounds[3] > 0)
			assert(frame.atlas_rect[2] == prop.metadata.frame_width)
		if first.is_empty():
			first = prop
		else:
			assert(first == prop, "prop bake is not deterministic")
	var missing := await _await_result(generator.bake_prop_async("user://missing-prop.obj"))
	assert(!missing.ok and !missing.error.is_empty() and !missing.has("prop"))
	var invalid := await _await_result(generator.bake_prop_async(path, {"angles_deg": []}))
	assert(!invalid.ok and !invalid.error.is_empty())
	var unknown := await _await_result(generator.bake_prop_async(path, {"typo_setting": 1}))
	assert(!unknown.ok and unknown.error.contains("unknown field"))
	var recovered := await _await_result(generator.bake_prop_async(path))
	assert(recovered.ok and recovered.prop.metadata.frames.size() == 4)
	# Optional separately licensed online asset; keep dependencies beside its OBJ.
	var external_asset := OS.get_environment("INFERNAL_PROP_SMOKE_ASSET")
	if !external_asset.is_empty():
		var imported := await _await_result(generator.bake_prop_async(external_asset, {"framing": "fit", "pixels_per_unit": 256.0}))
		assert(imported.ok, str(imported.get("error", "external prop failed")))
		assert(imported.prop.metadata.frames.size() == 4)
		var imported_image := Infernal.image_from_rgba(imported.prop.sprites)
		assert(imported_image.get_used_rect().has_area())
		var output := OS.get_environment("INFERNAL_PROP_SMOKE_OUTPUT")
		if !output.is_empty():
			assert(DirAccess.make_dir_recursive_absolute(output) == OK)
			assert(imported_image.save_png(output.path_join("sprites.png")) == OK)
			var metadata := FileAccess.open(output.path_join("prop.json"), FileAccess.WRITE)
			assert(metadata != null)
			metadata.store_string(JSON.stringify(imported.prop.metadata, "  "))
			metadata.close()
		print("INFERNAL_GDEXTENSION_EXTERNAL_PROP_OK: ", external_asset)

func _run() -> void:
	generator = Infernal.new()
	assert(generator.generate_in_memory_async("wolf", -1) == -1)
	# Descriptor projection accepts old records and never copies AI or attack data.
	var legacy := Infernal.locomotion_profile({
		"movement_modes": [{"id": "WALK", "speed": 20.0, "animation_id": "move"}],
		"behavior": {"approach_mode": "WALK"},
		"attacks": [{"steps": [{"target": "PLAYER"}]}],
	})
	assert(legacy.modes[0].category == "UNKNOWN")
	assert(!legacy.has("behavior") and !legacy.has("attacks"))
	assert(legacy.pixels_per_unit == 0.0)
	var job := generator.generate_in_memory_async("wolf", 42)
	var result := await _await_result(job)
	assert(result.ok, str(result.get("error", "generation failed")))
	assert(!result.packages.is_empty())
	var root: Dictionary = result.packages.back()
	var profile: Dictionary = root.locomotion
	assert(profile.contract_version == 1)
	assert(profile.distance_unit == "GENERATOR_UNIT")
	assert(!profile.modes.is_empty() and !profile.clips.is_empty())
	var clip_ids: Array[String] = []
	for clip: Dictionary in profile.clips:
		clip_ids.append(clip.id)
		assert(!clip.semantic_state.contains("ATTACK"))
		for frame: Dictionary in clip.frames:
			assert(frame.duration_ms > 0)
			assert(!frame.has("root_dx") and frame.has("reference_root_dx"))
	for mode: Dictionary in profile.modes:
		assert(!mode.category.is_empty() and mode.category != "UNKNOWN")
		assert(mode.animation_id in clip_ids)
		assert(!mode.has("target") and !mode.has("approach_mode"))
	var image := Infernal.image_from_rgba(root.sprites)
	assert(image.get_width() == root.sprites.width and image.get_height() == root.sprites.height)
	assert(!image.is_empty())
	await _test_props()
	var path := "user://smoke/wolf"
	var saved := await _await_result(generator.save_in_memory_async(job, path))
	assert(saved.ok, str(saved.get("error", "save failed")))
	var monster := MonsterPb.InfernalMonster.new()
	assert(monster.from_bytes(FileAccess.get_file_as_bytes(path.path_join("monster.pb"))) == MonsterPb.PB_ERR.NO_ERRORS)
	assert(monster.get_movement_modes()[0].get_category() == root.monster.movement_modes[0].category)
	assert(monster.get_movement_modes()[0].get_modifiers() == root.monster.movement_modes[0].modifiers)
	var cbor := await _await_result(generator.save_in_memory_async_format(job, path, Infernal.FORMAT_CBOR))
	assert(cbor.ok, str(cbor.get("error", "CBOR save failed")))
	assert(FileAccess.file_exists(path.path_join("monster.cbor")))
	assert(!FileAccess.file_exists(path.path_join("monster.pb")))
	assert(generator.release_in_memory(job))
	# Give the bounded worker queue time to consume release before another request.
	var stale_save := -1
	while stale_save == -1:
		await process_frame
		stale_save = generator.save_in_memory_async(job, path)
	var stale := await _await_result(stale_save)
	assert(!stale.ok and stale.error.contains("not found"))
	# Recover after the expected stale-object error and verify deterministic replay.
	var replay_job := generator.generate_in_memory_async("wolf", 42)
	var replay := await _await_result(replay_job)
	assert(replay.ok, str(replay.get("error", "replay failed")))
	var replay_root: Dictionary = replay.packages.back()
	assert(replay_root.monster == root.monster)
	assert(replay_root.locomotion == root.locomotion)
	assert(replay_root.sprites.rgba == root.sprites.rgba)
	assert(replay_root.emission.rgba == root.emission.rgba)
	assert(generator.release_in_memory(replay_job))
	# Repeated requests must apply backpressure and retain accepted FIFO job IDs.
	var accepted: Array[int] = []
	var first := -1
	while first == -1:
		first = generator.suggest_prompt_async(100, 1)
		await process_frame
	accepted.append(first)
	var rejected := false
	for index in range(16):
		var queued := generator.suggest_prompt_async(101 + index, 1)
		if queued == -1:
			rejected = true
		else:
			accepted.append(queued)
	assert(rejected, "bounded queue accepted an unbounded burst")
	for queued in accepted:
		var suggestion := await _await_result(queued)
		assert(suggestion.ok and !suggestion.prompt.is_empty())
	print("INFERNAL_GDEXTENSION_SMOKE_OK")
	quit(0)
