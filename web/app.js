const $ = (id) => document.getElementById(id);
const canvas = $('arena');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;
const W = canvas.width;
const H = canvas.height;
const FLOOR = 430;
const clamp = (value, low, high) => Math.max(low, Math.min(high, value));
const MOTION_KINDS = new Set(['DASH', 'LEAP', 'SWOOP', 'BURROW', 'TELEPORT', 'DODGE']);
const motionStep = (attack) => attack?.steps?.find(step => MOTION_KINDS.has(step.kind));
function teleportDestination(heroX, preferredSide) {
  const left = clamp(heroX - 190, 35, W - 35);
  const right = clamp(heroX + 190, 35, W - 35);
  const preferred = preferredSide < 0 ? left : right;
  const other = preferredSide < 0 ? right : left;
  return Math.abs(preferred - heroX) >= 160 ? preferred : other;
}
const canFly = (monster) => monster?.animations?.some(clip => clip.state === 'FLY');
const isAnchored = (monster) => monster?.tags?.includes('ANCHORED');
const state = {
  monster: null, sheet: null, projectileSheet: null, original: null, companions: {}, mode: 'viewer',
  clip: null, clipClock: 0, direction: 0, viewerMirror: false, lastTime: 0,
  hero: { x: 245, y: FLOOR, vy: 0, knockbackVX: 0, knockbackClock: 0, hp: 100, attackClock: 0, attackCooldown: 0, shotCooldown: 0, facing: 1, hurtClock: 0 },
  foe: { x: 720, y: FLOOR, hp: 0, attackCooldown: 0, windup: 0, attack: null, action: null, cooldowns: {}, attackCursor: 0, teleportAttempts: 0, dodgeCooldown: 0, hurtClock: 0 },
  keys: new Set(), jumpQueued: false, strikeQueued: false, fireQueued: false, shots: [], heroShots: [], sparks: [], fields: [], minions: [],
  pendingSurvivor: 0, summonCooldown: 0, message: '', messageClock: 0,
};

function setStatus(message) { $('status').textContent = message; }
function clipFor(id) { return state.monster?.animations.find(a => a.id === id || a.state === id); }
function playClip(id, restart = false) {
  const clip = clipFor(id);
  if (clip && (restart || state.clip?.id !== clip.id)) {
    state.clip = clip;
    state.clipClock = 0;
  }
}
function clipLength(clip) { return clip?.frames.reduce((sum, frame) => sum + Math.max(1, frame.duration), 0) || 1; }
function combatFacing(monster, left) {
  const angles = monster?.sprites?.angles || [0];
  const direction = angles.reduce((best, angle, index) => {
    const distance = Math.abs(((angle + 180) % 360) - 180);
    return distance < best.distance ? { index, distance } : best;
  }, { index: 0, distance: Infinity }).index;
  return { direction, mirror: left };
}
function hurtCircles(monster, x, ground, mirror) {
  const sprites = monster?.sprites;
  const direction = combatFacing(monster, mirror).direction;
  const anchorX = monster?.size?.anchorX ?? sprites?.frameWidth / 2 ?? 48;
  const anchorY = monster?.size?.anchorY ?? sprites?.frameHeight * .57 ?? 55;
  const circles = monster?.colliders?.flatMap(collider => {
    const view = collider.views?.find(item => item.direction === direction);
    if (!view) return [];
    return [{ x: x + (mirror ? anchorX - view.x : view.x - anchorX) * 2,
      y: ground + (view.y - anchorY) * 2, radius: Math.max(5, view.radius * 2) }];
  }) || [];
  if (circles.length) return circles;
  const width = monster?.size?.visibleWidth || 40;
  const height = monster?.size?.visibleHeight || 50;
  return [{ x, y: ground - Math.max(48, height), radius: Math.max(25, Math.min(70, width * .7)) }];
}
function hitsHurtbox(monster, x, ground, mirror, px, py, padding = 0) {
  return hurtCircles(monster, x, ground, mirror)
    .some(circle => Math.hypot(px - circle.x, py - circle.y) <= circle.radius + padding);
}
function swingHits(monster, x, ground) {
  const facing = state.hero.facing;
  const mirror = state.hero.x < x;
  return [35, 55, 75].some(offset => hitsHurtbox(monster, x, ground, mirror,
    state.hero.x + facing * offset, state.hero.y - 61, 14));
}
function currentFrame() {
  if (!state.clip?.frames.length) return 0;
  const length = clipLength(state.clip);
  let time = state.clip.loop ? state.clipClock % length : Math.min(state.clipClock, length - 1);
  for (const frame of state.clip.frames) {
    time -= Math.max(1, frame.duration);
    if (time < 0) return frame.id;
  }
  return state.clip.frames.at(-1).id;
}

function setupMonster(monster, sheet, projectileSheet) {
  state.original = { monster, sheet, projectileSheet, companions: state.companions };
  state.monster = monster;
  state.sheet = sheet;
  state.projectileSheet = projectileSheet;
  $('monster-name').textContent = monster.name;
  $('detail-name').textContent = monster.name;
  $('description').textContent = monster.description;
  $('size-badge').textContent = monster.size?.class || 'UNKNOWN SIZE';
  $('health').textContent = monster.health;
  $('defense').textContent = monster.defense;
  $('visible-size').textContent = monster.size ? `${monster.size.visibleWidth} × ${monster.size.visibleHeight} px` : '—';
  const parts = $('health-parts');
  parts.replaceChildren();
  for (const [label, value] of Object.entries(monster.healthParts)) {
    const element = document.createElement('span');
    element.textContent = `${label.toUpperCase()} +${value}`;
    parts.append(element);
  }
  const tags = $('tags');
  tags.replaceChildren();
  for (const tag of monster.tags) {
    const element = document.createElement('span');
    element.textContent = tag;
    tags.append(element);
  }
  $('details').hidden = false;
  const clipSelect = $('clip');
  clipSelect.replaceChildren();
  for (const clip of monster.animations) {
    const option = document.createElement('option');
    option.value = clip.id;
    option.textContent = `${clip.state} · ${clip.id}`;
    clipSelect.append(option);
  }
  const directionSelect = $('direction');
  directionSelect.replaceChildren();
  for (const [index, angle] of monster.sprites.angles.entries()) {
    const option = document.createElement('option');
    option.value = index;
    option.textContent = `${angle}°`;
    directionSelect.append(option);
    if (angle === 0) {
      const mirror = document.createElement('option');
      mirror.value = 'mirror0';
      mirror.textContent = 'Mirror 0°';
      directionSelect.append(mirror);
    }
  }
  state.direction = 0;
  state.viewerMirror = false;
  playClip('IDLE', true);
  clipSelect.value = state.clip?.id || clipSelect.options[0]?.value;
  $('arena-message').textContent = 'Select an animation or enter the arena.';
  resetFight();
}

