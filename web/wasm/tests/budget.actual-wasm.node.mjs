/** Real portable WASM/BERT admission and losslessness regression (no renderer mocks).
 * Run after tools/build_wasm.sh. Node hosts the actual exported WASM API; browser
 * worker/deployment coverage remains in the separate browser integration tests.
 */
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import * as api from '../pkg/infernal_diffusion.js';

await api.default({ module_or_path: await readFile(new URL('../pkg/infernal_diffusion_bg.wasm', import.meta.url)) });
assert.equal(api.live_generation_budget_version(), 1);
assert.equal(api.generator_info().liveGenerationBudgetVersion, 1);
const buildInfo = JSON.parse(await readFile(new URL('../pkg/build-info.json', import.meta.url), 'utf8'));
assert.equal(buildInfo.liveGenerationBudgetVersion, 1);
assert.equal(api.generator_info().threaded, false, 'Run this Node smoke against the portable build');
const phases = [];
assert.throws(() => api.generate_monster_compact_progress(
  'orc riding an orc riding an orc riding a mammoth riding a mammoth',
  42n, 'v2', phase => phases.push(phase)), /generation budget exceeded: mount depth/);
assert.deepEqual(phases, [], 'Reject before BERT parsing or rendering begins');

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const metadata = m => {
  const { sprites, format_version, ...rest } = m;
  return JSON.stringify(rest, (_, v) => typeof v === 'bigint' ? v.toString() : v);
};
function expand(p, emission, width, height) {
  const s = p.monster.sprites;
  const output = new Uint8Array(width * height * 4);
  if (emission && !s.emission_file) return output;
  const source = emission ? p.emission : p.sprites;
  for (const [id, r] of s.frame_regions.entries()) for (let y = 0; y < r.height; y++) {
    const from = ((r.y + y) * source.width + r.x) * 4;
    const to = ((Math.floor(id / s.columns) * s.frame_height + r.offset_y + y) * width
      + id % s.columns * s.frame_width + r.offset_x) * 4;
    output.set(source.rgba.subarray(from, from + r.width * 4), to);
  }
  return output;
}
for (const [prompt, count] of [
  ['small ice slime', 1],
  ['goblin riding a wolf; mount survives', 3],
  ['spider that summons spiders', 2],
]) {
  let dense = api.generate_monster_with_profile(prompt, 42n, 'v2');
  const expected = dense.packages.map(p => ({ path: p.path, metadata: metadata(p.monster),
    width: p.sprites.width, height: p.sprites.height, color: hash(p.sprites.rgba),
    emission: hash(p.emission.rgba), projectiles: p.projectiles ? hash(p.projectiles.rgba) : null }));
  dense = null;
  const result = api.generate_monster_compact_progress(prompt, 42n, 'v2', () => {});
  assert.equal(result.packages.length, count, prompt);
  for (const [i, p] of result.packages.entries()) {
    const e = expected[i];
    assert.equal(p.path, e.path);
    if (p.path === '.') assert.equal(p.monster.generation.parser, 'candle-bert-prototypes-v2');
    assert.equal(metadata(p.monster), e.metadata, 'Preserve every attack, special, animation frame, event and companion');
    assert.equal(hash(expand(p, false, e.width, e.height)), e.color, 'Preserve every sprite pixel');
    assert.equal(hash(expand(p, true, e.width, e.height)), e.emission, 'Preserve every emission pixel');
    assert.equal(p.projectiles ? hash(p.projectiles.rgba) : null, e.projectiles);
  }
  console.log(JSON.stringify({ prompt, packages: count, allMetadataAndPixelsPreserved: true }));
}
console.log('Real WASM/BERT budget rejection, recovery, ordinary monster and companions passed');
