// The small Crater's reading of what Cargo, Node's runner and rust-js say,
// and its cut of a refused test from the JS's copy of a crate (#1087,
// scripts/crater.ts). The net itself runs each night.
import { expect, test } from "bun:test";
import { cut, cutFor, decidedOf, declaredIn, differences, errorsAt, items, jsOutcomes, libraryTestsOnly, modulePath, nativeOutcomes, testOnlyFile, undecided } from "../scripts/crater";

test("each test's outcome is read of cargo test's lines and of Node's runner's", () => {
  expect(nativeOutcomes("running 3 tests\ntest a::t1 ... ok\ntest a::t2 ... FAILED\ntest t3 ... ignored, slow\n")).toEqual({
    "a::t1": "ok",
    "a::t2": "failed",
    t3: "ignored",
  });
  expect(jsOutcomes("✔ a::t1 (0.1ms)\n✖ a::t2 (1ms)\n﹣ t3 (0ms) # SKIP\nℹ tests 3\n")).toEqual({ "a::t1": "ok", "a::t2": "failed", t3: "ignored" });
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
