// The test map, which the selector reads to run only the test files and
// corpus cases a change to the compiler's code can affect
// (scripts/testMap.ts, #877; scripts/select.ts, test/select.test.ts): what
// such a change picks, where it can't say, and how many maps it keeps.

import { expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

import { cachePast, ciMapOf, ciMapPrefix, hunksOf, joinedSpans, mapFile, mapVersion, mergeRecordings, nearestMap, nestedItem, pick, planUnits, pruneMaps, readsOf, readsSources, type Change, type Plan, type TestMap } from "../scripts/testMap";

import { shard } from "../scripts/shard";
import { caseShard, fixture, root } from "./support";

const bits = (...functions: number[]) => {
  const bytes = new Uint8Array(1);
  for (const f of functions) bytes[f >> 3] |= 1 << (f & 7);
  return Buffer.from(bytes).toString("base64");
};

// Two functions of a.rs, lines 10-20 and 30-40, the second with its code
// in two spans, a nested item's lines between; one of runtime.rs, that
// reads helper.js at its line 5; and of c.rs, a function, lines 1-8, with a
// closure in it, lines 3-5, and two more after it, side by side.
const map: TestMap = {
  version: 1,
  commit: "0",
  toolchain: "0",
  made: 0,
  functions: [
    { file: "src/a.rs", spans: [[10, 20]] },
    {
      file: "src/a.rs",
      spans: [
        [30, 33],
        [37, 40],
      ],
    },
    { file: "src/runtime.rs", spans: [[1, 9]] },
    {
      file: "src/c.rs",
      spans: [
        [1, 2],
        [6, 8],
      ],
    },
    { file: "src/c.rs", spans: [[3, 5]] },
    { file: "src/c.rs", spans: [[10, 12]] },
    { file: "src/c.rs", spans: [[13, 14]] },
  ],
  units: {
    "test/one.test.ts": bits(0, 3),
    "test/two.test.ts": bits(1, 2, 4),
    "test/reads.test.ts": null,
    "test/corpus.test.ts": bits(0),
    "test/corpus/x.rs": bits(1),
    "test/corpus/y.rs": bits(2),
    // The corpus built with std's iterator structs, each case its own.
    "test/iterator-forms.test.ts": bits(1),
    "test/iterator-forms.test.ts test/corpus/x.rs": bits(4),
    "test/iterator-forms.test.ts test/corpus/y.rs": bits(0),
  },
  includes: { "src/runtime/helper.js": [{ file: "src/runtime.rs", line: 5 }] },
  corpusReaders: ["test/two.test.ts"],
  noCompiler: [],
};
const files = ["test/one.test.ts", "test/two.test.ts", "test/reads.test.ts"];
const cases = ["x.rs", "y.rs"];

test("a change to a function's code runs the tests that ran it, and those that run on every change", () => {
  expect(pick(map, [{ file: "src/a.rs", hunks: [[12, 2]] }], files, cases)).toEqual({
    kind: "picked",
    files: ["test/one.test.ts", "test/reads.test.ts", "test/corpus.test.ts"],
    cases: [],
    forms: ["y.rs"],
    why: "1 of the compiler's functions changed",
  });
  expect(pick(map, [{ file: "src/a.rs", hunks: [[38, 1]] }], files, cases)).toMatchObject({
    files: ["test/two.test.ts", "test/reads.test.ts", "test/iterator-forms.test.ts"],
    cases: ["x.rs"],
    forms: [],
  });
});

test("a line added between two of a function's lines is in it; one added after its last isn't", () => {
  expect(pick(map, [{ file: "src/a.rs", hunks: [[15, 0]] }], files, cases)).toMatchObject({ kind: "picked", files: ["test/one.test.ts", "test/reads.test.ts", "test/corpus.test.ts"] });
  expect(pick(map, [{ file: "src/a.rs", hunks: [[20, 0]] }], files, cases)).toMatchObject({ kind: "all" });
  expect(pick(map, [{ file: "src/a.rs", hunks: [[9, 0]] }], files, cases)).toMatchObject({ kind: "all" });
  // Between a function's line and a closure's in it: in the function.
  expect(pick(map, [{ file: "src/c.rs", hunks: [[2, 0]] }], files, cases)).toMatchObject({ kind: "picked", files: ["test/one.test.ts", "test/two.test.ts", "test/reads.test.ts"], forms: ["x.rs"] });
  expect(pick(map, [{ file: "src/c.rs", hunks: [[5, 0]] }], files, cases)).toMatchObject({ kind: "picked" });
  // Between two functions side by side, or after one, where an item may go.
  expect(pick(map, [{ file: "src/c.rs", hunks: [[12, 0]] }], files, cases)).toMatchObject({ kind: "all" });
  expect(pick(map, [{ file: "src/c.rs", hunks: [[8, 0]] }], files, cases)).toMatchObject({ kind: "all" });
});

test("an item written inside a function may be used outside it: the map can't say", () => {
  expect(pick(map, [{ file: "src/a.rs", hunks: [[15, 0, ["    let x = 1;"]]] }], files, cases).kind).toBe("picked");
  for (const item of ["    impl Foo {", "    const C: u32 = 2;", "    static S: u8 = 0;"]) {
    expect(pick(map, [{ file: "src/a.rs", hunks: [[15, 1, [item]]] }], files, cases)).toMatchObject({ kind: "all" });
  }
});

test("a change the map can't say runs what the check runs without it", () => {
  // Outside any function's code: a type, a constant, an attribute.
  expect(pick(map, [{ file: "src/a.rs", hunks: [[25, 1]] }], files, cases).kind).toBe("all");
  // A nested item's lines, between a function's spans.
  expect(pick(map, [{ file: "src/a.rs", hunks: [[35, 1]] }], files, cases).kind).toBe("all");
  // One changed line of many outside.
  expect(pick(map, [{ file: "src/a.rs", hunks: [[19, 3]] }], files, cases).kind).toBe("all");
  // A file the map has no function of, a new one, or one outside `src/`.
  expect(pick(map, [{ file: "src/b.rs", hunks: [[1, 1]] }], files, cases).kind).toBe("all");
  expect(pick(map, [{ file: "src/a.rs", hunks: "new" }], files, cases).kind).toBe("all");
  for (const file of ["test/support.ts", "std/lib.rs", "std/package.json", "Cargo.lock", "docs/guide/limitations.md", "test/corpus.test.ts"]) {
    expect(pick(map, [{ file, hunks: [[1, 1]] }], files, cases).kind).toBe("all");
  }
});

test("a file read by `include_str!` is the line that reads it", () => {
  expect(pick(map, [{ file: "src/runtime/helper.js", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/two.test.ts", "test/reads.test.ts"], cases: ["y.rs"], forms: [] });
  const outside = { ...map, includes: { "src/runtime/helper.js": [{ file: "src/runtime.rs", line: 12 }] } };
  expect(pick(outside, [{ file: "src/runtime/helper.js", hunks: [[1, 1]] }], files, cases).kind).toBe("all");
});

test("a function's counted lines are joined over what's its own, not over an item nested in it", () => {
  const text = [
    "fn f(x: u32) -> u32 {", // 1, counted
    "    use std::fmt;", // 2
    "    // a comment", // 3
    "    let y = match x {", // 4, counted
    "        0 => 1,", // 5
    "        _ => 2,", // 6, counted
    "    };", // 7
    "    impl Foo {", // 8
    "        const C: u32 = 3;", // 9
    "    }", // 10
    "    y", // 11, counted
    "}", // 12, counted
  ];
  expect(
    joinedSpans(
      [
        [1, 1],
        [4, 4],
        [6, 6],
        [11, 12],
      ],
      text,
    ),
  ).toEqual([
    [1, 6],
    [11, 12],
  ]);
  for (const line of ["    impl Foo {", "    pub(crate) const C: u32 = 1;", "    #[derive(Clone)] struct S;", "    unsafe impl Send for S {}", "    macro_rules! m {", "    static N: u8 = 0;", "    type T = u8;"]) {
    expect(nestedItem.test(line)).toBe(true);
  }
  for (const line of ["    use std::fmt;", "    fn g() {}", "    let constant = 1;", "    }", "    // impl Foo", "    typed(x);", "    Self::module();"]) expect(nestedItem.test(line)).toBe(false);
});

test("a diff's hunks are the lines of the map's commit it changes", () => {
  const diff = [
    "diff --git a/src/a.rs b/src/a.rs",
    "index 1..2 100644",
    "--- a/src/a.rs",
    "+++ b/src/a.rs",
    "@@ -12 +12 @@ fn f() {",
    "-    a();",
    "+    b();",
    "@@ -15,0 +16,2 @@",
    "+    c();",
    "++++ d;",
    "@@ -20,3 +22 @@",
    "diff --git a/src/b.rs b/src/b.rs",
    "new file mode 100644",
    "@@ -0,0 +1,4 @@",
  ].join("\n");
  expect(hunksOf(diff)).toEqual([
    {
      file: "src/a.rs",
      hunks: [
        [12, 1, ["    b();"]],
        [15, 0, ["    c();", "+++ d;"]],
        [20, 3, []],
      ],
    },
    { file: "src/b.rs", hunks: "new" },
  ]);
});

test("a test that reads the compiler's sources is found, through what it imports", () => {
  expect(readsSources("test/architecture.test.ts")).toBe(true);
  expect(readsSources("test/runtime-package.test.ts")).toBe(true);
  expect(readsSources("test/lowering-cells.test.ts")).toBe(false);
});

test("`RUST_JS_CASES=i/n:<file>` is a part of the cases the file names", () => {
  const list = join(fixture("test-map-cases"), "cases.txt");
  writeFileSync(list, "b.rs\nc.rs\nd.rs\n");
  const given = process.env.RUST_JS_CASES;
  try {
    process.env.RUST_JS_CASES = `1/2:${list}`;
    expect(caseShard(["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"])).toEqual(["b.rs", "d.rs"]);
    process.env.RUST_JS_CASES = "2/2";
    expect(caseShard(["a.rs", "b.rs", "c.rs"])).toEqual(["b.rs"]);
  } finally {
    if (given === undefined) delete process.env.RUST_JS_CASES;
    else process.env.RUST_JS_CASES = given;
  }
});

test("the maps kept are the three most recently used, fewer below the disk's floor", () => {
  const dir = fixture("test-map-maps");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (let i = 0; i < 5; i++) {
    writeFileSync(join(dir, `${i}.json`), "{}");
    utimesSync(join(dir, `${i}.json`), 1000 + i, 1000 + i);
  }
  // A disk with room, so only the cap decides, however full this
  // machine's is (#963).
  pruneMaps(dir, 3, () => Number.POSITIVE_INFINITY);
  expect(readdirSync(dir).sort()).toEqual(["2.json", "3.json", "4.json"]);
  pruneMaps(dir, 3, () => 0);
  expect(readdirSync(dir)).toEqual([]);

  // A map of another toolchain, a stale key, goes however recently used.
  const current = mapFile(dir, "a".repeat(40), "1234567890abcdef");
  const other = mapFile(dir, "b".repeat(40), "fedcba0987654321");
  writeFileSync(current, "{}");
  writeFileSync(other, "{}");
  pruneMaps(dir, 3, () => Number.POSITIVE_INFINITY, "1234567890abcdef");
  expect(readdirSync(dir).map((name) => join(dir, name))).toEqual([current]);
});

// The maps are the machine's (#932): a new worktree reads one another
// worktree recorded of its commit or an ancestor's, the nearest, of its
// toolchain, never one of a commit it doesn't descend from.
test("a worktree reads the map another recorded of its commit or its nearest ancestor", () => {
  const dir = fixture("test-map-store");
  rmSync(dir, { recursive: true, force: true });
  const [repo, other, store] = ["repo", "other", "store"].map((name) => join(dir, name));
  mkdirSync(repo, { recursive: true });
  mkdirSync(store);
  const git = (cwd: string, ...args: string[]) => {
    const p = Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
    expect(p.exitCode).toBe(0);
    return p.stdout.toString().trim();
  };
  git(repo, "init", "-q", "-b", "main");
  const commit = (cwd: string, text: string) => {
    writeFileSync(join(cwd, "a.txt"), text);
    git(cwd, "add", "a.txt");
    git(cwd, "commit", "-q", "-m", text);
    return git(cwd, "rev-parse", "HEAD");
  };
  const first = commit(repo, "1");
  const second = commit(repo, "2");
  git(repo, "worktree", "add", "-q", "-b", "side", other, first);
  const side = commit(other, "side");
  const write = (at: string, toolchain = "abc") => {
    const path = mapFile(store, at, toolchain);
    writeFileSync(path, JSON.stringify({ ...map, version: mapVersion, commit: at, toolchain }));
    return path;
  };
  // Recorded by the first worktree, of its first commit: the other, on a
  // commit after it, reads it.
  write(first);
  expect(nearestMap(store, other, "abc")?.commit).toBe(first);
  // Not of another toolchain, nor a commit it doesn't descend from.
  expect(nearestMap(store, other, "abd")).toBeUndefined();
  write(second);
  expect(nearestMap(store, other, "abc")?.commit).toBe(first);
  // Of two ancestors, the nearest, though the other was read since.
  const own = write(side);
  utimesSync(own, 1000, 1000);
  expect(nearestMap(store, other, "abc")?.commit).toBe(side);
  expect(nearestMap(store, repo, "abc")?.commit).toBe(second);
  // One half written isn't read.
  writeFileSync(own, "{");
  expect(nearestMap(store, other, "abc")?.commit).toBe(first);
});

// A test that reads a file of the checkout's as text, ci.test.ts the crate
// corpus's script, fails on a change to it that runs none of its code: a
// change to the file picks it, whatever else picks what runs (a9aedb875
// moved a Cargo run ci.test.ts counts, and plain check:fast didn't run it).
// What a test names is read; what a comment names, a fixture's own `root`,
// or a folder a literal names, which is as often an app's, isn't.
// A recording in parts on many machines, CI's (`build --shards=N`, `units
// --shard=i/N`, `merge`), writes the map one machine's recording does: of
// a small fixture's tests, each compile's bits written where its part ran,
// the merge of three parts is the merge of one, byte for byte; and a part
// lost, or one read twice, is refused, never a map short of it.
test("the merge of a recording's parts is the map of one part's recording", () => {
  const caseNames = ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"];
  // And the metamorphic sample, recorded in parts as the push suite runs it.
  const files = ["test/one.test.ts", "test/two.test.ts", "test/fails.test.ts", "test/reads.test.ts", "test/unknown.test.ts", "test/metamorphic.test.ts", "test/none.test.ts"];
  const names = Array.from({ length: 12 }, (_, i) => `f${i}`);
  // Some functions of the twelve, by what was compiled.
  const ran = (compiled: string) => {
    const bytes = new Uint8Array(2);
    for (let i = 0; i < compiled.length; i++) {
      const at = (compiled.charCodeAt(i) * 7 + i) % 12;
      bytes[at >> 3] |= 1 << (at & 7);
    }
    return bytes;
  };
  const planOf = (shards: number, perShard: number): Plan => ({
    version: mapVersion,
    commit: "c0",
    toolchain: "t0",
    shards,
    parts: shards * perShard,
    names,
    functions: names.map((_, i) => ({ file: "src/a.rs", spans: [[i * 10 + 1, i * 10 + 5]] })),
    files,
    caseNames,
    readsSources: ["test/reads.test.ts"],
    corpusReaders: ["test/two.test.ts"],
    includes: { "src/runtime/helper.js": [{ file: "src/a.rs", line: 3 }] },
    units: planUnits(files, shards, perShard, new Map([["test/two.test.ts", 30]])),
  });
  // What each part's tests write, as testMapCompiler.ts does: a file of
  // bits for each compile, of the test or the case it ran for, two of each
  // case, and one of its file's own for each case, a helper's compile.
  const record = (plan: Plan, dir: string) => {
    let id = 0;
    const write = (part: string, unit: string, compiled: string) => writeFileSync(join(part, "units", `${unit.replaceAll("/", "__")}.1-${id++}.bits`), ran(compiled));
    for (let at = 1; at <= plan.shards; at++) {
      const part = join(dir, String(at));
      mkdirSync(join(part, "units"), { recursive: true });
      const ended: Record<string, number> = {};
      for (const unit of plan.units.filter((unit) => unit.shard === at)) {
        ended[unit.name] = unit.name === "test/fails.test.ts" ? 1 : 0;
        const [file, cases] = unit.name.split(" ");
        if (file === "test/metamorphic.test.ts") {
          const [i, n] = cases.split("/").map(Number);
          for (const name of shard(plan.caseNames, i, n)) write(part, file, `${file} ${name}`);
          continue;
        }
        if (file === "test/none.test.ts") continue;
        if (cases === undefined) {
          write(part, file, file);
          if (file === "test/unknown.test.ts") writeFileSync(join(part, "unknown"), `${file}\tf99\n`);
          continue;
        }
        const [i, n] = cases.split("/").map(Number);
        for (const name of shard(plan.caseNames, i, n)) {
          const unitName = file === "test/corpus.test.ts" ? `test/corpus/${name}` : `${file} test/corpus/${name}`;
          write(part, unitName, unitName);
          write(part, unitName, `${unitName}!`);
          write(part, file, `${file} ${name}`);
        }
      }
      writeFileSync(join(part, "ended.json"), JSON.stringify(ended));
    }
    return readdirSync(dir).map((name) => join(dir, name));
  };
  const dir = fixture("test-map-merge");
  const one = planOf(1, 2);
  const three = planOf(3, 2);
  // Each test in exactly one part, each part two of the corpus's six.
  const named = three.units.map((unit) => unit.name);
  expect(new Set(named).size).toBe(named.length);
  expect(new Set(three.units.filter((unit) => files.includes(unit.name)).map((unit) => unit.name))).toEqual(new Set(files.filter((file) => file !== "test/metamorphic.test.ts")));
  const corpusParts = three.units.filter((unit) => unit.name.startsWith("test/corpus.test.ts ")).map((unit) => `${unit.shard} ${unit.env.RUST_JS_CASES}`);
  expect(corpusParts).toEqual(["1 1/6", "1 4/6", "2 2/6", "2 5/6", "3 3/6", "3 6/6"]);
  const whole = mergeRecordings(one, record(one, join(dir, "one")), 1);
  const parted = mergeRecordings(three, record(three, join(dir, "three")), 1);
  expect(JSON.stringify(parted.map)).toBe(JSON.stringify(whole.map));
  expect(parted.always).toEqual(whole.always);
  // A test's functions are the union of its compiles'.
  const union = (...compiled: string[]) => Buffer.from(compiled.map(ran).reduce((a, b) => a.map((byte, i) => byte | b[i]))).toString("base64");
  expect(parted.map.units["test/corpus/c.rs"]).toBe(union("test/corpus/c.rs", "test/corpus/c.rs!"));
  expect(parted.map.units["test/corpus.test.ts"]).toBe(union(...caseNames.map((name) => `test/corpus.test.ts ${name}`)));
  // A file recorded in parts is each part's compiles'; one that ran no
  // compiler is said so.
  expect(three.units.filter((unit) => unit.name.startsWith("test/metamorphic.test.ts ")).map((unit) => unit.env.RUST_JS_CASES)).toEqual(["1/6", "4/6", "2/6", "5/6", "3/6", "6/6"]);
  expect(parted.map.units["test/metamorphic.test.ts"]).toBe(union(...caseNames.map((name) => `test/metamorphic.test.ts ${name}`)));
  expect(parted.map.noCompiler).toEqual(["test/none.test.ts"]);
  expect(whole.always).toEqual([
    "test/fails.test.ts (it failed)",
    "test/reads.test.ts (it reads the compiler's sources)",
    "test/unknown.test.ts (a compile of it ran a function the table hasn't)",
    "test/none.test.ts (it ran no compiler)",
  ]);
  // A part lost, and a part read twice.
  const parts = readdirSync(join(dir, "three")).map((name) => join(dir, "three", name));
  expect(() => mergeRecordings(three, parts.slice(1), 1)).toThrow(/no part of the recording ran/);
  expect(() => mergeRecordings(three, [...parts, parts[0]], 1)).toThrow(/ended in two parts/);
  rmSync(dir, { recursive: true, force: true });
});

// CI's test map is the kept one, `kept`'s, or recorded in parts on many
// machines and merged (test-map.yml, the one recording): the parts and the
// merge are jobs only where none is kept, skipped outright where one is.
// Of each push to `main` that changes what a test runs it records HEAD's,
// beside the push's check, not after it, one at a time, the newest waiting,
// named by its commit for `bun run push` to fetch; rustc-tests.yml's
// mutations call it, and wait for it. One way to ask whether one's kept:
// `ensure`, which recorded it on one machine, 38.6 min (check
// 43789968835), is gone, and refused.
test("CI's test map is the kept one, or recorded in parts and merged where none is", () => {
  const map = readFileSync(join(root, ".github/workflows/test-map.yml"), "utf8");
  const job = (text: string, name: string) => {
    const at = text.indexOf(`\n  ${name}:\n`);
    expect([name, at]).not.toEqual([name, -1]);
    const end = text.indexOf("\n\n  ", at + 1);
    return text.slice(at, end === -1 ? undefined : end);
  };
  const on = (Bun.YAML.parse(map) as { on: Record<string, unknown> }).on;
  // Beside the check, not after it: the map trailed some 40 min of pushes.
  expect(on.push).toEqual({ branches: ["main"], paths: ["**", "!docs/**", "!**/*.md"] });
  expect(Object.keys(on)).not.toContain("workflow_run");
  expect(Object.keys(on)).toContain("workflow_call");
  // The running recording is never cancelled, or one a check's end
  // cancelled every one (no map for 9 commits); of those waiting, GitHub
  // keeps the newest.
  const group = (Bun.YAML.parse(map) as { concurrency: { "cancel-in-progress": boolean } }).concurrency;
  expect(group["cancel-in-progress"]).toBe(false);
  expect(map).toContain("format('test-map-of-{0}', env.SHA)");
  expect(job(map, "test-map")).toContain("bun scripts/testMap.ts kept");
  expect(job(map, "test-map")).toContain("bun scripts/testMap.ts build --shards=");
  expect(job(map, "test-map-part")).toContain("bun scripts/testMap.ts units --shard=${{ matrix.part }}/${{ needs.test-map.outputs.shards }}");
  expect(job(map, "test-map-merge")).toContain("bun scripts/testMap.ts merge");
  for (const name of ["test-map-part", "test-map-merge"]) {
    expect([name, job(map, name)]).toEqual([name, expect.stringContaining("    if: ${{ needs.test-map.outputs.record == 'true' }}\n")]);
  }
  // The merge's map alone in its commit's cache entry: none restored first.
  expect(job(map, "test-map-merge")).not.toContain("actions/cache/restore");
  const tests = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  expect(job(tests, "test-map")).toContain("uses: ./.github/workflows/test-map.yml");
  expect(job(tests, "mutations")).toContain("needs: test-map");
  expect(job(tests, "mutations")).toContain("needs.test-map.result == 'success'");
  for (const file of readdirSync(join(root, ".github/workflows"))) {
    expect([file, readFileSync(join(root, ".github/workflows", file), "utf8").includes("testMap.ts ensure")]).toEqual([file, false]);
  }
  const ensure = Bun.spawnSync(["bun", "scripts/testMap.ts", "ensure"], { cwd: root, stdout: "pipe", stderr: "pipe" });
  expect(ensure.exitCode).toBe(2);
  expect(ensure.stderr.toString()).toContain("kept [--head]");
});

// `bun run push` fetches CI's map of HEAD's nearest ancestor on `main`, of
// this repository's runs alone, a fork's never; and the cache's maps past
// the newest 8 go.
test("CI's map of HEAD's nearest ancestor on main is the one fetched, and the cache keeps the newest 8", () => {
  const run = (id: number, branch = "main", fork = false) => ({ id, head_branch: branch, repository_id: 1, head_repository_id: fork ? 2 : 1 });
  const sha = (c: string) => c.repeat(40);
  const artifacts = [
    { name: `${ciMapPrefix}${sha("a")}`, expired: false, workflow_run: run(1) },
    { name: `${ciMapPrefix}${sha("b")}`, expired: false, workflow_run: run(2) },
    { name: `${ciMapPrefix}${sha("c")}`, expired: false, workflow_run: run(3, "other") },
    { name: `${ciMapPrefix}${sha("d")}`, expired: false, workflow_run: run(4, "main", true) },
    { name: `${ciMapPrefix}${sha("e")}`, expired: true, workflow_run: run(5) },
    { name: "test-map", expired: false, workflow_run: run(6) },
  ];
  const behind: Record<string, number> = { [sha("a")]: 5, [sha("b")]: 2, [sha("c")]: 0, [sha("d")]: 0, [sha("e")]: 0 };
  expect(ciMapOf(artifacts, (commit) => behind[commit])).toEqual({ run: 2, name: `${ciMapPrefix}${sha("b")}`, commit: sha("b"), behind: 2 });
  expect(ciMapOf(artifacts, (commit) => (commit === sha("a") ? 5 : undefined))?.run).toBe(1);
  expect(ciMapOf(artifacts, () => undefined)).toBeUndefined();
  const listed = Array.from({ length: 11 }, (_, i) => ({ id: 100 - i, key: `test-map-Linux-t-${i}` }));
  expect(cachePast(listed, 8).map(({ id }) => id)).toEqual([92, 91, 90]);
});
