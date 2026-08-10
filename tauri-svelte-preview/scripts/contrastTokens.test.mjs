/**
 * Computed contrast contract for the /next semantic theme tokens.
 *
 * Run: node --experimental-strip-types scripts/contrastTokens.test.mjs
 */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  assertContrast,
  composite,
  contrastRatio,
  parseCssColor,
  relativeLuminance,
} from "../src/lib/shell/themes/contrast.ts";
import {
  PALETTE_TOKEN_NAMES,
  THEMES,
  TOKEN_NAMES,
} from "../src/lib/shell/themes/themeRegistry.ts";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  cwd: projectRoot,
  encoding: "utf8",
}).trim();
const read = (path) => readFileSync(resolve(projectRoot, path), "utf8");
const stripComments = (source) => source.replace(/\/\*[\s\S]*?\*\//g, "");

const NEXT_TOKENS_PATH = "src/lib/shell/styles/nextTokens.css";
const PRODUCT_PATHS = [
  NEXT_TOKENS_PATH,
  "src/lib/shell/styles/themeChrome.css",
  "src/lib/shell/themes/themeRegistry.ts",
  "src/lib/shell/themes/contrast.ts",
];
const PACKET_PATHS = [
  ...PRODUCT_PATHS,
  "scripts/contrastTokens.test.mjs",
  "scripts/themeRegistry.test.mjs",
  "scripts/nextTokens.test.mjs",
];

function readDeclarations(css) {
  const declarations = new Map();
  for (const match of stripComments(css).matchAll(
    /(--[a-zA-Z0-9-]+)\s*:\s*([^;}]+)/g
  )) {
    declarations.set(match[1], match[2].trim().replace(/\s+/g, " "));
  }
  return declarations;
}

const stylesheetTokens = readDeclarations(read(NEXT_TOKENS_PATH));

// Parser and alpha behavior are exact: transparency is composited only over
// an explicit opaque backdrop, and unresolved alpha is an error.
{
  assert.deepEqual(parseCssColor("#abc"), {
    red: 170,
    green: 187,
    blue: 204,
    alpha: 1,
  });
  assert.deepEqual(parseCssColor("#102030"), {
    red: 16,
    green: 32,
    blue: 48,
    alpha: 1,
  });
  assert.deepEqual(parseCssColor("rgb(16, 32, 48)"), {
    red: 16,
    green: 32,
    blue: 48,
    alpha: 1,
  });
  assert.deepEqual(parseCssColor("#ff000080", parseCssColor("#0000ff")), {
    red: 128,
    green: 0,
    blue: 127,
    alpha: 1,
  });
  assert.deepEqual(
    parseCssColor("rgba(255, 0, 0, 0.5)", parseCssColor("#0000ff")),
    {
      red: 128,
      green: 0,
      blue: 128,
      alpha: 1,
    }
  );
  assert.deepEqual(
    composite(
      { red: 255, green: 0, blue: 0, alpha: 0.5 },
      { red: 0, green: 0, blue: 255, alpha: 1 }
    ),
    { red: 128, green: 0, blue: 128, alpha: 1 }
  );
  assert.equal(relativeLuminance(parseCssColor("#000")), 0);
  assert.equal(contrastRatio(parseCssColor("#fff"), parseCssColor("#000")), 21);

  for (const malformed of [
    "",
    "#12",
    "#1234",
    "#gggggg",
    "transparent",
    "hsl(0, 0%, 0%)",
    "rgb(256, 0, 0)",
    "rgb(-1, 0, 0)",
    "rgb(0.5, 0, 0)",
    "rgba(0, 0, 0, 1.1)",
    "rgba(0, 0, 0, -0.1)",
  ]) {
    assert.throws(
      () => parseCssColor(malformed),
      undefined,
      `must reject ${JSON.stringify(malformed)}`
    );
  }
  assert.throws(
    () => parseCssColor("rgba(255, 255, 255, 0.5)"),
    /requires an opaque backdrop/
  );
  assert.throws(
    () =>
      parseCssColor("rgba(255, 255, 255, 0.5)", {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0.5,
      }),
    /translucent backdrop/
  );
  assert.throws(
    () => assertContrast("#777", "#fff", 4.5, "labeled pair"),
    /labeled pair/
  );
  assert.throws(
    () => assertContrast("not-a-color", "#fff", 4.5, "unknown foreground"),
    /unknown foreground: Unsupported CSS color/
  );
  assert.throws(
    () =>
      assertContrast(
        "#fff",
        "rgba(0, 0, 0, 0.5)",
        4.5,
        "unresolved background"
      ),
    /unresolved background:.*requires an opaque backdrop/
  );
}

