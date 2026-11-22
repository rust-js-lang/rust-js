// The compiler's crates (ADR 1181): the layers docs/architecture.md draws,
// held by Cargo's crate graph where a crate holds a layer, not only by the
// module rules of architecture.test.ts. A crate can't name what it doesn't
// depend on, so its dependencies are its contract, and they're checked here.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { binaryCrate, compilerCrates, compilerRustFiles, crateOf, crateOfModule, isBinarySource, isCompilerSource, modulePath, topModules } from "../scripts/compiler-crates";
import { root } from "./support";

type Manifest = { package?: { name: string }; dependencies?: Record<string, unknown>; workspace?: { members?: string[]; exclude?: string[] } };
const manifest = (dir: string): Manifest => Bun.TOML.parse(readFileSync(join(root, dir, "Cargo.toml"), "utf8")) as Manifest;
const binary = binaryCrate();
const members = compilerCrates().filter((crate) => crate !== binary);

// Each crate of the workspace, by the compiler's crates it may depend on:
// owned output on none. A crate added names its place here.
const allowed: Record<string, string[]> = {
  "rust-js-program": [],
  "rust-js-emit": ["rust-js-program"],
  // The lowering (#2299): its base reads owned output; MIR's builds on it.
  "rust-js-lower": ["rust-js-program"],
  "rust-js-mir": ["rust-js-lower", "rust-js-program"],
};

test("each crate depends only on the compiler's crates its layer may", () => {
  expect(members.map((crate) => crate.package).sort()).toEqual(Object.keys(allowed).sort());
  const ours = new Set(compilerCrates().map((crate) => crate.package));
  for (const crate of members) {
    const depends = Object.keys(manifest(crate.dir).dependencies ?? {}).filter((name) => ours.has(name));
    expect([crate.package, depends.sort()]).toEqual([crate.package, allowed[crate.package]!.sort()]);
  }
  // The binary depends on each of them, by its path.
  const theirs = manifest("").dependencies ?? {};
  for (const crate of members) expect(theirs[crate.package], crate.package).toEqual({ path: crate.dir });
});

