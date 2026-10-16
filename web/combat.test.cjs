const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function arena() {
  const elements = new Map();
  const drawings = [];
  const drawing = { imageSmoothingEnabled: false,
    save() {}, restore() {}, translate(...args) { drawings.push(['translate', ...args]); },
    scale(...args) { drawings.push(['scale', ...args]); },
    drawImage(...args) { drawings.push(['drawImage', ...args]); } };
  function element(id) {
    if (!elements.has(id)) elements.set(id, {
      id, textContent: '', hidden: false, value: '', options: [],
      classList: { toggle() {} }, addEventListener() {}, replaceChildren() {},
      append() {}, focus() {}, getContext() { return drawing; },
      width: 960, height: 520,
    });
    return elements.get(id);
  }
  const context = vm.createContext({
    document: { getElementById: element, querySelectorAll: () => [], createElement: () => element('new'), activeElement: { tagName: 'BODY' } },
    window: { addEventListener() {} },
    requestAnimationFrame() {},
    Image: class {},
    fetch: async () => { throw new Error('network is not used in combat tests'); },
    Math, Number, Set,
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, 'app.js'), 'utf8'), context);
  const state = vm.runInContext('state', context);
  state.monster = {
    name: 'Test Monster', health: 100, defense: 0,
    size: { visibleWidth: 40, visibleHeight: 50 },
    sprites: { angles: [0, 90, 180, 270], stride: 10, columns: 8, frameWidth: 96, frameHeight: 96 },
    attacks: [], movementModes: [{ speed: 70 }],
    animations: [{ id: 'idle', state: 'IDLE', loop: true, frames: [{ id: 0, duration: 100 }] },
      { id: 'death', state: 'DEATH', loop: false, frames: [{ id: 1, duration: 100 }] }],
    projectiles: [],
  };
  state.mode = 'fight';
  state.clip = state.monster.animations[0];
  state.foe.hp = 100;
  state.foe.attackCooldown = 99;
  return { context, state, drawings };
}

test('combat mirrors the same 0-degree side view in either direction', () => {
  const { context, state, drawings } = arena();
  assert.equal(context.combatFacing(state.monster, true).direction, 0);
  assert.equal(context.combatFacing(state.monster, true).mirror, true);
  assert.equal(context.combatFacing(state.monster, false).direction, 0);
  assert.equal(context.combatFacing(state.monster, false).mirror, false);
  state.monster.sprites.angles = [0, 45, 90, 135, 180, 225, 270, 315];
  assert.equal(context.combatFacing(state.monster, true).direction, 0);
  context.drawSprite(state.monster, {}, 2, 0, 300, 430, 1, true);
  assert.ok(drawings.some(call => call[0] === 'scale' && call[1] === -1));
  assert.ok(drawings.some(call => call[0] === 'drawImage' && call[2] === 192));
});

test('projected hurtboxes mirror around the sprite anchor', () => {
  const { context, state } = arena();
  state.monster.size.anchorX = 48;
  state.monster.size.anchorY = 55;
  state.monster.colliders = [{ views: [{ direction: 0, x: 70, y: 40, radius: 7 }] }];
  assert.equal(context.hitsHurtbox(state.monster, 300, 430, false, 344, 400), true);
  assert.equal(context.hitsHurtbox(state.monster, 300, 430, true, 256, 400), true);
  assert.equal(context.hitsHurtbox(state.monster, 300, 430, true, 344, 400), false);
});

test('spawned ranged mobs wind up and fire their own projectile', () => {
  const { context, state } = arena();
  const projectileSheet = {};
  state.companions.minion = { monster: {
    sprites: state.monster.sprites, size: { visibleWidth: 30, visibleHeight: 30 },
    health: 20, defense: 0, movementModes: [{ speed: 75 }],
    animations: [{ id: 'attack_primary', state: 'ATTACK', frames: [{ id: 2, duration: 100 }] },
      { id: 'move', state: 'MOVE', frames: [{ id: 3, duration: 100 }] }],
    attacks: [{ id: 'primary', delivery: 'PROJECTILE', damage: 5, telegraph: 200,
      cooldown: 1000, range: 190, animation: 'attack_primary', projectile: 'spit', steps: [] }],
    projectiles: [{ id: 'spit', firstFrame: 0, frameDuration: 100, frameCount: 1 }],
  }, projectileSheet };
  state.minions = [{ x: 400, y: 430, hp: 20, attackCooldown: 0, clock: 0,
    clip: null, clipClock: 0, windup: 0, attack: null, action: null, attackCursor: 0 }];
  context.updateMinions(.05);
  assert.equal(state.minions[0].clip.id, 'attack_primary');
  assert.ok(state.minions[0].windup > 0);
  context.updateMinions(.21);
  assert.equal(state.shots.length, 1);
  assert.equal(state.shots[0].sheet, projectileSheet);
});

