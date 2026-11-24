// The small Crater's reading of what Cargo, Node's runner and rust-js say,
// and its cut of a refused test from the JS's copy of a crate (#1087,
// scripts/crater.ts). The net itself runs each night.
import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterDeaths, afterTimeout, bothRuns, boundCrater, cascaded, cutFirst, decidedOf, differences, firstError, executables, forTheJs, integrationFiles, jsOutcomes, inShard, judged, knownOf, libraryTestsOnly, listedButSame, merged, nativeOutcomes, oneIntegrationTest, ranked, reasonOf, said, testsMadeBy, undecided } from "../scripts/crater";
import { attributesFrom, cut, cutFor, declaredIn, dropped, errorsAt, followed, innerAttributes, items, modulePath, offTarget, onlyForTests, placeOf, testModulePath, testOnlyFile, testsOfExpanded, underItems } from "../scripts/crater-cuts";

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
  const verdict = judged({ "a::refused": "ok", "a::wrong": "ok", "a::ok": "ok" }, js, {}, "/nonexistent", new Map());
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
  expect(cutFor("src/a.rs", source, 10)).toEqual({ from: 9, item: "a::tests::helper" });
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
  expect(cutFor("src/tests/iter.rs", files["src/tests/iter.rs"], 2, true)).toEqual({ from: 1, item: "tests::iter::helper" });
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
  expect(cutFor("tests/util/mod.rs", "pub fn helper() {\n    1;\n}\n", 2, true, ["util"])).toEqual({ from: 1, item: "util::helper" });
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
      '[target.\'cfg(target_arch = "wasm32")\'.dev-dependencies.crater-getrandom-0-3]\npackage = "getrandom"\nversion = "0.3"\nfeatures = ["wasm_js"]\n\n' +
      '[target.\'cfg(target_arch = "wasm32")\'.dev-dependencies.crater-getrandom-0-1]\npackage = "getrandom"\nversion = "0.1"\nfeatures = ["wasm-bindgen"]\n\n',
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
  // A test a cfg of its own leaves off, unicode-ident's 64-bit one, and
  // one beside it that's on.
  const sized = '#[cfg(target_pointer_width = "64")]\n#[test]\nfn test_trieset_size() {}\n\n#[test]\nfn test_size() {}\n';
  expect(offTarget("tests/static_size.rs::test_trieset_size", read({ "tests/static_size.rs": sized }))).toBe(true);
  expect(offTarget("tests/static_size.rs::test_size", read({ "tests/static_size.rs": sized }))).toBe(false);
  const inline = 'mod tests {\n    #[cfg(target_pointer_width = "64")]\n    #[test]\n    fn wide() {}\n    #[test]\n    fn any() {}\n}\n';
  expect(offTarget("tests::wide", read({ "src/lib.rs": inline }))).toBe(true);
  expect(offTarget("tests::any", read({ "src/lib.rs": inline }))).toBe(false);
});

// What stops the crates' tests, ranked by the crates each reason stops, then
// the native tests: a cut test, one refused at run time, every test of a
// library refused, and a crate that couldn't be asked (#1174).
test("each refusal's reason is ranked by the crates and then the native tests it stops", () => {
  expect(reasonOf("rust-js does not support `io::Read` (src/de.rs:12)")).toBe("rust-js does not support `io::Read`");
  const verdicts = new Map<string, Parameters<typeof said>[1]>([
    ["a", { native: { t1: "ok", t2: "ok", t3: "failed" }, js: { t1: "ok" }, cut: { t2: "rust-js does not support `io::Read`", t3: "rust-js does not support `io::Read`" }, differ: [] }],
    ["b", { native: { u1: "ok", u2: "ok", u3: "ignored" }, refused: ["rust-js does not support `io::Read` (src/lib.rs:4)", "rust-js does not support `thread::spawn` (src/lib.rs:9)"] }],
    ["c", { native: { v1: "ok" }, js: {}, cut: {}, differ: [], refusedAtRun: [{ name: "v1", why: "rust-js does not support a byte view" }] }],
    ["d", { problem: "its tests don't run natively: x" }],
  ]);
  expect(ranked(verdicts)).toEqual([
    { why: "rust-js does not support `io::Read`", crates: ["a", "b"], tests: 3 },
    { why: "rust-js does not support `thread::spawn`", crates: ["b"], tests: 2 },
    { why: "at run time: rust-js does not support a byte view", crates: ["c"], tests: 1 },
    { why: "not asked: its tests don't run natively: x", crates: ["d"], tests: 0 },
  ]);
});