async function generate(event) {
  event.preventDefault();
  const prompt = $('prompt').value.trim();
  const seed = Number($('seed').value);
  if (!prompt || !Number.isSafeInteger(seed) || seed < 0) {
    setStatus('Enter a prompt and a nonnegative integer seed.');
    return;
  }
  $('generate').disabled = true;
  setStatus('Building anatomy, physics, animations, and the sprite atlas…');
  try {
    const response = await fetch('/api/generate', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt, seed }),
    });
    const monster = await response.json();
    if (!response.ok) throw new Error(monster.error || `HTTP ${response.status}`);
    const loadImage = (src) => new Promise((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error(`Could not load ${src}`));
      image.src = src;
    });
    const sheet = await loadImage(monster.sprites.url);
    const projectileSheet = monster.projectiles.length ? await loadImage(monster.projectileAtlas) : null;
    async function loadCompanions(parent) {
      const companions = {};
      for (const [role, child] of Object.entries(parent.companions || {})) {
        companions[role] = {
          monster: child, sheet: await loadImage(child.sprites.url),
          projectileSheet: child.projectiles.length ? await loadImage(child.projectileAtlas) : null,
          companions: await loadCompanions(child),
        };
      }
      return companions;
    }
    state.companions = await loadCompanions(monster);
    setupMonster(monster, sheet, projectileSheet);
    setMode('viewer');
    setStatus(`Generated ${monster.name}. ${monster.animations.length} animations are ready.`);
  } catch (error) {
    setStatus(`Generation failed: ${error.message}`);
  } finally {
    $('generate').disabled = false;
  }
}

function setMode(mode) {
  state.mode = mode;
  $('viewer-mode').classList.toggle('active', mode === 'viewer');
  $('fight-mode').classList.toggle('active', mode === 'fight');
  $('clip-control').hidden = mode === 'fight';
  $('direction-control').hidden = mode === 'fight';
  $('controls').hidden = mode !== 'fight';
  $('reset-fight').hidden = mode !== 'fight';
  $('arena-message').hidden = mode === 'fight';
  if (mode === 'fight' && state.monster) resetFight();
  if (mode === 'viewer' && state.monster) {
    playClip($('clip').value || 'IDLE', true);
    state.direction = Number($('direction').value);
  }
}

function resetFight() {
  if (state.original) {
    state.monster = state.original.monster;
    state.sheet = state.original.sheet;
    state.projectileSheet = state.original.projectileSheet;
    state.companions = state.original.companions;
  }
  state.hero = { x: 245, y: FLOOR, vy: 0, knockbackVX: 0, knockbackClock: 0, hp: 100, attackClock: 0, attackCooldown: 0, shotCooldown: 0, facing: 1, hurtClock: 0 };
  state.foe = { x: isAnchored(state.monster) ? 470 : 720, y: canFly(state.monster) ? FLOOR - 78 : FLOOR,
    hp: state.monster?.health || 0, attackCooldown: 0, windup: 0, attack: null,
    action: null, cooldowns: {}, attackCursor: 0, teleportAttempts: 0, dodgeCooldown: 0, hurtClock: 0 };
  state.shots = [];
  state.heroShots = [];
  state.sparks = [];
  state.fields = [];
  state.minions = [];
  state.pendingSurvivor = 0;
  state.summonCooldown = 2.5;
  state.jumpQueued = false;
  state.strikeQueued = false;
  state.fireQueued = false;
  state.message = '';
  state.messageClock = 0;
  playClip('IDLE', true);
}

function foeVulnerable() {
  return !['BURROW', 'TELEPORT', 'DODGE'].includes(state.foe.action?.kind);
}

function hitMonster(damage) {
  if (state.foe.hp <= 0 || !foeVulnerable()) return false;
  state.foe.hp = Math.max(0, state.foe.hp - damage);
  state.foe.hurtClock = 0.28;
  if (state.foe.hp === 0) {
    state.foe.action = null;
    state.foe.attack = null;
    state.foe.windup = 0;
    playClip('DEATH', true);
    if (state.monster.mount && state.companions[state.monster.mount.survivor.toLowerCase()]) {
      state.pendingSurvivor = Math.max(.5, clipLength(clipFor('DEATH')) / 1000);
      state.message = `${state.monster.mount.survivor} SURVIVES!`;
      state.messageClock = 2;
    } else if (!state.minions.length) {
      state.message = 'VICTORY — the creature falls.';
      state.messageClock = 0;
    }
  } else {
    playClip('IMPACT_LIGHT', true);
  }
  return true;
}

function tryDodge() {
  const foe = state.foe;
  if (foe.hp <= 0 || foe.dodgeCooldown > 0 || foe.windup > 0 || foe.action) return false;
  const attack = state.monster?.attacks.find(a => motionStep(a)?.kind === 'DODGE');
  if (!attack) return false;
  foe.dodgeCooldown = 4.4;
  foe.cooldowns[attack.id] = attack.cooldown / 1000;
  startMotion(attack, motionStep(attack));
  playClip(attack.animation, true);
  return true;
}

function damageMonster() {
  if (!state.monster) return;
  const damage = Math.max(4, 19 - state.monster.defense);
  const minionInfo = state.companions.minion?.monster;
  const minion = state.minions.find(m => Math.sign(m.x - state.hero.x) === state.hero.facing
    && swingHits(minionInfo, m.x, m.y ?? FLOOR));
  if (minion) {
    minion.hp = Math.max(0, minion.hp - Math.max(4, 19 - state.companions.minion.monster.defense));
    state.minions = state.minions.filter(m => m.hp > 0);
    if (state.foe.hp === 0 && !state.pendingSurvivor && !state.minions.length) {
      state.message = 'VICTORY — the creatures fall.';
      state.messageClock = 0;
    }
    return;
  }
  if (state.foe.hp <= 0 || Math.sign(state.foe.x - state.hero.x) !== state.hero.facing) return;
  if (!swingHits(state.monster, state.foe.x, state.foe.y) || !foeVulnerable()) return;
  if (tryDodge()) return;
  hitMonster(damage);
}

