import { MonsterGenerator } from '../client.js';
const output = document.querySelector('#result');
let ticks = 0;
setInterval(() => document.querySelector('#ticks').textContent = String(++ticks), 25);
const assert = (condition, message) => { if (!condition) throw new Error(message); };
const json = (value) => JSON.stringify(value, (_, v) => typeof v === 'bigint' ? v.toString() : v);
const hash = async (bytes) => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).map(v => v.toString(16).padStart(2, '0')).join('');
async function summarize(result) {
  return Promise.all(result.packages.map(async pkg => ({ path: pkg.path, metadata: json(pkg.monster), atlases: await Promise.all([pkg.sprites, pkg.emission, pkg.projectiles].map(async atlas => {
    if (!atlas) return null;
    assert(atlas.rgba instanceof Uint8Array, 'RGBA must be Uint8Array');
    assert(atlas.rgba.length === atlas.width * atlas.height * 4, 'RGBA dimensions');
    return { width: atlas.width, height: atlas.height, sha256: await hash(atlas.rgba) };
  })) })));
}
document.querySelector('#run').onclick = async () => {
  const button = document.querySelector('#run');
  button.disabled = true;
  const log = { status: 'running', checks: [], userAgent: navigator.userAgent, isolated: crossOriginIsolated };
  const update = () => output.textContent = JSON.stringify(log, null, 2);
  const g = new MonsterGenerator();
  try {
    assert(crossOriginIsolated, 'Cross-origin isolation required');
    update();
    const ready = await g.ready();
    assert(ready.threadCount >= 2 && ready.threadCount <= 4, 'Real pool initialized');
    log.threadCount = ready.threadCount;
    log.checks.push('real WASM and thread pool ready'); update();
    const start = ticks;
    const a = await g.generate('stormtrooper with a laser blaster', 42n);
    log.heartbeatTicksDuringGeneration = ticks - start;
    assert(ticks > start, 'UI heartbeat responsive');
    const monster = a.packages[0].monster;
    assert(json(monster).includes('RANDOM_DIRECTION'), 'Stormtrooper RANDOM_DIRECTION metadata');
    for (const key of ['physics','gameplay','size','animations','attacks','movement_modes','colliders','projectiles']) assert(key in monster, `Metadata field ${key}`);
    const first = await summarize(a);
    log.checks.push('stormtrooper complete metadata, RANDOM_DIRECTION, typed atlases, responsive heartbeat'); update();
    const b = await g.generate('stormtrooper with a laser blaster', 42n);
    assert(json(first) === json(await summarize(b)), 'Same prompt and seed deterministic');
    log.atlases = first.map(pkg => pkg.atlases);
    log.checks.push('deterministic metadata and all atlas SHA-256 hashes'); update();
    const controller = new AbortController();
    const error = async promise => { try { await promise; return null; } catch(e) { return e.name; } };
    const cancelled = error(g.generate('fire dragon', 7n, { signal: controller.signal }));
    const queued = error(g.generate('ice slime', 8n));
    await new Promise(resolve => setTimeout(resolve, 25));
    controller.abort();
    assert(json(await Promise.all([cancelled,queued])) === '["AbortError","AbortError"]', 'Cancel all pending requests');
    log.checks.push('busy generation and queued request cancelled'); update();
    const restarted = await g.generate('small ice slime', 9n);
    assert(restarted.packages[0].monster.generation.prompt === 'small ice slime', 'Fresh worker/pool after cancellation');
    log.checks.push('fresh worker and pool generate successfully after cancellation');
    assert(await error(g.generate('',1n)) === 'TypeError', 'Invalid prompt rejected');
    assert(await error(g.generate('slime',-1n)) === 'RangeError', 'Invalid seed rejected');
    log.checks.push('invalid prompt/seed errors');
    log.status = 'PASS';
  } catch(e) { log.status = 'FAIL'; log.error = { name: e.name, message: e.message, stack: e.stack }; }
  finally { g.dispose(); update(); button.disabled = false; }
};