// base64's `src/tests.rs`, `#[cfg(test)] mod tests;`, has an impl of
// `Distribution` for its tests; a refusal in it is its tests', never the
// library's. An array type's `;` in a signature ends no item (#1174).
test("an impl only tests build is cut as a function is, and an array type's `;` ends no item", () => {
  const tests = [
    "use super::*;",
    "",
    "impl Distribution<Mode> for Standard {",
    "    fn sample<R: Rng>(&self, rng: &mut R) -> Mode {",
    "        Mode::A",
    "    }",
    "}",
    "",
    "fn takes(bytes: [u8; 4], f: impl Fn() -> u8) -> impl Iterator<Item = u8> {",
    "    bytes.into_iter()",
    "}",
  ].join("\n");
  expect(cutFor("src/tests.rs", tests, 5, true)).toEqual({ from: 3 });
  expect(cutFor("src/tests.rs", tests, 10, true)).toEqual({ from: 9, item: "tests::takes" });
  expect(cutFor("src/lib.rs", tests, 5)).toBeUndefined();
  expect(items(tests).map((item) => [item.kind, item.name, item.from, item.to])).toEqual([
    ["fn", "sample", 4, 6],
    ["impl", "", 3, 7],
    ["fn", "takes", 9, 11],
  ]);
});

// base64's test files `use crate::tests::random_config;`, which an earlier
// round cut; the `use` is cut, never the library, and a test cut that
// didn't pass natively isn't counted, proc-macro2's (#1174).
test("a `use` only tests build is cut, and only tests that passed natively count as cut", () => {
  const tests = ["use super::*;", "use crate::{", "    alphabet,", "    tests::{random_config, random_engine},", "};", "", "fn helper() {}"].join("\n");
  expect(cutFor("src/read/decoder_tests.rs", tests, 3, true)).toEqual({ from: 2 });
  expect(cutFor("src/read/decoder_tests.rs", tests, 1, true)).toEqual({ from: 1 });
  // Of a `use` of several names, the one unresolved alone, the rest kept.
  const unresolved = cutFor("src/read/decoder_tests.rs", tests, 4, true, ["read", "decoder_tests"], "unresolved import `crate::tests::random_config`");
  expect(unresolved).toEqual({ from: 4, drop: ["random_config"] });
  // Several of one `use`, rustc's one error of them: those of its line.
  const both = "unresolved imports `crate::tests::random_config`, `crate::tests::random_engine`, `crate::other`";
  expect(cutFor("src/read/decoder_tests.rs", tests, 4, true, ["read", "decoder_tests"], both)).toEqual({ from: 4, drop: ["random_config", "random_engine"] });
  expect(dropped(tests, 4, "random_config").split("\n")[3]).toBe("    tests::{random_engine},");
  expect(dropped(tests, 3, "alphabet").split("\n")[2]).toBe("    ");
  expect(cutFor("src/lib.rs", tests, 3)).toBeUndefined();
  const inModule = ["pub fn f() {}", "", "#[cfg(test)]", "mod tests {", "    use crate::gone;", "}"].join("\n");
  expect(cutFor("src/lib.rs", inModule, 5)).toEqual({ from: 5 });
  const crate = { name: "c", version: "1.0.0" } as Parameters<typeof said>[0];
  const verdict = judged({ a: "ok", b: "ok", c: "failed" }, { a: "ok" }, { b: "refused", c: "refused" }, "/nonexistent", new Map());
  expect(said(crate, verdict)).toBe("c 1.0.0: 1 of 2 as native, 1 refused");
});

