// The small Crater's reading of what Cargo, Node's runner and rust-js say,
// and its cut of a refused test from the JS's copy of a crate (#1087,
// scripts/crater.ts). The net itself runs each night.
import { expect, test } from "bun:test";
import { attributesFrom, cut, cutFor, decidedOf, declaredIn, differences, errorsAt, executables, forTheJs, innerAttributes, integrationFiles, items, jsOutcomes, judged, libraryTestsOnly, modulePath, nativeOutcomes, offTarget, oneIntegrationTest, onlyForTests, said, testModulePath, testOnlyFile, undecided } from "../scripts/crater";

test("each test's outcome is read of cargo test's lines and of Node's runner's", () => {
  expect(nativeOutcomes("running 3 tests\ntest a::t1 ... ok\ntest a::t2 ... FAILED\ntest t3 ... ignored, slow\n")).toEqual({
    "a::t1": "ok",
    "a::t2": "failed",
    t3: "ignored",
  });
  expect(jsOutcomes("✔ a::t1 (0.1ms)\n✖ a::t2 (1ms)\n﹣ t3 (0ms) # SKIP\nℹ tests 3\n")).toEqual({ "a::t1": "ok", "a::t2": "failed", t3: "ignored" });
});

// Node's runner's own words, of a test that throws rust-js's refusal and one
// whose assertion fails (#1192).
const refusedRun = `✖ a::refused (0.3ms)
✖ a::wrong (0.1ms)
✔ a::ok (0.1ms)
ℹ tests 3

✖ failing tests:

test at src/lib.test.js:3:1
✖ a::refused (0.3ms)
  Error: rust-js does not support a pointer read as another type's, which reads memory by where it is
      at TestContext.<anonymous> (file:///src/lib.test.js:4:9)

test at src/lib.test.js:6:1
✖ a::wrong (0.1ms)
  Error: assertion \`left == right\` failed
    left: 1
   right: 2
      at TestContext.<anonymous> (file:///src/lib.test.js:7:9)
`;

test("a test that fails with rust-js's refusal at run time is refused, with its message, and no wrong answer", () => {
  const js = jsOutcomes(refusedRun);
  expect(js).toEqual({
    "a::refused": "refused at run time: rust-js does not support a pointer read as another type's, which reads memory by where it is",
    "a::wrong": "failed",
    "a::ok": "ok",
  });
  const crate = { name: "c", version: "1.0.0" } as Parameters<typeof said>[0];
  const verdict = judged({ "a::refused": "ok", "a::wrong": "ok", "a::ok": "ok" }, js, {}, "/nonexistent");
  expect(said(crate, verdict)).toBe(
    "c 1.0.0: 1 of 3 as native, 0 refused, 1 refused at run time: a::refused (rust-js does not support a pointer read as another type's, which reads memory by where it is), 1 WRONG: a::wrong (failed)",
  );
});

test("a test that passes natively and not as JS differs, unless it was cut as refused", () => {
  const native = { a: "ok", b: "ok", c: "ok", d: "failed" } as const;
  expect(differences(native, { a: "ok", b: "failed" }, { c: "refused" })).toEqual([{ name: "b", js: "failed" }]);
  expect(differences(native, { a: "ok", b: "ok" })).toEqual([{ name: "c", js: "not run" }]);
});

test("an error's place is read of rustc's and rust-js's diagnostics", () => {
  const said = "error: rust-js does not support calling `std::io::stdin` yet\n    --> src/lib.rs:1837:14\n     |\nerror[E0425]: cannot find function `f`\n --> src/a/mod.rs:3:5\n";
  expect(errorsAt(said)).toEqual([
    { why: "rust-js does not support calling `std::io::stdin` yet", file: "src/lib.rs", line: 1837 },
    { why: "cannot find function `f`", file: "src/a/mod.rs", line: 3 },
  ]);
});

const source = `pub fn lib() -> &'static str {
    "{ not a brace"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper() -> char {
        '{'
    }

    #[test]
    #[should_panic]
    fn refused() {
        // }
        let _ = helper();
    }

    #[test]
    fn kept() {}
}
`;

