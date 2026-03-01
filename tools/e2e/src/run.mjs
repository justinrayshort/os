import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";
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
const styleDir = path.join(artifactsRoot, "style");
const timingDir = path.join(artifactsRoot, "timing");
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
  styleDir,
  timingDir,
  tracesDir,
  diffsDir,
  reportsDir,
].forEach((dir) => fs.mkdirSync(dir, { recursive: true }));

const deterministicEpochMs = Date.UTC(2026, 0, 1, 12, 0, 0);
const e2eReadySelector = '[data-ui-kind="desktop-root"][data-e2e-ready="true"]';
const styleTokenNames = [
  "--sys-color-surface",
  "--sys-color-accent",
  "--sys-radius-control",
  "--sys-radius-panel",
  "--sys-space-2",
  "--sys-space-3",
  "--sys-space-4",
  "--sys-elevation-raised",
  "--sys-elevation-inset",
  "--sys-focus-ring",
];
const styleSelectors = [
  '[data-ui-kind="taskbar"]',
  '[data-ui-kind="window-frame"]',
  '[data-ui-kind="menu-surface"]',
  '[data-ui-kind="button"][data-ui-slot="start-button"]',
];
const stylePropertyNames = [
  "backgroundColor",
  "color",
  "boxShadow",
  "borderRadius",
  "outlineColor",
  "outlineWidth",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "gap",
  "transform",
];
const pixelThresholds = {
  desktop: 0.0010,
  tablet: 0.0010,
  mobile: 0.0015,
};

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

function createFailure(code, category, message, detail = null) {
  return { code, category, message, detail };
}