// A crate of the workspace has no rustc in it: it builds and tests on any
// target, the playground's WASI too, with no `rustc_private`, which
// `.cargo/config.toml` lets only the binary have. rustc-hash, a crate of
// its own that rustc's crates use too, is the compiler's maps' hasher.
// The lowering's two crates are rustc's as the binary is (#2299): they read
// its THIR and MIR.
const rustcs = new Set(["rust-js-lower", "rust-js-mir"]);
test("no crate but the binary and the lowering's uses rustc", () => {
  const strays = compilerRustFiles()
    .filter((file) => crateOf(file) !== binary && !rustcs.has(crateOf(file)?.package ?? ""))
    .filter((file) => /\brustc_(?!hash::)\w+|#!\[feature|extern crate/.test(readFileSync(join(root, file), "utf8").replace(/\/\/.*$/gm, "")));
  expect(strays).toEqual([]);
  // The binary's and the lowering's crates alone (#2299).
  expect(readFileSync(join(root, ".cargo/config.toml"), "utf8").match(/rust_js_\w+/g)).toEqual(["rust_js_lower", "rust_js_mir"]);
});

// Every Cargo package in the checkout is the binary, a member, its own
// workspace or one the root excludes: Cargo refuses to build a package
// under a workspace's root that's none of these, `cargo check` in next/.
test("each Cargo package of the checkout is the workspace's or its own", () => {
  const { workspace } = manifest("");
  const tracked = Bun.spawnSync(["git", "ls-files", "--", "*Cargo.toml"], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const orphans = tracked
    .map((file) => file.slice(0, -"Cargo.toml".length).replace(/\/$/, ""))
    .filter((dir) => dir !== "" && !workspace!.members!.includes(dir))
    .filter((dir) => !workspace!.exclude!.some((excluded) => dir === excluded || dir.startsWith(`${excluded}/`)))
    .filter((dir) => manifest(dir).workspace === undefined);
  expect(orphans).toEqual([]);
  // Each member is a crate of `crates/`, by its package's name.
  for (const crate of members) expect(crate.dir).toBe(`crates/${crate.package}`);
});

test("which crate a file and a module are of is the layout's answer", () => {
  expect(binary.package).toBe("rust-js");
  expect(isBinarySource("src/pipeline/settle.rs")).toBe(true);
  expect(isBinarySource("crates/rust-js-lower/src/copies.rs")).toBe(false);
  expect(isBinarySource("crates/rust-js-program/src/lines.rs")).toBe(false);
  expect(crateOf("src/pipeline/settle.rs")).toBe(binary);
  expect(crateOf("crates/rust-js-lower/src/copies.rs")?.name).toBe("rust_js_lower");
  expect(crateOf("crates/rust-js-program/src/collections.rs")?.name).toBe("rust_js_program");
  expect(crateOf("crates/rust-js-program/Cargo.toml")).toBeUndefined();
  expect(isCompilerSource("test/corpus/a.rs")).toBe(false);
  expect(modulePath("crates/rust-js-lower/src/drops/types.rs")).toEqual(["drops", "types"]);
  expect(modulePath("src/pipeline/settle.rs")).toEqual(["pipeline", "settle"]);
  expect(modulePath("src/main.rs")).toEqual([]);
  expect(modulePath("crates/rust-js-program/src/lib.rs")).toEqual([]);
  expect(modulePath("crates/rust-js-program/src/decisions.rs")).toEqual(["decisions"]);
  expect(crateOfModule("collections")?.package).toBe("rust-js-program");
  expect(crateOfModule("pipeline")).toBe(binary);
  // A top-level module of the binary is no other crate's, so a path through
  // its root is one crate's. Two of the workspace's may share a name, the
  // lowering's base and MIR's `drops` as `lower::` and `lower::mir::` had
  // (#2299): each names its own first, and move-paths leaves a name two
  // declare where a third names it.
  const own = new Set(topModules(binary));
  const others = compilerCrates().filter((crate) => crate !== binary).flatMap((crate) => topModules(crate));
  expect(others.filter((module) => own.has(module))).toEqual([]);
});

// What a test makes and a compile never does, the `testing` feature of a
// crate (a `Name` of a word alone, #2034), is the tests' alone: a normal
// build of the binary, the compiler a user runs, has none of it.
test("a crate's `testing` feature never reaches a normal build", () => {
  const tree = Bun.spawnSync(["cargo", "tree", "--locked", "--offline", "-p", "rust-js", "-e", "normal,features", "--prefix", "none"], { cwd: root });
  expect(tree.exitCode).toBe(0);
  const lines = tree.stdout.toString().split("\n");
  expect(lines.some((line) => line.startsWith("rust-js-program"))).toBe(true);
  expect(lines.filter((line) => /feature "testing"/.test(line))).toEqual([]);
  // And the tests' build has it, so the guard is real.
  const dev = Bun.spawnSync(["cargo", "tree", "--locked", "--offline", "-p", "rust-js", "-e", "dev,features", "--prefix", "none"], { cwd: root });
  expect(dev.stdout.toString()).toMatch(/rust-js-program feature "testing"/);
});

// The front end builds owned output and never prints it: of the compiler's
// crates it names owned output's and the lowering's own, the base's and
// MIR's (#2297, #2299), printing's crate the driver's to call.
test("the front end names no crate but owned output's", () => {
  const front = compilerRustFiles().filter(
    (file) =>
      file.startsWith("crates/rust-js-lower/src/") ||
      file.startsWith("crates/rust-js-mir/src/") ||
      file === "src/pipeline.rs" ||
      file.startsWith("src/pipeline/") ||
      file === "src/jsx_syntax.rs" ||
      file.startsWith("src/jsx_syntax/"),
  );
  expect(front.length).toBeGreaterThan(100);
  const front_crates = ["rust-js-program", "rust-js-lower", "rust-js-mir"];
  const others = members.filter((crate) => !front_crates.includes(crate.package)).map((crate) => crate.name);
  const strays = front.filter((file) => others.some((name) => new RegExp(`\\b${name}::`).test(readFileSync(join(root, file), "utf8"))));
  expect(strays).toEqual([]);
});
