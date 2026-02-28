import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { chromium, firefox, webkit } from "playwright";

const profile = process.env.OS_E2E_PROFILE ?? "unknown";
const baseUrl = process.env.OS_E2E_BASE_URL;
const artifactDir = process.env.OS_E2E_ARTIFACT_DIR;
const scenarioIds = (process.env.OS_E2E_SCENARIO_IDS ?? "")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);
const browsers = (process.env.OS_E2E_BROWSERS ?? "chromium")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);
const headless = (process.env.OS_E2E_HEADLESS ?? "true") === "true";

if (!baseUrl) {
  console.error("OS_E2E_BASE_URL is required");
  process.exit(1);
}

if (!artifactDir) {
  console.error("OS_E2E_ARTIFACT_DIR is required");
  process.exit(1);
}

if (scenarioIds.length === 0) {
  console.error("OS_E2E_SCENARIO_IDS is required");
  process.exit(1);
}

const screenshotsDir = path.join(artifactDir, "screenshots");
const reportsDir = path.join(artifactDir, "reports");
fs.mkdirSync(screenshotsDir, { recursive: true });
fs.mkdirSync(reportsDir, { recursive: true });

function themeForSkin(skin) {
  return {
    skin,
    wallpaper_id: "cloud-bands",
    high_contrast: false,
    reduced_motion: false,
    audio_enabled: true,
  };
}

async function applySkin(page, skin) {
  await page.evaluate((requestedSkin) => {
    localStorage.removeItem("retrodesk.layout.v1");
    localStorage.removeItem("retrodesk.terminal_history.v1");
    localStorage.setItem("retrodesk.theme.v1", JSON.stringify({
      skin: requestedSkin,
      wallpaper_id: "cloud-bands",
      high_contrast: false,
      reduced_motion: false,
      audio_enabled: true,
    }));
  }, skin);
  await page.reload({ waitUntil: "networkidle" });
  await page.waitForTimeout(300);
}

async function createPage(browserType) {
  const browser = await browserType.launch({ headless });
  const context = await browser.newContext({
    viewport: { width: 1366, height: 900 },
  });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => {
    pageErrors.push(String(error));
  });
  page.on("console", (message) => {
    if (message.type() === "error") {
      pageErrors.push(message.text());
    }
  });
  return { browser, context, page, pageErrors };
}

async function runShellBoot(browserName, browserType) {
  const { browser, context, page, pageErrors } = await createPage(browserType);
  try {
    await page.goto(baseUrl, { waitUntil: "networkidle" });
    await page.waitForSelector('[data-ui-kind="desktop-backdrop"]', { timeout: 5000 });
    const screenshot = path.join(screenshotsDir, `${browserName}-shell-boot.png`);
    await page.screenshot({ path: screenshot, fullPage: true });
    if (pageErrors.length > 0) {
      throw new Error(`page errors detected: ${pageErrors.join("; ")}`);
    }
    return { scenario: "shell.boot", browser: browserName, screenshot };
  } finally {
    await context.close();
    await browser.close();
  }
}

async function runShellSettingsNavigation(browserName, browserType) {
  const { browser, context, page } = await createPage(browserType);
  try {
    await page.goto(baseUrl, { waitUntil: "networkidle" });
    await page.click('[data-ui-kind="desktop-backdrop"]', { button: "right", timeout: 5000 });
    await page.waitForSelector("#desktop-context-menu", { timeout: 2500 });
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await page.waitForSelector("text=Personalize your desktop", { timeout: 5000 });
    await page.focus('[role="tab"]:has-text("Appearance")');
    await page.keyboard.press("Enter");
    await page.waitForSelector("text=Choose a shell skin", { timeout: 2500 });
    const screenshot = path.join(
      screenshotsDir,
      `${browserName}-shell-settings-navigation.png`,
    );
    await page.screenshot({ path: screenshot, fullPage: true });
    return {
      scenario: "shell.settings-navigation",
      browser: browserName,
      screenshot,
    };
  } finally {
    await context.close();
    await browser.close();
  }
}

