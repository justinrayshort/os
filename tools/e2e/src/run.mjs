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
const tracePolicy = process.env.OS_E2E_TRACE ?? "off";
const retries = Number.parseInt(process.env.OS_E2E_RETRIES ?? "0", 10);
const slowMoMs = Number.parseInt(process.env.OS_E2E_SLOW_MO_MS ?? "0", 10);

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
const tracesDir = path.join(artifactDir, "traces");
fs.mkdirSync(screenshotsDir, { recursive: true });
fs.mkdirSync(reportsDir, { recursive: true });
fs.mkdirSync(tracesDir, { recursive: true });

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

function shouldCaptureTrace() {
  return tracePolicy !== "off";
}

function shouldKeepTraceOnSuccess() {
  return tracePolicy === "on";
}

function tracePathFor(browserName, scenarioId, attempt, suffix = "") {
  const safeScenarioId = scenarioId.replaceAll(".", "-");
  const safeSuffix = suffix ? `-${suffix}` : "";
  return path.join(
    tracesDir,
    `${browserName}-${safeScenarioId}-attempt-${attempt}${safeSuffix}.zip`,
  );
}

async function createSession(browserType, browserName, scenarioId, attempt, viewport) {
  const browser = await browserType.launch({
    headless,
    slowMo: slowMoMs > 0 ? slowMoMs : undefined,
  });
  const context = await browser.newContext({
    viewport,
  });
  if (shouldCaptureTrace()) {
    await context.tracing.start({ screenshots: true, snapshots: true });
  }
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
  async function finalize({ keepTrace, suffix }) {
    const tracePath = tracePathFor(browserName, scenarioId, attempt, suffix);
    let savedTrace = null;
    try {
      if (shouldCaptureTrace()) {
        if (keepTrace) {
          await context.tracing.stop({ path: tracePath });
          savedTrace = tracePath;
        } else {
          await context.tracing.stop();
        }
      }
    } finally {
      await context.close();
      await browser.close();
    }
    return savedTrace;
  }
  return { page, pageErrors, finalize };
}

async function runShellBoot(browserName, browserType, attempt) {
  const session = await createSession(
    browserType,
    browserName,
    "shell.boot",
    attempt,
    { width: 1366, height: 900 },
  );
  try {
    await session.page.goto(baseUrl, { waitUntil: "networkidle" });
    await session.page.waitForSelector('[data-ui-kind="desktop-backdrop"]', { timeout: 5000 });
    const screenshot = path.join(screenshotsDir, `${browserName}-shell-boot.png`);
    await session.page.screenshot({ path: screenshot, fullPage: true });
    if (session.pageErrors.length > 0) {
      throw new Error(`page errors detected: ${session.pageErrors.join("; ")}`);
    }
    const trace = await session.finalize({ keepTrace: shouldKeepTraceOnSuccess() });
    return { scenario: "shell.boot", browser: browserName, screenshot, trace, attempt };
  } catch (error) {
    await session.finalize({ keepTrace: true, suffix: "failure" });
    throw error;
  }
}

async function runShellSettingsNavigation(browserName, browserType, attempt) {
  const session = await createSession(
    browserType,
    browserName,
    "shell.settings-navigation",
    attempt,
    { width: 1366, height: 900 },
  );
  try {
    await session.page.goto(baseUrl, { waitUntil: "networkidle" });
    await session.page.click('[data-ui-kind="desktop-backdrop"]', { button: "right", timeout: 5000 });
    await session.page.waitForSelector("#desktop-context-menu", { timeout: 2500 });
    await session.page.keyboard.press("ArrowDown");
    await session.page.keyboard.press("Enter");
    await session.page.waitForSelector("text=Personalize your desktop", { timeout: 5000 });
    await session.page.focus('[role="tab"]:has-text("Appearance")');
    await session.page.keyboard.press("Enter");
    await session.page.waitForSelector("text=Choose a shell skin", { timeout: 2500 });
    const screenshot = path.join(
      screenshotsDir,
      `${browserName}-shell-settings-navigation.png`,
    );
    await session.page.screenshot({ path: screenshot, fullPage: true });
    const trace = await session.finalize({ keepTrace: shouldKeepTraceOnSuccess() });
    return {
      scenario: "shell.settings-navigation",
      browser: browserName,
      screenshot,
      trace,
      attempt,
    };
  } catch (error) {
    await session.finalize({ keepTrace: true, suffix: "failure" });
    throw error;
  }
}

