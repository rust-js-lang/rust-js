// The test map, which the selector reads to run only the test files and
// corpus cases a change to the compiler's code can affect
// (scripts/testMap.ts, #877; scripts/select.ts, test/select.test.ts): what
// such a change picks, where it can't say, and how many maps it keeps.

import { expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

import { casesReaching, changesDiff, ciMapOf, ciMapPrefix, hunksOf, mapLines, placedSimilarity, joinedSpans, mapFile, mapVersion, mergeRecordings, nearestMap, nestedItem, pick, planUnits, pruneMaps, readsOf, readsSources, carriedUnits, carryOf, remapBits, unreferencedModule, type Change, type Plan, type Sources, type TestMap } from "../scripts/testMap";

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
    { file: "crates/rust-js-program/src/runtime.rs", spans: [[1, 9]] },
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
  includes: { "crates/rust-js-program/src/runtime/helper.js": [{ file: "crates/rust-js-program/src/runtime.rs", line: 5 }] },
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
    functions: [0],
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

test("a constant's value, an array's length with it, is the change of the functions that name it (dab80f433's `HELD_FLIPPED`)", () => {
  // src/a.rs: function 0 at 10-20 names `FLIPPED` at 15; the constant at 22-26.
  const text = Array.from({ length: 50 }, (_, i) => `// line ${i + 1}`);
  text[14] = "    let flipped = FLIPPED.iter();";
  text[21] = "pub(crate) const FLIPPED: [&str; 2] = [";
  text[22] = '    "a",';
  text[23] = '    "b",';
  text[24] = "];";
  text[26] = "/// `FLIPPED`'s adapters, a comment that names it.";
  text[44] = "const OTHER: usize = FLIPPED.len();";
  const sources = (also: { file: string; line: number }[] = []): Sources => ({
    text: (file) => (file === "src/a.rs" ? text.join("\n") : undefined),
    naming: (word) => (word === "FLIPPED" ? [{ file: "src/a.rs", line: 15 }, { file: "src/a.rs", line: 22 }, { file: "src/a.rs", line: 27 }, ...also] : []),
  });
  const grown: Change = { file: "src/a.rs", hunks: [[22, 1, ["pub(crate) const FLIPPED: [&str; 3] = ["], ["pub(crate) const FLIPPED: [&str; 2] = ["]], [24, 0, ['    "c",'], []]] };
  const asFunction = pick(map, [{ file: "src/a.rs", hunks: [[15, 1]] }], files, cases);
  expect(pick(map, [grown], files, cases, sources())).toEqual({ ...asFunction, why: expect.any(String) } as never);
  // Read where no function is, another constant's initializer: everything.
  expect(pick(map, [grown], files, cases, sources([{ file: "src/a.rs", line: 45 }])).kind).toBe("all");
  // Its type changed past its length: everything.
  const retyped: Change = { file: "src/a.rs", hunks: [[22, 1, ["pub(crate) const FLIPPED: [String; 2] = ["], ["pub(crate) const FLIPPED: [&str; 2] = ["]]] };
  expect(pick(map, [retyped], files, cases, sources()).kind).toBe("all");
});

test("a new module of items nothing refers to selects nothing but what its users' changes do; one that may reach code no change names, everything (4ba38fa64's import_names.rs)", () => {
  // Functions, types, their own `impl`s, a trait's for one of them too, and
  // plain constants: reached only through the `mod` line and code changed
  // to use them.
  expect(unreferencedModule("use std::fmt;\npub(crate) struct Words<'a>(&'a str);\nimpl Words<'_> {\n    fn f(&self) {}\n}\nimpl fmt::Display for Words<'_> {}\nconst N: usize = 3;\npub(super) fn g() {}")).toBe(true);
  // An `impl` of a type it doesn't declare, a `trait`, a `static`, a
  // `const` that calls code, a macro defined or invoked as an item, a test.
  for (const text of ["struct A;\nimpl Clone for B {}", "pub trait T {}", "static X: u8 = 1;", "const N: usize = f(3);", "macro_rules! m { () => {} }", "inventory::submit! { X }", "#[test]\nfn t() {}"]) {
    expect([text, unreferencedModule(text)]).toEqual([text, false]);
  }
  // Of the map, nothing but what runs on every change; a new file the
  // selector can't read, everything.
  expect(pick(map, [{ file: "crates/rust-js-program/src/prepare/import_names.rs", hunks: "new" }], files, cases)).toMatchObject({ kind: "picked", files: ["test/reads.test.ts"], cases: [] });
  expect(pick(map, [{ file: "src/not-there.rs", hunks: "new" }], files, cases).kind).toBe("all");
});

test("a name no item declared, added to a `use`, changes nothing but the code changed to use it (4ba38fa64's crates/rust-js-program/src/js.rs:43)", () => {
  const text = Array.from({ length: 50 }, (_, i) => `// line ${i + 1}`);
  text[21] = "pub use self::{";
  text[22] = "    imports::{Import, Package},";
  text[23] = "    walk::*,";
  text[24] = "};";
  const sources = (declared: boolean): Sources => ({
    text: (file) => (file === "src/a.rs" ? text.join("\n") : file === "src/b.rs" ? "pub struct ImportNames;" : undefined),
    naming: (word) => (word === "ImportNames" && declared ? [{ file: "src/b.rs", line: 1 }] : []),
  });
  const added: Change = { file: "src/a.rs", hunks: [[23, 1, ["    imports::{Import, ImportNames, Package},"], ["    imports::{Import, Package},"]]] };
  expect(pick(map, [added], files, cases, sources(false))).toMatchObject({ kind: "picked", files: ["test/reads.test.ts"], cases: [] });
  // A name some item declared at the map's commit may shadow a glob's: everything.
  expect(pick(map, [added], files, cases, sources(true)).kind).toBe("all");
  // A name taken out of a `use`: everything.
  const taken: Change = { file: "src/a.rs", hunks: [[23, 1, ["    imports::{Import},"], ["    imports::{Import, Package},"]]] };
  expect(pick(map, [taken], files, cases, sources(false)).kind).toBe("all");
});

test("a test carried over keeps its functions by their symbols, and one new since isn't any carried test's", () => {
  // f1 and f2 swap places; f3 is new; f0 is gone.
  const from = ["f0", "f1", "f2"];
  const to = ["f2", "f1", "f3"];
  expect([...remapBits(new Uint8Array([0b111]), from, to)]).toEqual([0b011]);
  expect([...remapBits(new Uint8Array([0b010]), from, to)]).toEqual([0b010]);
});

test("a file read by `include_str!` is the line that reads it", () => {
  expect(pick(map, [{ file: "crates/rust-js-program/src/runtime/helper.js", hunks: [[1, 1]] }], files, cases)).toMatchObject({ files: ["test/two.test.ts", "test/reads.test.ts"], cases: ["y.rs"], forms: [] });
  const outside = { ...map, includes: { "crates/rust-js-program/src/runtime/helper.js": [{ file: "crates/rust-js-program/src/runtime.rs", line: 12 }] } };
  expect(pick(outside, [{ file: "crates/rust-js-program/src/runtime/helper.js", hunks: [[1, 1]] }], files, cases).kind).toBe("all");
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
        [12, 1, ["    b();"], ["    a();"], 12, 1],
        [15, 0, ["    c();", "+++ d;"], [], 16, 2],
        [20, 3, [], [], 22, 1],
      ],
    },
    { file: "src/b.rs", hunks: "new" },
  ]);
});

