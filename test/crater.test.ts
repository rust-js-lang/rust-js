// The small Crater's reading of what Cargo, Node's runner and rust-js say,
// and its cut of a refused test from the JS's copy of a crate (#1087,
// scripts/crater.ts). The net itself runs each night.
import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { boundCrater, cutFirst, decidedOf, differences, firstError, executables, forTheJs, integrationFiles, jsOutcomes, inShard, judged, knownOf, libraryTestsOnly, listedButSame, merged, nativeOutcomes, oneIntegrationTest, ranked, reasonOf, said, undecided } from "../scripts/crater";
import { attributesFrom, cut, cutFor, declaredIn, dropped, errorsAt, innerAttributes, items, modulePath, offTarget, onlyForTests, placeOf, testModulePath, testOnlyFile, underItems } from "../scripts/crater-cuts";

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
  const verdict = judged({ a: "ok", b: "ok", c: "failed" }, { a: "ok" }, { b: "refused", c: "refused" }, "/nonexistent");
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
  const verdict = judged({ "tests::a": "ok", "tests::b": "ok", "tests::d": "ok" }, { "tests::d": "ok" }, {}, "/nonexistent");
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
