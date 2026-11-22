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
test("no crate but the binary uses rustc", () => {
  const strays = compilerRustFiles()
    .filter((file) => crateOf(file) !== binary)
    .filter((file) => /\brustc_(?!hash::)\w+|#!\[feature|extern crate/.test(readFileSync(join(root, file), "utf8").replace(/\/\/.*$/gm, "")));
  expect(strays).toEqual([]);
  expect(readFileSync(join(root, ".cargo/config.toml"), "utf8")).not.toMatch(/rust_js_\w+/);
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
  expect(isBinarySource("src/lower/copies.rs")).toBe(true);
  expect(isBinarySource("crates/rust-js-program/src/lines.rs")).toBe(false);
  expect(crateOf("src/lower/copies.rs")).toBe(binary);
  expect(crateOf("crates/rust-js-program/src/collections.rs")?.name).toBe("rust_js_program");
  expect(crateOf("crates/rust-js-program/Cargo.toml")).toBeUndefined();
  expect(isCompilerSource("test/corpus/a.rs")).toBe(false);
  expect(modulePath("src/lower/drops/types.rs")).toEqual(["lower", "drops", "types"]);
  expect(modulePath("src/main.rs")).toEqual([]);
  expect(modulePath("crates/rust-js-program/src/lib.rs")).toEqual([]);
  expect(modulePath("crates/rust-js-program/src/decisions.rs")).toEqual(["decisions"]);
  expect(crateOfModule("collections")?.package).toBe("rust-js-program");
  expect(crateOfModule("lower")).toBe(binary);
  // Each top-level module is one crate's.
  const modules = compilerCrates().flatMap((crate) => topModules(crate));
  expect(modules.length).toBe(new Set(modules).size);
});
