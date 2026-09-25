#ifndef INFERNAL_DIFFUSION_H
#define INFERNAL_DIFFUSION_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* ABI version 1. All returned strings are UTF-8 and freed with infernal_free_string. */
uint32_t infernal_abi_version(void);
/* Selected CPU raster kernel: x64, x64_v2, x64_v3, x86, or arm64.
   The returned static string is owned by the library. */
const char *infernal_cpu_kernel_name(void);
/* Deterministic, recipe-aware NPC speech. Difficulty is 1..3.
   The returned string is freed with infernal_free_string. */
char *infernal_random_prompt(uint64_t seed, uint32_t difficulty, char **error_out);
/* One-based round; difficulty rises after rounds 3 and 6. The monster seed
   is safe to pass to Godot's signed 64-bit generation API. */
char *infernal_arena_prompt(uint64_t run_seed, uint32_t round,
                            uint32_t *difficulty_out, uint64_t *monster_seed_out,
                            char **error_out);
/* Bakes the default four-angle 3D atlas and portable package. Ranged attacks
   also write projectiles.png. Returns 0 on success. */
int32_t infernal_generate(const char *prompt, uint64_t seed,
                          const char *output_dir, char **error_out);
/* Explicit legacy single-view 2D bake. */
int32_t infernal_generate_2d(const char *prompt, uint64_t seed,
                             const char *output_dir, char **error_out);
/* Bakes a four-angle 3D mesh atlas using the same runtime package format. */
int32_t infernal_generate_3d(const char *prompt, uint64_t seed,
                             const char *output_dir, char **error_out);
/* Same as infernal_generate, with an optional directory containing
   config.json, tokenizer.json, and model.safetensors for Candle BERT.
   Requires a library built with the `bert` feature. */
int32_t infernal_generate_with_model(const char *prompt, uint64_t seed,
                                     const char *output_dir,
                                     const char *model_dir, char **error_out);
/* Accepts a UTF-8 JSON MonsterSpec, bypassing natural-language parsing. */
int32_t infernal_generate_spec_json(const char *spec_json, uint64_t seed,
                                   const char *output_dir, char **error_out);
/* Reads and validates monster.pb, sprites.png, and emission.png when present. */
int32_t infernal_validate_package(const char *package_dir, char **error_out);

/* In-memory generation avoids protobuf and PNG encode/decode until saved.
   The handle and its borrowed pixels remain valid until infernal_object_free. */
typedef struct InfernalGeneratedMonster InfernalGeneratedMonster;
InfernalGeneratedMonster *infernal_generate_object(const char *prompt, uint64_t seed,
                                                    char **error_out);
void infernal_object_free(InfernalGeneratedMonster *handle);
uint32_t infernal_object_count(const InfernalGeneratedMonster *handle);
/* Returns an allocated UTF-8 path; free with infernal_free_string. */
char *infernal_object_package_path(const InfernalGeneratedMonster *handle, uint32_t index);
/* kind: 0 sprite RGBA, 1 emission RGBA, 2 projectile RGBA.
   Pixels are tightly packed width * height * 4 bytes. Returns 0 on success. */
int32_t infernal_object_pixels(const InfernalGeneratedMonster *handle, uint32_t index,
                              uint32_t kind, const uint8_t **pixels_out, size_t *len_out,
                              uint32_t *width_out, uint32_t *height_out);
/* Visitor events: 0 null, 1 object start, 2 object end, 3 array start,
   4 array end, 5 string, 6 integer, 7 float, 8 boolean.
   key and string pointers are borrowed for the callback only. */
typedef void (*InfernalObjectVisitor)(void *context, uint32_t kind, const char *key,
                                      const char *string, int64_t integer, double real);
int32_t infernal_object_visit(const InfernalGeneratedMonster *handle, uint32_t index,
                             void *context, InfernalObjectVisitor callback);
int32_t infernal_object_save(const InfernalGeneratedMonster *handle,
                            const char *output_dir, char **error_out);
void infernal_free_string(char *value);
#ifdef __cplusplus
}
#endif
#endif
