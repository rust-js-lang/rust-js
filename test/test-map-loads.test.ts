// What the test map records each test file loading (scripts/testMapLoads.ts,
// #2482): its own process's modules and each child's, Bun's and Node's, a
// Vite config by what its bundle was made of, the recorder out of what a fork
// is given; a child no recorder follows said; and each test's list in the
// map, its parts' together, carried as its functions are.

import { expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { loadedFiles, recording, unitLoads } from "../scripts/testMapLoads";
import { dropsRecording, viteSources } from "../scripts/testMapLoads/shared.mjs";
import { mapVersion, mergeRecordings, type Plan } from "../scripts/testMap";
import { bunTest, fixture, root } from "./support";

/** A Vite config's bundle, as Vite writes it: its code, then an inline source
 * map naming the config it was made of. */
const bundle = (source: string) =>
  `export default {};\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,${Buffer.from(JSON.stringify({ version: 3, sources: [source], mappings: "" })).toString("base64")}\n`;

test("a test's loads are its process's and each child's, Bun's and Node's, a Vite config's by its bundle's sources", () => {
  const dir = fixture("test-map-loads");
  const temp = join(dir, "node_modules", ".vite-temp");
  mkdirSync(temp, { recursive: true });
  writeFileSync(join(temp, "vitest.config.ts.timestamp-1-a.mjs"), bundle(join(root, "browser", "vitest.config.ts")));
  writeFileSync(join(temp, "vite.config.ts.timestamp-2-b.mjs"), bundle(join(root, "browser", "playwright.config.ts")));
  // A Bun child, which says what a fork of it would be given; a Node child;
  // and a Bun child given an environment of its own.
  writeFileSync(join(dir, "child.ts"), `import ${JSON.stringify(join(root, "tooling", "manifest.js"))};\nawait import(${JSON.stringify(join(temp, "vitest.config.ts.timestamp-1-a.mjs"))});\nconsole.log(JSON.stringify(process.execArgv));\n`);
  writeFileSync(join(dir, "child.mjs"), `import ${JSON.stringify(join(root, "react", "cfg.js"))};\nawait import(${JSON.stringify(join(temp, "vite.config.ts.timestamp-2-b.mjs"))});\n`);
  writeFileSync(
    join(dir, "inner.test.ts"),
    `import { expect, test } from "bun:test";
import { crates } from ${JSON.stringify(join(root, "tooling", "crates.js"))};
test("children", () => {
  expect(crates.length).toBeGreaterThan(0);
  const bun = Bun.spawnSync(["bun", "child.ts"], { cwd: ${JSON.stringify(dir)}, env: process.env, stdout: "pipe", stderr: "inherit" });
  expect(bun.exitCode).toBe(0);
  console.log("forked with " + bun.stdout.toString().trim());
  const node = Bun.spawnSync(["node", "child.mjs"], { cwd: ${JSON.stringify(dir)}, env: process.env, stderr: "inherit" });
  expect(node.exitCode).toBe(0);
  if (require("node:fs").existsSync("lose")) Bun.spawnSync(["bun", "-e", "0"], { env: { PATH: process.env.PATH } });
});
`,
  );
  const run = (lose: boolean) => {
    const loads = join(dir, lose ? "lost" : "loads");
    mkdirSync(loads, { recursive: true });
    if (lose) writeFileSync(join(dir, "lose"), "");
    const env = { ...process.env, RUST_JS_TEST_MAP_ROOT: root };
    // Its test file's run, as `runUnits` gives it: the recorder after `bun test`.
    const recorded = recording("bun test ./inner.test.ts".split(" "), env, loads);
    expect(recorded.cmd.join(" ")).toBe(`bun test --preload=${join(root, "scripts", "testMapLoads", "test.ts")} ./inner.test.ts`);
    const p = bunTest(recorded.cmd.slice(2), { cwd: dir, env: { ...env, ...recorded.env } });
    const said = p.stdout + p.stderr;
    expect([p.code, said]).toEqual([0, expect.any(String)]);
    return { said, loads: unitLoads(loads, root) };
  };
  const { said, loads } = run(false);
  expect(loads).toEqual({ files: ["browser/playwright.config.ts", "browser/vitest.config.ts", "react/cfg.js", "tooling/crates.js", "tooling/manifest.js"] });
  // The recorder takes itself out of what a fork is given, which Next.js's
  // dev server gives its workers.
  expect(said).toContain("forked with []");
  // A child given an environment that drops the folder: its loads are no
  // list's, so the test's are unknown.
  expect(run(true).loads.lost).toBe("bun -e 0");
  rmSync(dir, { recursive: true, force: true });
});

test("a load list is the checkout's files, from its root, each once: no package's, no target's, no recorder's", () => {
  const listed = [
    `file://${root}/tooling/build.js`,
    `${root}/tooling/build.js`,
    `${root}/node_modules/@rust-js/typescript/index.js`,
    `${root}/node_modules/vite/dist/node/index.js`,
    `${root}/target/fixtures/x/app.js`,
    `${root}/scripts/testMapLoads/bun.ts`,
    `${root}/test/[eval]`,
    "node:fs",
    "/elsewhere/a.js",
  ];
  // A workspace package by its folder.
  expect(loadedFiles(listed, root)).toEqual(["tooling/build.js", "typescript/index.js"]);
  // A recorder given once, where a recorded test records a test of its own.
  const once = recording(["x"], { BUN_OPTIONS: `--smol --preload=${join(root, "scripts", "testMapLoads", "bun.ts")}` }, "/x").env.BUN_OPTIONS;
  expect(once).toBe(`--smol --preload=${join(root, "scripts", "testMapLoads", "bun.ts")}`);
});

test("a Vite config's bundle is the files its source map names; a child of its own environment loses its loads", () => {
  const path = `${root}/node_modules/.vite-temp/vitest.config.ts.timestamp-9-c.mjs`;
  expect(viteSources(path, bundle(`${root}/browser/vitest.config.ts`))).toEqual([`${root}/browser/vitest.config.ts`]);
  expect(viteSources(path, bundle("../../browser/vitest.config.ts"))).toEqual([`${root}/browser/vitest.config.ts`]);
  expect(viteSources(`${root}/tooling/build.js`, bundle("x.ts"))).toEqual([]);
  expect(viteSources(path, "export default {};\n")).toEqual([]);
  expect(dropsRecording("bun", { PATH: "/bin" })).toBe(true);
  expect(dropsRecording("/usr/local/bin/node --test", { PATH: "/bin" })).toBe(true);
  expect(dropsRecording("bun", { PATH: "/bin", RUST_JS_TEST_MAP_LOADS: "/x" })).toBe(false);
  expect(dropsRecording("bun", undefined)).toBe(false);
  expect(dropsRecording("cargo", { PATH: "/bin" })).toBe(false);
});

test("the map keeps each test file's loads, its parts' together, one table of paths; a failed or lost one unknown, a carried one carried", () => {
  const dir = fixture("test-map-loads-merge");
  const files = ["test/a.test.ts", "test/b.test.ts", "test/c.test.ts", "test/d.test.ts", "test/metamorphic.test.ts"];
  const unit = (name: string, shard: number) => ({ name, cmd: [] as string[], env: {}, shard });
  const plan = (shards: number): Plan => ({
    version: mapVersion,
    commit: "c1",
    toolchain: "t",
    shards,
    parts: 1,
    names: ["f0"],
    functions: [{ file: "src/a.rs", spans: [[1, 2]] }],
    files,
    caseNames: [],
    readsSources: [],
    corpusReaders: [],
    includes: {},
    units: [...files.filter((file) => file !== "test/metamorphic.test.ts").map((file, at) => unit(file, (at % shards) + 1)), unit("test/metamorphic.test.ts 1/2", 1), unit("test/metamorphic.test.ts 2/2", shards)],
  });
  const said: Record<string, { files: string[]; lost?: string }> = {
    "test/a.test.ts": { files: ["tooling/build.js", "tooling/cli.js"] },
    "test/b.test.ts": { files: ["browser/vitest.config.ts"] },
    "test/c.test.ts": { files: ["tooling/build.js"], lost: "bun -e 0" },
    "test/d.test.ts": { files: ["tooling/build.js"] },
    "test/metamorphic.test.ts 1/2": { files: ["serde/cargo.ts"] },
    "test/metamorphic.test.ts 2/2": { files: ["serde/flags.ts"] },
  };
  const record = (of: Plan, at: string, lists = said) => {
    for (let shard = 1; shard <= of.shards; shard++) {
      const part = join(dir, at, String(shard));
      mkdirSync(join(part, "units"), { recursive: true });
      mkdirSync(join(part, "loads"), { recursive: true });
      const ended: Record<string, number> = {};
      of.units.filter((u) => u.shard === shard).forEach((u, n) => {
        ended[u.name] = u.name === "test/d.test.ts" ? 1 : 0;
        const file = u.name.split(" ")[0]!;
        writeFileSync(join(part, "units", `${file.replaceAll("/", "__")}.${n}-${shard}.bits`), new Uint8Array([1]));
        writeFileSync(join(part, "loads", `${n}.json`), JSON.stringify({ unit: u.name, ...lists[u.name] }));
      });
      writeFileSync(join(part, "ended.json"), JSON.stringify(ended));
    }
    return Array.from({ length: of.shards }, (_, i) => join(dir, at, String(i + 1)));
  };
  const whole = mergeRecordings(plan(1), record(plan(1), "one"), 1).map;
  const parted = mergeRecordings(plan(2), record(plan(2), "two"), 1).map;
  expect(JSON.stringify(parted)).toBe(JSON.stringify(whole));
  expect(whole.paths).toEqual(["browser/vitest.config.ts", "serde/cargo.ts", "serde/flags.ts", "tooling/build.js", "tooling/cli.js"]);
  expect(whole.loads!.of).toMatchObject({
    "test/a.test.ts": [3, 4],
    "test/b.test.ts": [0],
    // A child no recorder followed, and a test that failed: unknown.
    "test/c.test.ts": null,
    "test/d.test.ts": null,
    "test/metamorphic.test.ts": [1, 2],
  });
  // An incremental recording: what it records, its own lists; the rest
  // carried from the map it carries from, by their paths.
  const recorded = ["test/a.test.ts", "test/c.test.ts", "test/d.test.ts"];
  const incremental: Plan = { ...plan(1), carry: { from: "c1", base: { commit: "c1", made: 1 }, recorded, cases: [], forms: [] } };
  incremental.units = incremental.units.filter((u) => recorded.includes(u.name));
  const carried = mergeRecordings(incremental, record(incremental, "carried", { ...said, "test/a.test.ts": { files: ["wasm/web/site.ts"] } }), 2, whole).map;
  expect(carried.paths).toEqual(["browser/vitest.config.ts", "serde/cargo.ts", "serde/flags.ts", "wasm/web/site.ts"]);
  expect(carried.loads!.of).toMatchObject({ "test/a.test.ts": [3], "test/b.test.ts": [0], "test/c.test.ts": null, "test/metamorphic.test.ts": [1, 2] });
  rmSync(dir, { recursive: true, force: true });
});