test('hero bolt intercepts a monster projectile before it reaches the hero', () => {
  const { context, state } = arena();
  state.foe.hp = 0;
  state.shots = [{ x: 302, prevX: 302, y: 377, vx: -220, vy: 0, age: 0, attack: { damage: 12 } }];
  state.heroShots = [{ x: 254, prevX: 254, y: 377, vx: 350, vy: 0, age: 0, damage: 10 }];
  context.updateFight(.05);
  context.updateFight(.05);
  assert.equal(state.shots.length, 0);
  assert.equal(state.heroShots.length, 0);
  assert.equal(state.hero.hp, 100);
  assert.ok(state.sparks.length > 0);
});

test('a timed melee strike parries a monster projectile', () => {
  const { context, state } = arena();
  state.foe.hp = 0;
  state.hero.attackClock = .3;
  state.shots = [{ x: state.hero.x + 40, prevX: state.hero.x + 40,
    y: state.hero.y - 57, vx: -40, vy: 0, age: 0, attack: { damage: 12 } }];
  context.updateFight(.02);
  assert.equal(state.shots.length, 0);
  assert.equal(state.hero.hp, 100);
  assert.ok(state.sparks.length > 0);
});

test('the hero projectile damages a flying-height target', () => {
  const { context, state } = arena();
  state.foe.x = 320;
  state.foe.y = 350;
  state.heroShots = [{ x: 280, prevX: 280, y: 290, vx: 350, vy: 0, age: 0, damage: 10 }];
  context.updateFight(.05);
  assert.equal(state.heroShots.length, 0);
  assert.equal(state.foe.hp, 90);
});

test('swoop and leap move the monster vertically', () => {
  const { context, state } = arena();
  state.hero.x = 500;
  state.foe.x = 720;
  state.foe.y = 352;
  const attack = { damage: 12, steps: [{ kind: 'SWOOP', duration: 500 }, { kind: 'SWIPE', radius: 74 }] };
  context.startMotion(attack, attack.steps[0]);
  context.updateMotion(.25);
  assert.ok(state.foe.y > 405);
  state.foe.action = null;
  state.foe.y = 430;
  const leap = { damage: 14, steps: [{ kind: 'LEAP', duration: 600 }, { kind: 'SLAM', radius: 92 }] };
  context.startMotion(leap, leap.steps[0]);
  context.updateMotion(.3);
  assert.ok(state.foe.y < 300);
});

test('attack selection rotates through ranged and movement patterns', () => {
  const { context, state } = arena();
  state.monster.attacks = [
    { id: 'primary', delivery: 'PROJECTILE', range: 190, steps: [] },
    { id: 'secondary', delivery: 'MELEE', range: 40, steps: [] },
    { id: 'charge', delivery: 'MELEE', range: 440, steps: [{ kind: 'DASH', duration: 440 }] },
  ];
  assert.equal(context.chooseAttack(300).id, 'primary');
  assert.equal(context.chooseAttack(300).id, 'charge');
  state.foe.cooldowns.charge = 2;
  assert.equal(context.chooseAttack(300).id, 'primary');
});

test('burrow hides the monster, while blink repositions it', () => {
  const { context, state } = arena();
  state.hero.x = 490;
  state.foe.x = 720;
  const burrow = { damage: 15, steps: [{ kind: 'BURROW', duration: 600 }, { kind: 'BITE', radius: 84 }] };
  context.startMotion(burrow, burrow.steps[0]);
  context.updateMotion(.3);
  assert.equal(context.foeVulnerable(), false);
  assert.ok(state.foe.y > 500);
  state.foe.action = null;
  state.foe.y = 430;
  const blink = { damage: 12, steps: [{ kind: 'TELEPORT', duration: 400 }, { kind: 'SWIPE', radius: 67 }] };
  context.startMotion(blink, blink.steps[0]);
  context.updateMotion(.2);
  assert.ok(Math.abs(state.foe.x - state.hero.x) >= 160);
  assert.equal(context.foeVulnerable(), false);
});

