import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { chromium, firefox, webkit } from "playwright";

const profile = process.env.OS_E2E_PROFILE ?? "unknown";
const baseUrl = process.env.OS_E2E_BASE_URL;
const artifactDir = process.env.OS_E2E_ARTIFACT_DIR;
const baselineRoot = process.env.OS_E2E_BASELINE_ROOT;
const manifestPath = process.env.OS_E2E_MANIFEST_PATH;
const scenarioIds = splitCsv(process.env.OS_E2E_SCENARIO_IDS);
const sliceFilter = process.env.OS_E2E_SLICE_ID?.trim() || null;
const browsers = splitCsv(process.env.OS_E2E_BROWSERS || "chromium");
const headless = (process.env.OS_E2E_HEADLESS ?? "true") === "true";
const tracePolicy = process.env.OS_E2E_TRACE ?? "off";
const retries = Number.parseInt(process.env.OS_E2E_RETRIES ?? "0", 10);
const slowMoMs = Number.parseInt(process.env.OS_E2E_SLOW_MO_MS ?? "0", 10);
const mode = process.env.OS_E2E_MODE ?? "validate";
const viewportSet = process.env.OS_E2E_VIEWPORT_SET ?? "responsive-core";
const artifactLevel = process.env.OS_E2E_ARTIFACT_LEVEL ?? "standard";
const captureAccessibility = (process.env.OS_E2E_CAPTURE_ACCESSIBILITY ?? "true") === "true";
const captureDom = (process.env.OS_E2E_CAPTURE_DOM ?? "true") === "true";
const captureLayout = (process.env.OS_E2E_CAPTURE_LAYOUT ?? "true") === "true";
const captureConsole = (process.env.OS_E2E_CAPTURE_CONSOLE ?? "true") === "true";
const captureNetwork = (process.env.OS_E2E_CAPTURE_NETWORK ?? "false") === "true";
const snapshotDiff = process.env.OS_E2E_SNAPSHOT_DIFF ?? "hybrid";
const noDiff = (process.env.OS_E2E_NO_DIFF ?? "false") === "true";

if (!baseUrl) {
  console.error("OS_E2E_BASE_URL is required");
  process.exit(1);
}

if (!artifactDir) {
  console.error("OS_E2E_ARTIFACT_DIR is required");
  process.exit(1);
}

if (!baselineRoot) {
  console.error("OS_E2E_BASELINE_ROOT is required");
  process.exit(1);
}

if (!manifestPath) {
  console.error("OS_E2E_MANIFEST_PATH is required");
  process.exit(1);
}

if (scenarioIds.length === 0) {
  console.error("OS_E2E_SCENARIO_IDS is required");
  process.exit(1);
}

const artifactsRoot = path.join(artifactDir, "artifacts");
const screenshotsDir = path.join(artifactsRoot, "screenshots");
const domDir = path.join(artifactsRoot, "dom");
const a11yDir = path.join(artifactsRoot, "a11y");
const layoutDir = path.join(artifactsRoot, "layout");
const logsDir = path.join(artifactsRoot, "logs");
const networkDir = path.join(artifactsRoot, "network");
const tracesDir = path.join(artifactsRoot, "traces");
const diffsDir = path.join(artifactsRoot, "diffs");
const reportsDir = path.join(artifactDir, "reports");

[
  screenshotsDir,
  domDir,
  a11yDir,
  layoutDir,
  logsDir,
  networkDir,
  tracesDir,
  diffsDir,
  reportsDir,
].forEach((dir) => fs.mkdirSync(dir, { recursive: true }));

const deterministicEpochMs = Date.UTC(2026, 0, 1, 12, 0, 0);

const viewportSets = {
  "desktop-standard": [{ id: "desktop", width: 1440, height: 900 }],
  "responsive-core": [
    { id: "desktop", width: 1440, height: 900 },
    { id: "tablet", width: 1024, height: 768 },
    { id: "mobile", width: 390, height: 844 },
  ],
  "debug-focused": [{ id: "desktop", width: 1440, height: 900 }],
};

const supportedSkins = [
  { id: "soft-neumorphic", label: "Soft Neumorphic" },
  { id: "modern-adaptive", label: "Modern Adaptive" },
  { id: "classic-xp", label: "Classic XP" },
  { id: "classic-95", label: "Classic 95" },
];