function buildArtifactStem(browserName, scenarioId, sliceId, viewportId) {
  return [
    safeName(browserName),
    safeName(scenarioId),
    safeName(sliceId),
    safeName(viewportId),
  ].join("--");
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

function buildSceneUrl({ scene, skin = "soft-neumorphic", highContrast, reducedMotion }) {
  const url = new URL(baseUrl);
  url.searchParams.set("e2e-scene", scene);
  url.searchParams.set("e2e-skin", skin);
  if (typeof highContrast === "boolean") {
    url.searchParams.set("e2e-high-contrast", String(highContrast));
  }
  if (typeof reducedMotion === "boolean") {
    url.searchParams.set("e2e-reduced-motion", String(reducedMotion));
  }
  return url.toString();
}

async function waitForSceneReady(page, url) {
  const gotoStartedAt = Date.now();
  await page.goto(url, { waitUntil: "domcontentloaded" });
  const selectorWaitStartedAt = Date.now();
  await page.waitForSelector(e2eReadySelector, { timeout: 5000 });
  await page.waitForFunction(
    () => window.performance.getEntriesByName("os:e2e-ready").length > 0,
    null,
    { timeout: 5000 },
  );
  await page.waitForLoadState("load");
  await freezeMotion(page);

  const readiness = await page.evaluate(({ readySelector, startedAt, selectorStartedAt }) => {
    const navigation = performance.getEntriesByType("navigation")[0];
    const readyEntries = performance.getEntriesByName("os:e2e-ready");
    const readyMark = readyEntries.at(-1);
    return {
      goto_ms: Date.now() - startedAt,
      shell_ready_ms: readyMark ? readyMark.startTime : null,
      scene_setup_ms: readyMark && navigation ? Math.max(readyMark.startTime - navigation.loadEventEnd, 0) : null,
      dom_content_loaded_ms: navigation ? navigation.domContentLoadedEventEnd : null,
      load_event_ms: navigation ? navigation.loadEventEnd : null,
      os_e2e_ready_mark_ms: readyMark ? readyMark.startTime : null,
      readiness_selector_wait_ms: Date.now() - selectorStartedAt,
      ready_selector: readySelector,
    };
  }, { readySelector: e2eReadySelector, startedAt: gotoStartedAt, selectorStartedAt: selectorWaitStartedAt });

  return readiness;
}

async function openScene(page, sceneConfig) {
  const url = buildSceneUrl(sceneConfig);
  return waitForSceneReady(page, url);
}

function sceneSlice({
  sliceId,
  trackedRoot,
  baseline = true,
  viewports,
  diffStrategy = "hybrid",
  assertions = [],
  scene,
  skin = "soft-neumorphic",
  highContrast,
  reducedMotion,
}) {
  return {
    sliceId,
    trackedRoot,
    baseline,
    viewports,
    diffStrategy,
    assertions,
    setup: async (page) =>
      openScene(page, {
        scene,
        skin,
        highContrast,
        reducedMotion,
      }),
  };
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
    setup: async (page) => openScene(page, { scene: "shell-default", skin: skin.id }),
  }));

  return {
    "shell.boot": {
      sliceFamily: "legacy-shell-boot",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.boot",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: false,
          viewports: desktopOnly,
          diffStrategy: "none",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-default",
        }),
      ],
    },
    "shell.settings-navigation": {
      sliceFamily: "legacy-shell-settings",
      slices: [
        sceneSlice({
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: false,
          viewports: desktopOnly,
          diffStrategy: "none",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          scene: "settings-appearance",
        }),
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
        setup: async (page) => openScene(page, { scene: "settings-accessibility", skin: skin.id }),
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
        sceneSlice({
          sliceId: "shell.soft-neumorphic.context-menu-open",
          trackedRoot: "#desktop-context-menu",
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: "#desktop-context-menu" }],
          scene: "shell-context-menu-open",
        }),
        sceneSlice({
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          scene: "settings-appearance",
        }),
      ],
    },
    "ui.shell.interaction-state": {
      sliceFamily: "shell-interaction",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.start-button-hover",
          trackedRoot: '[data-ui-kind="taskbar"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '#taskbar-start-button[data-e2e-state="hover"]' }],
          scene: "start-button-hover",
        }),
        sceneSlice({
          sliceId: "shell.soft-neumorphic.start-button-focus",
          trackedRoot: '[data-ui-kind="taskbar"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '#taskbar-start-button[data-e2e-state="focus-visible"]' }],
          scene: "start-button-focus",
        }),
        sceneSlice({
          sliceId: "shell.soft-neumorphic.high-contrast",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-high-contrast",
          highContrast: true,
        }),
      ],
    },
    "ui.shell.edge-cases": {
      sliceFamily: "shell-edge-cases",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.narrow-mobile",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: ["mobile"],
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-default",
        }),
        sceneSlice({
          sliceId: "settings.desktop.accessibility-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "High contrast" }],
          scene: "settings-accessibility",
        }),
      ],
    },
    "ui.shell.responsive-matrix": {
      sliceFamily: "shell-responsive",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.responsive",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          baseline: true,
          viewports: responsiveIds,
          diffStrategy: "hybrid",
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-default",
        }),
      ],
    },
    "ui.app.slice-baseline": {
      sliceFamily: "app-baseline",
      slices: [
        sceneSlice({
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          baseline: true,
          viewports: desktopOnly,
          diffStrategy: "hybrid",
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          scene: "settings-appearance",
        }),
      ],
    },
    "ui.neumorphic.layout": {
      sliceFamily: "neumorphic-layout",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.default",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          viewports: responsiveIds,
          assertions: [
            { kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' },
            { kind: "selector", target: '[data-ui-kind="taskbar"]' },
          ],
          scene: "shell-default",
        }),
      ],
    },
    "ui.neumorphic.navigation": {
      sliceFamily: "neumorphic-navigation",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.context-menu-open",
          trackedRoot: "#desktop-context-menu",
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: "#desktop-context-menu" }],
          scene: "shell-context-menu-open",
        }),
        sceneSlice({
          sliceId: "settings.desktop.appearance-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          viewports: desktopOnly,
          assertions: [{ kind: "text", target: "Choose a shell skin" }],
          scene: "settings-appearance",
        }),
      ],
    },
    "ui.neumorphic.interaction": {
      sliceFamily: "neumorphic-interaction",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.start-button-hover",
          trackedRoot: '[data-ui-kind="taskbar"]',
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: '#taskbar-start-button[data-e2e-state="hover"]' }],
          scene: "start-button-hover",
        }),
        sceneSlice({
          sliceId: "shell.soft-neumorphic.start-button-focus",
          trackedRoot: '[data-ui-kind="taskbar"]',
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: '#taskbar-start-button[data-e2e-state="focus-visible"]' }],
          scene: "start-button-focus",
        }),
      ],
    },
    "ui.neumorphic.accessibility": {
      sliceFamily: "neumorphic-accessibility",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.high-contrast",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-high-contrast",
          highContrast: true,
        }),
        sceneSlice({
          sliceId: "shell.soft-neumorphic.reduced-motion",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-reduced-motion",
          reducedMotion: true,
        }),
        sceneSlice({
          sliceId: "settings.desktop.accessibility-tab",
          trackedRoot: '[data-ui-kind="window-frame"]',
          viewports: desktopOnly,
          assertions: [{ kind: "text", target: "High contrast" }],
          scene: "settings-accessibility",
        }),
      ],
    },
    "ui.neumorphic.apps": {
      sliceFamily: "neumorphic-apps",
      slices: [
        sceneSlice({
          sliceId: "system.ui-showcase.controls",
          trackedRoot: '[data-ui-kind="window-frame"]',
          viewports: desktopOnly,
          assertions: [{ kind: "text", target: "Neumorphic UI Showcase" }],
          scene: "ui-showcase-controls",
        }),
        sceneSlice({
          sliceId: "terminal.desktop.default",
          trackedRoot: '[data-ui-kind="window-frame"]',
          viewports: desktopOnly,
          assertions: [{ kind: "text", target: "Use `help list` to inspect commands." }],
          scene: "terminal-default",
        }),
      ],
    },
    "ui.neumorphic.cross-browser": {
      sliceFamily: "neumorphic-cross-browser",
      slices: [
        sceneSlice({
          sliceId: "shell.soft-neumorphic.default",
          trackedRoot: '[data-ui-kind="desktop-backdrop"]',
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: '[data-ui-kind="desktop-backdrop"]' }],
          scene: "shell-default",
        }),
        sceneSlice({
          sliceId: "shell.soft-neumorphic.context-menu-open",
          trackedRoot: "#desktop-context-menu",
          viewports: desktopOnly,
          assertions: [{ kind: "selector", target: "#desktop-context-menu" }],
          scene: "shell-context-menu-open",
        }),
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
        failures.push(
          createFailure(
            "assertion_failed",
            "ui-contract-violation",
            `selector '${assertion.target}' not found`,
            assertion.target,
          ),
        );
      }
    } else if (assertion.kind === "text") {
      const found = await page.locator(`text=${assertion.target}`).count();
      if (found > 0) {
        results.push({ kind: assertion.kind, target: assertion.target, status: "passed", detail: null });
      } else {
        results.push({ kind: assertion.kind, target: assertion.target, status: "failed", detail: "text not found" });
        failures.push(
          createFailure(
            "assertion_failed",
            "ui-contract-violation",
            `text '${assertion.target}' not found`,
            assertion.target,
          ),
        );
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

async function captureStyleSnapshot(page) {
  return page.evaluate(({ tokenNames, selectors, propertyNames }) => {
    const root = document.querySelector('[data-ui-kind="desktop-root"]') ?? document.documentElement;
    const rootStyle = getComputedStyle(root);
    const tokens = Object.fromEntries(tokenNames.map((name) => [name, rootStyle.getPropertyValue(name).trim()]));
    const computed = selectors.map((selector) => {
      const element = document.querySelector(selector);
      if (!element) {
        return { selector, missing: true };
      }
      const style = getComputedStyle(element);
      const values = Object.fromEntries(propertyNames.map((name) => [name, style[name]]));
      return { selector, values };
    });
    return {
      root: '[data-ui-kind="desktop-root"]',
      tokens,
      computed,
    };
  }, { tokenNames: styleTokenNames, selectors: styleSelectors, propertyNames: stylePropertyNames });
}

function compareJsonArtifacts(label, currentValue, baselinePath, stem, category) {
  const currentString = JSON.stringify(currentValue, null, 2);
  if (!fs.existsSync(baselinePath)) {
    const diffPath = path.join(diffsDir, `${stem}--${label}-diff.json`);
    writeJson(diffPath, { label, status: "missing-baseline" });
    return {
      equal: false,
      failure: createFailure(`${label}_baseline_missing`, "baseline-missing", `missing baseline for ${label}`, baselinePath),
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
    failure: createFailure(`${label}_diff_failed`, category, `${label} artifact changed`, diffPath),
    diffArtifact: absolute(diffPath),
  };
}

function compareScreenshot(currentPath, baselinePath, stem, viewportId, enforced) {
  if (!fs.existsSync(baselinePath)) {
    const pixelDiffPath = path.join(diffsDir, `${stem}--pixel-diff.png`);
    fs.copyFileSync(currentPath, pixelDiffPath);
    return {
      equal: false,
      failure: enforced
        ? createFailure("pixel_baseline_missing", "baseline-missing", "missing baseline screenshot", baselinePath)
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

  const currentPng = PNG.sync.read(fs.readFileSync(currentPath));
  const baselinePng = PNG.sync.read(fs.readFileSync(baselinePath));
  if (currentPng.width !== baselinePng.width || currentPng.height !== baselinePng.height) {
    const pixelDiffPath = path.join(diffsDir, `${stem}--pixel-diff.png`);
    fs.copyFileSync(currentPath, pixelDiffPath);
    return {
      equal: false,
      failure: enforced
        ? createFailure(
            "pixel_diff_failed",
            "visual-regression",
            "screenshot dimensions changed",
            `${baselinePng.width}x${baselinePng.height} -> ${currentPng.width}x${currentPng.height}`,
          )
        : null,
      pixelDiff: absolute(pixelDiffPath),
      ratio: 1,
      current_hash: currentHash,
      baseline_hash: baselineHash,
    };
  }

  const diffPng = new PNG({ width: currentPng.width, height: currentPng.height });
  const mismatchedPixels = pixelmatch(
    currentPng.data,
    baselinePng.data,
    diffPng.data,
    currentPng.width,
    currentPng.height,
    {
      threshold: 0.10,
      includeAA: false,
    },
  );
  const ratio = mismatchedPixels / (currentPng.width * currentPng.height);
  const pixelDiffPath = path.join(diffsDir, `${stem}--pixel-diff.png`);
  fs.writeFileSync(pixelDiffPath, PNG.sync.write(diffPng));
  const allowedRatio = pixelThresholds[viewportId] ?? pixelThresholds.desktop;
  return {
    equal: ratio <= allowedRatio,
    failure: enforced
      && ratio > allowedRatio
      ? createFailure(
          "pixel_diff_failed",
          "visual-regression",
          `screenshot changed beyond tolerance (${ratio.toFixed(6)} > ${allowedRatio.toFixed(4)})`,
          pixelDiffPath,
        )
      : null,
    pixelDiff: absolute(pixelDiffPath),
    ratio,
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

  const styleSnapshot = await captureStyleSnapshot(page);
  const stylePath = path.join(styleDir, `${stem}.style.json`);
  writeJson(stylePath, styleSnapshot);

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
    stylePath,
    styleSnapshot,
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
    style: path.join(root, "style.json"),
  };
}

function loadBaselineMetadata(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function classifyStructuredDiff(label) {
  if (label === "style") {
    return "ui-contract-violation";
  }
  return "visual-regression";
}

function buildEnvironmentMetadata() {
  return {
    browser: browsers.length === 1 ? browsers[0] : "multi-browser",
    color_scheme: "light",
    reduced_motion: "reduce",
    fixed_epoch: new Date(deterministicEpochMs).toISOString(),
    deterministic_math_random: true,
    motion_frozen: true,
    viewport_set: viewportSet,
    workers: 1,
  };
}

function buildManifest(runId) {
  const startedAt = new Date().toISOString();
  return {
    schema_version: 2,
    run_id: runId,
    profile,
    mode,
    base_url: baseUrl,
    started_at: startedAt,
    finished_at: null,
    status: "running",
    artifact_root: absolute(artifactDir),
    environment: buildEnvironmentMetadata(),
    summary: {
      scenario_count: scenarioIds.length,
      slice_count: 0,
      passed: 0,
      failed: 0,
      diff_failures: 0,
      assertion_failures: 0,
      console_errors: 0,
      flaky_slice_count: 0,
      retry_success_count: 0,
      failure_categories: {},
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
  manifest.summary.flaky_slice_count = manifest.scenarios.filter((entry) =>
    entry.failure_categories?.includes("flaky"),
  ).length;
  manifest.summary.retry_success_count = manifest.scenarios.filter((entry) =>
    entry.status === "passed" && (entry.attempt ?? 1) > 1,
  ).length;
  manifest.summary.failure_categories = manifest.scenarios.reduce((counts, entry) => {
    for (const category of entry.failure_categories ?? []) {
      counts[category] = (counts[category] ?? 0) + 1;
    }
    return counts;
  }, {});
}

function writeManifest(manifest) {
  manifest.finished_at = new Date().toISOString();
  writeJson(manifestPath, manifest);
  writeJson(path.join(reportsDir, "report.json"), manifest);
}

async function executeSlice(browserName, browserType, scenarioId, slice, viewport, attempt) {
  const sliceStartedAt = Date.now();
  const session = await createSession(browserType, browserName, scenarioId, slice.sliceId, viewport, attempt);
  const failures = [];
  let structuredDiffPath = null;
  let pixelDiffPath = null;
  let trace = null;
  let timingPath = null;

  try {
    const readinessTiming = await slice.setup(session.page);
    const assertionResult = await runAssertions(session.page, slice.assertions);
    failures.push(...assertionResult.failures);

    if (captureConsole) {
      const consoleErrors = session.consoleEntries.filter((entry) => entry.type === "error");
      if (consoleErrors.length > 0) {
        failures.push(
          createFailure(
            "console_error_detected",
            "javascript-runtime",
            `console emitted ${consoleErrors.length} error message(s)`,
            consoleErrors.map((entry) => entry.text).join("; "),
          ),
        );
      }
    }

    if (session.pageErrors.length > 0) {
      failures.push(
        createFailure(
          "page_error_detected",
          "javascript-runtime",
          `page emitted ${session.pageErrors.length} uncaught error(s)`,
          session.pageErrors.map((entry) => entry.message).join("; "),
        ),
      );
    }

    const requestFailures = session.networkEntries.filter((entry) => entry.event === "requestfailed");
    if (requestFailures.length > 0) {
      failures.push(
        createFailure(
          "network_request_failed",
          "network-failure",
          `network emitted ${requestFailures.length} failed request(s)`,
          requestFailures.map((entry) => entry.url).join("; "),
        ),
      );
    }

    const artifactCaptureStartedAt = Date.now();
    const captured = await captureArtifacts(
      session.page,
      browserName,
      scenarioId,
      slice.sliceId,
      viewport.id,
      slice.trackedRoot,
      session,
    );
    const artifactCaptureMs = Date.now() - artifactCaptureStartedAt;

    const timingSnapshot = {
      ...readinessTiming,
      artifact_capture_ms: artifactCaptureMs,
      total_slice_ms: Date.now() - sliceStartedAt,
    };
    timingPath = path.join(timingDir, `${captured.stem}.timing.json`);
    writeJson(timingPath, timingSnapshot);

    const diffMode = effectiveDiffStrategy(slice);
    const diff = {
      strategy: diffMode,
      pixel: { changed: false, ratio: 0 },
      dom: { changed: false },
      a11y: { changed: false },
      layout: { changed: false },
      style: { changed: false },
    };

    if (slice.baseline && diffMode !== "none") {
      const baseline = baselinePaths(scenarioId, slice.sliceId, browserName, viewport.id);
      const screenshotDiff = compareScreenshot(
        captured.screenshotPath,
        baseline.screenshot,
        captured.stem,
        viewport.id,
        slice.baseline,
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
        const result = compareJsonArtifacts(
          "dom",
          captured.domSnapshot,
          baseline.dom,
          captured.stem,
          classifyStructuredDiff("dom"),
        );
        structuredDiff.dom = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = result.diffArtifact;
          diff.dom.changed = true;
        }
      }
      if (diffMode === "hybrid" && captured.a11yPath) {
        const result = compareJsonArtifacts(
          "a11y",
          captured.a11yTree,
          baseline.a11y,
          captured.stem,
          classifyStructuredDiff("a11y"),
        );
        structuredDiff.a11y = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = structuredDiffPath ?? result.diffArtifact;
          diff.a11y.changed = true;
        }
      }
      if (diffMode === "hybrid" && captured.layoutPath) {
        const result = compareJsonArtifacts(
          "layout",
          captured.layoutMetrics,
          baseline.layout,
          captured.stem,
          classifyStructuredDiff("layout"),
        );
        structuredDiff.layout = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = structuredDiffPath ?? result.diffArtifact;
          diff.layout.changed = true;
        }
      }
      if (diffMode === "hybrid" && captured.stylePath) {
        const result = compareJsonArtifacts(
          "style",
          captured.styleSnapshot,
          baseline.style,
          captured.stem,
          classifyStructuredDiff("style"),
        );
        structuredDiff.style = result;
        if (!result.equal) {
          failures.push(result.failure);
          structuredDiffPath = structuredDiffPath ?? result.diffArtifact;
          diff.style.changed = true;
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
      attempt,
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
        style_snapshot: captured.stylePath ? absolute(captured.stylePath) : null,
        console_log: absolute(captured.consolePath),
        page_errors: absolute(captured.pageErrorsPath),
        network_log: captured.networkPath ? absolute(captured.networkPath) : null,
        trace,
        pixel_diff: pixelDiffPath,
        structured_diff: structuredDiffPath,
        timing_snapshot: timingPath ? absolute(timingPath) : null,
      },
      assertions: assertionResult.results,
      metrics: {
        console_error_count: session.consoleEntries.filter((entry) => entry.type === "error").length,
        page_error_count: session.pageErrors.length,
        network_error_count: session.networkEntries.filter((entry) => entry.event === "requestfailed").length,
        timing: timingSnapshot,
      },
      diff,
      failure_categories: Array.from(new Set(failures.map((failure) => failure.category))),
      failures,
    };
  } catch (error) {
    const stem = buildArtifactStem(browserName, scenarioId, slice.sliceId, viewport.id);
    const consolePath = path.join(logsDir, `${stem}.console.jsonl`);
    const pageErrorsPath = path.join(logsDir, `${stem}.page-errors.json`);
    const networkPath = path.join(networkDir, `${stem}.network.jsonl`);
    writeJsonl(consolePath, captureConsole ? session.consoleEntries : []);
    writeJson(pageErrorsPath, session.pageErrors);
    if (captureNetwork || artifactLevel === "full") {
      writeJsonl(networkPath, session.networkEntries);
    }
    const timingSnapshot = {
      goto_ms: null,
      shell_ready_ms: null,
      scene_setup_ms: null,
      artifact_capture_ms: null,
      total_slice_ms: Date.now() - sliceStartedAt,
      dom_content_loaded_ms: null,
      load_event_ms: null,
      os_e2e_ready_mark_ms: null,
      readiness_selector_wait_ms: null,
      error_stage: "setup",
    };
    timingPath = path.join(timingDir, `${stem}.timing.json`);
    writeJson(timingPath, timingSnapshot);
    trace = await session.finalize({ keepTrace: true, suffix: "failure" });
    const message = String(error.message ?? error);
    const category = message.includes(e2eReadySelector)
      ? "readiness-timeout"
      : "race-condition";
    const failure = createFailure("setup_failed", category, message, String(error.stack ?? error));
    return {
      id: scenarioId,
      slice_id: slice.sliceId,
      browser: browserName,
      attempt,
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
        style_snapshot: null,
        console_log: absolute(consolePath),
        page_errors: absolute(pageErrorsPath),
        network_log: captureNetwork || artifactLevel === "full" ? absolute(networkPath) : null,
        trace,
        pixel_diff: null,
        structured_diff: null,
        timing_snapshot: absolute(timingPath),
      },
      assertions: [],
      metrics: {
        console_error_count: session.consoleEntries.filter((entry) => entry.type === "error").length,
        page_error_count: session.pageErrors.length,
        network_error_count: session.networkEntries.filter((entry) => entry.event === "requestfailed").length,
        timing: timingSnapshot,
      },
      diff: {
        strategy: effectiveDiffStrategy(slice),
        pixel: { changed: false, ratio: 0 },
        dom: { changed: false },
        a11y: { changed: false },
        layout: { changed: false },
        style: { changed: false },
      },
      failure_categories: [failure.category],
      failures: [failure],
    };
  }
}

async function runScenarioWithRetries(browserName, browserType, scenarioId, slice, viewport) {
  let lastResult = null;
  let firstFailureCategories = [];
  for (let attempt = 1; attempt <= retries + 1; attempt += 1) {
    const result = await executeSlice(browserName, browserType, scenarioId, slice, viewport, attempt);
    if (result.status === "passed") {
      if (attempt > 1) {
        result.failure_categories = Array.from(new Set([...(result.failure_categories ?? []), ...firstFailureCategories, "flaky"]));
      }
      return result;
    }
    lastResult = result;
    if (attempt === 1) {
      firstFailureCategories = result.failure_categories ?? [];
    }
    if (attempt <= retries) {
      console.warn(
        `retrying scenario=${scenarioId} slice=${slice.sliceId} browser=${browserName} viewport=${viewport.id} after attempt ${attempt} failed`,
      );
    }
  }
  if (lastResult && lastResult.attempt > 1) {
    lastResult.failure_categories = Array.from(new Set([...(lastResult.failure_categories ?? []), "flaky"]));
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