test('teleport is rare and never lands on the hero, even at arena edges', () => {
  const { context, state } = arena();
  state.monster.attacks = [
    { id: 'blink', delivery: 'MELEE', range: 440, steps: [{ kind: 'TELEPORT', duration: 400 }] },
    { id: 'primary', delivery: 'PROJECTILE', range: 440, steps: [] },
  ];
  for (let attempt = 1; attempt < 12; attempt++) {
    assert.equal(context.chooseAttack(200).id, 'primary');
  }
  assert.equal(context.chooseAttack(200).id, 'blink');
  state.hero.x = 35;
  state.hero.facing = 1;
  state.foe.x = 600;
  context.startMotion(state.monster.attacks[0], state.monster.attacks[0].steps[0]);
  state.hero.x = state.foe.action.targetX;
  context.updateMotion(.2);
  assert.ok(Math.abs(state.foe.x - state.hero.x) >= 160);
});

test('damage knocks the hero away from its source', () => {
  const { context, state } = arena();
  state.foe.hp = 0;
  context.damageHero(12, state.hero.x + 50);
  const before = state.hero.x;
  context.updateFight(.1);
  assert.ok(state.hero.x < before);
  assert.ok(state.hero.y < 430);
  assert.equal(state.hero.hp, 88);
});

test('a summoned mob teleports to a safe distance', () => {
  const { context, state } = arena();
  state.hero.x = 35;
  const attack = { damage: 8, steps: [{ kind: 'TELEPORT', duration: 400 }] };
  const minion = { x: 600, y: 430, attack };
  context.useMinionAttack(minion, { monster: state.monster });
  assert.ok(Math.abs(minion.action.targetX - state.hero.x) >= 160);
});

test('anchored monsters hold position while waiting for the hero', () => {
  const { context, state } = arena();
  state.monster.tags = ['ANCHORED'];
  state.foe.x = 470;
  state.foe.attackCooldown = 0;
  context.updateFight(.2);
  assert.equal(state.foe.x, 470);
});

test('a hovering flyer stays above a grounded melee swing', () => {
  const { context, state } = arena();
  state.monster.animations.push({ id: 'fly', state: 'FLY', loop: true, frames: [{ id: 2, duration: 100 }] });
  state.foe.x = state.hero.x + 60;
  state.foe.y = 430 - 78;
  state.hero.attackClock = .23;
  context.updateFight(.02);
  assert.equal(state.foe.hp, 100);
  assert.ok(state.foe.y < 430 - 65);
});

test('a charge covers its range before the impact frame', () => {
  const { context, state } = arena();
  state.hero.x = 245;
  state.foe.x = 600;
  const charge = { damage: 16, steps: [
    { kind: 'DASH', time: 300, duration: 500 },
    { kind: 'SLAM', time: 740, radius: 68 },
  ] };
  context.startMotion(charge, charge.steps[0]);
  context.updateMotion(.45);
  assert.ok(Math.abs(state.foe.x - state.hero.x) < 68);
  assert.equal(state.hero.hp, 84);
});

test('viewer follows per-frame timing, speed, pause and restart independently of combat', () => {
  const { context, state } = arena();
  state.mode = 'viewer';
  state.monster.animations.push({ id: 'timed', state: 'MOVE', loop: true,
    frames: [{ id: 2, duration: 100 }, { id: 3, duration: 200 }] });
  context.selectViewerClip('timed');
  assert.equal(context.currentFrame(), 2);
  context.advanceAnimation(99);
  assert.equal(context.currentFrame(), 2);
  context.advanceAnimation(1);
  assert.equal(context.currentFrame(), 3);
  context.setViewerPlaying(false);
  context.advanceAnimation(1000);
  assert.equal(state.clipClock, 100);
  state.playbackSpeed = 2;
  context.setViewerPlaying(true);
  context.advanceAnimation(100);
  assert.equal(context.currentFrame(), 2);
  context.selectViewerClip('timed');
  assert.equal(state.clipClock, 0);
  assert.equal(state.viewerPlaying, true);
  state.mode = 'fight';
  context.setViewerPlaying(false);
  context.advanceAnimation(100);
  assert.equal(state.clipClock, 100, 'combat ignores viewer pause and speed');
});

