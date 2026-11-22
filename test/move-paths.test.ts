// scripts/move-paths.ts's Rust paths: a module moved to another of the
// compiler's crates is named by that crate's path everywhere its old
// crate's root named it, `crate::collections` as `rust_js_program::
// collections` (ADR 1181), however it was written.
import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { binaryCrate, compilerCrates, type CompilerCrate } from "../scripts/compiler-crates";
import { boundNames, codeOf, movedModules, privateSites, rewritten, rustPathsRewritten, segmented, statementLine, textFiles } from "../scripts/move-paths";

const crates = compilerCrates();
const binary = binaryCrate();
const program = crates.find((crate) => crate.package === "rust-js-program")!;
// A layout of the test's own: `collections` and `js` of the program's
// crate, `pipeline` and `output` of the binary's.
const modulesOf = new Map<string, CompilerCrate>([
  ["collections", program],
  ["js", program],
  ["pipeline", binary],
  ["output", binary],
]);
const rewrite = (file: string, text: string) => rustPathsRewritten(file, text, modulesOf);

test("a path through the root to a moved module is its new crate's", () => {
  expect(rewrite("src/pipeline/a.rs", "fn f(s: &crate::collections::IndexSet<u32>) -> crate::pipeline::Body {}\n").text).toBe(
    "fn f(s: &rust_js_program::collections::IndexSet<u32>) -> crate::pipeline::Body {}\n",
  );
  // A longer name it begins is another module.
  expect(rewrite("src/pipeline/a.rs", "use crate::collections_of::X;\n").text).toBe("use crate::collections_of::X;\n");
});

test("the `super`s that reach the root are its path, and no fewer", () => {
  // `src/pipeline/a/b.rs` is `pipeline::a::b`: three `super`s reach the root.
  expect(rewrite("src/pipeline/a/b.rs", "use super::super::super::js::Expr;\nuse super::super::js::Other;\n").text).toBe(
    "use rust_js_program::js::Expr;\nuse super::super::js::Other;\n",
  );
  // In an inline module, one more.
  expect(rewrite("crates/rust-js-emit/src/output.rs", "mod tests {\n    use super::super::js::Expr;\n}\nuse super::js::Expr;\n").text).toBe(
    "mod tests {\n    use rust_js_program::js::Expr;\n}\nuse rust_js_program::js::Expr;\n",
  );
});

test("a use tree's group is split where some of it moved", () => {
  const { text } = rewrite("src/pipeline/a.rs", "pub(crate) use crate::{\n    collections::{IndexMap, IndexSet},\n    js::{self, Expr},\n    pipeline::Body,\n};\n");
  expect(text).toBe("pub(crate) use crate::pipeline::Body;\npub(crate) use rust_js_program::{collections::{IndexMap, IndexSet}, js::{self, Expr}};\n");
  // All of it moved: no group left of the old crate's; `self` of the root stays.
  expect(rewrite("src/pipeline/a.rs", "use crate::{collections::IndexMap};\n").text).toBe("use rust_js_program::collections::IndexMap;\n");
  expect(rewrite("src/pipeline/a.rs", "use crate::{self as root, js};\n").text).toBe("use crate::{self as root};\nuse rust_js_program::js;\n");
  // Its attributes go with each part.
  expect(rewrite("src/pipeline/a.rs", "#[cfg(test)]\nuse crate::{js::Expr, pipeline::Body};\n").text).toBe("#[cfg(test)]\nuse crate::pipeline::Body;\n#[cfg(test)]\nuse rust_js_program::js::Expr;\n");
  // A comment in it isn't lost: the tree is left, and said.
  const commented = rewrite("src/pipeline/a.rs", "use crate::{\n    // the maps\n    collections::IndexMap,\n    pipeline::Body,\n};\n");
  expect(commented.problems).toEqual(["src/pipeline/a.rs:1: a comment in a use tree that splits, left as it is"]);
});

