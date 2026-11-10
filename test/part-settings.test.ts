// A run that sets what selects a part of a test file and runs no file that
// reads it fails, not passes as if the part had run (#1210,
// `part-settings.ts`).

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { parts } from "../scripts/shard";
import { readersOf, unreadMessage, unreadSettings } from "./part-settings";
import { underLoad } from "../scripts/timing";
import { root } from "./support";

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
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) if (value !== undefined && !settings.includes(key)) env[key] = value;
  const run = (extra: Record<string, string>) => Bun.spawnSync([process.execPath, "test", "test/node-surface.test.ts"], { cwd: root, env: { ...env, ...extra }, stderr: "pipe", stdout: "pipe" });
  const unread = run({ METAMORPHIC: "all" });
  expect(unread.exitCode).not.toBe(0);
  expect(unread.stderr.toString()).toContain("METAMORPHIC=all selects a part of test/metamorphic.test.ts, which this run didn't run");
  expect(run({}).exitCode).toBe(0);
}, underLoad(120_000));