function summonMinions(attack) {
  const child = state.companions.minion;
  if (!child || !attack.spawn) return;
  const available = Math.max(0, attack.spawn.maxActive - state.minions.length);
  for (let i = 0; i < Math.min(available, attack.spawn.count); i++) {
    const toward = Math.sign(state.hero.x - state.foe.x) || -1;
    state.minions.push({ x: clamp(state.foe.x + (i % 2 ? -12 : 12), 45, W - 45),
      y: state.foe.y - 35,
      hp: child.monster.health, attackCooldown: .9 + i * .25, clock: i * 173,
      clip: null, clipClock: 0, windup: 0, attack: null, action: null, attackCursor: i,
      launchClock: .38, launchVX: toward * (160 + i * 40), launchVY: -45 - i * 15 });
  }
}
function minionClip(minion, monster, stateName, restart = false) {
  const clip = monster.animations.find(item => item.id === stateName || item.state === stateName);
  if (clip && (restart || minion.clip?.id !== clip.id)) {
    minion.clip = clip;
    minion.clipClock = 0;
  }
}
function minionAttack(minion, monster, distance) {
  const attacks = monster.attacks?.filter(attack => attack.delivery !== 'SUMMON') || [];
  for (let offset = 0; offset < attacks.length; offset++) {
    const index = (minion.attackCursor + offset) % attacks.length;
    const attack = attacks[index];
    if (motionStep(attack)?.kind === 'TELEPORT') {
      minion.teleportAttempts = (minion.teleportAttempts || 0) + 1;
      if (minion.teleportAttempts % 12 !== 0) continue;
    }
    const reach = attack.delivery === 'PROJECTILE' ? 460
      : attack.delivery === 'FIELD' ? 230 : Math.max(90, attack.range + 35);
    if (distance > reach) continue;
    minion.attackCursor = (index + 1) % attacks.length;
    return attack;
  }
  return null;
}
function useMinionAttack(minion, child) {
  const attack = minion.attack;
  minion.attack = null;
  if (!attack || state.hero.hp <= 0) return;
  const hero = state.hero;
  const distance = Math.abs(hero.x - minion.x);
  const motion = motionStep(attack);
  if (motion) {
    const direction = Math.sign(hero.x - minion.x) || -1;
    minion.action = { kind: motion.kind, elapsed: 0,
      duration: clamp(motion.duration / 1000 || .45, .25, 1.2), startX: minion.x,
      targetX: clamp(motion.kind === 'DODGE' ? minion.x - direction * 90
        : motion.kind === 'TELEPORT' ? teleportDestination(hero.x, -hero.facing) : hero.x,
      35, W - 35), attack, hit: false };
  } else if (attack.delivery === 'PROJECTILE') {
    launchProjectile(attack, minion.x, minion.y - 42, child.monster, child.projectileSheet);
  } else if (attack.delivery === 'FIELD') {
    const step = attack.steps?.find(item => item.kind === 'SPAWN_FIELD');
    state.fields.push({ x: hero.x, radius: step?.radius || 38, age: 0,
      life: Math.max(.7, (step?.duration || 900) / 1000), tick: 0,
      damage: Math.max(2, Math.round(attack.damage * .4)) });
  } else if (distance < Math.max(90, attack.range + 35)
      && Math.abs(hero.y - minion.y) < 100) {
    damageHero(attack.damage, minion.x);
  }
}
function updateMinions(dt) {
  const child = state.companions.minion;
  if (!child) return;
  const monster = child.monster;
  const hero = state.hero;
  for (const minion of state.minions) {
    minion.clock += dt * 1000;
    minion.clipClock += dt * 1000;
    minion.attackCooldown = Math.max(0, minion.attackCooldown - dt);
    if (minion.launchClock > 0) {
      minion.launchClock = Math.max(0, minion.launchClock - dt);
      minion.x = clamp(minion.x + minion.launchVX * dt, 25, W - 25);
      minion.y += minion.launchVY * dt;
      minion.launchVY += 260 * dt;
      minionClip(minion, monster, canFly(monster) ? 'FLY' : 'FAST_MOVE');
      continue;
    }
    if (minion.action) {
      const action = minion.action;
      action.elapsed = Math.min(action.duration, action.elapsed + dt);
      const progress = action.elapsed / action.duration;
      if (action.kind === 'TELEPORT' && progress >= .36 && !action.arrived) {
        action.targetX = teleportDestination(hero.x, -hero.facing);
        action.arrived = true;
      }
      minion.x = action.kind === 'TELEPORT'
        ? (progress >= .36 ? action.targetX : action.startX)
        : action.startX + (action.targetX - action.startX) * progress;
      minion.y = action.kind === 'LEAP' || action.kind === 'SWOOP'
        ? FLOOR - Math.sin(progress * Math.PI) * 85
        : canFly(monster) ? FLOOR - 70 : FLOOR;
      if (!action.hit && progress >= .78) {
        action.hit = true;
        if (action.kind !== 'TELEPORT' && Math.abs(hero.x - minion.x) < 75 && Math.abs(hero.y - minion.y) < 95)
          damageHero(action.attack.damage, minion.x);
      }
      if (progress >= 1) minion.action = null;
      continue;
    }
    minion.y += ((canFly(monster) ? FLOOR - 70 : FLOOR) - minion.y) * Math.min(1, dt * 8);
    if (minion.windup > 0) {
      minion.windup = Math.max(0, minion.windup - dt);
      if (minion.windup === 0) useMinionAttack(minion, child);
      continue;
    }
    const gap = hero.x - minion.x;
    const distance = Math.abs(gap);
    if (hero.hp > 0 && minion.attackCooldown === 0) {
      const attack = minionAttack(minion, monster, distance);
      if (attack) {
        minion.attack = attack;
        minion.windup = Math.max(.18, attack.telegraph / 1000);
        minion.attackCooldown = Math.max(.6, attack.cooldown / 1000);
        minionClip(minion, monster, attack.animation, true);
        continue;
      }
    }
    const ranged = monster.attacks?.some(attack => attack.delivery === 'PROJECTILE');
    const preferred = canFly(monster) ? 160 : ranged ? 180 : 55;
    if (hero.hp > 0 && !isAnchored(monster) && distance > preferred + 16) {
      minion.x = clamp(minion.x + Math.sign(gap)
        * Math.min(110, monster.movementModes[0]?.speed || 65) * dt, 25, W - 25);
      minionClip(minion, monster, canFly(monster) ? 'FLY' : 'MOVE');
    } else if (hero.hp > 0 && !isAnchored(monster) && ranged && distance < preferred - 45) {
      minion.x = clamp(minion.x - Math.sign(gap) * 65 * dt, 25, W - 25);
      minionClip(minion, monster, 'MOVE');
    } else minionClip(minion, monster, canFly(monster) ? 'FLY' : 'IDLE');
  }
}