test('one-shot viewer preserves authored shade reversal and holds the final death frame', () => {
  const { context, state } = arena();
  state.mode = 'viewer';
  state.monster.animations.push({ id: 'shade', state: 'ATTACK', loop: false,
    frames: [5, 6, 7, 6, 5].map(id => ({ id, duration: 100 })) });
  context.selectViewerClip('shade');
  const frames = [context.currentFrame()];
  for (let i = 0; i < 4; i++) { context.advanceAnimation(100); frames.push(context.currentFrame()); }
  assert.deepEqual(frames, [5, 6, 7, 6, 5]);
  context.advanceAnimation(1000);
  assert.equal(context.currentFrame(), 5);
  assert.equal(state.viewerPlaying, false);
  context.selectViewerClip('death');
  context.advanceAnimation(1000);
  assert.equal(context.currentFrame(), 1);
  assert.equal(state.viewerPlaying, false);
  context.advanceAnimation(1000);
  assert.equal(state.clipClock, 100);
});

test('passive contact dancers never enter active windup or launch attacks', () => {
  const { context, state } = arena();
  state.monster.attacks = [{ id: 'v2_passive_contact', delivery: 'CONTACT', damage: 9,
    range: 18, cooldown: 500, telegraph: 0, animation: 'idle', steps: [{ kind: 'CONTACT' }] }];
  assert.equal(context.chooseAttack(0), null);
  state.hero.x = state.foe.x;
  state.hero.y = state.foe.y;
  const before = state.hero.hp;
  context.updateFight(.01);
  assert.ok(state.hero.hp < before);
  assert.equal(state.foe.windup, 0);
  assert.equal(state.foe.attack, null);
  assert.equal(state.shots.length, 0);
  const after = state.hero.hp;
  context.updateFight(.01);
  assert.equal(state.hero.hp, after);
});

test('one world scale transforms sprite pixels and projected hurtboxes identically', () => {
  const { context, state, drawings } = arena();
  Object.assign(state.monster.size, { anchorX: 48, anchorY: 55, worldScale: .5,
    pixelsPerUnit: 1.5, referenceHeight: 80, targetHeight: 40 });
  state.monster.colliders = [{ views: [{ direction: 0, x: 70, y: 40, radius: 7 }] }];
  context.drawSprite(state.monster, {}, 2, 0, 300, 430);
  const draw = drawings.find(call => call[0] === 'drawImage');
  assert.deepEqual(draw.slice(6), [276, 407, 48, 48]);
  assert.deepEqual({ ...context.hurtCircles(state.monster, 300, 430, false)[0] },
    { x: 311, y: 426.5, radius: 3.5 });
  assert.deepEqual({ ...context.hurtCircles(state.monster, 300, 430, true)[0] },
    { x: 289, y: 426.5, radius: 3.5 });
  assert.equal(context.bodyHeight(state.monster), 40);
  assert.equal(context.worldLength(state.monster, 80), 60, 'generator lengths include PPU once');
  // Frame size, mirror, animation and morphology must never affect the scalar.
  state.monster.sprites.frameWidth = 512;
  state.monster.sprites.frameHeight = 512;
  state.monster.size.morphologyScale = 9;
  context.drawSprite(state.monster, {}, 7, 3, 300, 430, 1, true);
  assert.equal(context.worldScale(state.monster), .5);
  assert.equal(context.hurtCircles(state.monster, 300, 430, true)[0].radius, 3.5);
});

