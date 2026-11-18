// A run that sets what selects a part of a test file and runs no file that
// reads it fails, not passes as if the part had run (#1210,
// `part-settings.ts`).

import { expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { parts } from "../scripts/shard";
import { readersOf, unreadMessage, unreadSettings } from "./part-settings";
import { underLoad } from "../scripts/timing";
import { bunTest, fixture, root, runModes } from "./support";

const settings = [...new Set(parts.map((part) => part.setting))];

test("a setting a run sets and nothing reads is unread, and its message names the file that reads it", () => {
  expect(unreadSettings({ METAMORPHIC: "all" }, new Set())).toEqual(["METAMORPHIC"]);
  expect(unreadSettings({ METAMORPHIC: "all" }, new Set(["METAMORPHIC"]))).toEqual([]);
  expect(unreadSettings({ METAMORPHIC: "" }, new Set())).toEqual([]);
  expect(unreadSettings({ PATH: "/bin" }, new Set())).toEqual([]);
  expect(unreadMessage([], {})).toBeUndefined();
  expect(unreadMessage(["METAMORPHIC"], { METAMORPHIC: "all" })).toContain("METAMORPHIC=all selects a part of test/metamorphic.test.ts");
});

test("each setting that selects a part is read through `partSetting`, so a run can say it was", () => {
  for (const setting of settings) expect(readersOf(setting), setting).not.toEqual([]);
  const direct = readdirSync(join(root, "test"))
    .filter((file) => /\.tsx?$/.test(file))
    .flatMap((file) => {
      const text = readFileSync(join(root, "test", file), "utf8");
      return settings.filter((setting) => new RegExp(`process\\.env(\\.${setting}\\b|\\[["']${setting}["']\\])`).test(text)).map((setting) => `${file}: ${setting}`);
    });
  expect(direct).toEqual([]);
});

test("`METAMORPHIC=all` of a file that runs no rewrite fails the run", () => {
  const run = (env: Record<string, string>) => bunTest(["test/node-surface.test.ts"], { env });
  const unread = run({ METAMORPHIC: "all" });
  expect(unread.code).not.toBe(0);
  expect(unread.stderr).toContain("METAMORPHIC=all selects a part of test/metamorphic.test.ts, which this run didn't run");
  expect(run({}).code).toBe(0);
}, underLoad(120_000));

// A test's own `bun test` child is given none of its parent's run modes:
// `BLESS=1` of a check's "Bless what differs" step blessed a fake compiler's
// JS in harness.test.ts's corpus child (#2066).
test("a test's own `bun test` child runs without its parent's run modes", () => {
  const dir = fixture("run-modes-child");
  const parent: Record<string, string> = { BLESS: "1", METAMORPHIC: "all", KEPT: "1" };
  writeFileSync(
    join(dir, "modes.test.ts"),
    `import { expect, test } from "bun:test";\ntest("modes", () => expect(Object.fromEntries(${JSON.stringify([...Object.keys(parent), "RUST_JS_SNAPSHOTS"])}.map((name) => [name, process.env[name]]))).toEqual({ KEPT: "1", RUST_JS_SNAPSHOTS: "only" }));\n`,
  );
  const was = Object.fromEntries(Object.keys(parent).map((name) => [name, process.env[name]]));
  Object.assign(process.env, parent);
  try {
    const ran = bunTest([join(dir, "modes.test.ts")], { env: { RUST_JS_SNAPSHOTS: "only" } });
    expect([ran.code, ran.stdout + ran.stderr]).toEqual([0, expect.stringContaining("1 pass")]);
  } finally {
    for (const [name, value] of Object.entries(was)) if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
  expect(runModes).toEqual(expect.arrayContaining(["BLESS", ...settings]));
}, underLoad(60_000));

// A test's own `bun test` child runs through support.ts's `bunTest`, which
// gives it none of its parent's run modes (`runModes`): a check's "Bless
// what differs" step ran harness.test.ts with `BLESS=1`, and its corpus
// child blessed a fake compiler's JS where it was to fail (#2066).
test("a test runs `bun test` of its own through `bunTest` alone", () => {
  const spawns = readdirSync(join(root, "test"))
    .filter((file) => /\.tsx?$/.test(file) && file !== "support.ts")
    .filter((file) => /(process\.execPath|"bun")\s*,\s*\[?\s*"test"/.test(readFileSync(join(root, "test", file), "utf8")));
  expect(spawns).toEqual([]);
});
