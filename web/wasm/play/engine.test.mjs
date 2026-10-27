import test from 'node:test';
import assert from 'node:assert/strict';
import { animationBindings, createSession, exercise, tick, frameAt, audioRecipe, duration, FLOOR } from './engine.mjs';

const frame = (sprite_frame_id, duration_ms) => ({ sprite_frame_id, duration_ms });
function fixture({ projectile = false, steps, looped = false } = {}) {
  return { monster: {
    id: 'monster-fixture', tags: [],
    sprites: { frame_width: 96, frame_height: 96, direction_angles_deg: [0, 90, 180, 270] },
    size: { world_scale: 1, pixels_per_unit: 1, anchor_x_px: 48, anchor_y_px: 90, target_height_px: 90 },
    animations: [
      { id: 'idle', semantic_state: 'IDLE', looped: true, frames: [frame(4, 100)] },
      { id: 'attack', semantic_state: 'ATTACK', looped, frames: [frame(32, 100), frame(65, 200)] },
      { id: 'hurt', semantic_state: 'HURT', looped: false, frames: [frame(127, 100)] },
      { id: 'death', semantic_state: 'DEATH', looped: false, frames: [frame(511, 200)] },
    ],
    attacks: [{ id: 'claw', animation_id: 'attack', delivery: projectile ? 'PROJECTILE' : 'MELEE', damage: 17,
      cooldown_ms: 400, range: 30, telegraph_ms: 100,
      steps: steps ?? [{ primitive: projectile ? 'SPAWN_PROJECTILE' : 'SWIPE', at_ms: 100, radius: projectile ? 3 : 30, speed: 300 }] }],
  } };
}
function advance(s, seconds, input = {}) {
  for (let left = seconds; left > 1e-9; left -= .01) tick(s, Math.min(.01, left), input);
}

test('animation bindings are stable, deterministic and include sparse arbitrary IDs', () => {
  const clips = Array.from({ length: 100 }, (_, n) => ({ id: `clip-${String(n * 13).padStart(4, '0')}` }));
  const expected = animationBindings(clips);
  assert.deepEqual(animationBindings([...clips].reverse()), expected);
  assert.equal(new Set(expected.map(b => `${b.page}:${b.code}:${b.shift}:${b.alt}`)).size, clips.length);
  assert.deepEqual(new Set(expected.map(b => b.id)), new Set(clips.map(c => c.id)));
  assert.ok(expected.every(b => b.label && b.code));
});

test('bindings distinguish pages even for unusually large animation sets', () => {
  const bindings = animationBindings(Array.from({ length: 300 }, (_, n) => ({ id: `clip-${n}` })));
  assert.equal(new Set(bindings.map(b => `${b.page}:${b.code}:${b.shift}:${b.alt}`)).size, 300);
});

test('frame timing respects duration, looping and sparse sprite IDs', () => {
  const clip = fixture().monster.animations[1];
  assert.equal(duration(clip), 300);
  assert.equal(frameAt(clip, 0), 32);
  assert.equal(frameAt(clip, 99), 32);
  assert.equal(frameAt(clip, 100), 65);
  assert.equal(frameAt(clip, 800), 65);
  assert.equal(frameAt({ ...clip, looped: true }, 300), 32);
  assert.equal(frameAt(null, 100), 0);
});

test('session validation rejects missing animation data', () => {
  assert.throws(() => createSession({ monster: { sprites: {} } }), /animation/);
  assert.throws(() => createSession({ monster: { animations: [] } }), /animation/);
});

test('every noncombat animation can be exercised without damaging dummy', () => {
  const s = createSession(fixture());
  for (const id of ['idle', 'hurt', 'death']) {
    assert.equal(exercise(s, id), true);
    assert.equal(s.clip.id, id);
    advance(s, .4);
  }
  assert.equal(s.dummy.hp, 1000);
  assert.equal(exercise(s, 'not-an-animation'), false);
});