async function runKeyboardSmoke(browserName, browserType, attempt) {
  const skins = ["soft-neumorphic", "modern-adaptive", "classic-xp", "classic-95"];
  const results = [];
  const traces = [];

  for (const skin of skins) {
    const failures = [];
    const session = await createSession(
      browserType,
      browserName,
      "ui.keyboard-smoke",
      attempt,
      { width: 1366, height: 900 },
    );

    try {
      await session.page.goto(baseUrl, { waitUntil: "networkidle" });
      await applySkin(session.page, skin);

      try {
        await session.page.click('[data-ui-kind="desktop-backdrop"]', {
          button: "right",
          timeout: 3000,
        });
        await session.page.waitForSelector("#desktop-context-menu", { timeout: 1500 });
      } catch {
        failures.push("context menu did not open");
      }

      try {
        await session.page.keyboard.press("ArrowDown");
        await session.page.keyboard.press("Enter");
        await session.page.waitForSelector("text=Personalize your desktop", { timeout: 2500 });
      } catch {
        failures.push("system settings did not open from keyboard flow");
      }

      try {
        await session.page.focus('[role="tab"]:has-text("Appearance")');
        await session.page.keyboard.press("Enter");
        await session.page.waitForSelector("text=Choose a shell skin", { timeout: 1500 });
      } catch {
        failures.push("appearance tab keyboard traversal failed");
      }

      try {
        await session.page.focus('[role="tab"]:has-text("Accessibility")');
        await session.page.keyboard.press("Enter");
        await session.page.waitForSelector("text=High contrast", { timeout: 1500 });
      } catch {
        failures.push("accessibility keyboard traversal failed");
      }

      const screenshot = path.join(
        screenshotsDir,
        `${browserName}-${skin}-keyboard-smoke.png`,
      );
      await session.page.screenshot({ path: screenshot, fullPage: true });
      const trace = await session.finalize({
        keepTrace: shouldKeepTraceOnSuccess(),
        suffix: skin,
      });
      if (trace) {
        traces.push(trace);
      }
      results.push({ skin, screenshot, failures, trace });
    } catch (error) {
      const trace = await session.finalize({ keepTrace: true, suffix: `${skin}-failure` });
      if (trace) {
        traces.push(trace);
      }
      throw error;
    }
  }

  const failed = results.filter((entry) => entry.failures.length > 0);
  if (failed.length > 0) {
    throw new Error(
      `ui.keyboard-smoke failed for ${failed.length} skin(s): ${failed
        .map((entry) => `${entry.skin}: ${entry.failures.join(", ")}`)
        .join("; ")}`,
    );
  }

  return { scenario: "ui.keyboard-smoke", browser: browserName, results, traces, attempt };
}

async function runScreenshotMatrix(browserName, browserType, attempt) {
  const skins = ["soft-neumorphic", "modern-adaptive", "classic-xp", "classic-95"];
  const viewports = [
    { name: "desktop", width: 1440, height: 900 },
    { name: "tablet", width: 1024, height: 768 },
    { name: "mobile", width: 390, height: 844 },
  ];
  const session = await createSession(
    browserType,
    browserName,
    "ui.screenshot-matrix",
    attempt,
    { width: 1440, height: 900 },
  );
  const captures = [];

  try {
    for (const skin of skins) {
      for (const viewport of viewports) {
        await session.page.setViewportSize({ width: viewport.width, height: viewport.height });
        await session.page.goto(baseUrl, { waitUntil: "networkidle" });
        await applySkin(session.page, skin);
        const screenshot = path.join(
          screenshotsDir,
          `${browserName}-${skin}-${viewport.name}.png`,
        );
        await session.page.screenshot({ path: screenshot, fullPage: true });
        captures.push({ skin, viewport: viewport.name, screenshot });
      }
    }
    const trace = await session.finalize({ keepTrace: shouldKeepTraceOnSuccess() });
    return { scenario: "ui.screenshot-matrix", browser: browserName, captures, trace, attempt };
  } catch (error) {
    await session.finalize({ keepTrace: true, suffix: "failure" });
    throw error;
  }
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

async function runScenarioWithRetries(browserName, browserType, scenarioId) {
  const handler = scenarioHandlers[scenarioId];
  if (!handler) {
    throw new Error(`unsupported scenario '${scenarioId}'`);
  }

  let lastError = null;
  for (let attempt = 1; attempt <= retries + 1; attempt += 1) {
    try {
      const result = await handler(browserName, browserType, attempt);
      return {
        ...result,
        attempts: attempt,
        retriesConfigured: retries,
      };
    } catch (error) {
      lastError = error;
      if (attempt > retries) {
        throw new Error(
          `scenario '${scenarioId}' on browser '${browserName}' failed after ${attempt} attempt(s): ${String(error.stack ?? error)}`,
        );
      }
      console.warn(
        `retrying ${scenarioId} on ${browserName} after attempt ${attempt} failed: ${String(error.message ?? error)}`,
      );
    }
  }

  throw lastError;
}

async function main() {
  const report = {
    profile,
    baseUrl,
    browsers,
    headless,
    tracePolicy,
    retries,
    slowMoMs,
    scenarioIds,
    results: [],
  };

  for (const browserName of browsers) {
    const browserType = browserTypes[browserName];
    if (!browserType) {
      throw new Error(`unsupported browser '${browserName}'`);
    }

    for (const scenarioId of scenarioIds) {
      report.results.push(await runScenarioWithRetries(browserName, browserType, scenarioId));
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
