// The test map, which plain `check:fast` reads to run only the test files
// and corpus cases a change can affect (scripts/testMap.ts, #877): what a
// change picks, where it can't say and runs what it runs without one, and
// how many maps it keeps.

import { expect, test } from "bun:test";
import { mkdirSync, readdirSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { hunksOf, joinedSpans, linesAtCommit, mapFile, mapVersion, nearestMap, nestedItem, pick, pruneMaps, readsSources, type Change, type TestMap } from "../scripts/testMap";
import { jobs } from "../scripts/checkFast";
import { caseShard, fixture } from "./support";

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
  for (const file of ["test/support.ts", "std/lib.rs", "runtime/package.json", "Cargo.lock", "docs/guide/limitations.md", "test/corpus.test.ts"]) {
    expect(pick(map, [{ file, hunks: [[1, 1]] }], files, cases).kind).toBe("all");
  }
});

test("a file read by `include_str!` is the line that reads it", () => {
  expect(pick(map, [{ file: "src/runtime/helper.js", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/two.test.ts", "test/reads.test.ts"], cases: ["y.rs"], forms: [] });
  const outside = { ...map, includes: { "src/runtime/helper.js": [{ file: "src/runtime.rs", line: 12 }] } };
  expect(pick(outside, [{ file: "src/runtime/helper.js", hunks: [[1, 1]] }], files, cases).kind).toBe("all");
});

test("a changed test file or corpus case runs itself, and a test the map hasn't recorded runs", () => {
  expect(pick(map, [{ file: "test/one.test.ts", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/one.test.ts", "test/reads.test.ts"], cases: [] });
  // And each test that reads the corpus's files itself.
  expect(pick(map, [{ file: "test/corpus/y.js", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/two.test.ts", "test/reads.test.ts"], cases: ["y.rs"], forms: ["y.rs"] });
  // std's iterator structs' file runs each of its cases, and its own tests.
  expect(pick(map, [{ file: "test/iterator-forms.test.ts", hunks: [[1, 1]] }], files, cases)).toMatchObject({
    files: ["test/reads.test.ts", "test/iterator-forms.test.ts"],
    cases: [],
    forms: ["x.rs", "y.rs"],
  });
  // Another suite's test file is no plain test's.
  expect(pick(map, [{ file: "test/crates.test.ts", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/reads.test.ts"] });
  expect(pick(map, [], [...files, "test/three.test.ts"], [...cases, "z.rs"])).toMatchObject({ files: ["test/reads.test.ts", "test/three.test.ts"], cases: ["z.rs"], forms: ["z.rs"] });
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

// A mutation's site, of the working tree's file, is found in the map's
// commit's lines (scripts/mutations.ts, #864): a line the working tree
// wrote is the change's already, and each other moved by what was written
// before it.
test("a working tree's lines are the map's commit's, moved by what the change wrote before them", () => {
  const change = hunksOf(
    ["diff --git a/src/a.rs b/src/a.rs", "--- a/src/a.rs", "+++ b/src/a.rs", "@@ -12 +12 @@", "-    a();", "+    b();", "@@ -15,0 +16,2 @@", "+    c();", "+    d();", "@@ -20,3 +21,0 @@", "-    e();", "-    f();", "-    g();"].join("\n"),
  )[0];
  const lines = (first: number, last: number) => linesAtCommit(change, first, last).map(([line]) => line);
  expect(lines(10, 15)).toEqual([10, 11, 13, 14, 15]);
  expect(lines(16, 17)).toEqual([]);
  expect(lines(18, 23)).toEqual([16, 17, 18, 19, 23, 24]);
  expect(lines(5, 6)).toEqual(linesAtCommit(undefined, 5, 6).map(([line]) => line));
  expect(linesAtCommit({ file: "src/a.rs", hunks: "new" }, 1, 3)).toEqual([]);
  // Two lines written above a function: its line 35 is the commit's 33, of
  // the second function's code, which x.rs ran; not its 35, between its
  // spans, which the map can't say of.
  const moved: Change = { file: "src/a.rs", hunks: [[5, 0, ["x", "y"]]] };
  expect(pick(map, [{ file: "src/a.rs", hunks: linesAtCommit(moved, 35, 35) }], files, cases)).toMatchObject({ kind: "picked", cases: ["x.rs"] });
  expect(pick(map, [{ file: "src/a.rs", hunks: [[35, 1]] }], files, cases)).toMatchObject({ kind: "all" });
});

test("a test that reads the compiler's sources is found, through what it imports", () => {
  expect(readsSources("test/architecture.test.ts")).toBe(true);
  expect(readsSources("test/runtime-package.test.ts")).toBe(true);
  expect(readsSources("test/lowering-cells.test.ts")).toBe(false);
});

test("plain check:fast runs each suite's file as a job, and of a change the map can say, only those it picked", () => {
  const plain = { all: false, changed: ["src/lower/drops.rs"], branch: [], added: [], extra: [], cores: 12 };
  const unpicked = jobs(plain).jobs;
  const lowering = unpicked.filter((job) => job.group === "lowering");
  expect(lowering.length).toBeGreaterThan(50);
  for (const job of lowering) expect(job.cmd).toEqual(["bun", "test", job.name]);
  // Of the paths' rules, JSX's only where the change is JSX's.
  expect(unpicked.some((job) => job.group === "JSX")).toBe(false);
  expect(unpicked.filter((job) => job.name.startsWith("corpus ")).length).toBe(12);

  const given = { caseList: "/cases.txt", formsList: "/forms.txt" };
  const picked = jobs({ ...plain, picked: { ...given, files: new Set(["test/lowering-cells.test.ts", "test/jsx.test.ts"]), cases: ["a.rs", "b.rs"], forms: [] } }).jobs;
  expect(picked.filter((job) => job.group !== undefined).map((job) => job.name)).toEqual(["test/lowering-cells.test.ts", "test/jsx.test.ts"]);
  expect(picked.filter((job) => job.name.startsWith("corpus ")).map((job) => job.env.RUST_JS_CASES)).toEqual(["1/1:/cases.txt"]);
  // No case picked, nor the corpus's own tests: no part.
  const none = jobs({ ...plain, picked: { ...given, files: new Set(), cases: [], forms: [] } }).jobs;
  expect(none.filter((job) => job.name.startsWith("corpus ") || job.group !== undefined)).toEqual([]);

  // Of an iterator change, the corpus with std's iterator structs runs the
  // cases picked, and none where none is picked (#876).
  const iterators = { ...plain, changed: ["src/lower/mir/iter/collect.rs"] };
  const forms = (of: Parameters<typeof jobs>[0]) => jobs(of).jobs.find((job) => job.name === "the corpus with std's iterator structs");
  expect(forms(iterators)?.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "forms", "6"]);
  expect(forms({ ...iterators, picked: { ...given, files: new Set(), cases: [], forms: ["a.rs"] } })?.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "forms", "6", "/forms.txt"]);
  expect(forms({ ...iterators, picked: { ...given, files: new Set(), cases: [], forms: [] } })).toBeUndefined();
  expect(forms({ ...iterators, picked: { ...given, files: new Set(["test/iterator-forms.test.ts"]), cases: [], forms: [] } })).toBeDefined();

  // Of `--all`, the corpus whole in its plain build, and in std's the cases
  // the map picks where it can say, none where it picks none (#986).
  const all = { ...iterators, all: true };
  expect(jobs({ ...all, forms: { forms: ["a.rs"], formsList: "/forms.txt" } }).jobs.filter((job) => job.name.startsWith("corpus ")).length).toBe(12);
  expect(forms({ ...all, forms: { forms: ["a.rs"], formsList: "/forms.txt" } })?.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "forms", "6", "/forms.txt"]);
  expect(forms({ ...all, forms: { forms: [], formsList: "/forms.txt" } })).toBeUndefined();
  expect(forms(all)?.cmd).toEqual(["bun", "scripts/iteratorSlice.ts", "run", "forms", "6"]);
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