async function runKeyboardSmoke(browserName, browserType) {
  const skins = ["soft-neumorphic", "modern-adaptive", "classic-xp", "classic-95"];
  const results = [];
  const browser = await browserType.launch({ headless });

  try {
    for (const skin of skins) {
      const failures = [];
      const context = await browser.newContext({
        viewport: { width: 1366, height: 900 },
      });
      const page = await context.newPage();

      try {
        await page.goto(baseUrl, { waitUntil: "networkidle" });
        await applySkin(page, skin);

        try {
          await page.click('[data-ui-kind="desktop-backdrop"]', {
            button: "right",
            timeout: 3000,
          });
          await page.waitForSelector("#desktop-context-menu", { timeout: 1500 });
        } catch {
          failures.push("context menu did not open");
        }

        try {
          await page.keyboard.press("ArrowDown");
          await page.keyboard.press("Enter");
          await page.waitForSelector("text=Personalize your desktop", { timeout: 2500 });
        } catch {
          failures.push("system settings did not open from keyboard flow");
        }

        try {
          await page.focus('[role="tab"]:has-text("Appearance")');
          await page.keyboard.press("Enter");
          await page.waitForSelector("text=Choose a shell skin", { timeout: 1500 });
        } catch {
          failures.push("appearance tab keyboard traversal failed");
        }

        try {
          await page.focus('[role="tab"]:has-text("Accessibility")');
          await page.keyboard.press("Enter");
          await page.waitForSelector("text=High contrast", { timeout: 1500 });
        } catch {
          failures.push("accessibility keyboard traversal failed");
        }

        const screenshot = path.join(
          screenshotsDir,
          `${browserName}-${skin}-keyboard-smoke.png`,
        );
        await page.screenshot({ path: screenshot, fullPage: true });
        results.push({ skin, screenshot, failures });
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
  }

  const failed = results.filter((entry) => entry.failures.length > 0);
  if (failed.length > 0) {
    throw new Error(
      `ui.keyboard-smoke failed for ${failed.length} skin(s): ${failed
        .map((entry) => `${entry.skin}: ${entry.failures.join(", ")}`)
        .join("; ")}`,
    );
  }

  return { scenario: "ui.keyboard-smoke", browser: browserName, results };
}

async function runScreenshotMatrix(browserName, browserType) {
  const skins = ["soft-neumorphic", "modern-adaptive", "classic-xp", "classic-95"];
  const viewports = [
    { name: "desktop", width: 1440, height: 900 },
    { name: "tablet", width: 1024, height: 768 },
    { name: "mobile", width: 390, height: 844 },
  ];
  const { browser, context, page } = await createPage(browserType);
  const captures = [];

  try {
    for (const skin of skins) {
      for (const viewport of viewports) {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await page.goto(baseUrl, { waitUntil: "networkidle" });
        await applySkin(page, skin);
        const screenshot = path.join(
          screenshotsDir,
          `${browserName}-${skin}-${viewport.name}.png`,
        );
        await page.screenshot({ path: screenshot, fullPage: true });
        captures.push({ skin, viewport: viewport.name, screenshot });
      }
    }
  } finally {
    await context.close();
    await browser.close();
  }

  return { scenario: "ui.screenshot-matrix", browser: browserName, captures };
}

const browserTypes = {
  chromium,
  firefox,
  webkit,
};

const scenarioHandlers = {
  "shell.boot": runShellBoot,
  "shell.settings-navigation": runShellSettingsNavigation,
  "ui.keyboard-smoke": runKeyboardSmoke,
  "ui.screenshot-matrix": runScreenshotMatrix,
};

async function main() {
  const report = {
    profile,
    baseUrl,
    browsers,
    headless,
    scenarioIds,
    results: [],
  };

  for (const browserName of browsers) {
    const browserType = browserTypes[browserName];
    if (!browserType) {
      throw new Error(`unsupported browser '${browserName}'`);
    }

    for (const scenarioId of scenarioIds) {
      const handler = scenarioHandlers[scenarioId];
      if (!handler) {
        throw new Error(`unsupported scenario '${scenarioId}'`);
      }
      report.results.push(await handler(browserName, browserType));
    }
  }

  const reportPath = path.join(reportsDir, "report.json");
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`wrote ${reportPath}`);
}

main().catch((error) => {
  const failurePath = path.join(reportsDir, "failure.txt");
  fs.writeFileSync(failurePath, `${String(error.stack ?? error)}\n`);
  console.error(error);
  process.exit(1);
});