// A wrong answer known, listed under its open issue, is shown as known and
// fails no check; and a listed test that runs as native fails `--check`, so
// the list only shrinks (#1174, strsim's under #1264).
test("a difference listed under an open issue is a known wrong answer, and a listed test that runs as native is to take off", () => {
  const decided = decidedOf("c tests::a  #1264 zip in generic code\nc tests::b  a NaN's sign isn't kept\nc tests::d  #1264 zip\n");
  const differ = [
    { name: "tests::a", js: "failed" },
    { name: "tests::b", js: "failed" },
  ];
  expect(knownOf("c", differ, decided)).toEqual([{ name: "tests::a", js: "failed" }]);
  expect(undecided("c", differ, decided)).toEqual([]);
  const crate = { name: "c", version: "1.0.0" } as Parameters<typeof said>[0];
  const verdict = judged({ "tests::a": "ok", "tests::b": "ok", "tests::d": "ok" }, { "tests::d": "ok" }, {}, "/nonexistent", new Map());
  expect(said(crate, verdict, decided)).toBe("c 1.0.0: 1 of 3 as native, 0 refused, 1 decided, 1 WRONG known (#1264)");
  expect(listedButSame("c", verdict, decided)).toEqual(["tests::d"]);
});

// The nightly's Crater in shards, each every n-th crate, merged as one run,
// a shard missing or twice refused (#1174).
test("the Crater's shards are every n-th crate, and merge as one run only whole", () => {
  expect(inShard([1, 2, 3, 4, 5], 2, 2)).toEqual([2, 4]);
  expect(inShard([1, 2, 3, 4, 5], 1, 1)).toEqual([1, 2, 3, 4, 5]);
  const verdict = { problem: "x" };
  const all = merged([{ shard: 2, of: 2, verdicts: { b: verdict } }, { shard: 1, of: 2, verdicts: { a: verdict } }]);
  expect(all instanceof Map && [...all.keys()].sort()).toEqual(["a", "b"]);
  expect(merged([{ shard: 1, of: 2, verdicts: {} }])).toEqual({ problem: "shards 1 of 2: each of 1, 2 once" });
  expect(merged([{ shard: 1, of: 2, verdicts: {} }, { shard: 1, of: 2, verdicts: {} }])).toEqual({ problem: "shards 1, 1 of 2: each of 1, 2 once" });
  expect(merged([])).toEqual({ problem: "there are no shards" });
});

// rstest's `#[apply(all_engines)] fn f`, no `#[test]`, makes tests under
// its path, `engine::tests::f::case_1`: cut with it, never read as not run
// (#1174, base64's).
test("a function only tests reach is cut with the tests a macro made of it", () => {
  const tests = ["#[apply(all_engines)]", "fn decode<E: EngineWrapper>(engine: E) {", "    refused();", "}"].join("\n");
  expect(cutFor("src/engine/tests.rs", tests, 3, true)).toEqual({ from: 1, item: "engine::tests::decode" });
  expect(underItems(["engine::tests::decode::case_1", "engine::tests::decoder", "engine::tests::other"], { "engine::tests::decode": "refused" })).toEqual({ "engine::tests::decode::case_1": "refused" });
});

// typenum's `uint::tests::binary` hangs rust-js's layout (#1269): the list
// cuts it before its JS is built, found by its path in its source (#1174).
test("a test the list cuts first is found by its path and never asked", () => {
  const decided = decidedOf("typenum uint::tests::binary  cut: #1269 a layout of half an hour\ntypenum uint::tests::other  #1264 zip\n");
  expect(cutFirst("typenum", decided)).toEqual({ "uint::tests::binary": "#1269 a layout of half an hour" });
  expect(cutFirst("other", decided)).toEqual({});
  const files: Record<string, string> = {
    "src/lib.rs": "mod uint;\n",
    "src/uint.rs": "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn binary() {}\n}\n",
    "tests/zip.rs": "mod inner {\n    #[test]\n    fn t() {}\n}\n",
  };
  const read = (path: string) => files[path];
  expect(placeOf("uint::tests::binary", read)).toEqual({ file: "src/uint.rs", from: 5 });
  expect(placeOf("uint::tests::gone", read)).toBeUndefined();
  expect(placeOf("tests/zip.rs::inner::t", read)).toEqual({ file: "tests/zip.rs", from: 2 });
});

