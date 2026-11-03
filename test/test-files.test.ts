// Each module's tests are its `.test.js`, beside its JS (ADR 1076): a module
// of tests, `#[cfg(test)] mod tests`, is its parent's test file; a `#[test]`
// of any other module is registered in its module's; each under its Rust
// path, by the runner's `test`, which the file imports. Compiled alone, by
// `rust-js --test`, and run by Node's test runner, the default.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";
import { counts } from "./rust-js-tests";

beforeAll(buildCompiler, 600_000);

/** A crate of `files`, by their paths, `lib.rs` its root, compiled with its
 * tests into `out/`: what it wrote, by path, and its manifest. */
function compiled(name: string, files: Record<string, string>) {
  const dir = fixture(name);
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  }
  const out = join(dir, "out");
  run([compiler, "--test", join(dir, "lib.rs"), "-o", join(out, "lib.js"), "--manifest", join(out, "manifest.json")]);
  const manifest = JSON.parse(readFileSync(join(out, "manifest.json"), "utf8"));
  const read = (path: string) => readFileSync(join(out, path), "utf8").replace(/\n\/\/# sourceMappingURL=.*\n$/, "\n");
  const has = (path: string) => existsSync(join(out, path));
  /** What Node's test runner says of its test files. */
  const tested = () => {
    const p = Bun.spawnSync(["node", "--test", ...manifest.tests.files], { cwd: dir, stdout: "pipe", stderr: "pipe" });
    const said = p.stdout.toString() + p.stderr.toString();
    return { code: p.exitCode, said, ...counts("node", said) };
  };
  return { out, manifest, read, has, tested };
}

test("a module's one module of tests is its test file, its tests and its modules' registered there", () => {
  const crate = compiled("test-files-placed", {
    "lib.rs": "pub mod shapes;\n",
    "shapes.rs": `pub fn area(side: u32) -> u32 {
    side * side
}

#[cfg(test)]
mod tests;

// Compiled for tests, with none of its own: a module as any other.
#[cfg(test)]
mod fixtures {
    pub const SIDE: u32 = 3;
}
`,
    "shapes/tests.rs": `use super::fixtures::SIDE;
use super::*;

// A helper named as the runner's \`test\`, which the file imports by another.
fn test(side: u32) -> u32 {
    area(side)
}

#[test]
fn squares() {
    assert_eq!(test(SIDE), 9);
}

#[test]
#[should_panic(expected = "negative")]
fn refuses() {
    panic!("negative side");
}

#[test]
#[ignore]
fn slow() {}

#[test]
fn returns() -> Result<(), String> {
    Ok(())
}

mod nested {
    #[test]
    fn of_one() {
        assert_eq!(super::super::area(1), 1);
    }
}
`,
  });
  expect(["shapes.js", "shapes.test.js", "shapes/fixtures.js", "shapes/tests.js", "shapes/tests/nested.js", "shapes/tests/nested.test.js", "lib.test.js"].map(crate.has)).toEqual([
    true,
    true,
    true,
    false,
    true,
    false,
    false,
  ]);
  const js = crate.read("shapes.test.js");
  expect(js).toContain('import { test as test$1 } from "node:test";');
  expect(js).toContain('import * as Nested from "./shapes/tests/nested.js";');
  expect(js).toContain("\nfunction test(side) {");
  expect(js).toContain(`test$1("shapes::tests::squares", squares);
test$1("shapes::tests::refuses", () => $shouldPanic(refuses, "negative"));
test$1.skip("shapes::tests::slow", slow);
test$1("shapes::tests::returns", () => $testResult(returns()));
test$1("shapes::tests::nested::of_one", Nested.ofOne);
`);
  // Its own functions, registered where they are, aren't exported.
  expect(js).not.toContain("export function squares");
  expect(crate.manifest.tests).toEqual({ runner: "node", module: "node:test", files: [join(crate.out, "shapes.test.js")] });
  const { code, pass, fail, skip, said } = crate.tested();
  expect([code, pass, fail, skip], said).toEqual([0, 4, 0, 1]);
});

test("a module's several modules of tests are their own files, its test file registering theirs", () => {
  const crate = compiled("test-files-several", {
    "lib.rs": `pub fn side() -> u32 {
    3
}

#[test]
fn at_the_root() {
    assert_eq!(side(), 3);
}

#[cfg(test)]
mod tests {
    #[test]
    fn one() {
        assert_eq!(super::side(), 3);
    }
}

#[cfg(all(test, not(browser)))]
mod proptests {
    #[test]
    fn two() {
        assert_eq!(super::side() - 1, 2);
    }
}

// Compiled for tests, with none of its own: a module as any other.
#[cfg(test)]
mod test_utils {
    pub fn three() -> u32 {
        3
    }
}

// Tests in a module of another cfg, not compiled for tests alone: its own
// test file's.
#[cfg(target_pointer_width = "32")]
pub mod checks {
    #[test]
    fn three() {
        assert_eq!(crate::side(), 3);
    }
}
`,
  });
  expect(["lib.test.js", "tests.js", "proptests.js", "test_utils.js", "checks.js", "checks.test.js"].map(crate.has)).toEqual([true, true, true, true, true, true]);
  expect(crate.read("lib.test.js")).toContain(`import { test } from "node:test";
import * as Lib from "./lib.js";
import * as Proptests from "./proptests.js";
import * as Tests from "./tests.js";

test("at_the_root", Lib.atTheRoot);
test("tests::one", Tests.one);
test("proptests::two", Proptests.two);
`);
  // What another file registers, each module exports.
  expect([crate.read("lib.js"), crate.read("tests.js")].map((js) => /^export function (atTheRoot|one)\(/m.test(js))).toEqual([true, true]);
  expect(crate.read("checks.test.js")).toContain('import * as Checks from "./checks.js";\n\ntest("checks::three", Checks.three);\n');
  const { code, pass, said } = crate.tested();
  expect([code, pass], said).toEqual([0, 4]);
});

test("the runner the crate names is the one its test files import test from", () => {
  for (const [runner, module] of [["bun", "bun:test"], ["vitest", "vitest"], ["jest", "@jest/globals"]]) {
    const crate = compiled(`test-files-${runner}`, {
      "Cargo.toml": `[package]\nname = "t"\nversion = "0.1.0"\n\n[package.metadata.rust-js.test]\nrunner = "${runner}"\n`,
      "lib.rs": "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    });
    expect(crate.read("lib.test.js")).toContain(`import { test } from "${module}";`);
    expect(crate.manifest.tests).toEqual({ runner, module, files: [join(crate.out, "lib.test.js")] });
  }
});
