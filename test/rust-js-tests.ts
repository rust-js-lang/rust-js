// Crates whose `#[test]`s `rust-js test` runs (ADRs 0916, 1076), for the
// tests of that command.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { stripVTControlCharacters } from "node:util";
import { compiler, fixture, root } from "./support";

/** A crate, `app`, of `files`, each by its path in the crate, `src/lib.rs`
 * among them, whose tests are for `runner`, where given, and which uses a
 * crate of its own, `util`, by its path, and with `js`, rust-js's `js`. */
export function testCrate(name: string, files: Record<string, string>, { runner, js = false }: { runner?: string; js?: boolean } = {}): string {
  const dir = fixture(name);
  const write = (path: string, text: string) => {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  };
  write("Cargo.toml", `[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
util = { path = "util" }
${js ? `js = { package = "rust-js-builtins", path = ${JSON.stringify(join(root, "builtins"))} }\n` : ""}${runner ? `\n[package.metadata.rust-js.test]\nrunner = "${runner}"\n` : ""}`);
  write("util/Cargo.toml", `[package]
name = "util"
version = "0.1.0"
edition = "2021"
`);
  write("util/src/lib.rs", `pub fn twice(n: u32) -> u32 {
    n * 2
}
`);
  for (const [path, text] of Object.entries(files)) write(path, text);
  write("package.json", JSON.stringify({ name, private: true }));
  return dir;
}

/** What `rust-js test -- ..args` says of the crate at `dir`, and its exit
 * code; into `targetDir`, where given. */
export function rustJsTest(dir: string, args: string[] = [], targetDir?: string) {
  const p = Bun.spawnSync(["bun", join(root, "tooling", "cli.js"), "test", ...(args.length ? ["--", ...args] : [])], {
    cwd: dir,
    env: { ...process.env, RUST_JS_COMPILER: compiler, ...(targetDir ? { CARGO_TARGET_DIR: targetDir } : {}) },
    stdout: "pipe",
    stderr: "pipe",
  });
  return { code: p.exitCode, said: p.stdout.toString() + p.stderr.toString() };
}

/** How many tests passed, failed and were skipped, as `runner`'s summary
 * says, its colors left out: Vitest colors it wherever `CI` is set, as
 * on GitHub's runners, though no terminal is there. */
export function counts(runner: string, colored: string): { pass: number; fail: number; skip: number } {
  const said = stripVTControlCharacters(colored);
  const count = (pattern: RegExp) => Number(pattern.exec(said)?.[1] ?? 0);
  switch (runner) {
    case "node":
      return { pass: count(/^ℹ pass (\d+)$/m), fail: count(/^ℹ fail (\d+)$/m), skip: count(/^ℹ skipped (\d+)$/m) };
    case "bun":
      return { pass: count(/^ *(\d+) pass$/m), fail: count(/^ *(\d+) fail$/m), skip: count(/^ *(\d+) skip$/m) };
    case "vitest": {
      const line = /^ *Tests +(.*)$/m.exec(said)?.[1] ?? "";
      const of = (what: string) => Number(new RegExp(`(\\d+) ${what}`).exec(line)?.[1] ?? 0);
      return { pass: of("passed"), fail: of("failed"), skip: of("skipped") };
    }
    case "jest": {
      const line = /^Tests: +(.*)$/m.exec(said)?.[1] ?? "";
      const of = (what: string) => Number(new RegExp(`(\\d+) ${what}`).exec(line)?.[1] ?? 0);
      return { pass: of("passed"), fail: of("failed"), skip: of("skipped") };
    }
  }
  throw new Error(`no runner ${runner}`);
}