// A site the map can't place because it's behind, its lines changed since
// the map's commit, its file new or moved too far, says so by a field: the
// judge that skips such a site matched the reason's last words, which #2337
// reworded, so a machine whose map was behind failed it on six tidy
// mutations (#2349). A site with no map at all isn't one.
test("a site the test map is behind says so, apart from its reason's words", () => {
  const map = { commit: "0123456789abcdef" } as TestMap;
  const at = (change: Change) => casesReaching("src/a.rs", 3, 4, [], { map, changes: [change] });
  const behind: Change[] = [
    { file: "src/a.rs", hunks: [[3, 1, ["x"], ["y"], 3, 1]] },
    { file: "src/a.rs", hunks: [[3, 1]] },
    { file: "src/a.rs", hunks: "new" },
    { file: "src/old.rs", now: "src/a.rs", similarity: 50, hunks: [] },
  ];
  for (const change of behind) expect(at(change)).toMatchObject({ kind: "all", mapBehind: true, why: expect.stringContaining("src/a.rs") });
  expect(casesReaching("src/a.rs", 3, 4, [], "no test map")).toEqual({ kind: "all", why: "no test map" });
});

// A moved file is the map's at its path there (#2337): git's renames, `-M`,
// read as the old path with the edit's hunks, so a pure move selects
// nothing and an edited one what its edit reaches, not everything; and the
// snapshot judge places a site's unchanged lines at their lines there,
// where they were. #2299's cut moved its files 95 to 100% alike, a path or
// a visibility edited, and its tidy mutations' sites the map couldn't place.
test("a moved file is the map's at its path there, its unchanged lines placed where they were", () => {
  const dir = fixture("map-renames");
  const git = (...args: string[]) => Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", ...args], { cwd: dir }).stdout.toString();
  git("init", "--quiet");
  mkdirSync(join(dir, "src"), { recursive: true });
  writeFileSync(join(dir, "src/a.rs"), Array.from({ length: 30 }, (_, i) => `fn f${i + 1}() -> u32 { ${i + 1} }`).join("\n") + "\n");
  git("add", "-A");
  git("commit", "--quiet", "-m", "the map's");
  // Moved, a line put first and line 20 edited: 95% alike.
  mkdirSync(join(dir, "src/moved"), { recursive: true });
  git("mv", "src/a.rs", "src/moved/a.rs");
  const lines = readFileSync(join(dir, "src/moved/a.rs"), "utf8").split("\n");
  lines[19] = "fn f20() -> u32 { 200 }";
  writeFileSync(join(dir, "src/moved/a.rs"), ["// moved", ...lines].join("\n"));
  const [change, ...rest] = hunksOf(git(...changesDiff, "HEAD"));
  expect(rest).toEqual([]);
  expect(change).toMatchObject({ file: "src/a.rs", now: "src/moved/a.rs" });
  expect(change!.similarity).toBeGreaterThanOrEqual(placedSimilarity);
  // Lines 11 and 12 now were 10 and 11; line 21 now, the edited 20, isn't placed.
  expect(mapLines(change!, 11, 12)).toEqual([10, 11]);
  expect(mapLines(change!, 21, 21)).toStartWith("changed at its lines");
  expect(mapLines(change!, 25, 26)).toEqual([24, 25]);
  // Never a guess: too unlike, new, or a deletion inside the site.
  expect(mapLines({ file: "a.rs", now: "b.rs", similarity: 60, hunks: [] }, 1, 2)).toStartWith("moved from a.rs, 60% alike");
  expect(mapLines({ file: "a.rs", hunks: "new" }, 1, 2)).toBe("is new since the map's commit");
  const deleted: Change = { file: "a.rs", hunks: [[5, 2, [], ["x", "y"], 4, 0]] };
  expect(mapLines(deleted, 3, 6)).toStartWith("changed at its lines");
  expect([mapLines(deleted, 1, 3), mapLines(deleted, 7, 8)]).toEqual([[1, 3], [9, 10]]);
  // An identical move: the same lines.
  expect(mapLines({ file: "a.rs", now: "b.rs", similarity: 100, hunks: [] }, 7, 9)).toEqual([7, 9]);
  // Selection: a pure move selects what no change does; an edited one, what
  // its edit would where it was.
  expect(pick(map, [{ file: "src/a.rs", now: "src/z.rs", similarity: 100, hunks: [] }], files, cases)).toEqual(pick(map, [], files, cases));
  const edit: Change["hunks"] = [[15, 0, ["    let x = 1;"], [], 15, 1]];
  expect(pick(map, [{ file: "src/a.rs", now: "src/z.rs", similarity: 97, hunks: edit }], files, cases)).toEqual(pick(map, [{ file: "src/a.rs", hunks: edit }], files, cases));
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
    includes: { "crates/rust-js-program/src/runtime/helper.js": [{ file: "src/a.rs", line: 3 }] },
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
        // Of an incremental plan, the cases it lists (`carriedUnits`).
        const listed = plan.carry === undefined ? plan.caseNames : plan.caseNames.filter((name) => (file === "test/corpus.test.ts" ? plan.carry!.cases : plan.carry!.forms).includes(name));
        for (const name of shard(listed, i, n)) {
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
  // An incremental recording, carried from the whole one: the tests it
  // records, its own; each other, the whole one's by each function's
  // symbol, so the same map, but for what says it carried.
  expect(whole.map.names).toEqual(names);
  const carry = carryOf(whole.map, { files: ["test/two.test.ts"], cases: ["c.rs"], forms: [] }, files, caseNames, { commit: "c0", made: 1 });
  // What it records: what's selected, and what the whole one has none of.
  expect([carry.recorded, carry.cases, carry.forms]).toEqual([["test/two.test.ts", "test/fails.test.ts", "test/reads.test.ts", "test/unknown.test.ts", "test/none.test.ts"], ["c.rs"], []]);
  const incremental: Plan = { ...one, carry, units: carriedUnits(one.units, carry) };
  expect(incremental.units.map((unit) => unit.name).sort()).toEqual(["test/corpus.test.ts 1/2", "test/corpus.test.ts 2/2", ...carry.recorded].sort());
  expect(incremental.units.find((unit) => unit.name === "test/corpus.test.ts 1/2")!.env.RUST_JS_CASES).toMatch(/^1\/2:target\/test-map\/carried-cases\.txt$/);
  const carried = mergeRecordings(incremental, record(incremental, join(dir, "incremental")), 1, whole.map);
  const { carried: said, ...map } = carried.map;
  expect(JSON.stringify(map)).toBe(JSON.stringify(whole.map));
  // Its five files, its one case and the two case files, whose own compiles are each part's.
  expect(said).toEqual({ from: "c0", recorded: 8, carried: Object.keys(whole.map.units).length - 8 });
  // A test neither recorded nor in the map carried from is refused.
  const lacking = { ...whole.map, units: { ...whole.map.units, "test/one.test.ts": null } };
  expect(() => mergeRecordings(incremental, readdirSync(join(dir, "incremental")).map((name) => join(dir, "incremental", name)), 1, lacking)).toThrow(/neither recorded nor carried/);
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
  expect(map).toContain("name: test-map-of-${{ env.SHA }}");
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
// this repository's runs alone, a fork's never.
test("CI's map of HEAD's nearest ancestor on main is the one fetched", () => {
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
});

// Of a diff's own lines, what can't change a test (ADR 1101, amended): a
// comment, a pure addition of items nothing referred to before, `mod
// derived;` of m1-lisa's heap split, and a signature line's visibility,
// `pub(super) fn`, #1983's, which is that function's change. An `impl`, a
// `use` or a macro added, or a signature's types changed, of sources it
// can't read, still can't be said.
test("a comment, a new item nothing refers to, and a signature's visibility select no more than they change", () => {
  const one = (hunk: [number, number, string[], string[]]) => pick(map, [{ file: "src/a.rs", hunks: [hunk] }], files, cases);
  expect(one([25, 1, ["// the new words"], ["// the old words"]]).kind).toBe("picked");
  expect(one([25, 1, ["// the new words"], ["const OLD: u32 = 1;"]]).kind).toBe("all");
  expect(one([22, 0, ["mod derived;"], []]).kind).toBe("picked");
  expect(one([22, 0, ["pub(super) fn helper() -> u32 {", "    1", "}"], []]).kind).toBe("picked");
  expect(one([22, 0, ["struct Fresh(u32);", "const LIMIT: u32 = 2;"], []]).kind).toBe("picked");
  for (const added of [["impl Fresh {", "    fn len(&self) -> u32 { 0 }", "}"], ["use std::fmt::Write;"], ["macro_rules! m { () => {} }"]]) {
    expect(one([22, 0, added, []]).kind).toBe("all");
  }
  // In a function, as before: its change.
  expect(one([15, 0, ["    let x = 1;"], []])).toMatchObject({ kind: "picked", files: ["test/one.test.ts", "test/reads.test.ts", "test/corpus.test.ts"] });
  // A signature's visibility: the function after it, `a.rs`'s second.
  const visible = one([29, 1, ["pub(super) fn second(x: u32) -> u32 {"], ["fn second(x: u32) -> u32 {"]]);
  expect(visible).toMatchObject({ kind: "picked", files: ["test/two.test.ts", "test/reads.test.ts", "test/iterator-forms.test.ts"], cases: ["x.rs"] });
  expect(one([29, 1, ["fn second(x: u64) -> u32 {"], ["fn second(x: u32) -> u32 {"]]).kind).toBe("all");
  // A diff's removed lines, which a hunk now keeps.
  expect(hunksOf("diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -29 +29 @@\n-fn second() {\n+pub fn second() {\n")).toEqual([
    { file: "src/a.rs", hunks: [[29, 1, ["pub fn second() {"], ["fn second() {"], 29, 1]] },
  ]);
});

// A signature's visibility alone, of a name the sources declare once, the
// change adds no other of, and no code beyond the crate names where it's a
// method's, selects nothing: if it compiles, each use resolves to it as
// before (#2162's sweep, whose narrowings selected every test). Of a name
// declared twice, a method's whose name std's or a dependency's code names,
// which rustc's method probe may take for a trait's where the crate's own
// isn't seen, or the prelude's `drop`: that function's change, as before.
test("a visibility change of a name declared once selects nothing", () => {
  const lines = Array.from({ length: 50 }, (_, i) => `// line ${i + 1}`);
  lines[28] = "fn second(x: u32) -> u32 {";
  const declared: Record<string, { file: string; line: number }[]> = { second: [{ file: "src/a.rs", line: 29 }] };
  const sources = (method: boolean, twice: boolean, beyond: string[] = []): Sources => ({
    text: (file) => (file === "src/a.rs" ? lines.map((line, i) => (i === 28 && method ? "fn second(&self, x: u32) -> u32 {" : line)).join("\n") : file === "src/b.rs" ? "pub fn second() {}" : undefined),
    naming: (word) => [...(declared[word] ?? []), ...(twice ? [{ file: "src/b.rs", line: 1 }] : [])],
    beyond: (names) => new Set(names.filter((name) => beyond.includes(name))),
  });
  const narrowed = (line: string, was: string): Change => ({ file: "src/a.rs", hunks: [[29, 1, [line], [was]]] });
  const free = narrowed("pub(super) fn second(x: u32) -> u32 {", "fn second(x: u32) -> u32 {");
  const nothing = pick(map, [free], files, cases, sources(false, false));
  expect(nothing).toMatchObject({ kind: "picked", functions: [], why: expect.stringContaining("1 visibility change of names declared once, which select nothing") });
  const its = { kind: "picked", files: ["test/two.test.ts", "test/reads.test.ts", "test/iterator-forms.test.ts"], cases: ["x.rs"] };
  expect(pick(map, [free], files, cases, sources(false, true))).toMatchObject(its);
  const method = narrowed("pub(super) fn second(&self, x: u32) -> u32 {", "fn second(&self, x: u32) -> u32 {");
  expect(pick(map, [method], files, cases, sources(true, false))).toMatchObject({ functions: [] });
  expect(pick(map, [method], files, cases, sources(true, false, ["second"]))).toMatchObject(its);
  // A change that adds another of its name: that function's change.
  // The prelude's `drop`, which a glob's call falls back to: its change.
  lines[28] = "fn drop(x: u32) -> u32 {";
  declared.drop = [{ file: "src/a.rs", line: 29 }];
  expect(pick(map, [narrowed("pub(super) fn drop(x: u32) -> u32 {", "fn drop(x: u32) -> u32 {")], files, cases, sources(false, false))).toMatchObject(its);
  lines[28] = "fn second(x: u32) -> u32 {";
  const another = pick(map, [free, { file: "src/c.rs", hunks: [[3, 0, ["fn second() {}"], []]] }], files, cases, sources(false, false));
  expect(another.kind === "all" || !another.why.includes("visibility change")).toBe(true);
});

// A private `use` changed, or a function's signature (ADR 1101, amended):
// selected as the code they can change, where the push's select-all of
// 191ad93e9 ran the crate corpus and every net for `use crate::js::{..,
// Name, ..};` at assigned_breaks.rs:5 and `fn function_names(e: &Expr,
// name: &Name)` at stepped_whiles.rs:96.
test("a private `use` changed is each function of its module's, where no code outside one names its words; a glob, a `pub` one, or a word a type names, everything", () => {
  const text = Array.from({ length: 50 }, (_, i) => `// line ${i + 1}`);
  text[2] = "use crate::js::{Expr, Stmt};";
  text[14] = "    let n: Name = Expr::of(s);";
  text[24] = "    held: Name,";
  const sources = (sites: { file: string; line: number }[]): Sources => ({
    text: (file) => (file === "src/a.rs" ? text.join("\n") : file === "src/a/b.rs" ? "    field: Name," : file === "src/c.rs" ? "    other: Name," : file === "crates/rust-js-program/src/js.rs" ? "pub struct Name;" : undefined),
    // Declared at the map's commit, so no name new to the crate.
    naming: (word) => (word === "Name" ? [{ file: "crates/rust-js-program/src/js.rs", line: 1 }, ...sites] : []),
  });
  const named: Change = { file: "src/a.rs", hunks: [[3, 1, ["use crate::js::{Expr, Name, Stmt};"], ["use crate::js::{Expr, Stmt};"]]] };
  // Every function of a.rs, as a trait it brings changes how a method of
  // either resolves, though only the first names it.
  const everyFunction = pick(map, [{ file: "src/a.rs", hunks: [[15, 1], [31, 1]] }], files, cases);
  expect(pick(map, [named], files, cases, sources([{ file: "src/a.rs", line: 15 }, { file: "src/c.rs", line: 1 }]))).toEqual({ ...everyFunction, why: expect.any(String) } as never);
  // A type's field of a.rs, or of a module in it, names it: everything.
  expect(pick(map, [named], files, cases, sources([{ file: "src/a.rs", line: 25 }])).kind).toBe("all");
  expect(pick(map, [named], files, cases, sources([{ file: "src/a/b.rs", line: 1 }])).kind).toBe("all");
  // A glob, whose names no word says, and a `pub` one, read wherever it's imported.
  const glob: Change = { file: "src/a.rs", hunks: [[3, 1, ["use crate::js::*;"], ["use crate::js::{Expr, Stmt};"]]] };
  expect(pick(map, [glob], files, cases, sources([])).kind).toBe("all");
  text[2] = "pub use crate::js::{Expr, Stmt};";
  const reexported: Change = { file: "src/a.rs", hunks: [[3, 1, ["pub use crate::js::{Expr, Name, Stmt};"], ["pub use crate::js::{Expr, Stmt};"]]] };
  expect(pick(map, [reexported], files, cases, sources([])).kind).toBe("all");
  text[2] = "use crate::js::{Expr, Stmt};";
  // A whole `use` added between two, as a trait's.
  const trait: Change = { file: "src/a.rs", hunks: [[3, 0, ["use std::fmt::Write;"], []]] };
  expect(pick(map, [trait], files, cases, sources([]))).toEqual({ ...everyFunction, why: expect.any(String) } as never);
});

test("a function's signature changed is its body's change and each that names it; one named outside any function, everything", () => {
  const text = Array.from({ length: 50 }, (_, i) => `// line ${i + 1}`);
  text[8] = "fn f(x: &str) -> u32 {";
  text[31] = "    f(&y);";
  text[24] = "const N: u32 = f;";
  const sources = (sites: { file: string; line: number }[]): Sources => ({
    text: (file) => (file === "src/a.rs" ? text.join("\n") : file === "src/c.rs" ? "x\n".repeat(8) + "static G: u32 = f(&Y);" : undefined),
    naming: (word) => (word === "f" ? sites : []),
  });
  const retyped: Change = { file: "src/a.rs", hunks: [[8, 2, ["/// Of a variable.", "fn f(x: &Name) -> u32 {"], ["/// Of a word.", "fn f(x: &str) -> u32 {"]]] };
  const both = pick(map, [{ file: "src/a.rs", hunks: [[15, 1], [32, 1]] }], files, cases);
  // Its body and its caller's; a private one's name elsewhere is another's.
  expect(pick(map, [retyped], files, cases, sources([{ file: "src/a.rs", line: 9 }, { file: "src/a.rs", line: 32 }, { file: "src/c.rs", line: 9 }]))).toEqual({ ...both, why: expect.any(String) } as never);
  expect(pick(map, [retyped], files, cases, sources([{ file: "src/a.rs", line: 25 }])).kind).toBe("all");
  // A function written before it, new: nothing names it yet.
  const added: Change = { file: "src/a.rs", hunks: [[8, 2, ["fn g() -> u32 {", "    1", "}", "", "fn f(x: &Name) -> u32 {"], ["/// Of a word.", "fn f(x: &str) -> u32 {"]]] };
  expect(pick(map, [added], files, cases, sources([{ file: "src/a.rs", line: 32 }]))).toEqual({ ...both, why: expect.any(String) } as never);
  // A `pub` one's callers anywhere: c.rs's, a static's, outside any function.
  const public_: Change = { file: "src/a.rs", hunks: [[9, 1, ["pub fn f(x: &Name) -> u32 {"], ["pub fn f(x: &str) -> u32 {"]]] };
  text[8] = "pub fn f(x: &str) -> u32 {";
  expect(pick(map, [public_], files, cases, sources([{ file: "src/c.rs", line: 9 }])).kind).toBe("all");
});

// The counting build counts each of the compiler's crates, and no
// dependency (ADR 1181): Cargo runs scripts/counting-rustc.sh as its
// `RUSTC_WRAPPER`, `<it> rustc ..`; a compile of a crate it's told to
// count is counted, any other's goes to the machine's cache.
test("the counting build's rustc counts the compiler's crates, and hands any other's to the cache", () => {
  const counting = join(root, "scripts", "counting-rustc.sh");
  const run = (args: string[], cache = "") =>
    Bun.spawnSync([counting, ...args], { env: { ...process.env, RUST_JS_COUNTED_CRATES: "rust_js rust_js_program", RUST_JS_CACHE_WRAPPER: cache } }).stdout.toString().trim();
  expect(run(["echo", "--edition=2024", "--crate-name", "rust_js_program"])).toBe("--edition=2024 --crate-name rust_js_program -C instrument-coverage");
  expect(run(["echo", "--crate-name", "rust_js", "--crate-type", "bin"])).toBe("--crate-name rust_js --crate-type bin -C instrument-coverage");
  // A question of rustc, `-vV`, isn't a compile; nor a build script's,
  // which Cargo runs where it'd leave its counts, a dependency's either.
  expect(run(["echo", "-vV"])).toBe("-vV");
  expect(run(["echo", "--crate-name", "build_script_build"])).toBe("--crate-name build_script_build");
  expect(run(["echo", "--crate-name", "indexmap"])).toBe("--crate-name indexmap");
  expect(run(["--crate-name", "indexmap"], "echo")).toBe("--crate-name indexmap");
  // A name that ends one counted isn't it.
  expect(run(["echo", "--crate-name", "js"])).toBe("--crate-name js");
});

// A test that lists the compiler's files by scripts/compiler-crates.ts
// reads every crate's sources, as one of `join(root, "src")` read the one.
test("a test listing the compiler's files reads each crate's sources", () => {
  const reads = readsOf("test/architecture-maps.test.ts");
  expect(reads.has("src") && reads.has("crates/rust-js-program/src")).toBe(true);
  expect(readsSources("test/architecture-maps.test.ts")).toBe(true);
});