test("a source's items are found by their braces, strings, characters and comments skipped", () => {
  const found = items(source).map(({ kind, name, from, to, test, cfgTest }) => [kind, name, from, to, test, cfgTest]);
  expect(found).toEqual([
    ["fn", "lib", 1, 3, false, false],
    ["fn", "helper", 9, 11, false, false],
    ["fn", "refused", 13, 18, true, false],
    ["fn", "kept", 20, 21, true, false],
    ["mod", "tests", 5, 22, false, true],
  ]);
});

test("what's cut for an error is its test, or a function only tests reach, never the library's code", () => {
  expect(modulePath("src/lib.rs")).toEqual([]);
  expect(modulePath("src/a/b.rs")).toEqual(["a", "b"]);
  expect(modulePath("src/a/mod.rs")).toEqual(["a"]);
  expect(cutFor("src/lib.rs", source, 17)).toEqual({ from: 13, test: "tests::refused" });
  expect(cutFor("src/a.rs", source, 10)).toEqual({ from: 9 });
  expect(cutFor("src/lib.rs", source, 2)).toBeUndefined();
  const cutSource = cut(source, [13, 9]);
  expect(cutSource.split("\n")[8]).toBe("    #[cfg(any())] // rust-js's Crater: refused");
  expect(cutSource.split("\n")[13]).toBe("    #[cfg(any())] // rust-js's Crater: refused");
  expect(cutSource.split("\n")[14]).toBe("    #[test]");
});

test("the JS's copy of a crate builds its library's tests alone, as natively", () => {
  const manifest = '[package]\nname = "hex"\nautotests = true\n\n[lib]\nname = "hex"\n\n[[test]]\nname = "version-number"\npath = "tests/version-number.rs"\n\n[dependencies]\n';
  expect(libraryTestsOnly(manifest)).toBe('[package]\nautotests = false\nname = "hex"\n\n[lib]\nname = "hex"\n\n[dependencies]\n');
  // A dev-dependency the library's source never names, chrono's wasm32
  // `wasm-bindgen-test`, which its integration tests alone use, isn't
  // built; one it names is, a table's or a line's (ADR 1112).
  const devs = '[package]\nname = "chrono"\n\n[dev-dependencies]\nserde_json = "1"\nbincode = "1"\n\n[dev-dependencies.similar-asserts]\nversion = "1"\n\n[target.\'cfg(target_arch = "wasm32")\'.dev-dependencies.wasm-bindgen-test]\nversion = "0.3"\n';
  expect(libraryTestsOnly(devs, (lib) => lib !== "bincode" && lib !== "wasm_bindgen_test")).toBe(
    '[package]\nautotests = false\nname = "chrono"\n\n[dev-dependencies]\nserde_json = "1"\n\n[dev-dependencies.similar-asserts]\nversion = "1"\n\n',
  );
});

test("a file only tests build, `#[cfg(test)] mod tests;`, has its functions cut, as an inline one's", () => {
  const files: Record<string, string> = {
    "src/lib.rs": "pub fn lib() {}\n\n#[cfg(test)]\nmod tests;\nmod util;\n",
    "src/tests/mod.rs": "mod iter;\n",
    "src/tests/iter.rs": "fn helper() -> u32 {\n    1\n}\n",
    "src/util.rs": "pub fn f() {}\n",
  };
  const read = (path: string) => files[path];
  expect(declaredIn("src/tests/iter.rs", (path) => path in files)).toEqual({ parent: "src/tests/mod.rs", name: "iter" });
  expect(testOnlyFile("src/tests/iter.rs", read)).toBe(true);
  expect(testOnlyFile("src/util.rs", read)).toBe(false);
  expect(cutFor("src/tests/iter.rs", files["src/tests/iter.rs"], 2, true)).toEqual({ from: 1 });
  expect(cutFor("src/util.rs", files["src/util.rs"], 1, false)).toBeUndefined();
});

