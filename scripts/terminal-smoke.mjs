// Development-preview UI checks only; all data comes from synthetic fixtures.
import {createRequire} from 'node:module';
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
const {chromium}=createRequire(import.meta.url)(process.env.GCD_PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1280,height:900}});
const errors=[];page.on('pageerror',e=>errors.push(e.message));
try {
 await page.goto('http://127.0.0.1:1420/?preview=1');await page.locator('.quota-card').first().waitFor();await page.waitForFunction(()=>!document.body.innerText.includes('Updating statistics'));
 await mkdir('.local-test/v05/screens',{recursive:true});
 assert.equal(await page.locator('nav button').count(),4);
 assert.equal(await page.getByRole('button',{name:'Browser chats'}).count(),0);
 assert.equal(await page.getByLabel('Remaining requests unavailable').textContent(),'—');
 assert.ok((await page.locator('.pro-allowance').innerText()).includes('170'));
 await page.screenshot({path:'.local-test/v05/screens/overview.png',fullPage:true});
 await page.locator('nav').getByRole('button',{name:/History/}).click();await page.locator('.log-line').first().waitFor();
 for(const width of [1280,1100,1000,900,760]){
  await page.setViewportSize({width,height:900});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`No page overflow at ${width}`);
  assert.ok(await page.locator('.log-line').first().evaluate(e=>e.getBoundingClientRect().height<40));
  assert.equal(await page.locator('.prompt-link').first().evaluate(e=>getComputedStyle(e).whiteSpace),'nowrap');
 }
 for(const theme of ['black','light','slate','midnight']){
  await page.evaluate(theme=>{document.documentElement.dataset.theme=theme;document.documentElement.style.setProperty('--font-scale','1.6');},theme);
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`Font 160% layout in ${theme}`);
 }
 await page.evaluate(()=>{document.documentElement.dataset.theme='black';document.documentElement.style.setProperty('--font-scale','1.2');});
 await page.setViewportSize({width:1280,height:900});await page.screenshot({path:'.local-test/v05/screens/history.png'});
 assert.deepEqual(errors,[]);
 await writeFile('.local-test/v05/ui-report.json',JSON.stringify({singleLineHistory:true,maxFontAndThemes:true,manualBrowserUIRemoved:true,proRemainingUnknown:true,responsiveWidths:[760,900,1000,1100,1280],errors},null,2));
 console.log('Terminal preview: compact rows, responsive layout, Pro unknown state and retired browser navigation passed.');
}finally{await browser.close()}