function activateSurvivor() {
  const survivor = state.companions[state.monster.mount.survivor.toLowerCase()];
  if (!survivor) return;
  state.monster = survivor.monster;
  state.sheet = survivor.sheet;
  state.projectileSheet = survivor.projectileSheet;
  state.companions = survivor.companions || {};
  state.foe.hp = survivor.monster.health;
  state.foe.attackCooldown = 1.1;
  state.foe.windup = 0;
  state.foe.attack = null;
  state.foe.action = null;
  state.foe.cooldowns = {};
  state.foe.attackCursor = 0;
  state.foe.teleportAttempts = 0;
  state.foe.dodgeCooldown = 0;
  state.foe.y = canFly(survivor.monster) ? FLOOR - 78 : FLOOR;
  state.shots = [];
  state.fields = [];
  state.pendingSurvivor = 0;
  state.message = `${survivor.monster.name.toUpperCase()} CONTINUES THE FIGHT`;
  state.messageClock = 2;
  playClip('IDLE', true);
}

function damageHero(amount, sourceX = state.foe.x) {
  if (state.hero.hp <= 0 || state.hero.hurtClock > 0) return;
  state.hero.hp = Math.max(0, state.hero.hp - amount);
  state.hero.hurtClock = 0.48;
  const away = Math.sign(state.hero.x - sourceX) || -state.hero.facing || -1;
  state.hero.knockbackVX = away * clamp(145 + amount * 4, 165, 275);
  state.hero.knockbackClock = .24;
  if (state.hero.y >= FLOOR - 2) state.hero.vy = -110;
  if (state.hero.hp === 0) {
    state.message = 'DEFEAT — press Restart to try again.';
    state.messageClock = 0;
  }
}

function launchProjectile(attack, sourceX = state.foe.x, sourceY = state.foe.y - 65,
    monster = state.monster, sheet = state.projectileSheet) {
  const dx = state.hero.x - sourceX;
  const dy = state.hero.y - 48 - sourceY;
  const angle = Math.atan2(dy, dx);
  const step = attack.steps?.find(item => item.kind === 'SPAWN_PROJECTILE');
  const count = clamp(step?.count || 1, 1, 7);
  const spread = (step?.spread || 0) * Math.PI / 180;
  const speed = clamp(step?.speed || 240, 120, 370);
  const projectile = monster.projectiles?.find(p => p.id === attack.projectile);
  for (let index = 0; index < count; index++) {
    const offset = count === 1 ? 0 : (index / (count - 1) - .5) * spread;
    const direction = angle + offset;
    state.shots.push({ x: sourceX + Math.cos(direction) * 28, y: sourceY,
      vx: Math.cos(direction) * speed, vy: Math.sin(direction) * speed,
      age: 0, attack, projectile, sheet, prevX: sourceX });
  }
}

function launchHeroProjectile() {
  const hero = state.hero;
  if (state.foe.hp > 0) hero.facing = Math.sign(state.foe.x - hero.x) || hero.facing;
  const x = hero.x + hero.facing * 24;
  const y = hero.y - 53;
  const targetX = state.foe.hp > 0 ? state.foe.x : x + hero.facing * 300;
  const targetY = state.foe.hp > 0 ? state.foe.y - 58 : y;
  const angle = Math.atan2(targetY - y, targetX - x);
  state.heroShots.push({ x, y, prevX: x, vx: Math.cos(angle) * 350,
    vy: Math.sin(angle) * 350, age: 0, damage: 10 });
  hero.shotCooldown = .58;
}

function startMotion(attack, step) {
  const foe = state.foe;
  const toward = Math.sign(state.hero.x - foe.x) || -1;
  const kind = step.kind;
  const impact = attack.steps?.find(item => ['SLAM', 'SWIPE', 'BITE', 'THRUST'].includes(item.kind));
  const defaultHitAt = kind === 'SWOOP' ? .78 : kind === 'TELEPORT' ? .58 : .88;
  foe.action = {
    kind, attack, elapsed: 0, duration: kind === 'SWOOP'
      ? clamp(step.duration / 1000 || .8, .8, 1.2)
      : clamp(step.duration / 1000 || .5, .25, 1.2),
    startX: foe.x, startY: foe.y, targetX: clamp(
      kind === 'DODGE' ? foe.x - toward * 100
        : kind === 'TELEPORT' ? teleportDestination(state.hero.x, -state.hero.facing)
          : state.hero.x + toward * (kind === 'DASH' ? 30 : 0), 35, W - 35),
    targetY: state.hero.y,
    radius: impact?.radius || 70,
    hitAt: Number.isFinite(impact?.time) && Number.isFinite(step.time) && step.duration > 0
      ? clamp((impact.time - step.time) / step.duration, .1, 1) : defaultHitAt,
    hit: false,
  };
  foe.windup = 0;
  foe.attack = null;
  if (kind === 'DODGE') foe.dodgeCooldown = Math.max(foe.dodgeCooldown, 3.5);
}