// The first-frame Houston stylesheet and every runtime theme are complete.
{
  const stylesheetPalette = [...stylesheetTokens.keys()].filter((name) =>
    name.startsWith("--color-")
  );
  assert.deepEqual(
    stylesheetPalette.sort(),
    [...PALETTE_TOKEN_NAMES].sort(),
    `${NEXT_TOKENS_PATH} must define the complete registry palette`
  );
  for (const theme of THEMES) {
    assert.deepEqual(
      Object.keys(theme.tokens).sort(),
      [...TOKEN_NAMES].sort(),
      `${theme.id} must define every theme token exactly once`
    );
  }
}

const TEXT_CONTRACT = [
  { name: "--color-text", minimum: 4.5, role: "body" },
  { name: "--color-text-2", minimum: 3, role: "secondary" },
  { name: "--color-text-3", minimum: 3, role: "secondary" },
  { name: "--color-disabled-text", minimum: 3, role: "secondary" },
];
const SURFACE_TOKENS = [
  "--color-bg",
  "--color-surface",
  "--color-elevated",
  "--color-selected",
  "--color-hover",
];
const BOUNDARY_TOKENS = [
  "--color-focus-solid",
  "--color-selected-border",
];
const STATUS_TOKENS = [
  "--color-live",
  "--color-good",
  "--color-bad",
  "--color-attention",
  "--color-idle",
  "--color-status-idle",
];

let lowestText = { ratio: Number.POSITIVE_INFINITY, label: "" };
let lowestIndicator = { ratio: Number.POSITIVE_INFINITY, label: "" };

for (const theme of THEMES) {
  for (const { name: foregroundName, minimum, role } of TEXT_CONTRACT) {
    for (const backgroundName of SURFACE_TOKENS) {
      const label = `${theme.id} ${role} ${foregroundName} on ${backgroundName}`;
      const foreground = theme.tokens[foregroundName];
      const background = theme.tokens[backgroundName];
      assertContrast(foreground, background, minimum, label);
      const ratio = contrastRatio(
        parseCssColor(foreground),
        parseCssColor(background)
      );
      if (ratio < lowestText.ratio) lowestText = { ratio, label };
    }
  }

  for (const foregroundName of [...BOUNDARY_TOKENS, ...STATUS_TOKENS]) {
    for (const backgroundName of SURFACE_TOKENS) {
      const label = `${theme.id} ${foregroundName} on ${backgroundName}`;
      const foreground = theme.tokens[foregroundName];
      const background = theme.tokens[backgroundName];
      assertContrast(foreground, background, 3, label);
      const ratio = contrastRatio(
        parseCssColor(foreground),
        parseCssColor(background)
      );
      if (ratio < lowestIndicator.ratio) lowestIndicator = { ratio, label };
    }
  }

  {
    // A hairline is a translucent surface boundary, not an attention signal.
    // Resolve it over the raised layer it separates before measuring it.
    const elevated = parseCssColor(theme.tokens["--color-elevated"]);
    const borderContrast = contrastRatio(
      parseCssColor(theme.tokens["--color-border"], elevated),
      elevated,
    );
    assert.ok(
      borderContrast >= 1.4 && borderContrast < 2,
      `${theme.id} border should be a quiet surface boundary (got ${borderContrast.toFixed(2)}:1)`,
    );
    assert.ok(
      contrastRatio(
        parseCssColor(theme.tokens["--color-text-2"]),
        parseCssColor(theme.tokens["--color-bg"]),
      ) > borderContrast,
      `${theme.id} secondary text must carry more contrast than its hairlines`,
    );
  }

  assert.equal(
    theme.tokens["--color-section-header-text"],
    theme.tokens["--color-text-3"],
    `${theme.id} section headings must use the muted text tone`
  );
  assert.equal(
    theme.tokens["--color-tab-unfocused-text"],
    theme.tokens["--color-text-2"],
    `${theme.id} unfocused tab labels carry information and must use the secondary-text tone`
  );
}