test('melee only damages at its active step and in collision range', () => {
  const s = createSession(fixture());
  s.dummy.x = s.x + 40;
  assert.equal(exercise(s, 'attack'), true);
  assert.equal(s.dummy.hp, 1000, 'pressing animation key is not itself damage');
  advance(s, .09);
  assert.equal(s.dummy.hp, 1000);
  advance(s, .02);
  assert.equal(s.dummy.hp, 983);
  advance(s, 1);
  assert.equal(s.dummy.hp, 983, 'holding the animation does not repeat a one-shot hit');
  assert.equal(s.dummy.total, 17);
  assert.equal(s.events.filter(e => e.kind === 'hit').length, 1);
});

test('far-away attacks and attacks facing away miss', () => {
  for (const [distance, facing] of [[400, 1], [40, -1]]) {
    const s = createSession(fixture());
    s.dummy.x = s.x + distance;
    s.facing = facing;
    exercise(s, 'attack');
    advance(s, .4);
    assert.equal(s.dummy.hp, 1000);
  }
});

test('multiple melee steps do not multiply a single attack damage', () => {
  const s = createSession(fixture({ steps: [
    { primitive: 'SWIPE', at_ms: 100, radius: 40 },
    { primitive: 'THRUST', at_ms: 150, radius: 40 },
  ] }));
  s.dummy.x = s.x + 30;
  exercise(s, 'attack');
  advance(s, .4);
  assert.equal(s.dummy.total, 17);
});

test('attack cooldown rejects retriggering without resetting animation', () => {
  const s = createSession(fixture());
  exercise(s, 'attack');
  advance(s, .15);
  const clock = s.clock;
  assert.equal(exercise(s, 'attack'), false);
  assert.equal(s.clock, clock);
  advance(s, .3);
  assert.equal(exercise(s, 'attack'), true);
  assert.equal(s.clock, 0);
});

test('projectiles deal damage only after travelling into dummy', () => {
  const s = createSession(fixture({ projectile: true }));
  s.dummy.x = s.x + 180;
  exercise(s, 'attack');
  advance(s, .11);
  assert.equal(s.shots.length, 1);
  assert.equal(s.dummy.hp, 1000);
  advance(s, .7);
  assert.equal(s.dummy.hp, 983);
  assert.equal(s.shots.length, 0);
  advance(s, .5);
  assert.equal(s.dummy.total, 17);
});

test('projectiles fired away never subtract target health', () => {
  const s = createSession(fixture({ projectile: true }));
  s.dummy.x = s.x + 100;
  s.facing = -1;
  exercise(s, 'attack');
  advance(s, 3.5);
  assert.equal(s.dummy.hp, 1000);
  assert.equal(s.shots.length, 0);
});

test('movement and jump exercise the actor, and anchored tags prevent translation', () => {
  const s = createSession(fixture());
  const start = s.x;
  advance(s, .2, { move: 1 });
  assert.ok(s.x > start);
  tick(s, .01, { jump: true });
  assert.ok(s.y < FLOOR);
  advance(s, 1);
  assert.equal(s.y, FLOOR);
  s.monster.tags.push('ANCHORED');
  const anchoredX = s.x;
  advance(s, .2, { move: -1 });
  assert.equal(s.x, anchoredX);
});

test('audio declaration has separate stable seed domain and does not mutate visual metadata', () => {
  const m = fixture().monster;
  const original = structuredClone(m);
  const a = audioRecipe(m, '9007199254740993', 'v1');
  assert.deepEqual(a, audioRecipe(m, '9007199254740993', 'v1'));
  assert.match(a.seed, /^audio-v1:9007199254740993:/);
  assert.equal(a.voice, null, 'unprovided sounds must not be represented as completed synthesis');
  assert.equal(a.death.variation, 'fallback-common');
  assert.equal(audioRecipe(m, '9007199254740993', 'v2').death.variation, 'transform-usually');
  assert.deepEqual(m, original);
});
