// The compiler's crates (ADR 1181): the binary's, at the checkout's root,
// and each member of the root Cargo.toml's `[workspace]`, one directory of
// `crates/` each. That list is their one authority: what's the compiler's
// source, which crate a file is of, its module there, and which crate a
// top-level module is of, a path's and a test's answer alike, are read of
// it here, never of a second list of directories.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const checkout = join(import.meta.dir, "..");

export type CompilerCrate = {
  /** Its Cargo package's name, `rust-js-program`, as `cargo -p` takes it. */
  package: string;
  /** Its name in Rust's paths, `rust_js_program`. */
  name: string;
  /** Its directory from the checkout, `crates/rust-js-program`; the binary's `""`. */
  dir: string;
  /** Its sources' directory, `crates/rust-js-program/src`; the binary's `src`. */
  src: string;
  /** Its root file, `crates/rust-js-program/src/lib.rs`; the binary's `src/main.rs`. */
  rootFile: string;
};

const cache = new Map<string, CompilerCrate[]>();

/** The compiler's crates in `at`, the checkout by default: the binary's
 * first, then the workspace's members as the root Cargo.toml lists them. */
export function compilerCrates(at = checkout): CompilerCrate[] {
  const kept = cache.get(at);
  if (kept) return kept;
  const manifest = Bun.TOML.parse(readFileSync(join(at, "Cargo.toml"), "utf8")) as {
    package: { name: string };
    workspace?: { members?: string[] };
  };
  const crate = (dir: string, name: string, rootFile: string): CompilerCrate => {
    const src = dir === "" ? "src" : `${dir}/src`;
    return { package: name, name: name.replaceAll("-", "_"), dir, src, rootFile: `${src}/${rootFile}` };
  };
  const members = (manifest.workspace?.members ?? []).map((dir) => {
    const member = Bun.TOML.parse(readFileSync(join(at, dir, "Cargo.toml"), "utf8")) as { package: { name: string } };
    return crate(dir, member.package.name, "lib.rs");
  });
  const crates = [crate("", manifest.package.name, "main.rs"), ...members];
  cache.set(at, crates);
  return crates;
}

/** The crate whose sources hold `file`, a path from the checkout; none for
 * a file that isn't the compiler's source. */
export function crateOf(file: string, at = checkout): CompilerCrate | undefined {
  return compilerCrates(at).find((crate) => file.startsWith(`${crate.src}/`));
}

/** The binary's crate, the root's package, `rust-js`: the driver and the
 * front end, rustc's. */
export function binaryCrate(at = checkout): CompilerCrate {
  return compilerCrates(at).find((crate) => crate.dir === "")!;
}

/** Whether `file`, a path from the checkout, is the binary's source. */
export function isBinarySource(file: string, at = checkout): boolean {
  return crateOf(file, at)?.dir === "";
}

/** Whether `file`, a path from the checkout, is one of the compiler's
 * sources: under a crate's `src/`, its Rust and what it includes. */
export function isCompilerSource(file: string, at = checkout): boolean {
  return crateOf(file, at) !== undefined;
}

/** The compiler's sources' directories, `src` first. */
export function compilerSourceDirs(at = checkout): string[] {
  return compilerCrates(at).map((crate) => crate.src);
}

/** A path filter's globs of the compiler's code: each crate's sources and
 * its manifest. */
export function compilerSourceGlobs(at = checkout): string[] {
  return compilerCrates(at).flatMap((crate) => (crate.dir === "" ? ["src/**"] : [`${crate.dir}/**`]));
}

/** Each `.rs` file of the compiler's crates, in name order, crate by crate. */
export function compilerRustFiles(at = checkout): string[] {
  const walk = (dir: string): string[] =>
    readdirSync(join(at, dir), { withFileTypes: true })
      .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
      .flatMap((entry) => (entry.isDirectory() ? walk(`${dir}/${entry.name}`) : entry.name.endsWith(".rs") ? [`${dir}/${entry.name}`] : []));
  return compilerCrates(at).flatMap((crate) => (existsSync(join(at, crate.src)) ? walk(crate.src) : []));
}

/** `file`'s module path in its crate, `src/lower/drops/types.rs`'s
 * `["lower", "drops", "types"]`; a crate's root's `[]`. */
export function modulePath(file: string, at = checkout): string[] {
  const crate = crateOf(file, at);
  if (!crate) throw new Error(`${file} isn't the compiler's source`);
  if (file === crate.rootFile) return [];
  const parts = file.slice(crate.src.length + 1).replace(/\.rs$/, "").split("/");
  if (parts.at(-1) === "mod") parts.pop();
  return parts;
}

/** The top-level modules a crate's root declares, `mod x;` or `pub mod x;`,
 * by name. */
export function topModules(crate: CompilerCrate, at = checkout): string[] {
  const text = readFileSync(join(at, crate.rootFile), "utf8").replace(/\/\/.*$/gm, "");
  return [...text.matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_]\w*)\s*;/gm)].map((m) => m[1]!);
}

/** The crate whose root declares top-level module `module`. */
export function crateOfModule(module: string, at = checkout): CompilerCrate | undefined {
  return compilerCrates(at).find((crate) => topModules(crate, at).includes(module));
}
