import { test, expect } from '../wasm/node_modules/@playwright/test/index.mjs';

test('local embedded BERT generates a V2 shade and plays the selected baked animations', async ({ page }, testInfo) => {
  await page.clock.install();
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.drawImage;
    CanvasRenderingContext2D.prototype.drawImage = function(image, ...args) {
      if (image instanceof HTMLImageElement && image.src.includes('sprites.png')) window.lastSpriteDraw = args;
      return original.call(this, image, ...args);
    };
  });
  await page.goto('/');
  await expect(page.locator('#parser-status')).toContainText('embedded BERT');
  await page.locator('#prompt').fill('inky shade with tendrils');
  await page.locator('#profile').selectOption('v2');
  const responsePromise = page.waitForResponse(response => response.url().endsWith('/api/generate'));
  await page.locator('#generate').click();
  const response = await responsePromise;
  expect(response.ok()).toBe(true);
  const monster = await response.json();
  expect(monster.parser).toMatch(/^candle-bert/);
  await expect(page.locator('#status')).toContainText('animations are ready');
  const attack = monster.animations.find(clip => clip.state === 'ATTACK' && !clip.loop);
  const death = monster.animations.find(clip => clip.state === 'DEATH');
  const loop = monster.animations.find(clip => clip.loop && clip.frames.length > 1);
  expect(attack).toBeTruthy();
  expect(death).toBeTruthy();
  expect(loop).toBeTruthy();

  await page.locator('#clip').selectOption(attack.id);
  await page.clock.runFor(160);
  await page.locator('#play-pause').click();
  const paused = await page.evaluate(() => ({ clock: state.clipClock, frame: currentFrame() }));
  await page.clock.runFor(500);
  expect(await page.evaluate(() => state.clipClock)).toBe(paused.clock);
  await expect(page.locator('#play-pause')).toHaveText('Play');
  await page.locator('#direction').selectOption('1');
  await page.clock.runFor(32);
  const id = paused.frame + monster.sprites.stride;
  const draw = await page.evaluate(() => window.lastSpriteDraw);
  expect(draw[0]).toBe((id % monster.sprites.columns) * monster.sprites.frameWidth);
  expect(draw[1]).toBe(Math.floor(id / monster.sprites.columns) * monster.sprites.frameHeight);
  await page.screenshot({ path: testInfo.outputPath('local-bert-playground.png'), fullPage: true });

  await page.locator('#restart-animation').click();
  expect(await page.evaluate(() => state.clipClock)).toBe(0);
  await expect(page.locator('#play-pause')).toHaveText('Pause');
  await page.locator('#playback-speed').selectOption('2');
  await page.clock.runFor(160);
  expect(await page.evaluate(() => state.clipClock)).toBeGreaterThanOrEqual(300);
  await page.locator('#playback-speed').selectOption('1');

  for (const clip of [attack, death]) {
    await page.locator('#clip').selectOption(clip.id);
    const length = clip.frames.reduce((sum, frame) => sum + Math.max(1, frame.duration), 0);
    await page.clock.runFor(length + 100);
    expect(await page.evaluate(() => currentFrame())).toBe(clip.frames.at(-1).id);
    await expect(page.locator('#play-pause')).toHaveText('Play');
    await page.clock.runFor(500);
    expect(await page.evaluate(() => currentFrame())).toBe(clip.frames.at(-1).id);
  }
  await page.locator('#clip').selectOption(loop.id);
  const length = loop.frames.reduce((sum, frame) => sum + Math.max(1, frame.duration), 0);
  await page.clock.runFor(length + 32);
  await expect(page.locator('#play-pause')).toHaveText('Pause');
  expect(await page.evaluate(() => currentFrame())).toBe(loop.frames[0].id);
  await page.locator('#direction').selectOption('mirror0');
  await page.locator('#fight-mode').click();
  await page.locator('#viewer-mode').click();
  expect(await page.evaluate(() => ({ direction: state.direction, mirror: state.viewerMirror })))
    .toEqual({ direction: 0, mirror: true });
});
