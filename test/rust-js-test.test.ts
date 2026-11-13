// `rust-js test` (ADRs 0916, 1076): a crate's `#[test]`s compiled to JS as
// `cargo check --profile test` builds them, a dependency's JS too, one
// `.test.js` beside each module's JS that has tests, run by Node's test
// runner, the default. What it says, and its exit code, are the command's.
// Each runner's own: test/rust-js-test-runners.test.ts.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { buildCompiler, outsideFixture, rustJsUnits } from "./support";
import { counts, rustJsTest, testCrate } from "./rust-js-tests";

beforeAll(buildCompiler, 600_000);

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

const failing = lib.replace("assert_eq!(quadruple(3), 12);", "assert_eq!(quadruple(3), 13);");

test("rust-js test runs a crate's #[test]s as JS, its dependency's JS too, by Node's runner", () => {
  const { code, said } = rustJsTest(testCrate("rust-js-test-passing", { "src/lib.rs": lib }));
  expect([counts("node", said), code], said).toEqual([{ pass: 2, fail: 0, skip: 0 }, 0]);
});

test("a failing #[test] fails rust-js test, saying what Rust's assert_eq! says", () => {
  const { code, said } = rustJsTest(testCrate("rust-js-test-failing", { "src/lib.rs": failing }));
  expect(said).toContain("assertion `left == right` failed");
  expect([counts("node", said).fail, code]).toEqual([1, 1]);
});

// A crate of no `#[test]` that's built says so, as `cargo test` says
// "running 0 tests", and passes as it does: a run of nothing that said
// nothing read as a pass (#1093).
test("a crate whose #[test]s are none that's built says it runs no test", () => {
  const none = lib.replaceAll("#[test]", "#[cfg(any())]\n    #[test]");
  expect(none).not.toBe(lib);
  const { code, said } = rustJsTest(testCrate("rust-js-test-none", { "src/lib.rs": none }));
  expect([said.includes("rust-js test: running 0 tests: app has no #[test] that's built"), code], said).toEqual([true, 0]);
});

test("what follows -- is the runner's, its name pattern picking the tests to run", () => {
  const { code, said } = rustJsTest(testCrate("rust-js-test-filtered", { "src/lib.rs": lib }), ["--test-name-pattern=quadruples"]);
  expect([counts("node", said).pass, said.includes("tests::panics"), code], said).toEqual([1, false, 0]);
});

// Cargo gives a crate the same unit's directory wherever its package is, a
// project moved or another of the same name sharing its target: the second
// build's JS replaces the first's there, each crate's tests its own. Outside
// the checkout, where no Prettier is found, as before #892 a crate's
// Prettier put its directory in rust-js's `rustc -vV`, and so in the
// unit's; now each crate names it in its dep-info alone (ADR 1003,
// amended).
test("two crates of one name at two paths, one target, each run their own tests", () => {
  const top = outsideFixture("test-namesakes");
  const shared = join(top, "target");
  const first = rustJsTest(testCrate("first", { "src/lib.rs": lib }, { within: top }), [], shared);
  const second = rustJsTest(testCrate("second", { "src/lib.rs": failing }, { within: top }), [], shared);
  expect([first.code, second.code]).toEqual([0, 1]);
  expect(counts("node", second.said).fail).toBe(1);
  // Both in one unit's directory, which the second replaced.
  expect(rustJsUnits(join(shared, "wasm32-unknown-unknown/debug")).filter((unit) => basename(unit).startsWith("app-"))).toHaveLength(1);
});

// ADR 1076: each module's tests in its own `.test.js` beside its JS, the
// module of tests' own items in it, under their Rust paths; an
// integration test's beside its own root's.
test("each module's tests are its .test.js, an integration test's too", () => {
  const dir = testCrate("rust-js-test-layout", {
    "src/lib.rs": `pub mod shapes;

pub fn sides() -> u32 {
    4
}

#[test]
fn has_four_sides() {
    assert_eq!(sides(), 4);
}
`,
    "src/shapes.rs": `pub fn area(radius: f64) -> f64 {
    std::f64::consts::PI * radius * radius
}

#[cfg(test)]
mod tests;
`,
    "src/shapes/tests.rs": `use super::*;

#[test]
fn a_unit_circle_has_area_pi() {
    assert_eq!(area(1.0), std::f64::consts::PI);
}

mod nested {
    #[test]
    fn halves() {
        assert_eq!(super::super::area(0.0), 0.0);
    }
}
`,
    "tests/quadruples.rs": `#[test]
fn sixteen_sides() {
    assert_eq!(app::sides() * 4, 16);
}
`,
  });
  const { code, said } = rustJsTest(dir);
  expect([counts("node", said), code], said).toEqual([{ pass: 4, fail: 0, skip: 0 }, 0]);
  const units = rustJsUnits(join(dir, "node_modules/.cache/rust-js/test/wasm32-unknown-unknown/debug"));
  const unit = (crate: string) =>
    units.find((d) => basename(d).startsWith(`${crate}-`) && existsSync(join(d, `${crate === "app" ? "lib" : crate}.test.js`))) ?? "";
  const app = unit("app");
  // shapes.js, and its module of tests is shapes.test.js: no shapes/tests.js.
  expect(["shapes.js", "shapes.test.js", "shapes/tests.js", "shapes/tests/nested.js", "lib.test.js"].map((f) => existsSync(join(app, f)))).toEqual([
    true,
    true,
    false,
    true,
    true,
  ]);
  const shapes = readFileSync(join(app, "shapes.test.js"), "utf8");
  expect(shapes).toContain('import { test } from "node:test";');
  expect(shapes).toContain('import { area } from "./shapes.js";');
  expect(shapes).toContain('import * as Nested from "./shapes/tests/nested.js";');
  expect(shapes).toContain("\nfunction aUnitCircleHasAreaPi() {");
  expect(shapes).toContain('test("shapes::tests::a_unit_circle_has_area_pi", aUnitCircleHasAreaPi);\ntest("shapes::tests::nested::halves", Nested.halves);');
  // A `#[test]` in a module of no module of tests: its module's, registered.
  expect(readFileSync(join(app, "lib.test.js"), "utf8")).toContain('import * as App from "./lib.js";\n\ntest("has_four_sides", App.hasFourSides);');
  // `tests/quadruples.rs`, a crate of its own: quadruples.js, and its tests.
  expect(readFileSync(join(unit("quadruples"), "quadruples.test.js"), "utf8")).toContain('test("sixteen_sides", Quadruples.sixteenSides);');
});
