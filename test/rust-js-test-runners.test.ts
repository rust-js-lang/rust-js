// `rust-js test` by each JS test runner (ADR 1076): the same Rust tests,
// written for the runner `[package.metadata.rust-js.test]` names, each
// test file importing its `test`, and run by it. Each outcome is Rust's:
// `#[ignore]` a skip, `#[should_panic]` passing only on a panic with its
// message, a `TypeError` never a panic, a `Result`'s `Err` a failure.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler } from "./support";
import { counts, rustJsTest, testCrate } from "./rust-js-tests";

beforeAll(buildCompiler, 600_000);

const lib = `pub fn quadruple(n: u32) -> u32 {
    util::twice(util::twice(n))
}

/// A URL, as JS has it (ADR 0111).
struct Url(core::marker::PhantomData<js::JsObject>);

// Throws a \`TypeError\` for a bad URL: JS going wrong, not a panic.
#[cfg_attr(rust_js, rust_js::link_name = "new URL")]
fn new_url(_href: &str) -> &'static Url {
    unreachable!()
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
    fn panics_with_the_right_message() {
        panic!("overflow of {}", quadruple(1));
    }

    #[test]
    #[should_panic(expected = "nope")]
    fn panics_with_the_wrong_message() {
        panic!("{:?} happened", "something");
    }

    #[test]
    #[should_panic]
    fn throws_a_type_error() {
        let _ = new_url("not a url");
    }

    #[test]
    #[ignore]
    fn ignored() {
        panic!("never runs");
    }

    #[test]
    fn returns_ok() -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn returns_an_err() -> Result<(), String> {
        Err("no stock".to_string())
    }
}
`;

/** Where a runner imports `test` from, as the compiler writes it. */
const modules: Record<string, string> = { node: "node:test", bun: "bun:test", vitest: "vitest", jest: "@jest/globals" };

for (const runner of ["node", "bun", "vitest", "jest"]) {
  test(`by ${runner}, each outcome as Rust's`, () => {
    const dir = testCrate(`rust-js-test-by-${runner}`, { "src/lib.rs": lib, "tests/sixteen.rs": "#[test]\nfn sixteen() {\n    assert_eq!(app::quadruple(4), 16);\n}\n" }, { runner, js: true });
    const { code, said } = rustJsTest(dir);
    // 4 pass, the integration test's among them; 3 fail; 1 skipped.
    expect([counts(runner, said), code], said).toEqual([{ pass: 4, fail: 3, skip: 1 }, 1]);
    expect(said).toContain('panic message: "\\"something\\" happened"');
    expect(said).toContain("TypeError");
    expect(said).toContain('Error: "no stock"');
    const units = join(dir, "node_modules/.cache/rust-js/test/wasm32-unknown-unknown/debug/deps/rust-js");
    const written = [...new Bun.Glob("app-*/lib.test.js").scanSync({ cwd: units })].map((file) => readFileSync(join(units, file), "utf8"));
    expect(written.map((js) => js.includes(`import { test } from "${modules[runner]}";`))).toEqual([true]);
  }, 180_000);
}

test("a runner rust-js doesn't know is refused, naming the four", () => {
  const { code, said } = rustJsTest(testCrate("rust-js-test-by-mocha", { "src/lib.rs": "#[test]\nfn t() {}\n" }, { runner: "mocha" }));
  expect(code).toBe(1);
  expect(said).toContain("unknown variant `mocha`, expected one of `node`, `bun`, `vitest`, `jest`");
});