test("a difference scripts/crater-decided.txt lists is decided, and any other is a wrong answer", () => {
  const decided = decidedOf("# a comment\nnum-traits float::tests::copysign  a NaN's sign isn't kept\n");
  expect([...decided]).toEqual([["num-traits float::tests::copysign", "a NaN's sign isn't kept"]]);
  const differ = [
    { name: "float::tests::copysign", js: "failed" },
    { name: "float::tests::other", js: "failed" },
  ];
  expect(undecided("num-traits", differ, decided)).toEqual([{ name: "float::tests::other", js: "failed" }]);
  expect(undecided("other", differ, decided)).toEqual(differ);
});

// uuid's `#[test]` above an attribute of several lines, `#[cfg_attr(..)]`:
// one test, its attributes from its `#[test]`, so a cut of it is the test's
// and says which (#1121).
test("a test's attributes of several lines are its own, its #[test] above them", () => {
  const source = "mod tests {\n    #[test]\n    #[cfg_attr(\n        all(target_arch = \"wasm32\"),\n        wasm_bindgen_test\n    )]\n    #[cfg(feature = \"std\")]\n    fn test_compare() {\n        let n = [1, 2];\n    }\n}\n";
  expect(attributesFrom(source.split("\n"), 8)).toBe(2);
  expect(cutFor("src/lib.rs", source, 9)).toEqual({ from: 2, test: "tests::test_compare" });
  // A line of code ending `]` above is no attribute of its.
  expect(attributesFrom(["let a = [", "    1]", "fn f() {}"], 3)).toBe(3);
});

// A module built only because of `test` on wasm32, chrono's tz_data,
// `#[cfg(all(any(target_os = "android", target_env = "ohos", test),
// feature = "clock"))]`, is test-only, as `#[cfg(test)]`'s is: its
// cfg false without `test` whatever the features are (#1153).
test("a cfg true on wasm32 only because of test is test-only, features unknown", () => {
  expect(onlyForTests('cfg(all(any(target_os = "android", target_env = "ohos", test), feature = "clock"))')).toBe(true);
  expect(onlyForTests("cfg(test)")).toBe(true);
  expect(onlyForTests('cfg(any(test, feature = "std"))')).toBe(false);
  expect(onlyForTests('cfg(target_arch = "wasm32")')).toBe(false);
  expect(onlyForTests("cfg(unix)")).toBe(false);
  expect(onlyForTests('cfg(all(test, not(feature = "x")))')).toBe(true);
  expect(onlyForTests("cfg(not(test))")).toBe(false);
});

// A native test of a module a cfg leaves off wasm32, chrono's `#[cfg(unix)]
// mod tz_info;`, is none of rust-js's target, counted apart, never as one
// that differs (#1153).
test("a test of a module a cfg leaves off wasm32 is of no test of its", () => {
  const files: Record<string, string> = {
    "src/lib.rs": "pub mod offset;\n",
    "src/offset.rs": "#[cfg(unix)]\nmod tz_info;\nmod fixed;\n",
    "src/offset/tz_info.rs": "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    "src/offset/fixed.rs": "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
  };
  const read = (path: string) => files[path];
  expect(offTarget("offset::tz_info::tests::t", read)).toBe(true);
  expect(offTarget("offset::fixed::tests::t", read)).toBe(false);
});

