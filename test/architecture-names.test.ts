// The rules of the architecture about the names the compiler gives: a
// temporary's, a label's, and a Rust name's in JS, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);
const sources = files("src").filter(path => path.endsWith(".rs"));

// A temporary's name is one rule (ADR 0569, #890): what made it suggests
// it, rust-analyzer's way, else its role's word, and one pick of the first
// untaken. `temporary_names.rs` alone spells it; tidy's renaming, hints
// and lowering ask it.
test("a temporary's name is suggested and picked in temporary_names.rs alone", () => {
  const rule = /const USELESS_(?:METHODS|NAMES)\b|\bfn (?:call_name|call_suggestion|held_type_name)\b|\bfn pick\(\s*suggestions\b|Role::Value => "value"/;
  expect(sources.filter(file => rule.test(read(file)))).toEqual(["src/lower/temporary_names.rs"]);
});

// A name lowering picks is made of a `Base` (#890 step 2): a `Role`, whose
// word is written in temporary_names.rs's catalogue alone, or a name the
// program wrote (`Base::Named`). 261 calls each wrote a word of their own,
// `spill("value", ..)`, `fresh("items")`; none may again, nor may a text
// become a `Base` without saying which it is.
test("spill and fresh take a Base, never a word", () => {
  const word = /\.(?:spill|spill_subject|fresh|held_in_place)\(\s*(?:&\s*)?(?:"|format!\("[a-z])/;
  expect(sources.filter(file => word.test(read(file)))).toEqual([]);
  expect(read("src/lower/temporary_names.rs")).not.toMatch(/impl\b[^{]*From<&[^>]*str>\s+for\s+Base\b/);
});

// Whether a label is MIR's, `bb12`, or one the program wrote, `'desc`, is
// answered by `is_block_label` alone (ADR 0742): tidy's decisions had a
// copy of its test.
test("whether a label is MIR's is asked of is_block_label", () => {
  const tests = /strip_prefix\("bb"\)/;
  expect(sources.filter(file => tests.test(read(file)))).toEqual(["src/lower/mir/queries.rs"]);
});

// A `snake_case` name is made camelCase in one place, `camel_case`, and
// a word joined into one by `upper_first` (ADR 0573): a module's import,
// an impl's dictionary's clash, a default import's class each joined its
// words their own way. serde's `rename_all` is serde's rule, which a
// program sees in its JSON, and is its own.
test("a snake_case name is made camelCase by camel_case alone", () => {
  const splits = /\.split\('_'\)|\bch == '_'/;
  const others = files("src").filter((file) => file.endsWith(".rs") && file !== "src/lower/casing.rs" && file !== "src/lower/serde.rs");
  expect(others.filter((file) => splits.test(read(file)))).toEqual([]);
  expect(read("src/lower/casing.rs")).toMatch(/fn camel_case\(name: &str\) -> String \{[^]*?\.split\('_'\)/);
});

// A JS key the compiler or the runtime writes is camelCase, as every Rust
// name is in JS (ADR 0573): a std trait's method in a dictionary is its
// `bindings::trait_method_name`, never spelled as Rust spells it. Std's
// dictionaries were keyed `as_ref` and `into_iter` while the crate's said
// `asRef`, so a call through a dictionary found none.
test("a JS key the compiler or the runtime writes is camelCase", () => {
  const snake = '"[a-z][a-z0-9]*_[a-z0-9_]*"';
  const keyed = new RegExp(`(?:Prop::Field\\(|Expr::member\\([^;]*?,\\s*)${snake}`);
  const code = (text: string) => text.split("\n").filter((line) => !/^\s*\/\//.test(line));
  const lowering = ["src/lower.rs", ...files("src/lower")].filter((file) => file.endsWith(".rs"));
  expect(lowering.flatMap((file) => code(read(file)).filter((line) => keyed.test(line)).map((line) => `${file}: ${line.trim()}`))).toEqual([]);
  // A helper's object key or method, `writeU32(n)`, `{ writeStr: .. }`.
  const runtime = files("src/runtime").filter((file) => file.endsWith(".js"));
  expect(runtime.length).toBeGreaterThan(100);
  const defined = /^\s+[a-z][a-z0-9]*_[a-z0-9_]*\s*(?:\(|:)/;
  expect(runtime.flatMap((file) => code(read(file)).filter((line) => defined.test(line)).map((line) => `${file}: ${line.trim()}`))).toEqual([]);
});

// A test that says the JS has no block label, `bb12`, asks for one at a
// word's start, `/\bbb\d+/`: the header names the fixture's directory,
// whose hash may hold `3bb5`, which a bare `/bb\d+/` matches (#296).
test("a block label is matched at a word's start", () => {
  const bare = readdirSync(join(root, "test"))
    .filter((name) => name.endsWith(".ts") && name !== "architecture-names.test.ts")
    .filter((name) => /\/bb\\d\+/.test(read(join("test", name))));
  expect(bare).toEqual([]);
});

// A value read in place knows the name its temporary is given,
// `Expr::held_as` (ADR 0569, #769): set where a pending temporary is taken,
// `mir/temporaries.rs`, and read by the spill of a subject alone,
// `spill_subject`, there too. Every other expression is made with none.
test("a temporary's name a value carries is set and read in one place each", () => {
  const files = [...new Bun.Glob("src/**/*.rs").scanSync(root)].filter((file) => file !== "src/js.rs");
  const with_ = (pattern: RegExp) => files.filter((file) => pattern.test(read(file))).sort();
  expect(with_(/HeldAs\(Some/)).toEqual(["src/lower/mir/temporaries.rs"]);
  expect(with_(/\.held_as\.0/)).toEqual(["src/lower/mir/temporaries.rs"]);
  expect(read("src/lower/mir/temporaries.rs").match(/\.held_as\.0/g)?.length).toBe(1);
});

// A name lowering makes that isn't spelled yet, `value#1`, is one form
// (#1681 change 3, M2, the start of M3's names table): `names.rs` writes
// it and reads it; lowering makes one by `FnCx::unspelled` alone, unique in
// its item; preparation spells it, `reclaim::spelled`; the validator
// refuses one printed. Nothing else makes or reads the form.
test("an unspelled name is made, spelled and refused in one place each", () => {
  const outsideTests = (file: string) => read(file).split("#[cfg(test)]")[0];
  const with_ = (pattern: RegExp) => sources.filter((file) => pattern.test(outsideTests(file)));
  expect(with_(/names::unspelled\(/)).toEqual(["src/lower.rs"]);
  expect(with_(/names::unspelled_base\(/)).toEqual(["src/js/validate.rs", "src/prepare/reclaim.rs"]);
});