// tinyvec's feature `real_blackbox = ["criterion/real_blackbox"]` names a
// dev-dependency its source doesn't: kept, or Cargo can't read the
// manifest (#1174).
test("a dev-dependency a feature names is kept for the library's tests", () => {
  const manifest = '[package]\nname = "tinyvec"\n\n[features]\nreal_blackbox = ["criterion/real_blackbox"]\nserde = ["dep:serde"]\n\n[dev-dependencies.criterion]\nversion = "0.3"\n\n[dev-dependencies.smallvec]\nversion = "1"\n';
  expect(libraryTestsOnly(manifest, () => false)).toBe('[package]\nautotests = false\nname = "tinyvec"\n\n[features]\nreal_blackbox = ["criterion/real_blackbox"]\nserde = ["dep:serde"]\n\n[dev-dependencies.criterion]\nversion = "0.3"\n\n');
});

// A build that failed is said by its first error, not Cargo's last words
// (#1174, futures-util's said only a note).
test("a failed build is said by its first error", () => {
  expect(firstError("   Checking a v1\nerror[E0432]: unresolved import `x`\n  --> src/lib.rs:3:5\nerror: aborting due to 1 previous error\nwarning: build failed, waiting for other jobs to finish...\nnote: this project has React 19.3.0\n")).toBe("error[E0432]: unresolved import `x` --> src/lib.rs:3:5");
  expect(firstError("a\nb\nc\nd\n")).toBe("b c d");
});

// The Crater's builds, 13 GiB of 100 crates' native tests, held to their cap
// with the disk's other stores; its verdicts never (#1174, #1213).
test("the Crater's store is held to its cap, its kept verdicts never", () => {
  const dir = mkdtempSync(join(tmpdir(), "crater-store-"));
  for (const name of ["kept", "native", "a-1.0.0-js"]) {
    mkdirSync(join(dir, name));
    writeFileSync(join(dir, name, "f"), "x".repeat(64 * 1024));
  }
  const held = boundCrater({ dir, cap: 0, free: () => Number.POSITIVE_INFINITY });
  expect(held.removed).toBe(2);
  expect(readdirSync(dir)).toEqual(["kept"]);
  rmSync(dir, { recursive: true, force: true });
});

// A native test a cut took with it, byteorder's macro-made tests of an
// `impl Arbitrary` cut a round before, is refused for the cut nearest it by
// module path: one its build's cut copy, expanded, no longer holds. One the
// copy still holds was built and lost, a difference, as is one of a build
// not expanded; a dead file changes neither (#1420).
test("a test a cut took with it is refused for that cut, of what its cut copy's expansion no longer holds", () => {
  const native = { "a::b": "ok", "stdtests::prop_i16::big": "ok", "other::c": "ok", "tests/t.rs::gone": "ok", "tests/t.rs::lost": "ok", "tests/u.rs::x": "ok" } as const;
  const js = { "a::b": "ok" } as const;
  const cut = { "stdtests::big": "user impl of Arbitrary", "a::z": "another", "tests/t.rs::helper_user": "refused in t" };
  const held = new Map([
    ["", new Set(["a::b", "other::c"])],
    ["tests/t.rs::", new Set(["tests/t.rs::lost"])],
  ]);
  const taken = { ...cut, "stdtests::prop_i16::big": "user impl of Arbitrary", "tests/t.rs::gone": "refused in t" };
  expect(cascaded(native, js, cut, held)).toEqual(taken);
  expect(cascaded(native, { ...js, "/t/lib.test.js": "failed" }, cut, held)).toEqual(taken);
  expect(cascaded(native, js, {}, held)).toEqual({});
});