test('missing, zero and invalid world scale preserve legacy art and combat units', () => {
  const { context, state, drawings } = arena();
  for (const worldScale of [undefined, 0, -1, NaN, Infinity]) {
    state.monster.size.worldScale = worldScale;
    state.monster.size.pixelsPerUnit = 3;
    context.drawSprite(state.monster, {}, 0, 0, 300, 430);
    assert.deepEqual(drawings.at(-1).slice(8), [192, 192]);
    assert.equal(context.worldScale(state.monster), 2);
    assert.equal(context.worldLength(state.monster, 70), 70);
    assert.equal(context.meleeReach(state.monster, { range: 38 }), 110);
  }
  delete state.monster.size;
  assert.equal(context.worldScale(state.monster), 2);
  assert.ok(Number.isFinite(context.hurtCircles(state.monster, 300, 430, false)[0].x));
});

test('melee selection, impact radius and movement use the same generator conversion', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: .5, pixelsPerUnit: 2, referenceHeight: 80 });
  const attack = { id: 'strike', delivery: 'MELEE', range: 38, damage: 12,
    steps: [{ kind: 'SWIPE', radius: 30 }], cooldown: 500, telegraph: 100 };
  state.monster.attacks = [attack];
  assert.equal(context.chooseAttack(55), null, 'small bodies cannot use old fixed 120px reach');
  assert.equal(context.chooseAttack(50).id, 'strike');
  state.monster.size.worldScale = 2;
  assert.equal(context.meleeReach(state.monster, attack), 168);
  assert.equal(context.chooseAttack(150).id, 'strike');
  const charge = { ...attack, steps: [{ kind: 'DASH', duration: 500 }, { kind: 'SWIPE', radius: 30 }] };
  context.startMotion(charge, charge.steps[0]);
  assert.equal(state.foe.action.radius, 120);
  state.foe.action = null;
  state.foe.attackCooldown = 0;
  state.monster.attacks = [];
  state.foe.x = 700;
  state.hero.x = 245;
  context.updateFight(.1);
  assert.equal(state.foe.x, 672, '70 generator units/s * 2 PPU * 2 scale * .1s');
});

test('projectiles originate at scaled posed views and retain their own world scale', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: .5, pixelsPerUnit: 2,
    anchorX: 48, anchorY: 55, referenceHeight: 80 });
  state.hero.x = 100;
  const attack = { id: 'spit', projectile: 'spit', damage: 8,
    originViews: [{ direction: 0, x: 60, y: 35, radius: 4 }],
    motionSamples: [{ time: 200, active: true, views: [{ direction: 0, x: 72, y: 25, radius: 4 }] }],
    steps: [{ kind: 'SPAWN_PROJECTILE', speed: 180, radius: 4 }] };
  state.monster.projectiles = [{ id: 'spit', collisionRadius: 6 }];
  context.launchProjectile(attack, 300, 430, state.monster, {}, 200);
  assert.equal(state.shots[0].x, 288, 'mirrored posed origin uses only worldScale');
  assert.equal(state.shots[0].y, 415);
  assert.equal(state.shots[0].scale, .5);
  assert.equal(state.shots[0].radius, 6, 'collision radius uses PPU and worldScale');
  assert.ok(Math.abs(Math.hypot(state.shots[0].vx, state.shots[0].vy) - 180) < 1e-9);
  state.monster.size.worldScale = 2;
  assert.equal(state.shots[0].scale, .5, 'in-flight shots keep the emitting creature scale');
  assert.equal(state.shots[0].age, 0, 'timing remains unscaled');
});

test('minions and scale-free fallback hurtboxes use their own metadata', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: 2, pixelsPerUnit: 2, referenceHeight: 80 });
  const child = { monster: { ...state.monster,
    size: { worldScale: .25, pixelsPerUnit: 2, referenceHeight: 40, visibleWidth: 20, visibleHeight: 40 } },
    projectileSheet: {} };
  assert.deepEqual({ ...context.hurtCircles(child.monster, 300, 430, false)[0] },
    { x: 300, y: 425, radius: 2.5 });
  const minion = { x: 300, y: 430, clipClock: 200, attack: { id: 'spit', delivery: 'PROJECTILE',
    steps: [{ kind: 'SPAWN_PROJECTILE', speed: 180 }] } };
  context.useMinionAttack(minion, child);
  assert.equal(state.shots[0].scale, .25);
  assert.equal(state.shots[0].y, 423.5, 'fallback origin is attached to the child body');
  assert.equal(state.shots[0].sheet, child.projectileSheet);
});

