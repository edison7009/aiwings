// Steps demo.html frame by frame (render(t) is a pure function of time) and saves PNGs.
// Usage: node record.cjs <zh|en> <outDir> [fps]     (needs the `playwright` package)
// Then:  python3 make_gif.py <outDir> <out.gif> [fps]
const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

(async () => {
  const [lang = 'zh', out = 'frames', fpsArg = '15'] = process.argv.slice(2);
  const fps = Number(fpsArg);
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {});
  const page = await browser.newPage({ viewport: { width: 960, height: 600 }, deviceScaleFactor: 1 });
  await page.goto('file://' + path.join(__dirname, 'demo.html') + '?lang=' + lang);
  await page.waitForLoadState('networkidle');
  const end = await page.evaluate(() => window.T_END);
  const frames = Math.round(end * fps);
  for (let i = 0; i < frames; i++) {
    await page.evaluate(t => window.render(t), i / fps);
    await page.screenshot({ path: path.join(out, String(i).padStart(4, '0') + '.png') });
  }
  await browser.close();
  console.log(`${frames} frames → ${out}`);
})();