// A test file whose process died: each test it hadn't reported run again
// alone, one that dies again failed, the rest their own outcomes.
test("a dead test file's remaining tests run again alone", async () => {
  const dir = mkdtempSync(join(tmpdir(), "crater-dead-"));
  const file = join(dir, "lib.test.js");
  writeFileSync(file, `import { test } from "node:test";
test("a::first", () => {});
test("a::dies", () => { process.exit(9); });
test(
  "a::after",
  () => {},
);
test("a::fails", () => { throw new Error("no"); });
`);
  const outcomes = await afterDeaths({ "a::first": "ok", [file]: "failed" }, dir, 60);
  expect(outcomes).toEqual({ "a::first": "ok", "a::dies": "failed", "a::after": "ok", "a::fails": "failed" });
  rmSync(dir, { recursive: true, force: true });
});

// A run past its budget printed only what ended before it: itertools'
// quick.rs ran past its 1,200 s, its first property still running, and all
// 92 of its tests were "not run" (#2460). Each test of a file the run wrote
// that said nothing runs again alone: one that ends is its own outcome; one
// still running at its own budget, a synchronous loop Node's
// `--test-timeout` never stops, timed out. A file written before the run,
// another build's, isn't run again. And each is given what both runs are,
// quickcheck's cases, which the file's own process reads, not the suite.
test("a run past its budget runs each test it didn't say again alone, one past its own timed out", async () => {
  const dir = mkdtempSync(join(tmpdir(), "crater-late-"));
  const built = join(dir, "target", "wasm32-unknown-unknown", "debug", "rust-js", "c-1");
  mkdirSync(built, { recursive: true });
  const older = join(built, "other.test.js");
  writeFileSync(older, `import { test } from "node:test";\ntest("o::untouched", () => {});\n`);
  utimesSync(older, new Date(0), new Date(0));
  const since = Date.now() - 1000;
  writeFileSync(join(built, "quick.test.js"), `import { test } from "node:test";
test("q::spins", () => { for (;;) {} });
test("q::slow", () => { const end = Date.now() + 300; while (Date.now() < end) {} });
test("q::said", () => {});
test("q::cases", () => { const { QUICKCHECK_TESTS: cases } = process.env; if (cases !== ${JSON.stringify(bothRuns.QUICKCHECK_TESTS)}) throw new Error("no"); });
`);
  const outcomes = await afterTimeout({ "q::said": "ok" }, join(dir, "target"), since, dir, 3);
  expect(outcomes).toEqual({ "q::said": "ok", "q::spins": "timed out: after 3 s, run alone", "q::slow": "ok", "q::cases": "ok" });
  rmSync(dir, { recursive: true, force: true });
});

// libtest says a `#[should_panic]` test's note after its name: still a
// test, byteorder's 288 of them (#1431).
test("a should-panic test is a native outcome", () => {
  expect(nativeOutcomes("test a::plain ... ok\ntest a::panics - should panic ... ok\ntest a::bad - should panic ... FAILED\n")).toEqual({
    "a::plain": "ok",
    "a::panics": "ok",
    "a::bad": "failed",
  });
});

// itertools' `generic_test!(k_smallest_sort, u8, ..)`, a macro invoked where
// an item goes, makes the tests of a helper an earlier round cut: the
// invocation is an item, cut as a function is where only tests build it; a
// function in one, quickcheck!'s, is still cut alone; no `macro_rules!` and
// no invocation in a function body is one (#1420).
test("a macro's invocation where an item goes is an item, cut where only tests build it", () => {
  const invoking = [
    "macro_rules! generic_test {",
    "    ($f:ident) => { fn x() {} };",
    "}",
    "",
    "#[cfg(not(miri))]",
    "generic_test!(k_smallest_sort, u8);",
    "qc::quickcheck! {",
    "    fn prop(a: u8) -> bool {",
    "        true",
    "    }",
    "}",
    "fn f() {",
    "    vec![1];",
    '    println!("{}", 1);',
    "}",
    "",
  ].join("\n");
  expect(items(invoking).map(({ kind, name, from, to }) => [kind, name, from, to])).toEqual([
    ["fn", "x", 2, 2],
    ["macro", "generic_test", 5, 6],
    ["fn", "prop", 8, 10],
    ["macro", "qc::quickcheck", 7, 11],
    ["fn", "f", 12, 15],
  ]);
  expect(cutFor("tests/test_std.rs", invoking, 6, true, [])).toEqual({ from: 5, invocation: true });
  expect(cutFor("tests/test_std.rs", invoking, 9, true, [])).toEqual({ from: 8, item: "prop" });
  expect(cutFor("src/lib.rs", invoking, 6)).toBeUndefined();
});

