// `rust-js test` (ADR 0914): a crate's library's `#[test]`s compiled to JS
// as `cargo check --profile test --lib` builds them, a dependency's JS
// too, run by bun, which gives the `test()` each calls. What bun says, and
// its exit code, are the command's.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

/** A crate, `app`, whose library is `lib`, using a crate of its own, `util`,
 * by its path. */
function crate(name: string, lib: string): string {
  const dir = fixture(name);
  mkdirSync(join(dir, "src"), { recursive: true });
  mkdirSync(join(dir, "util", "src"), { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), `[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
util = { path = "util" }
`);
  writeFileSync(join(dir, "util", "Cargo.toml"), `[package]
name = "util"
version = "0.1.0"
edition = "2021"
`);
  writeFileSync(join(dir, "util", "src", "lib.rs"), `pub fn twice(n: u32) -> u32 {
    n * 2
}
`);
  writeFileSync(join(dir, "src", "lib.rs"), lib);
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name, private: true }));
  return dir;
}

/** What `rust-js test -- ..args` says of the crate at `dir`, and its exit code. */
function rustJsTest(dir: string, ...args: string[]) {
  const p = Bun.spawnSync(["bun", join(root, "tooling", "cli.js"), "test", ...(args.length ? ["--", ...args] : [])], {
    cwd: dir,
    env: { ...process.env, RUST_JS_COMPILER: compiler },
    stdout: "pipe",
    stderr: "pipe",
  });
  return { code: p.exitCode, said: p.stdout.toString() + p.stderr.toString() };
}

const lib = `pub fn quadruple(n: u32) -> u32 {
    util::twice(util::twice(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadruples() {
        assert_eq!(quadruple(3), 12);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn panics() {
        panic!("overflow of {}", quadruple(1));
    }
}
`;

test("rust-js test runs a crate's #[test]s as JS, its dependency's JS too", () => {
  const { code, said } = rustJsTest(crate("rust-js-test-passing", lib));
  expect(said).toMatch(/\b2 pass\b/);
  expect(said).toMatch(/\b0 fail\b/);
  expect(code).toBe(0);
});

test("a failing #[test] fails rust-js test, saying what Rust's assert_eq! says", () => {
  const failing = lib.replace("assert_eq!(quadruple(3), 12);", "assert_eq!(quadruple(3), 13);");
  const { code, said } = rustJsTest(crate("rust-js-test-failing", failing));
  expect(said).toContain("assertion `left == right` failed");
  expect(said).toMatch(/\b1 fail\b/);
  expect(code).toBe(1);
});

test("what follows -- is bun's, -t picking the tests to run", () => {
  const { code, said } = rustJsTest(crate("rust-js-test-filtered", lib), "-t", "quadruples");
  expect(said).toMatch(/\b1 pass\b/);
  expect(said).toContain("1 filtered out");
  expect(code).toBe(0);
});