function updateMotion(dt) {
  const foe = state.foe;
  const action = foe.action;
  if (!action) return;
  action.elapsed = Math.min(action.duration, action.elapsed + dt);
  const t = action.elapsed / action.duration;
  const toward = Math.sign(action.targetX - action.startX) || -1;
  switch (action.kind) {
    case 'DASH':
      foe.x = action.startX + (action.targetX - action.startX) * t;
      foe.y = FLOOR;
      break;
    case 'LEAP':
      foe.x = action.startX + (action.targetX - action.startX) * t;
      foe.y = FLOOR - Math.sin(Math.PI * t) * 145;
      break;
    case 'SWOOP':
      foe.x = action.startX + (action.targetX - action.startX) * t;
      foe.y = action.startY + (FLOOR - action.startY - 8) * Math.sin(Math.PI * t);
      break;
    case 'BURROW':
      foe.x = action.startX + (action.targetX - action.startX) * t;
      foe.y = FLOOR + Math.sin(Math.PI * t) * 90;
      break;
    case 'TELEPORT':
      if (t >= .36 && !action.arrived) {
        action.targetX = teleportDestination(state.hero.x, -state.hero.facing);
        action.arrived = true;
      }
      if (action.arrived) foe.x = action.targetX;
      foe.y = action.startY;
      break;
    case 'DODGE':
      foe.x = t < .45
        ? action.startX + (action.targetX - action.startX) * (t / .45)
        : action.targetX + (state.hero.x - action.targetX - toward * 45) * ((t - .45) / .55);
      foe.y = FLOOR - Math.sin(Math.PI * t) * 65;
      break;
  }
  foe.x = clamp(foe.x, 30, W - 30);
  if (!action.hit && t >= action.hitAt) {
    action.hit = true;
    const nearX = Math.abs(state.hero.x - foe.x) < action.radius;
    const nearY = action.kind === 'LEAP' || action.kind === 'BURROW'
      ? state.hero.y > FLOOR - 80 : Math.abs(state.hero.y - foe.y) < 88;
    if (nearX && nearY && action.kind !== 'TELEPORT') damageHero(action.attack.damage, foe.x);
  }
  if (t >= 1) {
    foe.action = null;
    foe.y = canFly(state.monster) ? FLOOR - 78 : FLOOR;
    foe.attackCooldown = Math.max(foe.attackCooldown, .45);
    playClip(canFly(state.monster) ? 'FLY' : 'IDLE');
  }
}

function chooseAttack(distance) {
  const foe = state.foe;
  const attacks = state.monster.attacks;
  for (let offset = 0; offset < attacks.length; offset++) {
    const index = (foe.attackCursor + offset) % attacks.length;
    const attack = attacks[index];
    if (motionStep(attack)?.kind === 'TELEPORT') {
      foe.teleportAttempts = (foe.teleportAttempts || 0) + 1;
      if (foe.teleportAttempts % 12 !== 0) continue;
    }
    if ((foe.cooldowns[attack.id] || 0) > 0) continue;
    if (attack.delivery === 'SUMMON') {
      if (state.summonCooldown > 0 || !attack.spawn || state.minions.length >= attack.spawn.maxActive) continue;
    } else if (motionStep(attack)) {
      if (distance > attack.range) continue;
    } else if (attack.delivery === 'PROJECTILE') {
      if (distance > 480) continue;
    } else if (attack.delivery === 'FIELD') {
      if (distance > Math.max(220, attack.range + 80)) continue;
    } else if (distance > Math.max(120, attack.range + 55)) continue;
    foe.attackCursor = (index + 1) % attacks.length;
    return attack;
  }
  return null;
}