function hue(colorValue) {
  const color = parseCssColor(colorValue);
  const red = color.red / 255;
  const green = color.green / 255;
  const blue = color.blue / 255;
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const chroma = maximum - minimum;
  assert.ok(
    chroma >= 0.15,
    `${colorValue} is too neutral to carry status by hue`
  );
  if (maximum === red) return (((green - blue) / chroma + 6) % 6) * 60;
  if (maximum === green) return ((blue - red) / chroma + 2) * 60;
  return ((red - green) / chroma + 4) * 60;
}

for (const theme of THEMES) {
  const live = theme.tokens["--color-live"];
  const good = theme.tokens["--color-good"];
  assert.notEqual(
    live.toLowerCase(),
    good.toLowerCase(),
    `${theme.id} live and good must differ`
  );
  const rawDistance = Math.abs(hue(live) - hue(good));
  const hueDistance = Math.min(rawDistance, 360 - rawDistance);
  assert.ok(
    hueDistance >= 20,
    `${theme.id} live/good hue separation is only ${hueDistance.toFixed(
      1
    )} degrees`
  );
}

// Raw-color audit: unchanged registry literals are the explicit baseline
// allowance for Monaco/xterm. New product literals may only define a semantic
// custom property or registry token. Test files are an explicit fixture-only
// allowance because malformed and boundary colors are their subject.
{
  const rawColor = /#[0-9a-fA-F]{3,8}\b|rgba?\([^)]*\)/g;
  const baselineLines = new Map();
  const baselineSource = (path) => {
    const repositoryPath = relative(repositoryRoot, resolve(projectRoot, path));
    try {
      return execFileSync("git", ["show", `HEAD:${repositoryPath}`], {
        cwd: repositoryRoot,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      });
    } catch {
      return "";
    }
  };

  for (const path of PRODUCT_PATHS) {
    baselineLines.set(path, new Set(baselineSource(path).split("\n")));
    const violations = [];
    for (const [index, line] of read(path).split("\n").entries()) {
      if (![...line.matchAll(rawColor)].length) continue;
      if (baselineLines.get(path).has(line)) continue;
      if (
        path.endsWith("contrast.ts") &&
        line.includes("Unsupported CSS color")
      )
        continue;
      if (path.endsWith(".css") && /^\s*--[a-zA-Z0-9-]+\s*:/.test(line))
        continue;
      if (
        path.endsWith("themeRegistry.ts") &&
        /^\s*'--color-[a-zA-Z0-9-]+'\s*:/.test(line)
      ) {
        continue;
      }
      violations.push(`${path}:${index + 1}: ${line.trim()}`);
    }
    assert.deepEqual(
      violations,
      [],
      `new raw color literals must be semantic token declarations:\n${violations.join(
        "\n"
      )}`
    );
  }
}

// Packet-local type-size guard. A value below 12px needs an explicit
// decorative/aria-hidden marker on the same line; otherwise the evidence names
// the exact file and line for correction.
{
  const violations = [];
  for (const path of PACKET_PATHS) {
    for (const [index, line] of read(path).split("\n").entries()) {
      const sizes = [
        ...line.matchAll(/font-size\s*:\s*(\d+(?:\.\d+)?)px/gi),
        ...line.matchAll(/text-\[(\d+(?:\.\d+)?)px\]/gi),
      ];
      for (const match of sizes) {
        if (Number(match[1]) >= 12 || /decorative|aria-hidden/i.test(line))
          continue;
        violations.push(`${path}:${index + 1}: ${line.trim()}`);
      }
    }
  }
  assert.deepEqual(
    violations,
    [],
    `information-bearing text must be at least 12px:\n${violations.join("\n")}`
  );
}

console.log(
  `contrastTokens.test.mjs: all checks passed; lowest text ${lowestText.ratio.toFixed(
    2
  )}:1 ` +
    `(${lowestText.label}); lowest indicator ${lowestIndicator.ratio.toFixed(
      2
    )}:1 ` +
    `(${lowestIndicator.label})`
);
