import './prompt-controls.js';
import { MonsterGenerator } from './client.js';
const generator = new MonsterGenerator();
const $ = (id) => document.getElementById(id);
let ticks = 0;
setInterval(() => { $('heartbeat').textContent = String(++ticks); }, 100);
let busy = false;
let initialized = false;
const promptControls = new InfernalPromptControls({
  prompt: $('prompt'), seed: $('seed'), difficulty: $('difficulty'), profile: $('prompt-profile'),
  button: $('random-prompt'), status: $('suggestion-status'), disabled: true,
  suggest: (seed, difficulty, promptProfile) => generator.suggestPrompt(seed, difficulty, { promptProfile }),
});
function setBusy(value) {
  busy = value;
  $('generate').disabled = value || !initialized;
  $('cancel').disabled = !value;
  promptControls.setDisabled(value || !initialized);
}
function atlasView(label, atlas) {
  if (!atlas) return document.createTextNode('');
  const section = document.createElement('section');
  const heading = document.createElement('h3');
  heading.textContent = `${label} (${atlas.width} × ${atlas.height})`;
  const canvas = document.createElement('canvas');
  canvas.width = atlas.width;
  canvas.height = atlas.height;
  const pixels = new Uint8ClampedArray(atlas.rgba.buffer, atlas.rgba.byteOffset, atlas.rgba.byteLength);
  canvas.getContext('2d').putImageData(new ImageData(pixels, atlas.width, atlas.height), 0, 0);
  section.append(heading, canvas);
  return section;
}
function show(result) {
  $('results').replaceChildren();
  for (const pkg of result.packages) {
    const article = document.createElement('article');
    const heading = document.createElement('h2');
    heading.textContent = `${pkg.monster.display_name} · ${pkg.path || 'root package'}`;
    const metadata = document.createElement('details');
    const summary = document.createElement('summary');
    summary.textContent = 'Complete monster metadata';
    const pre = document.createElement('pre');
    pre.textContent = JSON.stringify(pkg.monster, (_, value) => typeof value === 'bigint' ? value.toString() : value, 2);
    metadata.append(summary, pre);
    const size = pkg.monster.size;
    const worldSize = document.createElement('p');
    worldSize.textContent = Number.isFinite(size?.world_scale) && size.world_scale > 0
      ? `World body: ${Math.round(size.target_height_px)} px · player reference: ${size.reference_player_height_px} px · baked-pixel scale: ${size.world_scale.toFixed(3)}×. Atlases below show render resolution.`
      : 'Legacy package: no world scale metadata. Atlas dimensions describe render resolution.';
    article.append(heading, worldSize, atlasView('Sprites', pkg.sprites), atlasView('Emission', pkg.emission), atlasView('Projectiles', pkg.projectiles), metadata);
    $('results').append(article);
  }
}
$('form').addEventListener('submit', async (event) => {
  event.preventDefault();
  if (busy || !initialized) return;
  setBusy(true);
  $('status').textContent = 'Generating in worker… The heartbeat stays responsive.';
  const started = performance.now();
  try {
    const result = await generator.generate($('prompt').value, $('seed').value, { profile: $('profile').value, onProgress: progress => { $('status').textContent = `${progress.phase}${progress.total > 1 ? ` ${progress.completed}/${progress.total}` : ''}… The heartbeat stays responsive.`; } });
    show(result);
    $('status').textContent = `Generated ${result.packages.length} package(s) in ${((performance.now() - started) / 1000).toFixed(2)}s`;
  } catch (error) {
    $('status').textContent = error.name === 'AbortError' ? 'Cancelled. Generate again to start a fresh worker.' : error.message;
  } finally { setBusy(false); }
});
document.querySelectorAll('[data-prompt]').forEach(button => {
  button.addEventListener('click', () => { promptControls.invalidate('Example selected.'); $('prompt').value = button.dataset.prompt; });
});
$('cancel').addEventListener('click', () => generator.cancel());
window.addEventListener('pagehide', event => {
  if (event.persisted) { promptControls.invalidate(); generator.cancel(); }
  else { promptControls.dispose(); generator.dispose(); }
});
try {
  const { threadCount, model } = await generator.ready({ onProgress: progress => { $('status').textContent = `${progress.phase}…`; } });
  $('status').textContent = 'Ready';
  $('status').title = `${model} ready in ${threadCount} worker(s)`;
  initialized = true;
  setBusy(false);
}
catch (error) { $('status').textContent = error.message; }