function updateFight(dt) {
  if (!state.monster) return;
  const hero = state.hero;
  const foe = state.foe;
  hero.hurtClock = Math.max(0, hero.hurtClock - dt);
  if (hero.knockbackClock > 0) {
    hero.x = clamp(hero.x + hero.knockbackVX * dt, 28, W - 28);
    hero.knockbackClock = Math.max(0, hero.knockbackClock - dt);
    hero.knockbackVX *= Math.max(0, 1 - dt * 2.5);
  }
  foe.hurtClock = Math.max(0, foe.hurtClock - dt);
  for (const spark of state.sparks) spark.life -= dt;
  state.sparks = state.sparks.filter(spark => spark.life > 0);
  if (state.messageClock > 0) {
    state.messageClock = Math.max(0, state.messageClock - dt);
    if (state.messageClock === 0) state.message = '';
  }
  state.summonCooldown = Math.max(0, state.summonCooldown - dt);
  if (state.pendingSurvivor > 0) {
    state.pendingSurvivor -= dt;
    if (state.pendingSurvivor <= 0) activateSurvivor();
  }
  hero.attackCooldown = Math.max(0, hero.attackCooldown - dt);
  hero.shotCooldown = Math.max(0, hero.shotCooldown - dt);
  if (hero.attackClock > 0) {
    const before = hero.attackClock;
    hero.attackClock = Math.max(0, hero.attackClock - dt);
    if (before > 0.22 && hero.attackClock <= 0.22) damageMonster();
  }
  if (hero.hp > 0) {
    const move = Number(state.keys.has('d') || state.keys.has('arrowright')) - Number(state.keys.has('a') || state.keys.has('arrowleft'));
    if (move && hero.knockbackClock === 0) {
      hero.x = clamp(hero.x + move * 190 * dt, 28, W - 28);
      hero.facing = Math.sign(move);
    }
    if (state.jumpQueued && hero.y >= FLOOR) hero.vy = -410;
    state.jumpQueued = false;
    if ((state.keys.has('x') || state.strikeQueued) && hero.attackCooldown === 0) {
      hero.attackClock = 0.34;
      hero.attackCooldown = 0.44;
      state.strikeQueued = false;
    }
    if ((state.keys.has('c') || state.fireQueued) && hero.shotCooldown === 0) {
      launchHeroProjectile();
      state.fireQueued = false;
    }
  }
  hero.vy += 1050 * dt;
  hero.y = Math.min(FLOOR, hero.y + hero.vy * dt);
  if (hero.y === FLOOR) hero.vy = 0;
  foe.attackCooldown = Math.max(0, foe.attackCooldown - dt);
  foe.dodgeCooldown = Math.max(0, foe.dodgeCooldown - dt);
  for (const id of Object.keys(foe.cooldowns)) foe.cooldowns[id] = Math.max(0, foe.cooldowns[id] - dt);
  if (foe.hp > 0 && hero.hp > 0) {
    const distance = Math.abs(hero.x - foe.x);
    if (foe.action) {
      updateMotion(dt);
    } else if (foe.windup > 0) {
      foe.windup = Math.max(0, foe.windup - dt);
      if (foe.windup === 0 && foe.attack) {
        const attack = foe.attack;
        const motion = motionStep(attack);
        if (motion) startMotion(attack, motion);
        else if (attack.delivery === 'SUMMON') summonMinions(attack);
        else if (attack.delivery === 'PROJECTILE') launchProjectile(attack);
        else if (attack.delivery === 'FIELD') {
          const fieldStep = attack.steps?.find(step => step.kind === 'SPAWN_FIELD');
          state.fields.push({ x: hero.x, radius: fieldStep?.radius || 55, age: 0,
            life: Math.max(1, (fieldStep?.duration || 1200) / 1000),
            tick: 0, damage: Math.max(2, Math.round(attack.damage * .45)) });
        } else if (distance < Math.max(110, attack.range + 40)
            && Math.abs(hero.y - foe.y) < 90) damageHero(attack.damage, foe.x);
        if (!motion) foe.attack = null;
        foe.attackCooldown = Math.max(foe.attackCooldown, .42);
      }
    } else if (foe.attackCooldown === 0) {
      const attack = chooseAttack(distance);
      if (attack) {
        foe.attack = attack;
        foe.windup = Math.max(.2, attack.telegraph / 1000);
        foe.cooldowns[attack.id] = Math.max(.4, attack.cooldown / 1000);
        if (attack.delivery === 'SUMMON') state.summonCooldown = Math.max(3, attack.cooldown / 1000);
        playClip(attack.animation, true);
      }
    }
    if (!isAnchored(state.monster) && !foe.action && foe.windup === 0 && foe.attackCooldown === 0) {
      const toward = Math.sign(hero.x - foe.x);
      const flying = canFly(state.monster);
      const preferred = flying ? 225 : state.monster.attacks.some(a => a.delivery === 'PROJECTILE') ? 250 : 95;
      const speed = clamp(state.monster.movementModes[0]?.speed || 70, 38, flying ? 145 : 125);
      if (distance > preferred + 25) {
        foe.x = clamp(foe.x + toward * speed * dt, 30, W - 30);
        playClip(flying ? 'FLY' : 'MOVE');
      } else if (flying && distance < 150) {
        foe.x = clamp(foe.x - toward * speed * dt, 30, W - 30);
        playClip('FLY');
      } else playClip(flying ? 'FLY' : 'IDLE');
      foe.y += ((flying ? FLOOR - 78 : FLOOR) - foe.y) * Math.min(1, dt * 8);
    }
  }
  updateMinions(dt);
  const minionInfo = state.companions.minion?.monster;
  for (const field of state.fields) {
    field.age += dt;
    field.tick -= dt;
    if (field.tick <= 0 && hero.hp > 0 && Math.abs(hero.x - field.x) < field.radius
        && hero.y > FLOOR - 75) {
      damageHero(field.damage, foe.x);
      field.tick = .75;
    }
  }
  state.fields = state.fields.filter(field => field.age < field.life);
  for (const shot of state.shots) {
    shot.prevX = shot.x;
    shot.x += shot.vx * dt;
    shot.y += (shot.vy || 0) * dt;
    shot.age += dt;
  }
  for (const bolt of state.heroShots) {
    bolt.prevX = bolt.x;
    bolt.x += bolt.vx * dt;
    bolt.y += bolt.vy * dt;
    bolt.age += dt;
    for (const shot of state.shots) {
      if (shot.age >= 3 || Math.abs(bolt.y - shot.y) > 23) continue;
      const crossed = Math.max(Math.min(bolt.prevX, bolt.x), Math.min(shot.prevX, shot.x))
        <= Math.min(Math.max(bolt.prevX, bolt.x), Math.max(shot.prevX, shot.x)) + 17;
      if (crossed) {
        state.sparks.push({ x: (bolt.x + shot.x) / 2, y: (bolt.y + shot.y) / 2, life: .24 });
        bolt.age = 99; shot.age = 99; break;
      }
    }
    if (bolt.age >= 3) continue;
    const minion = state.minions.find(m => hitsHurtbox(minionInfo, m.x, m.y ?? FLOOR,
      hero.x < m.x, bolt.x, bolt.y, 9));
    if (minion) {
      minion.hp -= bolt.damage;
      bolt.age = 99;
      state.minions = state.minions.filter(m => m.hp > 0);
    } else if (foe.hp > 0 && hitsHurtbox(state.monster, foe.x, foe.y,
      hero.x < foe.x, bolt.x, bolt.y, 9)) {
      if (!tryDodge()) hitMonster(Math.max(3, bolt.damage - Math.floor(state.monster.defense * .35)));
      bolt.age = 99;
    }
  }
  for (const shot of state.shots) {
    if (shot.age >= 3) continue;
    const parry = hero.attackClock > .1 && hero.attackClock < .31
      && Math.sign(shot.x - hero.x) === hero.facing
      && Math.abs(shot.x - hero.x) < 65 && Math.abs(shot.y - (hero.y - 57)) < 48;
    if (parry) {
      state.sparks.push({ x: shot.x, y: shot.y, life: .24 });
      shot.age = 99;
    }
    else if (Math.abs(shot.x - hero.x) < 21 && Math.abs(shot.y - (hero.y - 47)) < 43) {
      damageHero(shot.attack.damage, shot.x - shot.vx * Math.min(dt, .05));
      shot.age = 99;
    }
  }
  state.shots = state.shots.filter(shot => shot.age < 3 && shot.x > -30 && shot.x < W + 30);
  state.heroShots = state.heroShots.filter(bolt => bolt.age < 2.5 && bolt.x > -30 && bolt.x < W + 30 && bolt.y > -30 && bolt.y < H + 30);
  if (foe.hp === 0) foe.y += (FLOOR - foe.y) * Math.min(1, dt * 3);
  if (foe.hp === 0 && state.pendingSurvivor <= 0) playClip('DEATH');
}

