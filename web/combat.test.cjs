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