test("the root names its modules bare, but not a later part of a path", () => {
  expect(rewrite("src/main.rs", "use std::{collections::HashMap};\nfn main() { js::f(); a::js::g(); }\nmod tests {\n    fn t() { js::h(); }\n}\n").text).toBe(
    "use std::{collections::HashMap};\nfn main() { rust_js_program::js::f(); a::js::g(); }\nmod tests {\n    fn t() { js::h(); }\n}\n",
  );
});

test("a comment's path is rewritten where the move at hand moved its module, a string's never", () => {
  const text = '/// See [crate::js::Expr].\nconst S: &str = "crate::js::Expr";\n';
  expect(rustPathsRewritten("src/pipeline/a.rs", text, modulesOf, root, new Set(["js"])).text).toBe(
    '/// See [rust_js_program::js::Expr].\nconst S: &str = "crate::js::Expr";\n',
  );
  // In a settled tree, prose naming another crate's module is left: it
  // can't be told from an example (a sweep of main rewrote five files so).
  expect(rewrite("src/pipeline/a.rs", text).text).toBe(text);
});

test("a root's bare name it imports itself is its import's, never a moved module's", () => {
  const root = "use rust_js_program::js::{self, Expr};\nuse std::collections as output;\n\nfn f() -> js::Name {\n    output::x()\n}\n";
  expect(rewrite("src/main.rs", root).text).toBe(root);
  expect(boundNames(codeOf(root))).toEqual(new Set(["js", "Expr", "output"]));
  // An attribute's path is its tool's, never the binary's `clippy`.
  const attributed = "#[allow(clippy::vec_box)]\nfn f() {}\n";
  expect(rustPathsRewritten("crates/rust-js-program/src/lib.rs", attributed, new Map([["clippy", binary]]))).toEqual({ text: attributed, problems: [] });
});

test("the modules a move takes out of their crate are their old names, never the new crate's", () => {
  expect(movedModules([["src/jsx_syntax.rs", "crates/rust-js-jsx/src/lib.rs"], ["src/jsx_syntax/props.rs", "crates/rust-js-jsx/src/props.rs"]])).toEqual(new Set(["jsx_syntax"]));
  expect(movedModules([["src/collections.rs", "crates/rust-js-program/src/collections.rs"]])).toEqual(new Set(["collections"]));
  // Within a crate, nothing leaves it.
  expect(movedModules([["src/pipeline/a.rs", "src/pipeline/b.rs"]])).toEqual(new Set());
});

test("a crate's path to the binary's module is said, never written", () => {
  const { text, problems } = rewrite("crates/rust-js-program/src/js.rs", "use crate::collections::IndexMap;\nuse crate::pipeline::Body;\n");
  // Its own crate's module stays `crate::`.
  expect(text).toBe("use crate::collections::IndexMap;\nuse crate::pipeline::Body;\n");
  expect(problems).toEqual(["crates/rust-js-program/src/js.rs:2 names the binary's `pipeline` from rust-js-program, which no crate can: none depends on the binary"]);
});

test("a file that moved into the crate it named names it `crate::`", () => {
  expect(rewrite("crates/rust-js-program/src/js.rs", "use rust_js_program::collections::IndexMap;\n/// [rust_js_program::js::Expr]\n").text).toBe(
    "use crate::collections::IndexMap;\n/// [crate::js::Expr]\n",
  );
});

test("a path is rewritten where it's written whole, not where it ends a longer one", () => {
  // A pair of no file there is, so the script's own run leaves this alone.
  const pairs: [string, string][] = [["lib/x.rs", "crates/k/lib/x.rs"]];
  expect(rewritten('"lib/x.rs", `lib/x.rs`, crates/k/lib/x.rs, "../lib/x.rs", a/lib/x.rs, lib/x.rsx', pairs)).toBe(
    '"crates/k/lib/x.rs", `crates/k/lib/x.rs`, crates/k/lib/x.rs, "../crates/k/lib/x.rs", a/lib/x.rs, lib/x.rsx',
  );
});

