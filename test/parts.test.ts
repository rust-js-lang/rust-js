// How scripts/parts.ts runs a test file of cases in parts on one machine,
// as the nightly runs its nets (#1306): what a spec names, a part's line,
// a file of no cases or a second file refused, a list of cases each
// part's (#2618), and a part the machine kills said so.

import { expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { caseOf, givenList, partCases, partsOf, partsOfList, runParts, unparted, verdict } from "../scripts/parts";
import { underLoad } from "../scripts/timing";
import { root } from "./support";

// The nightly runs a test file in parts by scripts/parts.ts alone (#1306):
// each part's lines as they come, its peak memory and what held it, and a
// part the machine kills said so, with where it was. A shell loop of
// `bun test ... &` printed nothing till every part ended, and nightly
// 42884008739's metamorphic net ended in a bare exit 143.
test("a workflow runs a test file in parts by scripts/parts.ts alone", () => {
  for (const file of readdirSync(join(root, ".github/workflows")).filter((name) => name.endsWith(".yml"))) {
    const text = readFileSync(join(root, ".github/workflows", file), "utf8");
    expect(text, file).not.toMatch(/bun test[^\n]*&\s*$/m);
  }
});

test("a part's spec names its parts, and a part's line says how it ended", () => {
  expect(partsOf("4")).toEqual({ parts: [1, 2, 3, 4], count: 4 });
  expect(partsOf("5-8/24")).toEqual({ parts: [5, 6, 7, 8], count: 24 });
  for (const bad of ["0", "3-2/8", "7-9/8", "4/2", "x"]) expect(() => partsOf(bad), bad).toThrow();
  expect(caseOf("(pass) seed 3 group 14: 16 cases [43707.03ms]")).toBe("seed 3 group 14: 16 cases");
  expect(caseOf("(fail) a, rewritten")).toBe("a, rewritten");
  expect(caseOf(" 53 pass")).toBeUndefined();
  expect(verdict(0, null, "(pass) a\n 1 pass\n 0 fail\n")).toBe("passed");
  expect(verdict(1, null, "(fail) a\n 0 pass\n 1 fail\n")).toBe("failed");
  expect(verdict(1, null, 'Test filter "x" had no matches\n')).toBe("failed");
  expect(verdict(null, "SIGKILL", "(pass) a\n")).toBe("killed (memory?)");
  expect(verdict(143, null, "(pass) a\n")).toBe("killed (memory?)");
});

// A file in parts is one of cases each part takes its share of: a file of
// none, or a second file among the flags, ran whole in every part, and the
// suite's files in 10 parts filled the disk with 20 GiB of fixtures. Each
// file a workflow runs in parts is one. (The sources below are built, so
// this file reads as one of no cases, which it is.)
test("parts.ts runs one file of cases in parts, and refuses another, naming check:fast", () => {
  const cases = ["RUST", "JS", "CASES"].join("_");
  const sharded = `const [part, parts] = (process.env.${cases} ?? "1/1").split("/").map(Number);`;
  expect(unparted("test/a.test.ts", sharded, ["--timeout=60000"])).toBeUndefined();
  expect(unparted("test/a.test.ts", `for (const name of ${"caseShard"}(cases)) test(name, () => {});`, [])).toBeUndefined();
  expect(unparted("test/ci.test.ts", readFileSync(join(root, "test/ci.test.ts"), "utf8"), [])).toContain("each part would run it whole: run the suite with `bun run check:fast`");
  expect(unparted("test/a.test.ts", sharded, ["test/b.test.ts", "--timeout=1"])).toContain("test/b.test.ts: one test file runs in parts");
  const workflows = readdirSync(join(root, ".github/workflows")).filter((name) => name.endsWith(".yml"));
  const runs = workflows.flatMap((name) => [...readFileSync(join(root, ".github/workflows", name), "utf8").matchAll(/bun scripts\/parts\.ts .*?(test\/\S+\.test\.tsx?)([^\n]*)/g)]);
  expect(runs.length).toBeGreaterThan(0);
  for (const [, file, flags] of runs) expect(unparted(file!, readFileSync(join(root, file!), "utf8"), flags!.trim().split(/\s+/).filter(Boolean)), file).toBeUndefined();
});

// A list of cases given parts.ts, `RUST_JS_CASES=1/1:<file>`, is each
// part's, `i/n:<file>`: the list dropped, each part given `i/n` alone,
// ran the file's default set, the metamorphic net's push sample, and
// passed as if it had run the list (#2618). Its parts together run each
// case it names once and nothing else, at most a part a case.
test("parts.ts runs a list's cases in parts, each once, and nothing else", () => {
  const dir = mkdtempSync(join(tmpdir(), "parts-list-"));
  try {
    // (Its source built, as this file is one of no cases.)
    const shardOf = ["case", "Shard"].join("");
    const file = join(dir, "listed.test.ts");
    writeFileSync(
      file,
      `import { test } from "bun:test";\nimport { ${shardOf} } from ${JSON.stringify(join(root, "test", "support"))};\n` +
        `for (const name of ${shardOf}(["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"])) test(name, () => console.log(\`ran \${name}\`));\n`,
    );
    const list = join(dir, "cases.txt");
    writeFileSync(list, "b.rs\nd.rs\ne.rs\n");
    const p = Bun.spawnSync(["bun", "scripts/parts.ts", "4", file], { cwd: root, env: { ...process.env, RUST_JS_CASES: `1/1:${list}` } });
    const out = p.stdout.toString() + p.stderr.toString();
    expect(p.exitCode, out).toBe(0);
    const ran = [...out.matchAll(/^\[part \d+\/\d+\] ran (\S+)$/gm)].map((m) => m[1]);
    expect(ran.sort(), out).toEqual(["b.rs", "d.rs", "e.rs"]);
    expect(out).toContain("── 3 parts of 3, 3 at once");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, underLoad(60_000));

// As the spec names the parts, the environment's part is `1/1` or
// nothing: another, `2/3:<file>` or `3/4`, overwritten in each part, ran
// what the spec says, not what it asked (#2618). A list is made absolute,
// as each part runs in the checkout, and runs at most a part a case, but
// of a machine's share of a split, `a-b/n`, the parts it names.
test("parts.ts refuses a part of the environment's own, and runs a list at most a part a case", () => {
  expect(givenList(undefined)).toBeUndefined();
  expect(givenList("1/1")).toBeUndefined();
  expect(givenList("1/1:cases.txt", "/work")).toBe("/work/cases.txt");
  for (const given of ["2/3:cases.txt", "3/4", "1/2"]) {
    expect(() => givenList(given), given).toThrow(`RUST_JS_CASES=${given}: parts.ts's spec names its parts, and \`RUST_JS_CASES=1/1:<file>\` the cases they run, a line each; give no other part`);
  }
  const dir = mkdtempSync(join(tmpdir(), "parts-cap-"));
  try {
    const list = join(dir, "cases.txt");
    writeFileSync(list, "b.rs\nd.rs\nb.rs\n\n");
    expect(partsOfList("12", list)).toEqual({ parts: [1, 2], count: 2, list });
    expect(partsOfList("2", list)).toEqual({ parts: [1, 2], count: 2, list });
    expect(partsOfList("1", list)).toEqual({ parts: [1], count: 1, list });
    expect(partsOfList("5-8/12", list)).toEqual({ parts: [5, 6, 7, 8], count: 12, list });
    expect(partsOfList("12", undefined)).toEqual(partsOf("12"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  expect(partCases(2, 6)).toBe("2/6");
  expect(partCases(2, 6, "/work/cases.txt")).toBe("2/6:/work/cases.txt");
});

test("a part the machine kills is said killed, with what it ran and its peak", async () => {
  // Outside test/, where this checkout's `bun test` finds no file.
  const dir = mkdtempSync(join(tmpdir(), "parts-"));
  try {
    writeFileSync(join(dir, "killed.test.ts"), 'import { test } from "bun:test";\ntest("first", () => {});\ntest("second", async () => {\n  await Bun.sleep(1500);\n  process.kill(process.pid, "SIGKILL");\n});\n');
    const said: string[] = [];
    const log = console.log;
    console.log = (line: string) => said.push(line);
    try {
      expect(await runParts("./killed.test.ts", partsOf("1"), 1, [], dir)).toBe(false);
    } finally {
      console.log = log;
    }
    // `bun test` holds a file's lines till it ends where its output is a
    // pipe, but streams them under GitHub Actions, where it prints its
    // `::group::`s: the last case said is none, or the one before the kill.
    const ended = said.find((line) => line.startsWith("── part 1/1 killed (memory?)"));
    expect(ended, said.join("\n")).toMatch(/at most \d+\.\d GB.*; the last case it said ended (none|first)$/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, underLoad(60_000));
