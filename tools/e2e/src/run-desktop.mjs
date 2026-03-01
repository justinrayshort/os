import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { Builder, By, Capabilities, until } from "selenium-webdriver";

const profile = process.env.OS_E2E_PROFILE ?? "unknown";
const artifactDir = process.env.OS_E2E_ARTIFACT_DIR;
const scenarioIds = (process.env.OS_E2E_SCENARIO_IDS ?? "")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);
const driverUrl = process.env.OS_E2E_TAURI_DRIVER_URL;
const desktopBinary = process.env.OS_E2E_DESKTOP_BINARY;
const retries = Number.parseInt(process.env.OS_E2E_RETRIES ?? "0", 10);

if (!artifactDir) {
  console.error("OS_E2E_ARTIFACT_DIR is required");
  process.exit(1);
}

if (!driverUrl) {
  console.error("OS_E2E_TAURI_DRIVER_URL is required");
  process.exit(1);
}

if (!desktopBinary) {
  console.error("OS_E2E_DESKTOP_BINARY is required");
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

async function createDriver() {
  const capabilities = Capabilities.firefox();
  capabilities.setBrowserName("wry");
  capabilities.set("tauri:options", {
    application: desktopBinary,
  });

  const driver = await new Builder()
    .usingServer(driverUrl)
    .withCapabilities(capabilities)
    .build();

  return driver;
}

async function runDesktopBoot(attempt) {
  const driver = await createDriver();
  try {
    await driver.wait(until.titleContains("Retro Desktop OS"), 15000);
    await driver.wait(
      until.elementLocated(By.css('[data-ui-kind="desktop-backdrop"]')),
      10000,
    );
    const screenshot = path.join(screenshotsDir, "desktop-boot.png");
    const image = await driver.takeScreenshot();
    fs.writeFileSync(screenshot, image, "base64");
    return {
      scenario: "desktop.boot",
      screenshot,
      attempt,
    };
  } finally {
    await driver.quit();
  }
}

async function runDesktopSettingsNavigation(attempt) {
  const driver = await createDriver();
  try {
    const backdrop = await driver.wait(
      until.elementLocated(By.css('[data-ui-kind="desktop-backdrop"]')),
      10000,
    );
    await driver.actions({ bridge: true }).contextClick(backdrop).perform();
    await driver.wait(until.elementLocated(By.css("#desktop-context-menu")), 5000);
    await driver.actions().sendKeys("\uE015", "\uE007").perform();
    await driver.wait(
      until.elementLocated(By.xpath("//*[contains(text(),'Personalize your desktop')]")),
      10000,
    );
    const appearanceTab = await driver.findElement(
      By.xpath("//*[@role='tab' and contains(., 'Appearance')]"),
    );
    await appearanceTab.click();
    await driver.wait(
      until.elementLocated(By.xpath("//*[contains(text(),'Choose a shell skin')]")),
      5000,
    );
    const screenshot = path.join(screenshotsDir, "desktop-settings-navigation.png");
    const image = await driver.takeScreenshot();
    fs.writeFileSync(screenshot, image, "base64");
    return {
      scenario: "desktop.settings-navigation",
      screenshot,
      attempt,
    };
  } finally {
    await driver.quit();
  }
}

const scenarioHandlers = {
  "desktop.boot": runDesktopBoot,
  "desktop.settings-navigation": runDesktopSettingsNavigation,
};

async function runScenarioWithRetries(scenarioId) {
  const handler = scenarioHandlers[scenarioId];
  if (!handler) {
    throw new Error(`unsupported desktop scenario '${scenarioId}'`);
  }

  let lastError = null;
  for (let attempt = 1; attempt <= retries + 1; attempt += 1) {
    try {
      const result = await handler(attempt);
      return {
        ...result,
        attempts: attempt,
        retriesConfigured: retries,
      };
    } catch (error) {
      lastError = error;
      if (attempt > retries) {
        throw new Error(
          `desktop scenario '${scenarioId}' failed after ${attempt} attempt(s): ${String(error.stack ?? error)}`,
        );
      }
      console.warn(
        `retrying desktop scenario ${scenarioId} after attempt ${attempt} failed: ${String(error.message ?? error)}`,
      );
    }
  }

  throw lastError;
}

async function main() {
  const report = {
    profile,
    driverUrl,
    desktopBinary,
    retries,
    scenarioIds,
    results: [],
  };

  for (const scenarioId of scenarioIds) {
    report.results.push(await runScenarioWithRetries(scenarioId));
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