function drawBackground(time) {
  const sky = ctx.createLinearGradient(0, 0, 0, H);
  sky.addColorStop(0, '#1c1c2c'); sky.addColorStop(1, '#46313a');
  ctx.fillStyle = sky; ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = '#f6d6a128';
  for (let i = 0; i < 18; i++) {
    const x = (i * 173 + 47) % W; const y = (i * 61 + 24) % 230;
    ctx.fillRect(x, y, 2, 2);
  }
  ctx.fillStyle = '#5d4450';
  for (let i = 0; i < W; i += 95) {
    const height = 70 + (i * 17 % 58);
    ctx.fillRect(i + 4, FLOOR - height, 30, height);
    ctx.fillRect(i, FLOOR - height - 8, 38, 8);
  }
  ctx.fillStyle = '#211d29'; ctx.fillRect(0, FLOOR, W, H - FLOOR);
  ctx.fillStyle = '#95717b'; ctx.fillRect(0, FLOOR, W, 4);
  ctx.fillStyle = '#392a34';
  for (let i = 0; i < W; i += 42) ctx.fillRect(i + (i % 3) * 5, FLOOR + 17 + (i % 4) * 13, 15, 3);
  if (!state.monster) {
    ctx.fillStyle = '#cbbbc4'; ctx.font = '24px Georgia'; ctx.textAlign = 'center';
    ctx.fillText('The arena waits for a monster.', W / 2, H / 2);
  }
}

function drawSprite(monster, sheet, frame, direction, x, ground, alpha = 1, mirror = false) {
  if (!monster || !sheet) return;
  const sprites = monster.sprites;
  const id = frame + direction * sprites.stride;
  const sx = (id % sprites.columns) * sprites.frameWidth;
  const sy = Math.floor(id / sprites.columns) * sprites.frameHeight;
  const scale = 2;
  const anchorX = monster.size?.anchorX ?? sprites.frameWidth / 2;
  const anchorY = monster.size?.anchorY ?? sprites.frameHeight * .57;
  ctx.save();
  ctx.globalAlpha = alpha;
  if (mirror) {
    ctx.translate(Math.round(x) * 2, 0);
    ctx.scale(-1, 1);
  }
  ctx.drawImage(sheet, sx, sy, sprites.frameWidth, sprites.frameHeight,
    Math.round(x - anchorX * scale), Math.round(ground - anchorY * scale),
    sprites.frameWidth * scale, sprites.frameHeight * scale);
  ctx.restore();
}
function drawMonster(x, ground) {
  const action = state.mode === 'fight' ? state.foe.action : null;
  if (action?.kind === 'BURROW' && action.elapsed / action.duration > .17
      && action.elapsed / action.duration < .83) {
    ctx.fillStyle = '#b88a68';
    ctx.fillRect(x - 23, FLOOR - 2, 46, 4);
    ctx.fillStyle = '#6d5147';
    ctx.fillRect(x - 34, FLOOR + 4, 68, 3);
    return;
  }
  if (action?.kind === 'TELEPORT' && action.elapsed / action.duration < .36
      && Math.floor(action.elapsed * 24) % 2 === 0) return;
  const alpha = state.foe.hurtClock > 0 && state.mode === 'fight' && Math.sin(state.foe.hurtClock * 35) > 0 ? .55 : 1;
  const facing = state.mode === 'fight'
    ? combatFacing(state.monster, state.hero.x < x)
    : { direction: state.direction, mirror: state.viewerMirror };
  drawSprite(state.monster, state.sheet, currentFrame(), facing.direction, x, ground, alpha, facing.mirror);
}
function drawMinions() {
  const child = state.companions.minion;
  if (!child) return;
  for (const minion of state.minions) {
    const clip = minion.clip || child.monster.animations.find(a => a.state === 'IDLE') || child.monster.animations[0];
    let time = minion.clipClock % clipLength(clip);
    let frame = clip.frames[0]?.id || 0;
    for (const item of clip.frames) {
      time -= Math.max(1, item.duration);
      if (time < 0) { frame = item.id; break; }
    }
    const facing = combatFacing(child.monster, state.hero.x < minion.x);
    drawSprite(child.monster, child.sheet, frame, facing.direction, minion.x, minion.y ?? FLOOR, 1, facing.mirror);
  }
}

function drawStickman(hero, time) {
  const x = hero.x, y = hero.y;
  const walking = state.keys.has('a') || state.keys.has('d') || state.keys.has('arrowleft') || state.keys.has('arrowright');
  const swing = walking && hero.y === FLOOR ? Math.sin(time * 14) * 10 : 0;
  ctx.save();
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.strokeStyle = hero.hurtClock > 0 ? '#ff9e86' : '#f7e8d7';
  ctx.lineWidth = 5;
  ctx.beginPath(); ctx.arc(x, y - 75, 11, 0, Math.PI * 2); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(x, y - 64); ctx.lineTo(x, y - 30); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(x, y - 30); ctx.lineTo(x - 14 + swing, y); ctx.moveTo(x, y - 30); ctx.lineTo(x + 14 - swing, y); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(x, y - 55); ctx.lineTo(x - 17, y - 38); ctx.moveTo(x, y - 55);
  const striking = hero.attackClock > 0;
  ctx.lineTo(x + (striking ? 35 : 18) * hero.facing, y - (striking ? 72 : 38)); ctx.stroke();
  if (striking) {
    ctx.strokeStyle = '#ffc77c'; ctx.lineWidth = 4;
    ctx.beginPath(); ctx.arc(x + hero.facing * 42, y - 64, 18, -.9, 1.2); ctx.stroke();
  }
  ctx.restore();
}

function drawShots() {
  for (const shot of state.shots) {
    if (shot.projectile && (shot.sheet || state.projectileSheet)) {
      const p = shot.projectile;
      const frame = p.firstFrame + Math.floor(shot.age * 1000 / Math.max(1, p.frameDuration)) % p.frameCount;
      ctx.drawImage(shot.sheet || state.projectileSheet, (frame % p.columns) * p.frameWidth,
        Math.floor(frame / p.columns) * p.frameHeight, p.frameWidth, p.frameHeight,
        shot.x - p.frameWidth, shot.y - p.frameHeight, p.frameWidth * 2, p.frameHeight * 2);
    } else {
      ctx.fillStyle = '#ffab52'; ctx.beginPath(); ctx.arc(shot.x, shot.y, 10, 0, Math.PI * 2); ctx.fill();
    }
  }
  for (const bolt of state.heroShots) {
    const facing = Math.sign(bolt.vx) || 1;
    ctx.fillStyle = '#6ce0e8';
    ctx.fillRect(Math.round(bolt.x - 7), Math.round(bolt.y - 3), 15, 6);
    ctx.fillStyle = '#ebffff';
    ctx.fillRect(Math.round(bolt.x + facing * 7 - 2), Math.round(bolt.y - 2), 5, 4);
    ctx.fillStyle = '#459da9';
    ctx.fillRect(Math.round(bolt.x - facing * 13), Math.round(bolt.y - 1), 6, 2);
  }
  for (const spark of state.sparks) {
    const radius = Math.ceil(spark.life * 35);
    ctx.fillStyle = '#f9e6a2';
    ctx.fillRect(Math.round(spark.x - radius), Math.round(spark.y - 2), radius * 2, 4);
    ctx.fillRect(Math.round(spark.x - 2), Math.round(spark.y - radius), 4, radius * 2);
  }
}

