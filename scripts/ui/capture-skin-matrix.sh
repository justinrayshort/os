#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:1420}"
OUT_DIR="${2:-.artifacts/ui-conformance/screenshots}"

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to run screenshot automation" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

BASE_URL="$BASE_URL" OUT_DIR="$OUT_DIR" node <<'NODE'
const fs = require('fs');
const path = require('path');

const outDir = process.env.OUT_DIR;
const baseUrl = process.env.BASE_URL;

async function main() {
  let chromium;
  try {
    ({ chromium } = require('playwright'));
  } catch (error) {
    console.error('playwright npm package is required (npm i -D playwright)');
    process.exit(1);
  }

  const skins = ['modern-adaptive', 'classic-xp', 'classic-95'];
  const viewports = [
    { name: 'desktop', width: 1440, height: 900 },
    { name: 'tablet', width: 1024, height: 768 },
    { name: 'mobile', width: 390, height: 844 },
  ];

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  for (const skin of skins) {
    for (const viewport of viewports) {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
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
      await page.waitForTimeout(250);

      const output = path.join(outDir, `${skin}-${viewport.name}.png`);
      await page.screenshot({ path: output, fullPage: true });
      console.log(`captured ${output}`);
    }
  }

  await browser.close();
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE
