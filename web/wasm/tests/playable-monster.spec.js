import {test,expect} from '@playwright/test';
test('real generated monster can move, exercise animations and damage a dummy',async({page})=>{
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
  await page.screenshot({path:test.info().outputPath('playable-monster.png'),fullPage:true});
  expect(errors).toEqual([]);
});