// The tests an invocation makes are known exactly, of rustc's expansion
// without its cut and with it, even of a crate whose helper is cut and so
// doesn't resolve; never by its names, which `k_smallest_sort_alone` shares
// and no invocation made (#1420).
test("the tests a cut macro invocation made are those the expansion loses with its cut", async () => {
  const dir = mkdtempSync(join(tmpdir(), "crater-expanded-"));
  mkdirSync(join(dir, "tests"));
  const file = [
    "macro_rules! generic_test {",
    "    ($f:ident, $($t:ident),+) => {",
    "        mod $f {",
    "            $(",
    "                #[test]",
    "                fn $t() {",
    "                    super::$f::<$t>();",
    "                }",
    "            )+",
    "        }",
    "    };",
    "}",
    "",
    "#[cfg(not(miri))]",
    "generic_test!(k_smallest_sort, u8, i8);",
    "",
    "#[test]",
    "fn k_smallest_sort_alone() {}",
    "",
  ].join("\n");
  writeFileSync(join(dir, "tests/t.rs"), file);
  const at = cutFor("tests/t.rs", file, 15, true, []);
  expect(at).toEqual({ from: 14, invocation: true });
  const expand = async () => {
    const out = join(dir, "expanded.rs");
    rmSync(out, { force: true });
    spawnSync("rustc", ["--edition=2021", "--test", "-Zunpretty=expanded", join(dir, "tests/t.rs"), "-o", out], { env: { ...process.env, RUSTC_BOOTSTRAP: "1" } });
    return { tests: testsOfExpanded(readFileSync(out, "utf8")) };
  };
  expect([...(await expand()).tests].sort()).toEqual(["k_smallest_sort::i8", "k_smallest_sort::u8", "k_smallest_sort_alone"]);
  // rustc's printer puts a long path's name on a line of its own.
  expect([...testsOfExpanded('    #[rustc_test_marker =\n    "simd::neon::neon_code_matches_header_value_chars_table"]\n    pub const x: () = ();\n    #[rustc_test_marker = "a::b"]\n')]).toEqual(["simd::neon::neon_code_matches_header_value_chars_table", "a::b"]);
  const why = "rust-js does not support it; its cut left: cannot find function `k_smallest_sort` in this scope";
  expect(await testsMadeBy(dir, [{ file: "tests/t.rs", from: at!.from, why }], expand)).toEqual({ "k_smallest_sort::u8": why, "k_smallest_sort::i8": why });
  expect(readFileSync(join(dir, "tests/t.rs"), "utf8")).toBe(file);
  rmSync(dir, { recursive: true, force: true });
});

// A test whose helper an earlier round cut is refused for the helper's own
// reason, not for rustc's "cannot find" of it, which is kept as a note the
// ranking leaves off: the ranking names what refused (#1420).
test("a cut for a function an earlier round cut is refused for that function's reason", () => {
  const items = { k_smallest_sort: "rust-js does not support X", "a::h": "first", "b::h": "second" };
  const lost = "cannot find function `k_smallest_sort` in this scope";
  expect(followed(lost, items, [])).toBe(`rust-js does not support X; its cut left: ${lost}`);
  expect(followed("cannot find function `h` in this scope", items, ["b", "c"])).toBe("second; its cut left: cannot find function `h` in this scope");
  expect(followed("cannot find value `h` in this scope", items, ["a"])).toBe("first; its cut left: cannot find value `h` in this scope");
  expect(followed("mismatched types", items, [])).toBe("mismatched types");
  expect(followed("cannot find function `g` in this scope", items, [])).toBe("cannot find function `g` in this scope");
  // A chain: the root's reason, the round's own error after it.
  expect(followed("cannot find function `g` in this scope", { g: `root; its cut left: ${lost}` }, [])).toBe("root; its cut left: cannot find function `g` in this scope");
  expect(reasonOf(`root; its cut left: ${lost}`)).toBe("root");
});
