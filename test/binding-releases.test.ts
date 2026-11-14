// Two releases of one crate in one Cargo build (#1799, ADR 0100, amended):
// two of a Rust crate are two libraries, as rustc has them, each reported;
// two of a binding crate over one npm package are refused, as an app
// installs the package once and both would read that copy; over two npm
// names, an alias, they're two packages.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkCargo } from "../tooling/cargo.js";
import { buildCompiler, compiler, fixture } from "./support";
import { channel } from "../scripts/toolchain.ts";

beforeAll(buildCompiler, 600_000);

function crate(dir: string, name: string, version: string, body: string, dependencies = "", bindings = false) {
  mkdirSync(join(dir, "src"), { recursive: true });
  const metadata = bindings ? "\n[package.metadata.rust-js]\nbindings = true\n" : "";
  writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "${version}"\nedition = "2024"\n${metadata}\n[dependencies]\n${dependencies}`);
  writeFileSync(join(dir, "src", "lib.rs"), body);
}

/** A binding crate's release of `dom`, binding `computePosition` of `module`. */
function dom(dir: string, version: string, module: string) {
  crate(dir, "dom", version, `#[cfg_attr(rust_js, rust_js::link_name = "${module}#computePosition")]\npub fn compute_position(x: f64) -> f64 {\n    unreachable!("{x}")\n}\n`, "", true);
}

/** `a` of `dom` 0.1, `b` of `dom` 1.0, and an app of both. */
function graph(dir: string, older: string, newer: string) {
  dom(join(dir, "dom-0"), "0.1.0", older);
  dom(join(dir, "dom-1"), "1.0.0", newer);
  crate(join(dir, "a"), "a", "0.1.0", "pub fn left() -> f64 {\n    dom::compute_position(1.0)\n}\n", 'dom = { path = "../dom-0" }\n');
  crate(join(dir, "b"), "b", "0.1.0", "pub fn right() -> f64 {\n    dom::compute_position(2.0)\n}\n", 'dom = { path = "../dom-1" }\n');
  crate(join(dir, "app"), "app", "0.1.0", "pub fn both() -> f64 {\n    a::left() + b::right()\n}\n", 'a = { path = "../a" }\nb = { path = "../b" }\n');
  return join(dir, "app", "Cargo.toml");
}

test("two releases of a binding crate over one npm package are refused, naming both and their users", async () => {
  const manifestPath = graph(fixture("binding-releases-one"), "@floating-ui/dom", "@floating-ui/dom");
  const built = checkCargo({ manifestPath, toolchain: channel, compiler, offline: true, packageNames: ["app"] });
  await expect(built).rejects.toThrow("two releases of `dom` bind the npm package `@floating-ui/dom`, one used by `a`, the other by `b`");
}, 300_000);

test("two releases of a binding crate over two npm names, an alias, are two packages", async () => {
  const dir = fixture("binding-releases-aliased");
  const manifestPath = graph(dir, "dom-v0", "@floating-ui/dom");
  const built = await checkCargo({ manifestPath, toolchain: channel, compiler, offline: true, packageNames: ["app"] });
  // Each crate's JS imports the package its own release binds.
  const js = (name: string) => readFileSync(built.crates.find((c) => c.crate === name)!.js!, "utf8");
  expect([js("a").includes('from "dom-v0"'), js("b").includes('from "@floating-ui/dom"')]).toEqual([true, true]);
}, 300_000);

// Two releases of one Rust crate are two libraries, each reported, and
// each its own JS: the build's every crate, never the last of a name.
test("two releases of one Rust crate are each built and reported, by their manifests", async () => {
  const dir = fixture("crate-releases-reported");
  crate(join(dir, "geometry-1"), "geometry", "1.0.0", "pub fn unit() -> u32 {\n    1\n}\n");
  crate(join(dir, "geometry-2"), "geometry", "2.0.0", "pub fn unit() -> u32 {\n    2\n}\n");
  crate(join(dir, "app"), "app", "0.1.0", "pub fn sum() -> u32 {\n    old::unit() + now::unit()\n}\n", 'old = { package = "geometry", path = "../geometry-1" }\nnow = { package = "geometry", path = "../geometry-2" }\n');
  const built = await checkCargo({ manifestPath: join(dir, "app", "Cargo.toml"), toolchain: channel, compiler, offline: true, packageNames: ["app"] });
  const geometry = built.crates.filter((c) => c.crate === "geometry");
  expect(geometry.length).toBe(2);
  expect(new Set(geometry.map((c) => c.manifest)).size).toBe(2);
  expect(geometry.map((c) => readFileSync(c.js!, "utf8").includes("return 1;") ? 1 : 2).sort()).toEqual([1, 2]);
  expect((await import(built.js!)).sum()).toBe(3);
}, 300_000);