function splitCsv(value) {
  return (value ?? "")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function safeName(value) {
  return value.replace(/[^a-zA-Z0-9]+/g, "-").replace(/^-+|-+$/g, "").toLowerCase();
}

function absolute(filePath) {
  return path.resolve(filePath);
}

function sha256File(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function writeJsonl(filePath, rows) {
  const text = rows.map((row) => JSON.stringify(row)).join("\n");
  fs.writeFileSync(filePath, text.length > 0 ? `${text}\n` : "");
}

function createFailure(code, message, detail = null) {
  return { code, message, detail };
}

function buildArtifactStem(browserName, scenarioId, sliceId, viewportId) {
  return [
    safeName(browserName),
    safeName(scenarioId),
    safeName(sliceId),
    safeName(viewportId),
  ].join("--");
}

function defaultTheme(skin, overrides = {}) {
  return {
    skin,
    wallpaper_id: "cloud-bands",
    high_contrast: false,
    reduced_motion: false,
    audio_enabled: true,
    ...overrides,
  };
}

function scenarioViewportIds(explicit) {
  if (Array.isArray(explicit) && explicit.length > 0) {
    return explicit;
  }
  return viewportSets[viewportSet].map((item) => item.id);
}

function mapViewport(id) {
  for (const set of Object.values(viewportSets)) {
    const match = set.find((item) => item.id === id);
    if (match) {
      return { ...match };
    }
  }
  throw new Error(`unknown viewport '${id}'`);
}

function shouldCaptureTrace() {
  return tracePolicy !== "off";
}

function shouldKeepTraceOnSuccess() {
  return tracePolicy === "on";
}

function effectiveDiffStrategy(slice) {
  if (noDiff || mode === "capture") {
    return "none";
  }
  return slice.diffStrategy ?? snapshotDiff;
}

function scenarioDefinitions() {
  const responsiveIds = viewportSets["responsive-core"].map((item) => item.id);
  const desktopOnly = viewportSets["desktop-standard"].map((item) => item.id);

  const layoutSlices = supportedSkins.map((skin) => ({
    sliceId: `shell.${skin.id}.default`,
    trackedRoot: '[data-ui-kind="desktop-backdrop"]',
    baseline: true,
    viewports: responsiveIds,
    diffStrategy: "hybrid",
    assertions: [
      { kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' },
      { kind: "selector", target: '[data-ui-kind="taskbar"]' },
    ],
    setup: async (page) => {
      await openShell(page, defaultTheme(skin.id));
    },
  }));

  return {
    "shell.boot": {
      sliceFamily: "legacy-shell-boot",
      slices: [
        {
          sliceId: "shell.soft-neumorphic.boot",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: false,
          viewports: desktopOnly,
          diffStrategy: "none",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
    "shell.settings-navigation": {
      sliceFamily: "legacy-shell-settings",
      slices: [
        {
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: false,
          viewports: desktopOnly,
          diffStrategy: "none",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          setup: async (page) => {
            await openSettingsAppearance(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
    "ui.keyboard-smoke": {
      sliceFamily: "legacy-keyboard-smoke",
      slices: supportedSkins.map((skin) => ({
        sliceId: `shell.${skin.id}.keyboard-smoke`,
        trackedRoot: '[data-ui-kind="window-frame"]',
        baseline: false,
        viewports: desktopOnly,
        diffStrategy: "none",
        assertions: [{ kind: "text", target: "High contrast" }],
        setup: async (page) => {
          await openAccessibilitySettings(page, defaultTheme(skin.id));
        },
      })),
    },
    "ui.screenshot-matrix": {
      sliceFamily: "legacy-screenshot-matrix",
      slices: layoutSlices,
    },
    "ui.shell.layout-baseline": {
      sliceFamily: "shell-layout",
      slices: layoutSlices,
    },
    "ui.shell.navigation-state": {
      sliceFamily: "shell-navigation",
      slices: [
        {
          sliceId: "shell.soft-neumorphic.context-menu-open",
          trackedRoot: "#desktop-context-menu",
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: "#desktop-context-menu" }],
          setup: async (page) => {
            await openContextMenu(page, defaultTheme("soft-neumorphic"));
          },
        },
        {
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          setup: async (page) => {
            await openSettingsAppearance(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
    "ui.shell.interaction-state": {
      sliceFamily: "shell-interaction",
      slices: [
        {
          sliceId: "shell.soft-neumorphic.start-button-hover",
          trackedRoot: '[data-ui-kind="taskbar"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="taskbar"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic"));
            await page.hover('[data-ui-kind="start-button"]');
            await page.waitForTimeout(100);
          },
        },
        {
          sliceId: "shell.soft-neumorphic.start-button-focus",
          trackedRoot: '[data-ui-kind="taskbar"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="start-button"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic"));
            await page.focus('[data-ui-kind="start-button"]');
            await page.waitForTimeout(100);
          },
        },
        {
          sliceId: "shell.soft-neumorphic.high-contrast",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic", { high_contrast: true }));
          },
        },
      ],
    },
    "ui.shell.edge-cases": {
      sliceFamily: "shell-edge-cases",
      slices: [
        {
          sliceId: "shell.soft-neumorphic.narrow-mobile",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: ["mobile"],
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic"));
          },
        },
        {
          sliceId: "settings.desktop.accessibility-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "High contrast" }],
          setup: async (page) => {
            await openAccessibilitySettings(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
    "ui.shell.responsive-matrix": {
      sliceFamily: "shell-responsive",
      slices: [
        {
          sliceId: "shell.soft-neumorphic.responsive",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: responsiveIds,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          setup: async (page) => {
            await openShell(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
    "ui.app.slice-baseline": {
      sliceFamily: "app-baseline",
      slices: [
        {
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          setup: async (page) => {
            await openSettingsAppearance(page, defaultTheme("soft-neumorphic"));
          },
        },
      ],
    },
  };
}

const browserTypes = {
  chromium,
  firefox,
  webkit,
};

async function createSession(browserType, browserName, scenarioId, sliceId, viewport, attempt) {
  const browser = await browserType.launch({
    headless,
    slowMo: slowMoMs > 0 ? slowMoMs : undefined,
  });
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: 1,
    colorScheme: "light",
    reducedMotion: "reduce",
  });
  await context.addInitScript(({ epochMs }) => {
    const fixedNow = epochMs;
    const OriginalDate = Date;
    class FixedDate extends OriginalDate {
      constructor(...args) {
        if (args.length === 0) {
          super(fixedNow);
        } else {
          super(...args);
        }
      }
      static now() {
        return fixedNow;
      }
    }
    Object.defineProperty(window, "Date", { value: FixedDate });
    Object.defineProperty(Math, "random", { value: () => 0.123456789 });
  }, { epochMs: deterministicEpochMs });

  if (shouldCaptureTrace()) {
    await context.tracing.start({ screenshots: true, snapshots: true });
  }

  const page = await context.newPage();
  const consoleEntries = [];
  const pageErrors = [];
  const networkEntries = [];

  page.on("console", (message) => {
    consoleEntries.push({
      type: message.type(),
      text: message.text(),
      location: message.location(),
    });
  });

  page.on("pageerror", (error) => {
    pageErrors.push({ message: String(error) });
  });

  if (captureNetwork) {
    page.on("response", async (response) => {
      networkEntries.push({
        event: "response",
        status: response.status(),
        ok: response.ok(),
        url: response.url(),
        method: response.request().method(),
        resourceType: response.request().resourceType(),
      });
    });
    page.on("requestfailed", (request) => {
      networkEntries.push({
        event: "requestfailed",
        url: request.url(),
        method: request.method(),
        resourceType: request.resourceType(),
        failure: request.failure()?.errorText ?? "unknown",
      });
    });
  }

  async function finalize({ keepTrace, suffix }) {
    let trace = null;
    try {
      if (shouldCaptureTrace()) {
        if (keepTrace) {
          const tracePath = path.join(
            tracesDir,
            `${buildArtifactStem(browserName, scenarioId, sliceId, viewport.id)}${suffix ? `--${suffix}` : ""}.zip`,
          );
          await context.tracing.stop({ path: tracePath });
          trace = absolute(tracePath);
        } else {
          await context.tracing.stop();
        }
      }
    } finally {
      await context.close();
      await browser.close();
    }
    return trace;
  }

  return {
    page,
    consoleEntries,
    pageErrors,
    networkEntries,
    finalize,
    attempt,
  };
}

async function openShell(page, theme) {
  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.evaluate((requestedTheme) => {
    localStorage.clear();
    sessionStorage.clear();
    localStorage.setItem("retrodesk.theme.v1", JSON.stringify(requestedTheme));
    localStorage.removeItem("retrodesk.layout.v1");
    localStorage.removeItem("retrodesk.terminal_history.v1");
  }, theme);
  await page.reload({ waitUntil: "networkidle" });
  await page.waitForSelector('[data-ui-kind="desktop-backdrop"]', { timeout: 5000 });
  await freezeMotion(page);
}

async function freezeMotion(page) {
  await page.addStyleTag({
    content: `
      *, *::before, *::after {
        animation-duration: 0s !important;
        animation-delay: 0s !important;
        transition-duration: 0s !important;
        transition-delay: 0s !important;
        scroll-behavior: auto !important;
      }
    `,
  });
}

async function openContextMenu(page, theme) {
  await openShell(page, theme);
  await page.click('[data-ui-kind="desktop-backdrop"]', { button: "right", timeout: 3000 });
  await page.waitForSelector("#desktop-context-menu", { timeout: 2500 });
}

async function openSettingsAppearance(page, theme) {
  await openContextMenu(page, theme);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await page.waitForSelector("text=Personalize your desktop", { timeout: 5000 });
  await page.focus('[role="tab"]:has-text("Appearance")');
  await page.keyboard.press("Enter");
  await page.waitForSelector("text=Choose a shell skin", { timeout: 2500 });
}

async function openAccessibilitySettings(page, theme) {
  await openContextMenu(page, theme);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await page.waitForSelector("text=Personalize your desktop", { timeout: 5000 });
  await page.focus('[role="tab"]:has-text("Accessibility")');
  await page.keyboard.press("Enter");
  await page.waitForSelector("text=High contrast", { timeout: 2500 });
}

async function runAssertions(page, assertions) {
  const results = [];
  const failures = [];
  for (const assertion of assertions ?? []) {
    if (assertion.kind === "selector") {
      const count = await page.locator(assertion.target).count();
      if (count > 0) {
        results.push({ kind: assertion.kind, target: assertion.target, status: "passed", detail: null });
      } else {
        results.push({ kind: assertion.kind, target: assertion.target, status: "failed", detail: "selector not found" });
        failures.push(createFailure("assertion_failed", `selector '${assertion.target}' not found`, assertion.target));
      }
    } else if (assertion.kind === "text") {
      const found = await page.locator(`text=${assertion.target}`).count();
      if (found > 0) {
        results.push({ kind: assertion.kind, target: assertion.target, status: "passed", detail: null });
      } else {
        results.push({ kind: assertion.kind, target: assertion.target, status: "failed", detail: "text not found" });
        failures.push(createFailure("assertion_failed", `text '${assertion.target}' not found`, assertion.target));
      }
    }
  }
  return { results, failures };
}

function normalizeText(text) {
  return text.replace(/\s+/g, " ").trim();
}

async function captureDomSnapshot(page, selector) {
  return page.evaluate((rootSelector) => {
    function normalizeNode(node) {
      if (node.nodeType === Node.TEXT_NODE) {
        const value = node.textContent.replace(/\s+/g, " ").trim();
        return value ? { type: "text", value } : null;
      }
      if (node.nodeType !== Node.ELEMENT_NODE) {
        return null;
      }
      const element = node;
      const tag = element.tagName.toLowerCase();
      if (tag === "script" || tag === "style") {
        return null;
      }
      const attributes = Array.from(element.attributes)
        .filter((attribute) => {
          return ![
            "style",
            "data-playwright-internal-id",
            "data-reactid",
          ].includes(attribute.name);
        })
        .map((attribute) => [attribute.name, attribute.value])
        .sort((left, right) => left[0].localeCompare(right[0]));
      const children = [];
      for (const child of Array.from(element.childNodes)) {
        const normalized = normalizeNode(child);
        if (normalized) {
          children.push(normalized);
        }
      }
      return {
        type: "element",
        tag,
        attributes,
        text: element.children.length === 0 ? element.textContent.replace(/\s+/g, " ").trim() : "",
        children,
      };
    }

    const root = rootSelector ? document.querySelector(rootSelector) : document.body;
    if (!root) {
      return { missing_root: rootSelector };
    }
    return normalizeNode(root);
  }, selector);
}

async function captureAccessibilityTree(page, selector) {
  const locator = selector ? page.locator(selector).first() : page.locator("body");
  const snapshot = await locator.ariaSnapshot();
  return {
    root: selector ?? "body",
    snapshot,
  };
}

function pruneNulls(value) {
  if (Array.isArray(value)) {
    return value.map(pruneNulls).filter((entry) => entry !== null && entry !== undefined);
  }
  if (value && typeof value === "object") {
    const next = {};
    for (const [key, current] of Object.entries(value)) {
      if (current === null || current === undefined) {
        continue;
      }
      next[key] = pruneNulls(current);
    }
    return next;
  }
  return value;
}

async function captureLayoutMetrics(page, selector) {
  return page.evaluate((rootSelector) => {
    const trackedSelectors = [
      rootSelector,
      '[data-ui-kind="desktop-backdrop"]',
      '[data-ui-kind="taskbar"]',
      '[data-ui-kind="window-frame"]',
      '#desktop-context-menu',
      '[data-ui-kind="start-button"]',
    ].filter(Boolean);

    const metrics = trackedSelectors
      .map((tracked) => {
        const element = document.querySelector(tracked);
        if (!element) {
          return { selector: tracked, missing: true };
        }
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return {
          selector: tracked,
          rect: {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
          },
          scrollWidth: element.scrollWidth,
          scrollHeight: element.scrollHeight,
          clientWidth: element.clientWidth,
          clientHeight: element.clientHeight,
          overflowX: style.overflowX,
          overflowY: style.overflowY,
          zIndex: style.zIndex,
          opacity: style.opacity,
          display: style.display,
        };
      });

    const root = document.documentElement;
    return {
      viewport: {
        innerWidth: window.innerWidth,
        innerHeight: window.innerHeight,
        devicePixelRatio: window.devicePixelRatio,
      },
      document: {
        clientWidth: root.clientWidth,
        clientHeight: root.clientHeight,
        scrollWidth: root.scrollWidth,
        scrollHeight: root.scrollHeight,
      },
      metrics,
    };
  }, selector);
}

function compareJsonArtifacts(label, currentValue, baselinePath, stem) {
  const currentString = JSON.stringify(currentValue, null, 2);
  if (!fs.existsSync(baselinePath)) {
    const diffPath = path.join(diffsDir, `${stem}--${label}-diff.json`);
    writeJson(diffPath, { label, status: "missing-baseline" });
    return {
      equal: false,
      failure: createFailure(`${label}_diff_failed`, `missing baseline for ${label}`, baselinePath),
      diffArtifact: absolute(diffPath),
    };
  }

  const baselineValue = fs.readFileSync(baselinePath, "utf8");
  if (baselineValue === `${currentString}\n` || baselineValue === currentString) {
    return { equal: true, failure: null, diffArtifact: null };
  }

  const diffPath = path.join(diffsDir, `${stem}--${label}-diff.json`);
  writeJson(diffPath, {
    label,
    status: "changed",
    baseline: baselinePath,
    current_hash: crypto.createHash("sha256").update(currentString).digest("hex"),
    baseline_hash: crypto.createHash("sha256").update(baselineValue).digest("hex"),
  });
  return {
    equal: false,
    failure: createFailure(`${label}_diff_failed`, `${label} artifact changed`, diffPath),
    diffArtifact: absolute(diffPath),
  };
}

function compareScreenshot(currentPath, baselinePath, stem, enforced) {
  if (!fs.existsSync(baselinePath)) {
    const pixelDiffPath = path.join(diffsDir, `${stem}--pixel-diff.png`);
    fs.copyFileSync(currentPath, pixelDiffPath);
    return {
      equal: false,
      failure: enforced
        ? createFailure("pixel_diff_failed", "missing baseline screenshot", baselinePath)
        : null,
      pixelDiff: absolute(pixelDiffPath),
      ratio: 1,
      current_hash: sha256File(currentPath),
      baseline_hash: null,
    };
  }

  const currentHash = sha256File(currentPath);
  const baselineHash = sha256File(baselinePath);
  if (currentHash === baselineHash) {
    return {
      equal: true,
      failure: null,
      pixelDiff: null,
      ratio: 0,
      current_hash: currentHash,
      baseline_hash: baselineHash,
    };
  }

  const pixelDiffPath = path.join(diffsDir, `${stem}--pixel-diff.png`);
  fs.copyFileSync(currentPath, pixelDiffPath);
  return {
    equal: false,
    failure: enforced
      ? createFailure("pixel_diff_failed", "screenshot changed", pixelDiffPath)
      : null,
    pixelDiff: absolute(pixelDiffPath),
    ratio: 1,
    current_hash: currentHash,
    baseline_hash: baselineHash,
  };
}

async function captureArtifacts(page, browserName, scenarioId, sliceId, viewportId, trackedRoot, session) {
  const stem = buildArtifactStem(browserName, scenarioId, sliceId, viewportId);
  const screenshotPath = path.join(screenshotsDir, `${stem}.png`);
  await page.screenshot({ path: screenshotPath, fullPage: true });

  const consolePath = path.join(logsDir, `${stem}.console.jsonl`);
  const pageErrorsPath = path.join(logsDir, `${stem}.page-errors.json`);
  writeJsonl(consolePath, captureConsole ? session.consoleEntries : []);
  writeJson(pageErrorsPath, session.pageErrors);

  let domPath = null;
  let domSnapshot = null;
  if (captureDom || artifactLevel === "full") {
    domSnapshot = await captureDomSnapshot(page, trackedRoot);
    domPath = path.join(domDir, `${stem}.dom.json`);
    writeJson(domPath, domSnapshot);
  }

  let a11yPath = null;
  let a11yTree = null;
  if (captureAccessibility || artifactLevel === "full") {
    a11yTree = await captureAccessibilityTree(page, trackedRoot);
    a11yPath = path.join(a11yDir, `${stem}.a11y.json`);
    writeJson(a11yPath, a11yTree);
  }

  let layoutPath = null;
  let layoutMetrics = null;
  if (captureLayout || artifactLevel === "full") {
    layoutMetrics = await captureLayoutMetrics(page, trackedRoot);
    layoutPath = path.join(layoutDir, `${stem}.layout.json`);
    writeJson(layoutPath, layoutMetrics);
  }

  let networkPath = null;
  if (captureNetwork || artifactLevel === "full") {
    networkPath = path.join(networkDir, `${stem}.network.jsonl`);
    writeJsonl(networkPath, session.networkEntries);
  }

  return {
    stem,
    screenshotPath,
    domPath,
    domSnapshot,
    a11yPath,
    a11yTree,
    layoutPath,
    layoutMetrics,
    consolePath,
    pageErrorsPath,
    networkPath,
  };
}

function baselinePaths(scenarioId, sliceId, browserName, viewportId) {
  const root = path.join(baselineRoot, scenarioId, sliceId, browserName, viewportId);
  return {
    root,
    manifest: path.join(root, "manifest.json"),
    screenshot: path.join(root, "screenshot.png"),
    dom: path.join(root, "dom.json"),
    a11y: path.join(root, "a11y.json"),
    layout: path.join(root, "layout.json"),
  };
}

function loadBaselineMetadata(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function buildManifest(runId) {
  const startedAt = new Date().toISOString();
  return {
    schema_version: 1,
    run_id: runId,
    profile,
    mode,
    base_url: baseUrl,
    started_at: startedAt,
    finished_at: null,
    status: "running",
    artifact_root: absolute(artifactDir),
    summary: {
      scenario_count: scenarioIds.length,
      slice_count: 0,
      passed: 0,
      failed: 0,
      diff_failures: 0,
      assertion_failures: 0,
      console_errors: 0,
    },
    scenarios: [],
  };
}

function updateSummary(manifest) {
  manifest.summary.slice_count = manifest.scenarios.length;
  manifest.summary.passed = manifest.scenarios.filter((entry) => entry.status === "passed").length;
  manifest.summary.failed = manifest.scenarios.filter((entry) => entry.status !== "passed").length;
  manifest.summary.diff_failures = manifest.scenarios.reduce((count, entry) => {
    return count + entry.failures.filter((failure) => failure.code.endsWith("_diff_failed")).length;
  }, 0);
  manifest.summary.assertion_failures = manifest.scenarios.reduce((count, entry) => {
    return count + entry.failures.filter((failure) => failure.code === "assertion_failed").length;
  }, 0);
  manifest.summary.console_errors = manifest.scenarios.reduce((count, entry) => {
    return count + (entry.metrics?.console_error_count ?? 0);
  }, 0);
}

function writeManifest(manifest) {
  manifest.finished_at = new Date().toISOString();
  writeJson(manifestPath, manifest);
  writeJson(path.join(reportsDir, "report.json"), manifest);
}

async function executeSlice(browserName, browserType, scenarioId, slice, viewport, attempt) {
  const session = await createSession(browserType, browserName, scenarioId, slice.sliceId, viewport, attempt);
  const failures = [];
  let structuredDiffPath = null;
  let pixelDiffPath = null;
  let trace = null;

  try {
    await slice.setup(session.page);
    const assertionResult = await runAssertions(session.page, slice.assertions);
    failures.push(...assertionResult.failures);

    if (captureConsole) {
      const consoleErrors = session.consoleEntries.filter((entry) => entry.type === "error");
      if (consoleErrors.length > 0) {
        failures.push(
          createFailure(
            "console_error_detected",
            `console emitted ${consoleErrors.length} error message(s)`,
            consoleErrors.map((entry) => entry.text).join("; "),
          ),
        );
      }
    }

    if (session.pageErrors.length > 0) {
      failures.push(
        createFailure(
          "console_error_detected",
          `page emitted ${session.pageErrors.length} uncaught error(s)`,
          session.pageErrors.map((entry) => entry.message).join("; "),
        ),
      );
    }

    const captured = await captureArtifacts(
      session.page,
      browserName,
      scenarioId,
      slice.sliceId,
      viewport.id,
      slice.trackedRoot,
      session,
    );

    const diffMode = effectiveDiffStrategy(slice);
    const diff = {
      strategy: diffMode,
      pixel: { changed: false, ratio: 0 },
      dom: { changed: false },
      a11y: { changed: false },
      layout: { changed: false },
    };

    if (slice.baseline && diffMode !== "none") {
      const baseline = baselinePaths(scenarioId, slice.sliceId, browserName, viewport.id);
      const baselineMetadata = loadBaselineMetadata(baseline.manifest);
      const enforcePixelDiff = baselineMetadata?.profile === profile;
      const screenshotDiff = compareScreenshot(
        captured.screenshotPath,
        baseline.screenshot,
        captured.stem,
        enforcePixelDiff,
      );
      if (!screenshotDiff.equal) {
        if (screenshotDiff.failure) {
          failures.push(screenshotDiff.failure);
        }
        pixelDiffPath = screenshotDiff.pixelDiff;
        diff.pixel.changed = true;
        diff.pixel.ratio = screenshotDiff.ratio;
      }

      const structuredDiff = {};
      if ((diffMode === "dom" || diffMode === "hybrid") && captured.domPath) {
        const result = compareJsonArtifacts("dom", captured.domSnapshot, baseline.dom, captured.stem);
        structuredDiff.dom = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = result.diffArtifact;
          diff.dom.changed = true;
        }
      }
      if (diffMode === "hybrid" && captured.a11yPath) {
        const result = compareJsonArtifacts("a11y", captured.a11yTree, baseline.a11y, captured.stem);
        structuredDiff.a11y = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = structuredDiffPath ?? result.diffArtifact;
          diff.a11y.changed = true;
        }
      }
      if (diffMode === "hybrid" && captured.layoutPath) {
        const result = compareJsonArtifacts("layout", captured.layoutMetrics, baseline.layout, captured.stem);
        structuredDiff.layout = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = structuredDiffPath ?? result.diffArtifact;
          diff.layout.changed = true;
        }
      }

      if (Object.keys(structuredDiff).length > 0) {
        const structuredPath = path.join(diffsDir, `${captured.stem}--structured-diff.json`);
        writeJson(structuredPath, {
          baseline_root: baseline.root,
          diff: structuredDiff,
        });
        structuredDiffPath = absolute(structuredPath);
      }
    }

    trace = await session.finalize({
      keepTrace: failures.length > 0 || shouldKeepTraceOnSuccess(),
      suffix: failures.length > 0 ? "failure" : "success",
    });

    return {
      id: scenarioId,
      slice_id: slice.sliceId,
      browser: browserName,
      viewport: {
        id: viewport.id,
        width: viewport.width,
        height: viewport.height,
        device_scale_factor: 1,
      },
      status: failures.length === 0 ? "passed" : "failed",
      baseline_enabled: slice.baseline,
      diff_strategy: diffMode,
      artifacts: {
        screenshot: absolute(captured.screenshotPath),
        dom_snapshot: captured.domPath ? absolute(captured.domPath) : null,
        a11y_tree: captured.a11yPath ? absolute(captured.a11yPath) : null,
        layout_metrics: captured.layoutPath ? absolute(captured.layoutPath) : null,
        console_log: absolute(captured.consolePath),
        page_errors: absolute(captured.pageErrorsPath),
        network_log: captured.networkPath ? absolute(captured.networkPath) : null,
        trace,
        pixel_diff: pixelDiffPath,
        structured_diff: structuredDiffPath,
      },
      assertions: assertionResult.results,
      metrics: {
        console_error_count: session.consoleEntries.filter((entry) => entry.type === "error").length,
        page_error_count: session.pageErrors.length,
        network_error_count: session.networkEntries.filter((entry) => entry.event === "requestfailed").length,
      },
      diff,
      failures,
    };
  } catch (error) {
    trace = await session.finalize({ keepTrace: true, suffix: "failure" });
    return {
      id: scenarioId,
      slice_id: slice.sliceId,
      browser: browserName,
      viewport: {
        id: viewport.id,
        width: viewport.width,
        height: viewport.height,
        device_scale_factor: 1,
      },
      status: "failed",
      baseline_enabled: slice.baseline,
      diff_strategy: effectiveDiffStrategy(slice),
      artifacts: {
        screenshot: null,
        dom_snapshot: null,
        a11y_tree: null,
        layout_metrics: null,
        console_log: null,
        page_errors: null,
        network_log: null,
        trace,
        pixel_diff: null,
        structured_diff: null,
      },
      assertions: [],
      metrics: {
        console_error_count: session.consoleEntries.filter((entry) => entry.type === "error").length,
        page_error_count: session.pageErrors.length,
        network_error_count: session.networkEntries.filter((entry) => entry.event === "requestfailed").length,
      },
      diff: {
        strategy: effectiveDiffStrategy(slice),
        pixel: { changed: false, ratio: 0 },
        dom: { changed: false },
        a11y: { changed: false },
        layout: { changed: false },
      },
      failures: [createFailure("setup_failed", String(error.message ?? error), String(error.stack ?? error))],
    };
  }
}

async function runScenarioWithRetries(browserName, browserType, scenarioId, slice, viewport) {
  let lastResult = null;
  for (let attempt = 1; attempt <= retries + 1; attempt += 1) {
    const result = await executeSlice(browserName, browserType, scenarioId, slice, viewport, attempt);
    if (result.status === "passed") {
      return result;
    }
    lastResult = result;
    if (attempt <= retries) {
      console.warn(
        `retrying scenario=${scenarioId} slice=${slice.sliceId} browser=${browserName} viewport=${viewport.id} after attempt ${attempt} failed`,
      );
    }
  }
  return lastResult;
}

async function main() {
  const runId = path.basename(path.resolve(artifactDir));
  const manifest = buildManifest(runId);
  writeManifest(manifest);

  try {
    const definitions = scenarioDefinitions();

    for (const browserName of browsers) {
      const browserType = browserTypes[browserName];
      if (!browserType) {
        throw new Error(`unsupported browser '${browserName}'`);
      }

      for (const scenarioId of scenarioIds) {
        const scenario = definitions[scenarioId];
        if (!scenario) {
          throw new Error(`unsupported scenario '${scenarioId}'`);
        }

        const slices = scenario.slices.filter((slice) => !sliceFilter || slice.sliceId === sliceFilter);
        if (sliceFilter && slices.length === 0) {
          throw new Error(`scenario '${scenarioId}' does not define slice '${sliceFilter}'`);
        }

        for (const slice of slices) {
          const viewportIds = scenarioViewportIds(slice.viewports);
          for (const viewportId of viewportIds) {
            const result = await runScenarioWithRetries(
              browserName,
              browserType,
              scenarioId,
              slice,
              mapViewport(viewportId),
            );
            manifest.scenarios.push(result);
            updateSummary(manifest);
            writeManifest(manifest);
          }
        }
      }

    }

    updateSummary(manifest);
    manifest.status =
      manifest.summary.failed > 0
        ? "failed"
        : mode === "capture" || noDiff
          ? "capture-complete"
          : "passed";
    writeManifest(manifest);

    if (manifest.status === "failed") {
      const lines = manifest.scenarios
        .filter((entry) => entry.status !== "passed")
        .map((entry) => {
          return `${entry.id} / ${entry.slice_id} / ${entry.browser} / ${entry.viewport.id}: ${entry.failures
            .map((failure) => `${failure.code}: ${failure.message}`)
            .join(" | ")}`;
        });
      fs.writeFileSync(path.join(reportsDir, "failure.txt"), `${lines.join("\n")}\n`);
      process.exit(1);
    }
  } catch (error) {
    manifest.status = "failed";
    manifest.finished_at = new Date().toISOString();
    writeManifest(manifest);
    fs.writeFileSync(path.join(reportsDir, "failure.txt"), `${String(error.stack ?? error)}\n`);
    console.error(error);
    process.exit(1);
  }
}

main();
