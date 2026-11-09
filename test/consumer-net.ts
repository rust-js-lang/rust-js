// The consumer net (ITERATOR_TABLE=consumers): every std method that takes
// an iterator, or what makes one, by value, as its receiver or as an
// argument, given an iterator that prints each of its steps, clones and
// drops, of items that print theirs. A consumer of `Iterator` and
// `DoubleEndedIterator` that takes `self`; one that takes another iterator,
// `zip`, `chain`, `eq`, `cmp` and the rest; each `Extend` and `FromIterator`
// impl of core, alloc and std, a `String`'s of each item it takes, a tuple's,
// an `Option`'s and a `Result`'s; each `Sum` and `Product`; and each other
// function that takes an `IntoIterator`, `Vec::splice`'s replacement, the
// `Debug` builders' `entries`, `char::decode_utf16`. Each is a template, run
// of each source of the items it takes: the crate's own iterator, `Loud`,
// which prints as it steps, is cloned and dropped, and std's own, a `vec`'s,
// an array's, `repeat_n`, `repeat(..).take(..)` and `once`, each of which
// std specializes differently; and an empty one. Each runs as it is, with
// its closure panicking at its 2nd call, and with `Loud`'s 2nd step
// panicking, of its receiver or of its argument, each under `catch_unwind`.
// test/iterator-matrix.test.ts runs it in both builds against native Rust,
// as it runs the iterator tables. Clone counts, drop counts and their order
// are what Rust says they are: a map's `extend` that never dropped its
// iterator, an `eq` that dropped it twice and a `repeat(x).take(n).count()`
// that cloned `x` n times are each a line that differs.
//
// What it covers is read from std's own source (`stdMethods`), so a
// toolchain that adds a consumer, an `Extend` or a `FromIterator` fails
// test/consumer-net.test.ts until a template covers it or `excluded` says
// why none does. Lazy adapters are the iterator matrix's (ADR 1090); this
// net is of what consumes an iterator and what takes one as an argument.
//
// A `HashMap`'s or a `HashSet`'s order is native Rust's to choose each run:
// each is printed sorted, its items moved into a `Vec` first, and no case
// drops one holding more than one item that prints, but in sorted order.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import type { Row } from "./iterator-matrix";

// ── What std has ──────────────────────────────────────────────────────────

/** A method std has that takes an iterator by value: its id, as
 * `coveredBy` and `excluded` name it, and whether it's stable, unstable or
 * neither, std's own. */
export type StdMethod = { id: string; stability: "stable" | "unstable" | "internal"; file: string };

/** Where rustup keeps std's source, of the toolchain rust-toolchain.toml
 * pins. */
export function stdLibrary(): string {
  const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: join(import.meta.dir, "..") }).stdout.toString().trim();
  return join(sysroot, "lib/rustlib/src/rust/library");
}

