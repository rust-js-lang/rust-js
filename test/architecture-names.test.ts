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
// its word and reads it, and `js::Name::made` alone is made of it (S2);
// lowering makes one by `FnCx::unspelled` alone, unique in its item;
// preparation spells it, `reclaim::spelled`, asking the name; the
// validator, whose scopes keep words, refuses one printed; tidy's check of
// a name's form against its word skips one, asking `Name::is_unspelled`. A
// holder of another variable's value is one too (`Name::derived`, through
// `FnCx::unspelled_as`), of its source by its id. Nothing else makes or
// reads the form.
test("an unspelled name is made, spelled and refused in one place each", () => {
  const outsideTests = (file: string) => read(file).split("#[cfg(test)]")[0];
  const with_ = (pattern: RegExp) => sources.filter((file) => pattern.test(outsideTests(file)));
  expect(with_(/names::made\(/)).toEqual(["src/js/name.rs"]);
  expect(with_(/Name::made\(/)).toEqual(["src/lower.rs"]);
  expect(with_(/names::made_base\(/)).toEqual(["src/js/name.rs", "src/js/validate.rs"]);
  expect(with_(/\.made_base\(\)/)).toEqual(["src/prepare/reclaim.rs"]);
});

// A name given apart, `node$2` of `node`, is spelled by `names.rs` alone,
// which writes it (`given_apart`) and reads it (`apart`): a pass holding a
// name asks it (`Name::apart_of`, `root`, `given_apart_of`, #1681 change 3,
// S4c), never its word. Besides `name.rs`, the reader is preparation's
// speller, which reclaims `node$1` as `node` (`prepare/reclaim.rs`), and
// the two passes that keep names as words till S6 gives them names, a
// `catch`'s declarations and tidy's shadows' kin.
test("a name given apart is read by its name's API alone", () => {
  const outsideTests = (file: string) => read(file).split("#[cfg(test)]")[0];
  const reading = sources.filter((file) => file !== "src/names.rs" && /names::apart\(/.test(outsideTests(file)));
  expect(reading.sort()).toEqual(["src/js/name.rs", "src/prepare/catches.rs", "src/prepare/reclaim.rs"]);
});

// A place a tree reads is keyed by `js::Path` alone (ADR 1150, step 3): its
// root a variable by its id, its fields property keys, made by
// `Expr::path`, `Expr::called_path` and `Path`'s constructors. No key of a
// place is a name's word formatted with its fields, as `field_path`'s
// "element.editable", `call_path`'s "isItem()" and plain's "...items" were:
// two variables of one word would be one key.
test("a place is keyed by js::Path, never a name formatted with its fields", () => {
  const scope = sources.filter(file => file.startsWith("src/lower/") || file.startsWith("src/js/") || file === "src/js.rs");
  const shapes = [
    /fn \w+\((?:e|expr|value): &(?:js::)?Expr\) -> Option<String>/,
    /format!\("\.\.\.\{/,
    /fn (?:field_path|call_path)\((?:e|expr): &/,
  ];
  expect(scope.flatMap(file => shapes.filter(shape => shape.test(read(file))).map(shape => `${file}: ${shape}`))).toEqual([]);
});

// Link decides what each module imports, each import a `Name` of its own;
// it reads no name's word: preparation spells every binding, imports
// included (`prepare/import_names.rs`, ADR 1150).
test("link reads no name's word", () => {
  const text = read("src/link.rs");
  expect(text.match(/\.(?:into_)?word\(\)/g) ?? []).toEqual([]);
});

// JS's scoping is by spelling: `js::scopes`, its one owner, reads words, as
// preparation and printing do (ADR 1150). So that isn't a way around the
// ratchet, what's asked of it by words, its trees, renames and a local
// given apart from a module's read, is asked where names are spelled or
// checked: preparation and `js::validate`; anywhere else only whether two
// trees are alike but for their names (`alpha_eq`), which asks no word.
test("js::scopes' words are asked only where names are spelled or checked", () => {
  const spelling = (file: string) => file.startsWith("src/prepare/") || file === "src/prepare.rs"
    || file.startsWith("src/to_oxc") || file === "src/js/scopes.rs" || file === "src/js/validate.rs";
  const asked = sources.filter(file => !spelling(file)).flatMap(file => {
    const text = read(file);
    const items = [...text.matchAll(/js::scopes::(\w+)/g)].map(m => m[1])
      .filter(item => item !== "alpha_eq" && item !== "alpha_eq_functions")
      .map(item => `${file}: js::scopes::${item}`);
    // Nor brought in to be asked unqualified.
    const imported = /use crate::js::scopes\b|js::\{[^}]*\bscopes\b/.test(text) ? [`${file}: imports js::scopes`] : [];
    return [...items, ...imported];
  });
  expect(asked).toEqual([]);
});

// Kin by id (ADR 1150): whether a name is another's given apart, `x$1` of
// `x`, and what form it is, a temporary's `_$`, a helper's `$`, `__arg`,
// asked by parsing its word, here or in the fallback of
// `Name::given_apart_of` where a name records no origin. Counted, at most
// What a name lowering makes of another says of it after its word, a drop
// flag's `$live`, is a `js::Suffix`, a closed kind (ADR 1150): nothing
// writes one as text, `format!("{name}$live")`, outside its owner.
test("a suffix is a js::Suffix, never written as text", () => {
  const writing = sources.filter((file) => file !== "src/js/name.rs" && /\{[^}]*\}\$live/.test(read(file)));
  expect(writing).toEqual([]);
});

// `KIN_PARSES`, which only falls, to none: each is asked of the name's
// origin and form by its id instead. The spelling's owners, `js::name`,
// `names`, the runtime's helpers and printing, write and read it.
const KIN_PARSES = 10;
test("a name's kinship and form are asked by parsing its word only as counted", () => {
  const owners = (file: string) => file === "src/js/name.rs" || file === "src/names.rs"
    || file === "src/runtime.rs" || file.startsWith("src/to_oxc");
  const patterns = [/\.apart_of\(\)/g, /names::apart\(/g, /split\('\$'\)/g, /split_once\('\$'\)/g,
    /starts_with\('\$'\)/g, /starts_with\("_\$"\)/g, /strip_prefix\("__arg"\)/g, /\bparam\.root\(\)/g];
  const parsed = sources.filter(file => !owners(file)).flatMap(file =>
    patterns.flatMap(pattern => [...read(file).matchAll(pattern)].map(() => file)));
  if (read("src/js/name.rs").includes("self.apart_of().is_some_and(|(root, _)| root == of.root())")) {
    parsed.push("src/js/name.rs: given_apart_of's fallback");
  }
  expect(parsed.length).toBeLessThanOrEqual(KIN_PARSES);
});

// The spelling step (ADR 1150): a name's word is read where names are
// spelled, preparation and printing, and asked of a `Name` everywhere
// else. Each file's reads of a word outside them are at most
// `test/word-reads.txt`'s, a ratchet that only falls, none in a file it
// doesn't list.
test("a name's word is read outside preparation and printing only as the ratchet allows", () => {
  const allowed: Record<string, number> = Object.fromEntries(
    read("test/word-reads.txt").split("\n").filter((line) => line && !line.startsWith("#"))
      .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
  );
  const outside = (file: string) => !file.startsWith("src/prepare/") && file !== "src/prepare.rs"
    && !file.startsWith("src/to_oxc") && file !== "src/js/name.rs" && file !== "src/js/scopes.rs";
  const counted = sources.filter(outside)
    .map((file) => [file, (read(file).match(/\.(?:into_)?word\(\)/g) ?? []).length] as const)
    .filter(([, n]) => n > 0);
  expect(counted.filter(([file, n]) => n > (allowed[file] ?? 0)).map(([file, n]) => `${file}: ${n}, at most ${allowed[file] ?? 0}`)).toEqual([]);
});
