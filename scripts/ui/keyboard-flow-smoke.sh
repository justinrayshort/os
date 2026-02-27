#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:1420}"
OUT_DIR="${2:-.artifacts/ui-conformance/keyboard}"

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to run keyboard smoke automation" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

BASE_URL="$BASE_URL" OUT_DIR="$OUT_DIR" node <<'NODE'
const fs = require('fs');
const path = require('path');

const outDir = process.env.OUT_DIR;
const baseUrl = process.env.BASE_URL;

async function runSkin(page, skin) {
  const failures = [];

  await page.goto(baseUrl, { waitUntil: 'networkidle' });
  await page.evaluate((requestedSkin) => {
    const nextTheme = {
      skin: requestedSkin,
      wallpaper_id: 'cloud-bands',
      high_contrast: false,
      reduced_motion: false,
      audio_enabled: true,
    };
    localStorage.setItem('retrodesk.theme.v1', JSON.stringify(nextTheme));
  }, skin);
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForTimeout(300);

  try {
    await page.click('.desktop-wallpaper', { button: 'right' });
    await page.waitForSelector('.desktop-context-menu', { timeout: 1500 });
  } catch (error) {
    failures.push('context menu did not open');
  }

  try {
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.display-properties-dialog', { timeout: 1500 });
  } catch (error) {
    failures.push('display properties did not open from keyboard flow');
  }

  try {
    await page.focus('#display-properties-tab-appearance');
    await page.keyboard.press('Enter');
    await page.waitForTimeout(100);
    await page.focus('#skin-listbox');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await page.focus('#wallpaper-listbox');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
  } catch (error) {
    failures.push('tab/listbox keyboard traversal failed');
  }

  try {
    await page.focus('#display-properties-cancel-button');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.display-properties-dialog', { state: 'detached', timeout: 1500 });
  } catch (error) {
    failures.push('cancel flow did not close display properties');
  }

  const screenshotPath = path.join(outDir, `${skin}-keyboard-smoke.png`);
  await page.screenshot({ path: screenshotPath, fullPage: true });

  return {
    skin,
    ok: failures.length === 0,
    failures,
    screenshot: screenshotPath,
  };
}

async function main() {
  let chromium;
  try {
    ({ chromium } = require('playwright'));
  } catch (error) {
    console.error('playwright npm package is required (npm i -D playwright)');
    process.exit(1);
  }

  const skins = ['modern-adaptive', 'classic-xp', 'classic-95'];
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1366, height: 900 } });
  const page = await context.newPage();

  const results = [];
  for (const skin of skins) {
    results.push(await runSkin(page, skin));
  }

  await browser.close();

  const reportPath = path.join(outDir, 'keyboard-smoke-report.json');
  fs.writeFileSync(reportPath, JSON.stringify(results, null, 2) + '\n');

  const failed = results.filter((entry) => !entry.ok);
  if (failed.length > 0) {
    console.error(`keyboard flow smoke failed for ${failed.length} skin(s)`);
    process.exit(1);
  }

  console.log(`wrote ${reportPath}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE
