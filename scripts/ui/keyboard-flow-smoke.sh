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

async function runSkin(browser, skin) {
  const context = await browser.newContext({ viewport: { width: 1366, height: 900 } });
  const page = await context.newPage();
  const failures = [];

  try {
    await page.goto(baseUrl, { waitUntil: 'networkidle' });
    await page.evaluate((requestedSkin) => {
      localStorage.removeItem('retrodesk.layout.v1');
      localStorage.removeItem('retrodesk.terminal_history.v1');
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
      await page.click('[data-ui-kind="desktop-backdrop"]', { button: 'right', timeout: 3000 });
      await page.waitForSelector('#desktop-context-menu', { timeout: 1500 });
    } catch (error) {
      failures.push('context menu did not open');
    }

    try {
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('Enter');
      await page.waitForSelector('text=Personalize your desktop', { timeout: 2500 });
    } catch (error) {
      failures.push('system settings did not open from keyboard flow');
    }

    try {
      await page.focus('[role="tab"]:has-text("Appearance")');
      await page.keyboard.press('Enter');
      await page.waitForSelector('text=Choose a shell skin', { timeout: 1500 });
      await page.focus('button:has-text("Soft Neumorphic")');
      await page.keyboard.press('Enter');
    } catch (error) {
      failures.push('appearance tab keyboard traversal failed');
    }

    try {
      await page.focus('[role="tab"]:has-text("Accessibility")');
      await page.keyboard.press('Enter');
      await page.waitForSelector('text=High contrast', { timeout: 1500 });
      await page.focus('input[aria-label="High contrast"]');
      await page.keyboard.press('Space');
      await page.waitForTimeout(150);
    } catch (error) {
      failures.push('accessibility keyboard toggle failed');
    }

    const screenshotPath = path.join(outDir, `${skin}-keyboard-smoke.png`);
    await page.screenshot({ path: screenshotPath, fullPage: true });

    return {
      skin,
      ok: failures.length === 0,
      failures,
      screenshot: screenshotPath,
    };
  } finally {
    await context.close();
  }
}

async function main() {
  let chromium;
  try {
    ({ chromium } = require('playwright'));
  } catch (error) {
    console.error('playwright npm package is required (npm i -D playwright)');
    process.exit(1);
  }

  const skins = ['soft-neumorphic', 'modern-adaptive', 'classic-xp', 'classic-95'];
  const browser = await chromium.launch({ headless: true });

  const results = [];
  for (const skin of skins) {
    results.push(await runSkin(browser, skin));
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
