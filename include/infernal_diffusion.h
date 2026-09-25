#ifndef INFERNAL_DIFFUSION_H
#define INFERNAL_DIFFUSION_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* ABI version 1. All returned strings are UTF-8 and freed with infernal_free_string. */
uint32_t infernal_abi_version(void);
/* Bakes the default eight-angle 3D atlas and portable package. Ranged attacks
   also write projectiles.png. Returns 0 on success. */
int32_t infernal_generate(const char *prompt, uint64_t seed,
                          const char *output_dir, char **error_out);
/* Explicit legacy single-view 2D bake. */
int32_t infernal_generate_2d(const char *prompt, uint64_t seed,
                             const char *output_dir, char **error_out);
/* Bakes an eight-angle 3D mesh atlas using the same runtime package format. */
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
void infernal_free_string(char *value);
#ifdef __cplusplus
}
#endif
#endif
