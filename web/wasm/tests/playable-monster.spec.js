import {test,expect} from '@playwright/test';
test('real generated monster can move, exercise animations, damage a dummy and play licensed audio',async({page})=>{
  test.setTimeout(300000);
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto('/play/');
  await page.locator('#prompt').fill('a goblin with claws');
  await page.locator('#profile').selectOption('v2');
  await page.locator('#generate').click();
  await expect(page.locator('#status')).toHaveText('Ready — click the room to play',{timeout:220000});
  const canvas=page.locator('#arena');
  await expect(page.locator('#animations button').first()).toBeVisible();
  const initial=Number(await canvas.getAttribute('data-x'));
  await page.locator('#prompt').focus();
  await page.keyboard.down('d');await page.waitForTimeout(250);await page.keyboard.up('d');
  expect(Number(await canvas.getAttribute('data-x'))).toBe(initial);
  await canvas.focus();await page.keyboard.down('d');
  await page.waitForFunction(()=>Number(document.querySelector('#arena').dataset.x)>=695);
  await page.keyboard.up('d');
  expect(Number(await canvas.getAttribute('data-x'))).toBeGreaterThan(initial+300);
  // The actual generated attack clips, not an artificial damage hook.
  const attack=page.locator('#animations button').filter({hasText:/attack_primary$/i}).first();
  await expect(attack).toBeVisible();await attack.click();
  await expect(page.locator('#damage')).not.toContainText('Total damage 0',{timeout:15000});
  await page.locator('#reset').click();await expect(page.locator('#damage')).toContainText('Total damage 0');
  for(const button of await page.locator('#animations button').all()){
    const id=await button.getAttribute('data-animation-id');
    await button.click();await expect(canvas).toHaveAttribute('data-animation',id);
    await page.locator('#stop').click();
    await page.keyboard.press(await button.getAttribute('data-shortcut'));
    await expect(canvas).toHaveAttribute('data-animation',id);
    await page.locator('#stop').click();
  }
  // A real gesture unlocks Web Audio. No oscillator/test-only sound or mocked
  // decode is used: diagnostics increment only after a checked source has been
  // decoded, processed through OfflineAudioContext and started for playback.
  const catalog=await page.evaluate(async()=>{
    const response=await fetch('./audio/catalog.json');
    if(!response.ok)throw new Error(`Missing audio catalog: ${response.status}`);
    return response.json();
  });
  expect(catalog.schema_version).toBe(1);
  const byId=new Map(catalog.assets.map(asset=>[asset.id,asset]));
  await page.locator('#audio-enable').check();
  await expect(page.locator('#audio-status')).toContainText('Sound on');
  const death=page.locator('#animations button').filter({hasText:/death$/i}).first();
  const hurt=page.locator('#animations button').filter({hasText:/hurt$/i}).first();
  await expect(death).toBeVisible();
  await expect(hurt).toBeVisible();
  const played=()=>page.evaluate(()=>window.__audioDiagnostics.played);
  const assertRendered=async(before)=>{
    await expect.poll(played,{timeout:20000}).toBeGreaterThan(before);
    const diagnostics=await page.evaluate(()=>({...window.__audioDiagnostics}));
    await expect.poll(()=>page.evaluate(()=>window.__audioDiagnostics.outputRmsMax),{timeout:5000}).toBeGreaterThan(0);
    expect(diagnostics.decodedCount).toBeGreaterThan(0);
    expect(diagnostics.peak).toBeGreaterThan(0);
    expect(diagnostics.peak).toBeLessThanOrEqual(.320001);
    expect(diagnostics.active).toBeLessThanOrEqual(3);
    expect(diagnostics.selectedSourceIds.length).toBeGreaterThan(0);
    for(const id of diagnostics.selectedSourceIds){
      const asset=byId.get(id);
      expect(asset,`unknown selected source ${id}`).toBeTruthy();
      expect(asset.verification.source_page_verified).toBe(true);
      expect(asset.verification.blocked).not.toBe(true);
      expect(asset.verification.sha256).toMatch(/^[a-f0-9]{64}$/);
    }
    return diagnostics;
  };
  let before=await played();
  await death.click();
  await assertRendered(before);
  // Source-level cooldown is shared even across transformed variants.
  await page.waitForTimeout(4100);
  await page.locator('#stop').click();
  before=await played();
  await hurt.click();
  await assertRendered(before);

  await page.locator('#license-mode').selectOption('no-attribution');
  await expect.poll(()=>page.evaluate(()=>window.__audioDiagnostics.mode)).toBe('no-attribution');
  expect(await played()).toBe(0);
  await page.locator('#stop').click();
  await death.click();
  const filtered=await assertRendered(0);
  const allowedLicenses=['CC0-1.0','CC0','PD-self','PD-author','Public domain'];
  for(const id of filtered.selectedSourceIds){
    const asset=byId.get(id);
    expect(asset.attribution_required).toBe(false);
    expect(allowedLicenses).toContain(asset.license_id);
  }
  await page.locator('#audio-enable').uncheck();
  await expect.poll(()=>page.evaluate(()=>window.__audioDiagnostics.active)).toBe(0);
  before=await played();
  await page.locator('#stop').click();
  await hurt.click();
  await page.waitForTimeout(500);
  expect(await played()).toBe(before);
  await page.screenshot({path:test.info().outputPath('playable-monster.png'),fullPage:true});
  expect(errors).toEqual([]);
});
