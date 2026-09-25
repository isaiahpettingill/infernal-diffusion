## Example arena director. Connect boss_speaks to a throne NPC's dialogue bubble,
## and enemy_ready to the game's monster spawner. Call enemy_defeated() when
## that wave's enemy and any required minions are gone.
extends Node
const Infernal = preload("res://addons/infernal_diffusion/infernal_diffusion.gd")

signal boss_speaks(prompt: String, difficulty: int, round: int)
signal enemy_ready(packages: Array, difficulty: int, round: int)
signal encounter_complete()
signal encounter_failed(reason: String)

@export var run_seed: int = 42
@export_range(1, 30) var wave_count: int = 9

var _generator := Infernal.new()
var _round: int = 0
var _prompt_job: int = -1
var _monster_job: int = -1
var _difficulty: int = 1
var _awaiting_defeat := false

func start_encounter() -> void:
	if _round != 0:
		return
	_round = 1
	_request_prompt()

func enemy_defeated() -> void:
	if !_awaiting_defeat:
		return
	_awaiting_defeat = false
	_round += 1
	if _round > wave_count:
		encounter_complete.emit()
	else:
		_request_prompt()

func _request_prompt() -> void:
	_prompt_job = _generator.suggest_arena_prompt_async(run_seed, _round)
	if _prompt_job < 0:
		encounter_failed.emit("Could not queue the next arena prompt")

func _process(_delta: float) -> void:
	var result: Dictionary = _generator.poll_result()
	if result.is_empty():
		return
	if !result.ok:
		encounter_failed.emit(result.error)
		return
	if result.job_id == _prompt_job:
		_prompt_job = -1
		_difficulty = result.difficulty
		# The NPC says exactly what the generator will receive.
		boss_speaks.emit(result.prompt, _difficulty, _round)
		_monster_job = _generator.generate_in_memory_async(result.prompt, result.monster_seed)
		if _monster_job < 0:
			encounter_failed.emit("Could not queue monster generation")
	elif result.job_id == _monster_job:
		var finished_job := _monster_job
		_monster_job = -1
		_awaiting_defeat = true
		enemy_ready.emit(result.packages, _difficulty, _round)
		_generator.release_in_memory(finished_job)