// Each integration test's file is built by itself as JS, its outcomes keyed
// by its file, as `cargo test --lib --tests` heads each target's.
test("each test binary is run by itself, an integration test's outcomes keyed by its file", () => {
  const said = [
    "  Executable unittests src/lib.rs (out/debug/deps/semver-1)",
    "  Executable tests/test_version.rs (/abs/out/debug/deps/test_version-2)",
    "  Executable unittests src/main.rs (out/debug/deps/bin-3)",
    "     Running tests/compiletest.rs (out/debug/deps/compiletest-4)",
  ].join("\n");
  expect(executables(said)).toEqual([
    { target: "src/lib.rs", path: "out/debug/deps/semver-1" },
    { target: "tests/test_version.rs", path: "/abs/out/debug/deps/test_version-2" },
    { target: "src/main.rs", path: "out/debug/deps/bin-3" },
  ]);
  expect(integrationFiles({ "a::t1": "ok", "tests/test_version.rs::t1": "ok", "tests/test_version.rs::t2": "failed" })).toEqual(["tests/test_version.rs"]);
  expect(testModulePath("tests/test_version.rs", "tests/test_version.rs")).toEqual([]);
  expect(testModulePath("tests/util/mod.rs", "tests/test_version.rs")).toEqual(["util"]);
  expect(cutFor("tests/util/mod.rs", "pub fn helper() {\n    1;\n}\n", 2, true, ["util"])).toEqual({ from: 1 });
  expect(cutFor("tests/test_version.rs", "#[test]\nfn t1() {\n    1;\n}\n", 3, true, [])).toEqual({ from: 1, test: "t1" });
});

test("an integration test's file is built by itself, none of its library's tests, and none of no harness", () => {
  const manifest =
    '[package]\nname = "semver"\nautotests = false\n\n[lib]\nname = "semver"\npath = "src/lib.rs"\n\n[[test]]\nname = "test_a"\npath = "tests/test_a.rs"\n\n[[test]]\nname = "test_b"\npath = "tests/test_b.rs"\n\n[[test]]\nname = "ui"\npath = "tests/ui.rs"\nharness = false\n\n[dev-dependencies.criterion]\nversion = "0.4"\n\n';
  expect(oneIntegrationTest(manifest, "tests/test_b.rs", (lib) => lib !== "criterion")).toBe(
    '[package]\nautotests = false\nname = "semver"\n\n[lib]\ntest = false\nname = "semver"\npath = "src/lib.rs"\n\n[[test]]\nname = "test_b"\npath = "tests/test_b.rs"\n\n',
  );
  expect(oneIntegrationTest(manifest, "tests/ui.rs")).toBeUndefined();
});

test("the JS's copy turns on getrandom's WebAssembly feature of each release its lock has", () => {
  const manifest = '[package]\nname = "base64"\n\n[dev-dependencies.rand]\nversion = "0.8"\n';
  const lock = '[[package]]\nname = "getrandom"\nversion = "0.2.12"\n\n[[package]]\nname = "getrandom"\nversion = "0.3.4"\n\n[[package]]\nname = "getrandom"\nversion = "0.1.16"\n';
  expect(forTheJs(manifest, lock)).toBe(
    manifest +
      '[target.\'cfg(target_arch = "wasm32")\'.dev-dependencies.crater-getrandom-0-2]\npackage = "getrandom"\nversion = "0.2"\nfeatures = ["js"]\n\n' +
      '[target.\'cfg(target_arch = "wasm32")\'.dev-dependencies.crater-getrandom-0-3]\npackage = "getrandom"\nversion = "0.3"\nfeatures = ["wasm_js"]\n\n',
  );
  expect(forTheJs(manifest, "")).toBe(manifest);
});

// chrono's tests/dateutils.rs, `#![cfg(all(unix, feature = "clock", feature = "std"))]`:
// an integration test's file its own cfg leaves off wasm32 is of no test of its.
test("an integration test's file its own cfg leaves off wasm32 is of no test of its", () => {
  const text = '//! Its doc.\n#![cfg(all(unix, feature = "clock", feature = "std"))]\n\nuse chrono::Local;\n';
  expect(innerAttributes(text)).toBe('#![cfg(all(unix, feature = "clock", feature = "std"))]');
  const read = (files: Record<string, string>) => (path: string) => files[path];
  expect(offTarget("tests/dateutils.rs::try_verify", read({ "tests/dateutils.rs": text }))).toBe(true);
  expect(offTarget("tests/other.rs::t", read({ "tests/other.rs": '#![cfg(feature = "std")]\nfn a() {}\n' }))).toBe(false);
});