function drawFields() {
  for (const field of state.fields) {
    const pulse = Math.sin(field.age * 12) * 5;
    ctx.fillStyle = '#aa6a8a66';
    ctx.beginPath(); ctx.ellipse(field.x, FLOOR + 1, field.radius + pulse, 12, 0, 0, Math.PI * 2); ctx.fill();
    ctx.strokeStyle = '#eda2b6'; ctx.lineWidth = 3;
    ctx.beginPath(); ctx.ellipse(field.x, FLOOR + 1, field.radius + pulse, 12, 0, 0, Math.PI * 2); ctx.stroke();
  }
}

function drawTelegraph() {
  const foe = state.foe;
  if (foe.windup <= 0 || !foe.attack) return;
  const motion = motionStep(foe.attack);
  const label = motion ? {
    DASH: 'CHARGE', LEAP: 'LEAP SMASH', SWOOP: 'SWOOP', BURROW: 'BURROW',
    TELEPORT: 'BLINK', DODGE: 'DODGE',
  }[motion.kind] : foe.attack.delivery === 'SUMMON' ? 'SUMMONING'
    : foe.attack.delivery === 'PROJECTILE' ? `${foe.attack.element} SHOT`
      : foe.attack.delivery === 'FIELD' ? `${foe.attack.element} FIELD` : 'STRIKE';
  ctx.fillStyle = '#ffbf68'; ctx.font = 'bold 17px system-ui'; ctx.textAlign = 'center';
  ctx.fillText(`${label}!`, foe.x, Math.max(65, foe.y - 180));
  if (motion && ['LEAP', 'SWOOP', 'BURROW', 'TELEPORT'].includes(motion.kind)) {
    ctx.strokeStyle = '#ffbf68'; ctx.lineWidth = 3;
    ctx.beginPath(); ctx.ellipse(state.hero.x, FLOOR + 2, motion.kind === 'LEAP' ? 82 : 55, 11, 0, 0, Math.PI * 2); ctx.stroke();
  }
}

function healthBar(x, y, width, fraction, label, color) {
  ctx.fillStyle = '#110e19'; ctx.fillRect(x - 2, y - 2, width + 4, 26);
  ctx.fillStyle = '#513841'; ctx.fillRect(x, y, width, 22);
  ctx.fillStyle = color; ctx.fillRect(x, y, Math.max(0, width * fraction), 22);
  ctx.font = 'bold 13px system-ui'; ctx.fillStyle = '#fff0e1'; ctx.textAlign = 'left';
  ctx.fillText(label, x + 8, y + 16);
}

function draw(time) {
  drawBackground(time);
  if (!state.monster) return;
  if (state.mode === 'viewer') {
    drawMonster(W / 2, FLOOR);
    ctx.fillStyle = '#e8d1bd'; ctx.textAlign = 'center'; ctx.font = 'bold 13px system-ui';
    ctx.fillText(state.clip?.state || 'IDLE', W / 2, 30);
  } else {
    drawFields();
    drawMonster(state.foe.x, state.foe.y);
    drawMinions();
    drawStickman(state.hero, time);
    drawShots();
    healthBar(24, 22, 240, state.hero.hp / 100, `HERO  ${state.hero.hp}/100`, '#70baa5');
    healthBar(W - 314, 22, 290, state.foe.hp / state.monster.health,
      `${state.monster.name.toUpperCase()}  ${state.foe.hp}/${state.monster.health}`, '#e77965');
    drawTelegraph();
    if (state.message) {
      ctx.fillStyle = '#120f1ecc'; ctx.fillRect(0, 185, W, 95);
      ctx.fillStyle = '#ffe2b1'; ctx.font = 'bold 29px Georgia'; ctx.textAlign = 'center';
      ctx.fillText(state.message, W / 2, 242);
    }
  }
}

function tick(timestamp) {
  const dt = Math.min(.05, Math.max(0, (timestamp - state.lastTime) / 1000));
  state.lastTime = timestamp;
  state.clipClock += dt * 1000;
  if (state.mode === 'fight') updateFight(dt);
  draw(timestamp / 1000);
  requestAnimationFrame(tick);
}

$('generator').addEventListener('submit', generate);
for (const button of document.querySelectorAll('[data-prompt]')) {
  button.addEventListener('click', () => { $('prompt').value = button.dataset.prompt; $('prompt').focus(); });
}
$('viewer-mode').addEventListener('click', () => setMode('viewer'));
$('fight-mode').addEventListener('click', () => setMode('fight'));
$('reset-fight').addEventListener('click', resetFight);
$('clip').addEventListener('change', (event) => playClip(event.target.value, true));
$('direction').addEventListener('change', (event) => {
  state.viewerMirror = event.target.value === 'mirror0';
  if (!state.viewerMirror) state.direction = Number(event.target.value);
});
canvas.addEventListener('pointerdown', (event) => {
  if (state.mode !== 'fight' || ![0, 2].includes(event.button)) return;
  canvas.focus();
  if (event.button === 0) state.strikeQueued = true;
  else state.fireQueued = true;
  event.preventDefault();
});
canvas.addEventListener('contextmenu', event => { if (state.mode === 'fight') event.preventDefault(); });
window.addEventListener('keydown', (event) => {
  if (state.mode !== 'fight' || ['INPUT', 'SELECT'].includes(document.activeElement.tagName)) return;
  const key = event.key.toLowerCase();
  if ([' ', 'z', 'x', 'c', 'arrowleft', 'arrowright', 'a', 'd'].includes(key)) event.preventDefault();
  if ((key === ' ' || key === 'z') && !event.repeat) state.jumpQueued = true;
  state.keys.add(key);
});
window.addEventListener('keyup', (event) => state.keys.delete(event.key.toLowerCase()));
requestAnimationFrame(tick);