// A name two crates declare, the lowering's base's and MIR's `drops` (#2299),
// is each one's own through its own root: never rewritten to the other's.
test("a crate's path to its own module is its own, whoever else has the name", () => {
  const mir = crates.find((crate) => crate.package === "rust-js-mir")!;
  const lower = crates.find((crate) => crate.package === "rust-js-lower")!;
  const shared = new Map<string, CompilerCrate>([["drops", lower]]);
  expect(rustPathsRewritten("crates/rust-js-mir/src/elaborated.rs", "use crate::drops::Style;\n", shared).text).toBe("use crate::drops::Style;\n");
  expect(mir.name).toBe("rust_js_mir");
});

// What `--widen` makes public is where rustc says the item is defined,
// never the use's own span: of a field's literal or a parameter's type,
// `pub` there is a syntax error (#2299's cut wrote eleven of them).
test("an item rustc calls private is widened where it's defined, never where it's used", () => {
  const ours = (file: string) => file.startsWith("crates/");
  const message = {
    message: "function `is_noinline` is private",
    code: { code: "E0603" },
    spans: [{ file_name: "crates/rust-js-mir/src/lib.rs", line_start: 550, label: "private function" }],
    children: [{ message: "the function `is_noinline` is defined here", spans: [{ file_name: "crates/rust-js-lower/src/recognition.rs", line_start: 12 }] }],
  };
  expect(privateSites(message, ours)).toEqual([["crates/rust-js-lower/src/recognition.rs:12", "is_noinline"]]);
  // A field's, with no note of where it's defined: nowhere.
  expect(privateSites({ ...message, code: { code: "E0616" }, children: [] }, ours)).toEqual([]);
  // A multi-line `use` is widened at its first line, which holds its `use`.
  const lines = ["pub(crate) use self::nullable::{", "    holds_nested, holds_value,", "    is_held,", "};"];
  expect([statementLine(lines, 3), statementLine(lines, 1), statementLine(["a;", "use x::y;"], 2)]).toEqual([1, 1, 2]);
});

// A path a test or a script joins from string segments, `join(root, "src",
// "lower", "iterators.rs")`, is a form the rewrite can't see: #2299's cut
// moved two such files and a test read them still (CI's test shard). A
// move says where it finds one; and every one of the compiler's sources so
// spelled is there.
test("a source a test or a script joins from segments is there, and a move says where it's spelled so", () => {
  expect(segmented('join(root, "src", "lower", "iterators.rs")', "src/lower/")).toBe(true);
  expect(segmented('join(root, "src", "lowering")', "src/lower/")).toBe(false);
  expect(segmented('"src/lower/iterators.rs"', "src/lower/")).toBe(false);
  // A part with a `.`, escaped as a part, not taken for the match.
  expect(segmented('join(root, "src", "lower.rs")', "src/lower.rs")).toBe(true);
  expect(segmented('join(root, "src", "lowerxrs")', "src/lower.rs")).toBe(false);
  const joins = textFiles()
    // Where `root` is the checkout's, support's or a script's own, never a fixture's
    // directory a test names so; not this file, whose examples are strings.
    .filter((file) => /^(?:test|scripts)\/.*\.ts$/.test(file) && !file.startsWith("scripts/mutations/") && file !== "test/move-paths.test.ts")
    .filter((file) => /import \{[^}]*\broot\b(?! as)[^}]*\} from "\.\/support"|const root = join\(import\.meta\.dir, "\.\."\)/.test(readFileSync(join(root, file), "utf8")))
    .flatMap((file) =>
      [...readFileSync(join(root, file), "utf8").matchAll(/join\(\s*(?:root|checkout)\s*,((?:\s*"[^"\n]+"\s*,?)+)\)/g)].map((m) => ({
        file,
        path: [...m[1]!.matchAll(/"([^"]+)"/g)].map((s) => s[1]).join("/"),
      })),
    )
    .filter(({ path }) => /^(?:src|crates)\//.test(path));
  expect(joins.length).toBeGreaterThan(0);
  expect(joins.filter(({ path }) => !existsSync(join(root, path))).map(({ file, path }) => `${file}: ${path}`)).toEqual([]);
});