test('tiny melee creatures approach within scaled reach instead of stopping at legacy spacing', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: .1, pixelsPerUnit: 1, referenceHeight: 80 });
  state.monster.attacks = [{ id: 'bite', delivery: 'MELEE', range: 38,
    damage: 6, telegraph: 200, cooldown: 500, steps: [] }];
  state.foe.attackCooldown = 0;
  state.foe.x = state.hero.x + 30;
  context.updateFight(1);
  context.updateFight(1);
  assert.ok(state.foe.x - state.hero.x < 20);
  context.updateFight(.01);
  assert.equal(state.foe.attack?.id, 'bite');
});

test('tiny contact creatures collide with the hero feet rather than a fixed chest point', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: .1, pixelsPerUnit: 1, referenceHeight: 50 });
  state.monster.attacks = [{ id: 'contact', delivery: 'CONTACT', damage: 5, cooldown: 500 }];
  state.foe.x = state.hero.x + 10;
  context.updateFight(.01);
  assert.equal(state.hero.hp, 95);
  state.hero.hurtClock = 0;
  state.foe.cooldowns.contact = 0;
  state.hero.y = 300;
  context.updateFight(.01);
  assert.equal(state.hero.hp, 95, 'jumping above the tiny hurtbox avoids contact');
});


test('giant artwork, hurtboxes and attack origins share one neutral foot baseline', () => {
  const { context, state, drawings } = arena();
  Object.assign(state.monster.size, { worldScale: 3, pixelsPerUnit: 1.5,
    anchorX: 48, anchorY: 55, referenceHeight: 80 });
  state.monster.colliders = [
    { node: 'torso', views: [{ direction: 0, x: 48, y: 40, radius: 15 }] },
    { node: 'foot_0', views: [{ direction: 0, x: 55, y: 80, radius: 8 },
      { direction: 1, x: 50, y: 92, radius: 8 }] },
    { node: 'rider_foot_0', views: [{ direction: 0, x: 55, y: 100, radius: 8 }] },
    { node: 'tail', views: [{ direction: 0, x: 90, y: 110, radius: 12 }] },
  ];
  assert.equal(context.spriteAnchor(state.monster).y, 88, 'substrate feet set the package baseline');
  context.drawSprite(state.monster, {}, 0, 0, 300, 430);
  assert.equal(drawings.find(call => call[0] === 'drawImage')[7], 166);
  const foot = context.hurtCircles(state.monster, 300, 430, false)[1];
  assert.equal(foot.y + foot.radius, 430, 'neutral foot bottom stays on the floor even at 3x');
  const attack = { originViews: [{ direction: 0, x: 60, y: 20, radius: 4 }] };
  const origin = context.attackOrigin(state.monster, attack, 300, 430, true);
  assert.deepEqual({ ...origin }, { x: 264, y: 226, radius: 12 });
  state.direction = 1;
  state.clipClock = 9000;
  context.drawSprite(state.monster, {}, 9, 1, 300, 430, 1, true);
  const lastDraw = drawings.filter(call => call[0] === 'drawImage').at(-1);
  assert.equal(lastDraw[7], 166, 'direction and frame changes never recompute support from poses');
  assert.equal(context.spriteAnchor(state.monster).y, 88);
});

test('footless packages ground on neutral physical colliders and legacy anchors are untouched', () => {
  const { context, state } = arena();
  Object.assign(state.monster.size, { worldScale: 2.5, anchorY: 55 });
  state.monster.colliders = [
    { node: 'torso', views: [{ direction: 0, x: 48, y: 65, radius: 12 }] },
    { node: 'head', views: [{ direction: 0, x: 48, y: 35, radius: 8 }] },
  ];
  assert.equal(context.spriteAnchor(state.monster).y, 77);
  const torso = context.hurtCircles(state.monster, 300, 430, false)[0];
  assert.equal(torso.y + torso.radius, 430);
  delete state.monster.size.worldScale;
  assert.equal(context.spriteAnchor(state.monster).y, 55);
  state.monster.size.worldScale = 2.5;
  state.monster.colliders = [];
  assert.equal(context.spriteAnchor(state.monster).y, 55, 'missing neutral geometry retains the package anchor');
});
