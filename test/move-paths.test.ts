// scripts/move-paths.ts's Rust paths: a module moved to another of the
// compiler's crates is named by that crate's path everywhere its old
// crate's root named it, `crate::collections` as `rust_js_program::
// collections` (ADR 1181), however it was written.
import { expect, test } from "bun:test";
import { binaryCrate, compilerCrates, type CompilerCrate } from "../scripts/compiler-crates";
import { rustPathsRewritten } from "../scripts/move-paths";

const crates = compilerCrates();
const binary = binaryCrate();
const program = crates.find((crate) => crate.package === "rust-js-program")!;
// A layout of the test's own: `collections` and `js` of the program's
// crate, `lower` and `output` of the binary's.
const modulesOf = new Map<string, CompilerCrate>([
  ["collections", program],
  ["js", program],
  ["lower", binary],
  ["output", binary],
]);
const rewrite = (file: string, text: string) => rustPathsRewritten(file, text, modulesOf);

test("a path through the root to a moved module is its new crate's", () => {
  expect(rewrite("src/lower/a.rs", "fn f(s: &crate::collections::IndexSet<u32>) -> crate::lower::Body {}\n").text).toBe(
    "fn f(s: &rust_js_program::collections::IndexSet<u32>) -> crate::lower::Body {}\n",
  );
  // A longer name it begins is another module.
  expect(rewrite("src/lower/a.rs", "use crate::collections_of::X;\n").text).toBe("use crate::collections_of::X;\n");
});

test("the `super`s that reach the root are its path, and no fewer", () => {
  // `src/lower/a/b.rs` is `lower::a::b`: three `super`s reach the root.
  expect(rewrite("src/lower/a/b.rs", "use super::super::super::js::Expr;\nuse super::super::js::Other;\n").text).toBe(
    "use rust_js_program::js::Expr;\nuse super::super::js::Other;\n",
  );
  // In an inline module, one more.
  expect(rewrite("src/output.rs", "mod tests {\n    use super::super::js::Expr;\n}\nuse super::js::Expr;\n").text).toBe(
    "mod tests {\n    use rust_js_program::js::Expr;\n}\nuse rust_js_program::js::Expr;\n",
  );
});

test("a use tree's group is split where some of it moved", () => {
  const { text } = rewrite("src/lower/a.rs", "pub(crate) use crate::{\n    collections::{IndexMap, IndexSet},\n    js::{self, Expr},\n    lower::Body,\n};\n");
  expect(text).toBe("pub(crate) use crate::lower::Body;\npub(crate) use rust_js_program::{collections::{IndexMap, IndexSet}, js::{self, Expr}};\n");
  // All of it moved: no group left of the old crate's; `self` of the root stays.
  expect(rewrite("src/lower/a.rs", "use crate::{collections::IndexMap};\n").text).toBe("use rust_js_program::collections::IndexMap;\n");
  expect(rewrite("src/lower/a.rs", "use crate::{self as root, js};\n").text).toBe("use crate::{self as root};\nuse rust_js_program::js;\n");
  // Its attributes go with each part.
  expect(rewrite("src/lower/a.rs", "#[cfg(test)]\nuse crate::{js::Expr, lower::Body};\n").text).toBe("#[cfg(test)]\nuse crate::lower::Body;\n#[cfg(test)]\nuse rust_js_program::js::Expr;\n");
  // A comment in it isn't lost: the tree is left, and said.
  const commented = rewrite("src/lower/a.rs", "use crate::{\n    // the maps\n    collections::IndexMap,\n    lower::Body,\n};\n");
  expect(commented.problems).toEqual(["src/lower/a.rs:1: a comment in a use tree that splits, left as it is"]);
});

test("the root names its modules bare, but not a later part of a path", () => {
  expect(rewrite("src/main.rs", "use std::{collections::HashMap};\nfn main() { js::f(); a::js::g(); }\nmod tests {\n    fn t() { js::h(); }\n}\n").text).toBe(
    "use std::{collections::HashMap};\nfn main() { rust_js_program::js::f(); a::js::g(); }\nmod tests {\n    fn t() { js::h(); }\n}\n",
  );
});

test("a comment's path is rewritten, a string's isn't", () => {
  expect(rewrite("src/lower/a.rs", '/// See [crate::js::Expr].\nconst S: &str = "crate::js::Expr";\n').text).toBe(
    '/// See [rust_js_program::js::Expr].\nconst S: &str = "crate::js::Expr";\n',
  );
});

test("a crate's path to the binary's module is said, never written", () => {
  const { text, problems } = rewrite("crates/rust-js-program/src/js.rs", "use crate::collections::IndexMap;\nuse crate::lower::Body;\n");
  // Its own crate's module stays `crate::`.
  expect(text).toBe("use crate::collections::IndexMap;\nuse crate::lower::Body;\n");
  expect(problems).toEqual(["crates/rust-js-program/src/js.rs:2 names the binary's `lower` from rust-js-program, which no crate can: none depends on the binary"]);
});
