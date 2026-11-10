// What development keeps on disk, held by its owners (#1208, #1213, ADR
// 1118): each test of a store gives it the disk's answer, and only the
// stores' owners make a path where development keeps things outside a
// checkout.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");

// A test of what a store keeps gives it the disk's answer: a store keeps
// nothing below the disk's floor (test/bounded.ts), so one asked of the real
// disk passes or fails by the machine's free space, as the kept-app test
// (#1190) and the verified-runs test (#1208) did. Each call, in the files
// that test the stores, of a function that asks the floor passes its own
// `free` (or `room`, `floorBytes`, a `KeptApp`'s `keep`); the one test of
// the real disk itself (#963) is named so.
test("a test of what a store keeps gives it the disk's answer", () => {
  const asks = /\b(record|mayKeep|pruneKept|pruneKeptApps|pruneKeptStores|prune|bound|boundCrateProjects|boundEntries|keptCompile|new KeptApp)\(/g;
  const answered = /\bfree\b|\broom\b|floorBytes|, true\)$/;
  const unanswered: string[] = [];
  for (const file of ["test/support.test.ts", "test/compiles.test.ts", "test/manifest.test.ts"]) {
    const text = read(file);
    for (const match of text.matchAll(asks)) {
      // The call's text, to its closing parenthesis.
      let depth = 0;
      let end = match.index + match[0].length - 1;
      for (; end < text.length; end++) {
        if (text[end] === "(") depth++;
        if (text[end] === ")" && --depth === 0) break;
      }
      const call = text.slice(match.index, end + 1);
      const testName = [...text.slice(0, match.index).matchAll(/\btest\("([^"]*)"/g)].at(-1)?.[1] ?? "";
      if (testName.includes("real disk") || answered.test(call)) continue;
      const line = text.slice(0, match.index).split("\n").length;
      unanswered.push(`${file}:${line}: ${call.split("\n")[0]}`);
    }
  }
  expect(unanswered).toEqual([]);
});

// What development keeps has one owner of its places (#1213, ADR 1118):
// test/bounded.ts says where the machine's stores and the temp root are,
// scripts/disk.ts what the tools keep outside the checkouts, so `bun run
// disk` reports each and nothing grows unseen. No other script or test
// makes a path in the user's home: a store made there would be one no
// report or bound sees.
test("only the stores' owners name where development keeps things outside a checkout", () => {
  const owners = new Set(["test/bounded.ts", "scripts/disk.ts"]);
  const files = [
    ...readdirSync(join(root, "scripts")).filter((f) => /\.(ts|js)$/.test(f)).map((f) => `scripts/${f}`),
    ...readdirSync(join(root, "test")).filter((f) => /\.(ts|js)$/.test(f) && !f.endsWith(".test.ts")).map((f) => `test/${f}`),
  ];
  const named = files
    .filter((file) => !owners.has(file))
    .filter((file) => read(file).split("\n").some((line) => !/^\s*(\/\/|\*)/.test(line) && /join\(\s*homedir\(\)|["'`]~\//.test(line)));
  expect(named).toEqual([]);
});