/** The stability the attributes just above `at` in `text` say. */
function stabilityAt(text: string, at: number): StdMethod["stability"] {
  const lines = text.slice(0, at).split("\n");
  lines.pop();
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (line.startsWith("#[stable") || line.startsWith("#[rustc_const_stable")) {
      if (line.startsWith("#[stable")) return "stable";
      continue;
    }
    if (line.startsWith("#[unstable")) return "unstable";
    // Attributes, their continued lines, doc comments and comments.
    if (line.startsWith("#") || line.startsWith("//") || line.startsWith(")]") || /^[\w"]+\s*=/.test(line) || line === "" || line.endsWith(",")) continue;
    break;
  }
  return "internal";
}

/** The methods of `trait` in `file` that take `self` by value, or an
 * argument whose type is bounded by `IntoIterator` or `Iterator`. */
function traitMethods(library: string, file: string, trait: string): StdMethod[] {
  const text = readFileSync(join(library, file), "utf8");
  const start = text.search(new RegExp(`pub (?:const )?trait ${trait}\\b`));
  if (start < 0) throw new Error(`no trait ${trait} in ${file}`);
  const end = text.indexOf("\n}\n", start);
  const out: StdMethod[] = [];
  for (const m of text.slice(start, end).matchAll(/^ {4}(?:const )?(?:unsafe )?fn (\w+)(<[^(]*>)?\(([^)]*)\)([^{;]*)/gm)) {
    const [, name, generics = "", params, rest] = m;
    if (takesIterator(params, generics + rest, true)) out.push({ id: `${trait}::${name}`, stability: stabilityAt(text, start + m.index!), file });
  }
  return out;
}

/** Whether a function of `params`, its generics and where clause `bounds`,
 * takes an iterator by value: as `self`, where `receiver`, or as a
 * parameter of a type `IntoIterator` or `Iterator` bounds. */
function takesIterator(params: string, bounds: string, receiver: boolean): boolean {
  const list = params
    .split(",")
    .map((p) => p.trim())
    .filter(Boolean);
  if (receiver && list.length > 0 && /^(mut )?self$/.test(list[0])) return true;
  const sig = bounds.replace(/\s+/g, " ");
  return list.some((p) => {
    const ty = p.split(":").slice(1).join(":").trim();
    if (/^impl (IntoIterator|Iterator|DoubleEndedIterator)\b/.test(ty)) return true;
    if (!/^[A-Z]\w*$/.test(ty)) return false;
    return new RegExp(`\\b${ty}\\s*:\\s*(?:\\[const\\]\\s*)?(IntoIterator|Iterator|DoubleEndedIterator|ExactSizeIterator)\\b`).test(sig);
  });
}

/** Each impl of `Extend`, `FromIterator`, `Sum` or `Product` of std's
 * crates, `<trait><<item>> for <type>` as written, of one a macro writes
 * its macro's name first. */
function collectImpls(library: string, files: string[]): StdMethod[] {
  const out = new Map<string, StdMethod>();
  for (const file of files) {
    const text = readFileSync(join(library, file), "utf8");
    for (const m of text.matchAll(/^[ \t]*impl\b(<(?:[^<>]|<(?:[^<>]|<[^<>]*>)*>)*>)?\s+((?:Extend|FromIterator|Sum|Product)\b[^{]*?)\s*(?:where\b[^{]*)?\{/gm)) {
      const head = m[2].replace(/\s+/g, " ").trim();
      const before = text.slice(0, m.index);
      const macro = [...before.matchAll(/^macro_rules! (\w+)/gm)].at(-1);
      // Within a macro: one whose `macro_rules!` is the last before it, and
      // whose body hasn't ended, a `}` at the line's start, since.
      const inMacro = macro !== undefined && !/\n\}/.test(before.slice(macro.index));
      const id = `${inMacro ? `${macro[1]}: ` : ""}${head}`;
      if (!out.has(id)) out.set(id, { id, stability: inMacro ? "stable" : stabilityAt(text, m.index!), file });
    }
  }
  return [...out.values()];
}

/** Each other public function of std's crates that takes an iterator by
 * value, `<type>::<name>` of a method, `<module path>::<name>` of a free
 * function. */
function iteratorFunctions(library: string, files: string[]): StdMethod[] {
  const out: StdMethod[] = [];
  for (const file of files) {
    const text = readFileSync(join(library, file), "utf8");
    for (const m of text.matchAll(/^(\s*)pub (?:const )?(?:unsafe )?fn (\w+)(<[^(]*>)?\(([^)]*)\)([^{;]*)/gm)) {
      const [, indent, name, generics = "", params, rest] = m;
      if (!takesIterator(params, generics + rest, false)) continue;
      const before = text.slice(0, m.index);
      const owner =
        indent.replace(/\n/g, "") === ""
          ? file.replace(/\/src\//, "::").replace(/(\/mod)?\.rs$/, "").replaceAll("/", "::")
          : ([...before.matchAll(/^impl\b(?:<(?:[^<>]|<[^<>]*>)*>)?\s+([^{]+?)\s*(?:where\b[^{]*)?\{/gm)].at(-1)?.[1] ?? "?").replace(/\s+/g, " ");
      out.push({ id: `${owner}::${name}`, stability: stabilityAt(text, m.index! + indent.length), file });
    }
  }
  return out;
}

/** Each method std has that takes an iterator by value: `Iterator`'s and
 * `DoubleEndedIterator`'s, each `Extend`, `FromIterator`, `Sum` and
 * `Product` impl, and each other public function, of core, alloc and std,
 * but their tests and the platforms' own modules. */
export function stdMethods(library = stdLibrary()): StdMethod[] {
  const files = [...new Bun.Glob("{core,alloc,std}/src/**/*.rs").scanSync(library)]
    .filter((file) => !/(^|\/)(tests?|benches)(\/|\.rs$)|\/sys\/|\/os\/|\/async_iter\//.test(file))
    .sort();
  return [
    ...traitMethods(library, "core/src/iter/traits/iterator.rs", "Iterator"),
    ...traitMethods(library, "core/src/iter/traits/double_ended.rs", "DoubleEndedIterator"),
    ...collectImpls(library, files),
    ...iteratorFunctions(library, files),
  ];
}

// ── What runs ─────────────────────────────────────────────────────────────

/** A source of items: its name in a case's id, the item kind it yields,
 * its expression of the step that panics (0 for none), whether it's
 * `Loud`, whether it's double-ended, whether it's empty, of one item or
 * never ends, whether its closure calls `tick()`, and what it needs made
 * first. */
type Source = {
  name: string;
  kind: string;
  expr: (boom: number) => string;
  loud: boolean;
  back: boolean;
  empty?: boolean;
  one?: boolean;
  infinite?: boolean;
  closure?: boolean;
  before?: string;
};

/** `Loud` over a `Vec` of each list of `items`, named `name`, and over an
 * empty one; and the first list's `Vec`'s own `IntoIter`, which std
 * specializes, where `vec`. */
function louds(kind: string, ty: string, lists: [name: string, items: string][], name = "src", vec = true): Source[] {
  return [
    ...lists.map(([list, items]): Source => ({ name: list, kind, expr: (boom) => `loud(vec![${items}], "${name}", ${boom})`, loud: true, back: true })),
    { name: "empty", kind, expr: () => `loud(Vec::<${ty}>::new(), "${name}", 0)`, loud: true, back: true, empty: true },
    ...(vec
      ? [
          raw(kind, "vec", `vec![${lists[0][1]}].into_iter()`),
          // A std adapter whose step panics, where its closure's call is the
          // armed one: what std's own iterators drop as one unwinds.
          { ...raw(kind, "vec map", `vec![${lists[0][1]}].into_iter().map(|x| {\n    tick();\n    x\n})`), closure: true },
        ]
      : []),
  ];
}

/** `Loud` over a slice's `iter()` of `items`, `map`ped by `each` where
 * the kind is of pairs of references, its slice's `iter()` itself too. */
function borrowed(kind: string, owned: string, items: string, each = ""): Source[] {
  const before = `let items: Vec<${owned}> = vec![${items}];`;
  return [
    { name: "loud", kind, expr: (boom) => `loud(items.iter()${each}, "src", ${boom})`, loud: true, back: true, before },
    { name: "empty", kind, expr: () => `loud(items[..0].iter()${each}, "src", 0)`, loud: true, back: true, empty: true, before },
    { name: "slice iter", kind, expr: () => `items.iter()${each}`, loud: false, back: true, before },
  ];
}

/** A std iterator of `kind`, not `Loud`: what std specializes is its own. */
const raw = (kind: string, name: string, expr: string, back = true): Source => ({ name, kind, expr: () => expr, loud: false, back });

const ls = "L(1), L(2), L(3)";

/** Each kind of item a template takes, and its sources. */
export const sources: Source[] = [
  ...louds("L", "L", [
    ["loud", ls],
    ["loud unsorted", "L(3), L(1), L(2)"],
  ]),
  raw("L", "array", `[${ls}].into_iter()`),
  raw("L", "repeat_n", "std::iter::repeat_n(L(7), 3)"),
  raw("L", "repeat take", "std::iter::repeat(L(7)).take(3)", false),
  { ...raw("L", "once", "std::iter::once(L(7))"), one: true },
  // What never ends, which only what ends of it, or panics, is given.
  { ...raw("L", "repeat", "std::iter::repeat(L(7))", false), infinite: true },
  ...louds("u32", "u32", [
    ["loud", "1u32, 2, 3"],
    ["loud unsorted", "3u32, 1, 2"],
  ]),
  raw("u32", "range", "(1..4u32)"),
  ...borrowed("&u32", "u32", "1, 2, 3"),
  ...louds("f64", "f64", [["loud", "1.5f64, 2.5, 3.0"]]),
  ...borrowed("&f64", "f64", "1.5, 2.5, 3.0"),
  ...louds("Wrapping", "std::num::Wrapping<u32>", [["loud", "std::num::Wrapping(1u32), std::num::Wrapping(2), std::num::Wrapping(3)"]]),
  ...borrowed("&Wrapping", "std::num::Wrapping<u32>", "std::num::Wrapping(1), std::num::Wrapping(2), std::num::Wrapping(3)"),
  ...louds("Saturating", "std::num::Saturating<u32>", [["loud", "std::num::Saturating(1u32), std::num::Saturating(2), std::num::Saturating(3)"]]),
  ...borrowed("&Saturating", "std::num::Saturating<u32>", "std::num::Saturating(1), std::num::Saturating(2), std::num::Saturating(3)"),
  ...louds("Duration", "std::time::Duration", [["loud", "std::time::Duration::from_millis(1), std::time::Duration::from_millis(2), std::time::Duration::from_millis(3)"]]),
  ...borrowed("&Duration", "std::time::Duration", "std::time::Duration::from_millis(1), std::time::Duration::from_millis(2), std::time::Duration::from_millis(3)"),
  ...louds("Option<u32>", "Option<u32>", [
    ["loud", "Some(1u32), None, Some(3)"],
    ["loud all Some", "Some(1u32), Some(2), Some(3)"],
  ]),
  ...louds("Result<u32>", "Result<u32, L>", [
    ["loud", "Ok::<u32, L>(1), Err(L(2)), Ok(3)"],
    ["loud all Ok", "Ok::<u32, L>(1), Ok(2), Ok(3)"],
  ]),
  ...louds("Option<L>", "Option<L>", [
    ["loud", "Some(L(1)), None, Some(L(3))"],
    ["loud all Some", "Some(L(1)), Some(L(2)), Some(L(3))"],
  ]),
  ...louds("Result<L>", "Result<L, L>", [
    ["loud", "Ok::<L, L>(L(1)), Err(L(2)), Ok(L(3))"],
    ["loud all Ok", "Ok::<L, L>(L(1)), Ok(L(2)), Ok(L(3))"],
  ]),
  ...louds("char", "char", [["loud", "'a', 'b', 'c'"]]),
  raw("char", "chars", `"abc".chars()`),
  ...borrowed("&char", "char", "'a', 'b', 'c'"),
  ...louds("&str", "&str", [["loud", `"a", "b", "c"`]]),
  raw("&str", "split", `"a b c".split(' ')`),
  ...louds("String", "String", [["loud", `String::from("a"), String::from("b"), String::from("c")`]]),
  ...louds("Box<str>", "Box<str>", [["loud", `Box::<str>::from("a"), Box::from("b"), Box::from("c")`]]),
  ...louds("Cow<str>", "std::borrow::Cow<'static, str>", [["loud", `std::borrow::Cow::<str>::Borrowed("a"), std::borrow::Cow::Owned(String::from("b")), std::borrow::Cow::Borrowed("c")`]]),
  ...louds("(u32, L)", "(u32, L)", [["loud", "(1u32, L(1)), (2, L(2)), (1, L(3))"]]),
  ...borrowed("(&u32, &u32)", "(u32, u32)", "(1, 10), (2, 20), (1, 30)", ".map(|(k, v)| (k, v))"),
  ...louds("(&str, &str)", "(&str, &str)", [["loud", `("a", "1"), ("b", "2"), ("c", "3")`]]),
  ...louds("(L, L)", "(L, L)", [["loud", "(L(1), L(11)), (L(2), L(12)), (L(3), L(13))"]]),
  ...louds("(L,)", "(L,)", [["loud", "(L(1),), (L(2),), (L(3),)"]]),
  ...louds("()", "()", [["loud", "(), (), ()"]]),
  ...louds("u16", "u16", [
    ["loud", "0x61u16, 0xD800, 0x62"],
    ["loud valid", "0x61u16, 0x62, 0x63"],
  ]),
  ...louds("OsString", "std::ffi::OsString", [["loud", `std::ffi::OsString::from("a"), std::ffi::OsString::from("b"), std::ffi::OsString::from("c")`]]),
  ...louds("&OsStr", "&std::ffi::OsStr", [["loud", `std::ffi::OsStr::new("a"), std::ffi::OsStr::new("b"), std::ffi::OsStr::new("c")`]]),
  ...louds("Cow<OsStr>", "std::borrow::Cow<'static, std::ffi::OsStr>", [
    [
      "loud",
      `std::borrow::Cow::<std::ffi::OsStr>::Borrowed(std::ffi::OsStr::new("a")), std::borrow::Cow::Owned(std::ffi::OsString::from("b")), std::borrow::Cow::Borrowed(std::ffi::OsStr::new("c"))`,
    ],
  ]),
  // What takes no iterator: a collection's `append`.
  { name: "-", kind: "none", expr: () => "", loud: false, back: true },
];

/** The other iterators an argument's place is given, of each kind:
 * `Loud`, a shorter one, an empty one, and a `Vec`, an `IntoIterator`
 * that isn't one. */
export const args: Source[] = (
  [
    ["L", (n: number) => `L(${n})`],
    ["u32", (n: number) => `${n}u32`],
  ] as const
).flatMap(([kind, of]) => [
  ...louds(
    kind,
    kind,
    [
      ["arg", [1, 2, 4].map(of).join(", ")],
      ["short arg", of(1)],
    ],
    "arg",
    false,
  ).map((s) => (s.empty ? { ...s, name: "empty arg" } : s)),
  raw(kind, "vec arg", `vec![${[1, 2, 3].map(of).join(", ")}]`),
]);

/** A template: its name, the std methods it covers, as `stdMethods` names
 * them, the kind of item its source yields, whether its closure calls
 * `tick()`, whether it steps from the back, whether it takes an argument
 * iterator, whether it ends of a source that never does, and its
 * statements of its source `it` and argument `arg`. */
type Template = {
  name: string;
  covers: string[];
  kind: string;
  closure?: boolean;
  back?: boolean;
  arg?: boolean;
  infinite?: boolean;
  /** What generic code given its source needs of it besides being an
   * iterator, `Clone` of `cycle()`'s. */
  bound?: string;
  /** Whether it makes its source in a closure of its own, a `Debug`
   * builder's, which generic code can't of a value it's given. */
  made?: boolean;
  text: (it: string, arg: string) => string;
};

const t = (name: string, covers: string[], kind: string, text: Template["text"], more: Partial<Template> = {}): Template => ({ name, covers, kind, text, ...more });

/** Printed as a `HashSet`'s or a `HashMap`'s items are, sorted, by a
 * `Vec` of them. */
const unhashed = (c: string) => `sorted(${c}.into_iter().collect())`;

/** A collection `ty` made by `init` extended by `it`, under its own
 * `catch_unwind`, so what it holds after a panic is printed too. */
const extended = (ty: string, init: string, show = (c: string) => c) => (it: string) =>
  `let mut c: ${ty} = ${init};\nlet r = catch_unwind(AssertUnwindSafe(|| c.extend(${it})));\nprintln!("{} {:?}", caught(r), ${show("c")});`;

/** A collection `ty` collected of `it`. */
const collected = (ty: string, show = (c: string) => c) => (it: string) => `let c: ${ty} = ${it}.collect();\nprintln!("{:?}", ${show("c")});`;

const std = (path: string) => `std::collections::${path}`;

/** A `Vec` collected of an adapter of `it`, `what`. */
const vecOf = (ty: string, what: (it: string) => string) => (it: string) => `println!("{:?}", ${what(it)}.collect::<Vec<${ty}>>());`;

/** Of a template whose items are `L`s, one of `u32`s, which drop
 * nothing: what's refused of items with a destructor still runs, and its
 * iterators' steps and drops are still seen. */
const ofU32 = (template: Template, text = template.text): Template => ({ ...template, name: `${template.name} of u32`, kind: "u32", text });

const lTemplates: Template[] = [
  // `Iterator`'s and `DoubleEndedIterator`'s consumers that take `self`.
  // Of `repeat`, which never ends, std's `count` and `last` panic.
  t("count", ["Iterator::count"], "L", (it) => `println!("{}", ${it}.count());`, { infinite: true }),
  t("last", ["Iterator::last"], "L", (it) => `println!("{:?}", ${it}.last());`, { infinite: true }),
  t("for_each", ["Iterator::for_each"], "L", (it) => `${it}.for_each(|x| {\n    tick();\n    println!("each {x:?}");\n});`, { closure: true }),
  t(
    "partition",
    ["Iterator::partition"],
    "L",
    (it) => `let (a, b): (Vec<L>, Vec<L>) = ${it}.partition(|x| {\n    tick();\n    x.0 % 2 == 1\n});\nprintln!("{a:?} {b:?}");`,
    { closure: true },
  ),
  ...(["fold", "rfold"] as const).map((m) =>
    t(m, [m === "fold" ? "Iterator::fold" : "DoubleEndedIterator::rfold"], "L", (it) => `let all = ${it}.${m}(vec![L(0)], |mut acc, x| {\n    tick();\n    acc.push(x);\n    acc\n});\nprintln!("{all:?}");`, {
      closure: true,
      back: m === "rfold",
    }),
  ),
  t("reduce", ["Iterator::reduce"], "L", (it) => `println!("{:?}", ${it}.reduce(|a, b| {\n    tick();\n    if b > a { b } else { a }\n}));`, { closure: true }),
  ...(["max", "min"] as const).map((m) => t(m, [`Iterator::${m}`], "L", (it) => `println!("{:?}", ${it}.${m}());`)),
  ...(["max_by_key", "min_by_key"] as const).map((m) =>
    t(m, [`Iterator::${m}`], "L", (it) => `println!("{:?}", ${it}.${m}(|x| {\n    tick();\n    x.0 % 3\n}));`, { closure: true }),
  ),
  ...(["max_by", "min_by"] as const).map((m) =>
    t(m, [`Iterator::${m}`], "L", (it) => `println!("{:?}", ${it}.${m}(|a, b| {\n    tick();\n    a.0.cmp(&b.0)\n}));`, { closure: true }),
  ),
  t("is_sorted", ["Iterator::is_sorted"], "L", (it) => `println!("{}", ${it}.is_sorted());`),
  t("is_sorted_by", ["Iterator::is_sorted_by"], "L", (it) => `println!("{}", ${it}.is_sorted_by(|a, b| {\n    tick();\n    a <= b\n}));`, { closure: true }),
  t("is_sorted_by_key", ["Iterator::is_sorted_by_key"], "L", (it) => `println!("{}", ${it}.is_sorted_by_key(|x| {\n    tick();\n    x.0\n}));`, { closure: true }),
  t("cycle", ["Iterator::cycle"], "L", vecOf("L", (it) => `${it}.cycle().take(5)`), { infinite: true, bound: "Clone" }),
  t("flat_map", ["Iterator::flat_map"], "L", vecOf("L", (it) => `${it}.flat_map(|x| {\n    tick();\n    loud(vec![L(x.0 + 10), x], "inner", 0)\n})`), { closure: true }),
  t("flatten", ["Iterator::flatten"], "Option<L>", vecOf("L", (it) => `${it}.flatten()`)),
  t("unzip", ["Iterator::unzip"], "(L, L)", (it) => `let (a, b): (Vec<L>, Vec<L>) = ${it}.unzip();\nprintln!("{a:?} {b:?}");`),
];

/** What compares and joins two iterators, of `L`s and of `u32`s. */
const argTemplates: Template[] = [
  t("chain", ["Iterator::chain"], "L", (it, arg) => `println!("{:?}", ${it}.chain(${arg}).collect::<Vec<_>>());`, { arg: true }),
  t("zip", ["Iterator::zip"], "L", (it, arg) => `println!("{:?}", ${it}.zip(${arg}).collect::<Vec<_>>());`, { arg: true, infinite: true }),
  t("iter::chain", ["core::iter::adapters::chain::chain"], "L", (it, arg) => `println!("{:?}", std::iter::chain(${it}, ${arg}).collect::<Vec<_>>());`, { arg: true }),
  t("iter::zip", ["core::iter::adapters::zip::zip"], "L", (it, arg) => `println!("{:?}", std::iter::zip(${it}, ${arg}).collect::<Vec<_>>());`, { arg: true, infinite: true }),
  // Each ends where its argument does, of a receiver that never does.
  ...(["eq", "ne", "lt", "le", "gt", "ge", "cmp", "partial_cmp"] as const).map((m) =>
    t(m, [`Iterator::${m}`], "L", (it, arg) => `println!("{:?}", ${it}.${m}(${arg}));`, { arg: true, infinite: true }),
  ),
];

export const templates: Template[] = [
  ...lTemplates,
  ...argTemplates,
  ...[...argTemplates, ...lTemplates.filter((l) => ["count", "last", "max", "min", "is_sorted", "is_sorted_by"].includes(l.name))].map((l) => ofU32(l)),
  // Each `Sum` and `Product`.
  ...(
    [
      ["integer_sum_product", "u32", "u32", "&u32"],
      ["integer_sum_product", "std::num::Wrapping<u32>", "Wrapping", "&Wrapping"],
      ["saturating_integer_sum_product", "std::num::Saturating<u32>", "Saturating", "&Saturating"],
      ["float_sum_product", "f64", "f64", "&f64"],
    ] as const
  ).flatMap(([macro, ty, kind, ref]) =>
    (["Sum", "Product"] as const).flatMap((tr) => {
      const m = tr.toLowerCase();
      return [
        t(`${m}::<${ty}>`, [`${macro}: ${tr} for $a`, `Iterator::${m}`], kind, (it) => `println!("{:?}", ${it}.${m}::<${ty}>());`),
        t(`${m}::<${ty}> of refs`, [`${macro}: ${tr}<&'a $a> for $a`], ref, (it) => `println!("{:?}", ${it}.${m}::<${ty}>());`),
      ];
    }),
  ),
  t("sum::<Duration>", ["Sum for Duration"], "Duration", (it) => `println!("{:?}", ${it}.sum::<std::time::Duration>());`),
  t("sum::<Duration> of refs", ["Sum<&'a Duration> for Duration"], "&Duration", (it) => `println!("{:?}", ${it}.sum::<std::time::Duration>());`),
  ...(["Sum", "Product"] as const).flatMap((tr) => [
    t(`${tr.toLowerCase()}::<Option<u32>>`, [`${tr}<Option<U>> for Option<T>`], "Option<u32>", (it) => `println!("{:?}", ${it}.${tr.toLowerCase()}::<Option<u32>>());`),
    t(`${tr.toLowerCase()}::<Result<u32, L>>`, [`${tr}<Result<U, E>> for Result<T, E>`], "Result<u32>", (it) => `println!("{:?}", ${it}.${tr.toLowerCase()}::<Result<u32, L>>());`),
  ]),
  // Each `FromIterator`.
  t("collect Vec", ["FromIterator<T> for Vec<T>", "Iterator::collect"], "L", collected("Vec<L>")),
  t("collect Box<[T]>", ["FromIterator<I> for Box<[I]>"], "L", collected("Box<[L]>")),
  t("collect Rc<[T]>", ["FromIterator<T> for Rc<[T]>"], "L", collected("std::rc::Rc<[L]>")),
  t("collect Arc<[T]>", ["FromIterator<T> for Arc<[T]>"], "L", collected("std::sync::Arc<[L]>")),
  t("collect Cow<[T]>", ["FromIterator<T> for Cow<'a, [T]>"], "L", collected("std::borrow::Cow<'static, [L]>")),
  t("collect LinkedList", ["FromIterator<T> for LinkedList<T>"], "L", collected(std("LinkedList<L>"))),
  t("collect VecDeque", ["FromIterator<T> for VecDeque<T>"], "L", collected(std("VecDeque<L>"))),
  t("collect BTreeSet", ["FromIterator<T> for BTreeSet<T>"], "L", collected(std("BTreeSet<L>"))),
  t("collect BinaryHeap", ["FromIterator<T> for BinaryHeap<T>"], "L", collected(std("BinaryHeap<L>"), (c) => `${c}.into_sorted_vec()`)),
  t("collect HashSet", ["FromIterator<T> for HashSet<T, S>"], "L", collected(std("HashSet<L>"), unhashed)),
  t("collect BTreeMap", ["FromIterator<(K, V)> for BTreeMap<K, V>"], "(u32, L)", collected(std("BTreeMap<u32, L>"))),
  t("collect HashMap", ["FromIterator<(K, V)> for HashMap<K, V, S>"], "(u32, L)", collected(std("HashMap<u32, L>"), unhashed)),
  ...(
    [
      ["char", "char"],
      ["&char", "&'a char"],
      ["&str", "&'a str"],
      ["String", "String"],
      ["Box<str>", "Box<str, A>"],
      ["Cow<str>", "Cow<'a, str>"],
    ] as const
  ).flatMap(([kind, item]) => [
    t(`collect String of ${kind}`, [`FromIterator<${item}> for String`], kind, collected("String")),
    t(`collect Box<str> of ${kind}`, [`FromIterator<${item}> for Box<str>`], kind, collected("Box<str>")),
    t(`extend String of ${kind}`, [`Extend<${item}> for String`], kind, extended("String", `String::from("x")`)),
  ]),
  ...(
    [
      ["char", "char"],
      ["&str", "&'b str"],
      ["String", "String"],
    ] as const
  ).map(([kind, item]) => t(`collect Cow<str> of ${kind}`, [`FromIterator<${item}> for Cow<'a, str>`], kind, collected("std::borrow::Cow<'static, str>"))),
  t("collect Option", ["FromIterator<Option<A>> for Option<V>"], "Option<L>", collected("Option<Vec<L>>")),
  t("collect Result", ["FromIterator<Result<A, E>> for Result<V, E>"], "Result<L>", collected("Result<Vec<L>, L>")),
  t("collect ()", ["FromIterator<()> for ()"], "()", collected("()")),
  t("collect (Vec,)", ["FromIterator<(T,)> for (ExtendT,)"], "(L,)", collected("(Vec<L>,)")),
  t("collect (Vec, Vec)", ["impl_extend_tuple: FromIterator<($($ty,)+)> for ($($extend_ty,)+)"], "(L, L)", collected("(Vec<L>, Vec<L>)")),
  ...(
    [
      ["OsString", "OsString"],
      ["&OsStr", "&'a OsStr"],
      ["Cow<OsStr>", "Cow<'a, OsStr>"],
    ] as const
  ).flatMap(([kind, item]) => [
    t(`collect OsString of ${kind}`, [`FromIterator<${item}> for OsString`], kind, collected("std::ffi::OsString")),
    t(`extend OsString of ${kind}`, [`Extend<${item}> for OsString`], kind, extended("std::ffi::OsString", `std::ffi::OsString::from("x")`)),
  ]),
  t("collect PathBuf", ["FromIterator<P> for PathBuf"], "&str", collected("std::path::PathBuf")),
  // Each `Extend`.
  t("extend Vec", ["Extend<T> for Vec<T, A>"], "L", extended("Vec<L>", "vec![L(0)]")),
  t("extend Vec of refs", ["Extend<&'a T> for Vec<T, A>"], "&u32", extended("Vec<u32>", "vec![0]")),
  t("extend VecDeque", ["Extend<T> for VecDeque<T, A>"], "L", extended(std("VecDeque<L>"), `${std("VecDeque")}::from([L(0)])`)),
  t("extend VecDeque of refs", ["Extend<&'a T> for VecDeque<T, A>"], "&u32", extended(std("VecDeque<u32>"), `${std("VecDeque")}::from([0])`)),
  t("extend LinkedList", ["Extend<T> for LinkedList<T, A>"], "L", extended(std("LinkedList<L>"), `${std("LinkedList")}::from([L(0)])`)),
  t("extend LinkedList of refs", ["Extend<&'a T> for LinkedList<T, A>"], "&u32", extended(std("LinkedList<u32>"), `${std("LinkedList")}::from([0])`)),
  t("extend BTreeSet", ["Extend<T> for BTreeSet<T, A>"], "L", extended(std("BTreeSet<L>"), `${std("BTreeSet")}::from([L(0)])`)),
  t("extend BTreeSet of refs", ["Extend<&'a T> for BTreeSet<T, A>"], "&u32", extended(std("BTreeSet<u32>"), `${std("BTreeSet")}::from([0])`)),
  t("extend BinaryHeap", ["Extend<T> for BinaryHeap<T, A>"], "L", extended(std("BinaryHeap<L>"), `${std("BinaryHeap")}::from([L(0)])`, (c) => `${c}.into_sorted_vec()`)),
  t("extend BinaryHeap of refs", ["Extend<&'a T> for BinaryHeap<T, A>"], "&u32", extended(std("BinaryHeap<u32>"), `${std("BinaryHeap")}::from([0])`, (c) => `${c}.into_sorted_vec()`)),
  t("extend HashSet", ["Extend<T> for HashSet<T, S, A>"], "L", extended(std("HashSet<L>"), `${std("HashSet")}::from([L(0)])`, unhashed)),
  t("extend HashSet of refs", ["Extend<&'a T> for HashSet<T, S, A>"], "&u32", extended(std("HashSet<u32>"), `${std("HashSet")}::from([0])`, unhashed)),
  t("extend BTreeMap", ["Extend<(K, V)> for BTreeMap<K, V, A>"], "(u32, L)", extended(std("BTreeMap<u32, L>"), `${std("BTreeMap")}::from([(1, L(0))])`)),
  t("extend BTreeMap of refs", ["Extend<(&'a K, &'a V)> for BTreeMap<K, V, A>"], "(&u32, &u32)", extended(std("BTreeMap<u32, u32>"), `${std("BTreeMap")}::from([(1, 0)])`)),
  t("extend HashMap", ["Extend<(K, V)> for HashMap<K, V, S, A>"], "(u32, L)", extended(std("HashMap<u32, L>"), `${std("HashMap")}::from([(1, L(0))])`, unhashed)),
  t("extend HashMap of refs", ["Extend<(&'a K, &'a V)> for HashMap<K, V, S, A>"], "(&u32, &u32)", extended(std("HashMap<u32, u32>"), `${std("HashMap")}::from([(1, 0)])`, unhashed)),
  t("extend ()", ["Extend<()> for ()"], "()", extended("()", "()")),
  t("extend (Vec,)", ["Extend<(T,)> for (ExtendT,)"], "(L,)", extended("(Vec<L>,)", "(vec![L(0)],)")),
  t("extend (Vec, Vec)", ["impl_extend_tuple: Extend<($($ty,)+)> for ($($extend_ty,)+)"], "(L, L)", extended("(Vec<L>, Vec<L>)", "(vec![L(0)], Vec::new())")),
  t("extend PathBuf", ["Extend<P> for PathBuf"], "&str", extended("std::path::PathBuf", `std::path::PathBuf::from("x")`)),
  // Each other function that takes an iterator.
  t(
    "Vec::splice collected",
    ["Vec<T, A>::splice"],
    "L",
    (it) =>
      `let mut v = vec![L(10), L(11), L(12)];\nlet r = catch_unwind(AssertUnwindSafe(|| {\n    let removed: Vec<L> = v.splice(1..2, ${it}).collect();\n    println!("removed {removed:?}");\n}));\nprintln!("{} {v:?}", caught(r));`,
  ),
  t(
    "Vec::splice dropped",
    ["Vec<T, A>::splice"],
    "L",
    (it) => `let mut v = vec![L(10), L(11), L(12)];\nlet r = catch_unwind(AssertUnwindSafe(|| {\n    v.splice(1..2, ${it});\n}));\nprintln!("{} {v:?}", caught(r));`,
  ),
  t("char::decode_utf16", ["char::decode_utf16"], "u16", (it) => `for r in char::decode_utf16(${it}) {\n    println!("{r:?}");\n}`),
  t("std::char::decode_utf16", ["core::char::decode_utf16"], "u16", (it) => `for r in std::char::decode_utf16(${it}) {\n    println!("{r:?}");\n}`),
  // Each shown into a `String` first: a `Debug` that prints while it
  // formats prints before the line it's in, ADR 0377's accepted
  // difference, which the line formatted first doesn't show.
  t("DebugList::entries", ["DebugList<'a, 'b>::entries"], "L", (it) => `let shown = format!("{:?}", Listed(|| ${it}));\nprintln!("{shown}");`, { made: true }),
  t("DebugSet::entries", ["DebugSet<'a, 'b>::entries"], "L", (it) => `let shown = format!("{:?}", Setted(|| ${it}));\nprintln!("{shown}");`, { made: true }),
  t("DebugMap::entries", ["DebugMap<'a, 'b>::entries"], "(u32, L)", (it) => `let shown = format!("{:?}", Mapped(|| ${it}));\nprintln!("{shown}");`, { made: true }),
  t("env::join_paths", ["std::env::join_paths"], "&str", (it) => `println!("{:?}", std::env::join_paths(${it}));`),
  t(
    "Command::args",
    ["Command::args"],
    "&str",
    (it) => `let mut c = std::process::Command::new("x");\nc.args(${it});\nprintln!("{:?}", c.get_args().collect::<Vec<_>>());`,
  ),
  t(
    "Command::envs",
    ["Command::envs"],
    "(&str, &str)",
    (it) => `let mut c = std::process::Command::new("x");\nc.envs(${it});\nprintln!("{:?}", c.get_envs().collect::<Vec<_>>());`,
  ),
  // A collection's `append`, which takes another by `&mut`: what's moved,
  // and what a key already held drops.
  t("Vec::append", [], "none", () => `let mut a = vec![L(1)];\nlet mut b = vec![L(2), L(3)];\na.append(&mut b);\nprintln!("{a:?} {b:?}");`),
  t("VecDeque::append", [], "none", () => `let mut a = ${std("VecDeque")}::from([L(1)]);\nlet mut b = ${std("VecDeque")}::from([L(2), L(3)]);\na.append(&mut b);\nprintln!("{a:?} {b:?}");`),
  t("LinkedList::append", [], "none", () => `let mut a = ${std("LinkedList")}::from([L(1)]);\nlet mut b = ${std("LinkedList")}::from([L(2), L(3)]);\na.append(&mut b);\nprintln!("{a:?} {b:?}");`),
  t(
    "BTreeMap::append",
    [],
    "none",
    () => `let mut a = ${std("BTreeMap")}::from([(1u32, L(1)), (2, L(2))]);\nlet mut b = ${std("BTreeMap")}::from([(2u32, L(12)), (3, L(13))]);\na.append(&mut b);\nprintln!("{a:?} {b:?}");`,
  ),
  t("BTreeSet::append", [], "none", () => `let mut a = ${std("BTreeSet")}::from([L(1), L(2)]);\nlet mut b = ${std("BTreeSet")}::from([L(2), L(3)]);\na.append(&mut b);\nprintln!("{a:?} {b:?}");`),
  t(
    "BinaryHeap::append",
    [],
    "none",
    () => `let mut a = ${std("BinaryHeap")}::from([L(1)]);\nlet mut b = ${std("BinaryHeap")}::from([L(2), L(3)]);\na.append(&mut b);\nprintln!("{:?} {}", a.into_sorted_vec(), b.len());`,
  ),
];

/** Why no template covers a method std has, of those `stdMethods` finds:
 * the lazy adapters, which the iterator matrix runs through each consumer
 * (ADR 1090), and what std hasn't stabilized or keeps its own, which no
 * program on a stable toolchain can call. */
const lazy = "a lazy adapter: the iterator matrix covers it, through each consumer (ADR 1090)";
export const excluded: Record<string, string> = Object.fromEntries(
  ["step_by", "map", "filter", "filter_map", "enumerate", "peekable", "skip_while", "take_while", "map_while", "skip", "take", "scan", "fuse", "inspect", "rev", "copied", "cloned"].map((m) => [
    `Iterator::${m}`,
    lazy,
  ]),
);

/** Why no template covers `m`, or none where one must. */
export function whyNot(m: StdMethod): string | undefined {
  if (m.stability === "unstable") return "unstable: the stable toolchain native Rust runs on can't call it";
  if (m.stability === "internal") return "std's own, of no stability attribute: no program can name it";
  return excluded[m.id];
}

// ── Its cases and programs ────────────────────────────────────────────────

/** How a case runs: as it is; its closure panicking at its 2nd call; its
 * source's 2nd step panicking; or its argument's. */
export type Variant = "ok" | "closure" | "step" | "arg step";

/** Who calls it: the case's own code, of its concrete types; generic code
 * given its source as a type parameter's value, `fn of<I: Iterator>(it:
 * I)`, which its caller makes a JS iterator of; or generic code that makes
 * it by a function it's given, `F: Fn() -> I`, whose `I` is given as it
 * is, with its dictionary (ADR 0411): a std function rust-js lowers itself
 * that's given it must step it by that dictionary (#1147). */
export type Caller = "concrete" | "generic" | "generic made";

export type ConsumerCase = Row & { template: Template; source: Source; arg?: Source; variant: Variant; caller: Caller };

/** Each template of each source of the kind it takes, each argument where
 * it takes one, in each variant that applies: a closure's panic where it
 * calls one and its source yields two items; a step's where it's `Loud`'s,
 * of the source or the argument. */
export function consumerCases(): ConsumerCase[] {
  const out: Omit<ConsumerCase, "n">[] = [];
  for (const template of templates) {
    for (const source of sources.filter((s) => s.kind === template.kind && (!template.back || s.back) && (!s.infinite || template.infinite))) {
      for (const arg of template.arg ? args.filter((a) => a.kind === template.kind) : [undefined]) {
        const variants: Variant[] = ["ok"];
        if ((template.closure || source.closure) && !source.empty && !source.one && !source.infinite) variants.push("closure");
        if (source.loud && !source.empty) variants.push("step");
        if (arg?.loud && !arg.empty) variants.push("arg step");
        // Generic code's callers, of each source as it is and as its step
        // panics: what it drops as it unwinds there too.
        const callers: Caller[] = template.kind === "none" ? ["concrete"] : template.made ? ["concrete", "generic made"] : ["concrete", "generic", "generic made"];
        for (const caller of callers) {
          for (const variant of caller === "concrete" ? variants : variants.filter((v) => v !== "closure")) {
            const id = [template.name, source.name, ...(arg ? [arg.name] : []), variant, ...(caller === "concrete" ? [] : [caller])].join(" | ");
            out.push({ id, template, source, arg, variant, caller });
          }
        }
      }
    }
  }
  // Interleaved, by a hash of each id, so each program holds many
  // templates of many sources, and a slice of a few holds them all.
  const order = (id: string) => Number(Bun.hash(id) % 1_000_000_007n);
  return out
    .map((c) => ({ c, k: order(c.id) }))
    .sort((x, y) => x.k - y.k || (x.c.id < y.c.id ? -1 : 1))
    .map(({ c }, n) => ({ ...c, n }));
}

const prelude = `// Generated by test/consumer-net.ts. Each case prints its name, each
// step, clone and drop of the iterators it's given and of their items, what
// it makes, and whether it panicked.
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};

/** The call of a case's closures that panics, 0 for none, and the calls
 * so far. */
static ARMED: AtomicU32 = AtomicU32::new(0);
static CALLS: AtomicU32 = AtomicU32::new(0);
/** The clones of a \`Loud\` made so far in the case. */
static COPIES: AtomicU32 = AtomicU32::new(0);

/** The case's closures' \`n\`th call panics. */
fn arm(n: u32) {
    ARMED.store(n, Ordering::Relaxed);
}

/** A call of a case's closure, which panics where it's the armed one. */
fn tick() {
    let n = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if n == ARMED.load(Ordering::Relaxed) {
        panic!("closure");
    }
}

/** An item that prints as it's cloned and dropped; its clone is 100 more,
 * so its drop is told apart. */
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/** An iterator of \`it\`'s items that prints each of its steps, clones and
 * drops, as \`name\`, a clone's as \`name#n\`; its \`boom\`th step panics. */
struct Loud<I> {
    it: I,
    name: &'static str,
    copy: u32,
    boom: u32,
    steps: u32,
}

fn loud<I: IntoIterator>(items: I, name: &'static str, boom: u32) -> Loud<I::IntoIter> {
    Loud { it: items.into_iter(), name, copy: 0, boom, steps: 0 }
}

impl<I> Loud<I> {
    fn say(&self, what: &str) {
        if self.copy == 0 {
            println!("{what} {}", self.name);
        } else {
            println!("{what} {}#{}", self.name, self.copy);
        }
    }

    fn step(&mut self, what: &str) {
        self.steps += 1;
        self.say(what);
        if self.steps == self.boom {
            panic!("step");
        }
    }
}

impl<I: Iterator> Iterator for Loud<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.step("next");
        self.it.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Loud<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        self.step("next_back");
        self.it.next_back()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Loud<I> {}

impl<I: Clone> Clone for Loud<I> {
    fn clone(&self) -> Self {
        self.say("clone");
        let copy = COPIES.fetch_add(1, Ordering::Relaxed) + 1;
        Loud { it: self.it.clone(), name: self.name, copy, boom: self.boom, steps: self.steps }
    }
}

impl<I> Drop for Loud<I> {
    fn drop(&mut self) {
        self.say("drop");
    }
}

/** What \`debug_list().entries\` shows of the iterator \`.0\` makes. */
struct Listed<F>(F);

impl<F, I> fmt::Debug for Listed<F>
where
    F: Fn() -> I,
    I: Iterator,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries((self.0)()).finish()
    }
}

/** What \`debug_set().entries\` shows of the iterator \`.0\` makes. */
struct Setted<F>(F);

impl<F, I> fmt::Debug for Setted<F>
where
    F: Fn() -> I,
    I: Iterator,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries((self.0)()).finish()
    }
}

/** What \`debug_map().entries\` shows of the iterator of pairs \`.0\` makes. */
struct Mapped<F>(F);

impl<F, I, K, V> fmt::Debug for Mapped<F>
where
    F: Fn() -> I,
    I: Iterator<Item = (K, V)>,
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries((self.0)()).finish()
    }
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

/** A panic's message. */
fn said(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("?"))
}

/** Whether what \`catch_unwind\` ran ended, or its panic's message. */
fn caught(r: Result<(), Box<dyn std::any::Any + Send>>) -> String {
    match r {
        Ok(()) => String::from("ok"),
        Err(payload) => format!("panicked: {}", said(&*payload)),
    }
}

fn run(name: &str, case: fn()) {
    println!("case {name}");
    ARMED.store(0, Ordering::Relaxed);
    CALLS.store(0, Ordering::Relaxed);
    COPIES.store(0, Ordering::Relaxed);
    match catch_unwind(case) {
        Ok(()) => println!("ended"),
        Err(payload) => println!("panicked: {}", said(&*payload)),
    }
}`;

const indent = (text: string) =>
  text
    .split("\n")
    .map((line) => (line === "" ? "" : `    ${line}`))
    .join("\n");

/** The item type of each kind a template takes, as generic code names it,
 * its references of a lifetime `'a`. */
const itemTypes: Record<string, string> = {
  L: "L",
  u32: "u32",
  "&u32": "&'a u32",
  f64: "f64",
  "&f64": "&'a f64",
  Wrapping: "std::num::Wrapping<u32>",
  "&Wrapping": "&'a std::num::Wrapping<u32>",
  Saturating: "std::num::Saturating<u32>",
  "&Saturating": "&'a std::num::Saturating<u32>",
  Duration: "std::time::Duration",
  "&Duration": "&'a std::time::Duration",
  "Option<u32>": "Option<u32>",
  "Result<u32>": "Result<u32, L>",
  "Option<L>": "Option<L>",
  "Result<L>": "Result<L, L>",
  char: "char",
  "&char": "&'a char",
  "&str": "&'a str",
  String: "String",
  "Box<str>": "Box<str>",
  "Cow<str>": "std::borrow::Cow<'a, str>",
  "(u32, L)": "(u32, L)",
  "(&u32, &u32)": "(&'a u32, &'a u32)",
  "(&str, &str)": "(&'a str, &'a str)",
  "(L, L)": "(L, L)",
  "(L,)": "(L,)",
  "()": "()",
  u16: "u16",
  OsString: "std::ffi::OsString",
  "&OsStr": "&'a std::ffi::OsStr",
  "Cow<OsStr>": "std::borrow::Cow<'a, std::ffi::OsStr>",
};

/** A case's function, `case_<i>`, and of generic code's callers, the
 * generic function it calls, `generic_<i>`, whose body is the template's. */
function caseFn(c: ConsumerCase, i: number): string {
  const lines: string[] = [];
  if (c.variant === "closure") lines.push("arm(2);");
  if (c.source.before) lines.push(c.source.before);
  const it = c.source.expr(c.variant === "step" ? 2 : 0);
  const arg = c.arg?.expr(c.variant === "arg step" ? 2 : 0) ?? "";
  if (c.caller === "concrete") {
    lines.push(c.template.text(it, arg));
    return `fn case_${i}() {\n${indent(lines.join("\n"))}\n}`;
  }
  const item = itemTypes[c.template.kind];
  if (item === undefined) throw new Error(`no item type of ${c.template.kind}`);
  const bounds = ["Iterator<Item = " + item + ">", ...(c.template.back ? ["DoubleEndedIterator"] : []), ...(c.template.bound ? [c.template.bound] : [])].join(" + ");
  const params = ["'a", `I: ${bounds}`, ...(c.caller === "generic made" ? ["F: Fn() -> I"] : []), ...(c.arg ? [`J: IntoIterator<Item = ${item}>`] : [])];
  const given = [c.caller === "generic made" ? "make: F" : "it: I", ...(c.arg ? ["arg: J"] : [])];
  const body = c.template.text(c.caller === "generic made" ? "make()" : "it", "arg");
  lines.push(`generic_${i}(${[c.caller === "generic made" ? `|| ${it}` : it, ...(c.arg ? [arg] : [])].join(", ")});`);
  return `fn case_${i}() {\n${indent(lines.join("\n"))}\n}\n\nfn generic_${i}<${params.join(", ")}>(${given.join(", ")}) {\n${indent(body)}\n}`;
}

/** A program of `cases`, which runs each in turn. */
export function consumerProgram(cases: ConsumerCase[]): string {
  const fns = cases.map((c, i) => caseFn(c, i));
  const main = `fn main() {\n${cases.map((c, i) => `    run(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case (`slice()` in
 * test/iterator-matrix.ts): its template in its variant, and its source's
 * and argument's. */
export const consumerStrata = (c: ConsumerCase) => [
  `${c.template.name} | ${c.variant}`,
  `${c.source.kind} ${c.source.name} | ${c.variant}`,
  ...(c.arg ? [`arg ${c.arg.name} | ${c.variant}`] : []),
];
