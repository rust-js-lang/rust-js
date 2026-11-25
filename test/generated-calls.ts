// Calls generated of a Rust type, given alike by JS and by native Rust,
// and their results printed alike by each (#2481): the export net's, and
// declarations-conformance's after #2573. A type is a manifest's
// `OfferedType` tree (`crates/rust-js-program/src/offered.rs`): values of
// each plain form at its bounds and inside, each a JS literal and a Rust
// one; and a printer of each side, which prints a float by its bits and a
// string by its code points, so neither side's own formatting is compared.

/** An offered path's type, as the manifest writes it. */
export type Tree =
  | { kind: "int"; rust: string }
  | { kind: "float"; rust: string }
  | { kind: "bool" }
  | { kind: "char" }
  | { kind: "str"; owned: boolean }
  | { kind: "unit" }
  | { kind: "option"; of: Tree }
  | { kind: "vec"; of: Tree; borrowed: boolean }
  | { kind: "ref"; of: Tree; mutable?: boolean }
  | { kind: "box"; of: Tree }
  | { kind: "fn"; inputs: Tree[]; output: Tree }
  | { kind: "param"; name: string; index?: number }
  | { kind: "named"; crate: string; rust: string[]; args?: Tree[] }
  | { kind: "tuple"; items: Tree[] }
  | { kind: "result"; ok: Tree; err: Tree }
  | { kind: "array"; of: Tree; len: number }
  | { kind: "rc"; of: Tree; counted?: boolean }
  | { kind: "cell"; of: Tree; itself?: boolean }
  | { kind: "dyn"; traits: string[] }
  | { kind: "iterator"; back?: boolean; len?: boolean }
  | { kind: "weak"; of: Tree; counted?: boolean }
  | { kind: "non_zero"; of: Tree }
  | { kind: "pointer"; of: Tree; nullable?: boolean }
  | { kind: "set"; of: Tree }
  | { kind: "map"; key: Tree; value: Tree }
  | { kind: "other"; rust: string };

/** A value given alike: its JS literal and its Rust one. */
export type Given = { js: string; rust: string };

// Each integer's values, its bounds and inside: what JS gives a number of
// 32 bits or less, `usize`'s too (ADR 0160), and a BigInt of 64 and more.
const ints: Record<string, string[]> = {
  u8: ["0", "1", "127", "128", "255"],
  u16: ["0", "1", "32768", "65535"],
  u32: ["0", "1", "2147483648", "4294967295"],
  usize: ["0", "1", "4294967295"],
  i8: ["-128", "-1", "0", "1", "127"],
  i16: ["-32768", "-1", "0", "32767"],
  i32: ["-2147483648", "-1", "0", "1", "2147483647"],
  isize: ["-2147483648", "0", "2147483647"],
  u64: ["0", "1", "9007199254740993", "18446744073709551615"],
  i64: ["-9223372036854775808", "-1", "0", "9223372036854775807"],
  u128: ["0", "1", "340282366920938463463374607431768211455"],
  i128: ["-170141183460469231731687303715884105728", "0", "170141183460469231731687303715884105727"],
};
const wide = (rust: string) => /^[iu](64|128)$/.test(rust);
const floats: Record<string, [string, string][]> = {
  f64: [["0", "0.0"], ["-0", "-0.0"], ["1.5", "1.5"], ["-1.5", "-1.5"], ["NaN", "f64::NAN"], ["Infinity", "f64::INFINITY"], ["-Infinity", "f64::NEG_INFINITY"]],
  f32: [["0", "0.0"], ["1.5", "1.5"], ["-0.5", "-0.5"], ["NaN", "f32::NAN"]],
};
const chars = ["a", "é", "\u{1F600}"];
const strings = ["", "a", "\u{1F600}", "{", "null"];

/** `text` as a Rust literal's escapes and a JS one's alike: each code
 * point past ASCII's printable range by its number. */
const escaped = (text: string, quote: string) =>
  [...text].map((c) => {
    const cp = c.codePointAt(0)!;
    return cp >= 0x20 && cp < 0x7f && c !== quote && c !== "\\" ? c : `\\u{${cp.toString(16)}}`;
  }).join("");

/** The Rust type `tree` is, as a literal of it names it. */
export function rustType(tree: Tree): string {
  switch (tree.kind) {
    case "int": case "float": return tree.rust;
    case "bool": return "bool";
    case "char": return "char";
    case "str": return tree.owned ? "String" : "&str";
    case "unit": return "()";
    case "option": return `Option<${rustType(tree.of)}>`;
    case "vec": return tree.borrowed ? `&[${rustType(tree.of)}]` : `Vec<${rustType(tree.of)}>`;
    case "ref": return `&${tree.mutable ? "mut " : ""}${rustType(tree.of)}`;
    case "box": return `Box<${rustType(tree.of)}>`;
    case "fn": return `impl Fn(${tree.inputs.map(rustType).join(", ")}) -> ${rustType(tree.output)}`;
    case "param": return tree.name;
    case "named": return tree.rust.join("::");
    case "tuple": return `(${tree.items.map(rustType).join(", ")}${tree.items.length === 1 ? "," : ""})`;
    case "result": return `Result<${rustType(tree.ok)}, ${rustType(tree.err)}>`;
    case "array": return `[${rustType(tree.of)}; ${tree.len}]`;
    case "rc": return `::std::rc::Rc<${rustType(tree.of)}>`;
    case "cell": return `::std::cell::${tree.itself ? "RefCell" : "Cell"}<${rustType(tree.of)}>`;
    case "dyn": return `dyn ${tree.traits.join(" + ")}`;
    case "iterator": return "impl Iterator";
    case "weak": return `::std::rc::Weak<${rustType(tree.of)}>`;
    case "non_zero": return `::std::num::NonZero<${rustType(tree.of)}>`;
    case "pointer": return `*const ${rustType(tree.of)}`;
    case "set": return `::std::collections::HashSet<${rustType(tree.of)}>`;
    case "map": return `::std::collections::HashMap<${rustType(tree.key)}, ${rustType(tree.value)}>`;
    case "other": return tree.rust;
  }
}

/** Values of `tree`, each given alike, or `undefined` of a type it has
 * none of: another's, a `()` parameter. */
export function values(tree: Tree, small = false): Given[] | undefined {
  switch (tree.kind) {
    case "int": {
      // Of small ones alone, where a call of a bound runs past its time.
      const of = ints[tree.rust]?.filter((n) => !small || Math.abs(Number(n)) <= 255);
      return of?.map((n) => ({ js: wide(tree.rust) ? `${n}n` : n, rust: `${n}${tree.rust}` }));
    }
    case "float": return floats[tree.rust]?.map(([js, rust]) => ({ js, rust: /::/.test(rust) ? rust : `${rust}${tree.rust}` }));
    case "bool": return [{ js: "true", rust: "true" }, { js: "false", rust: "false" }];
    case "char": return chars.map((c) => ({ js: `"${escaped(c, '"')}"`, rust: `'${escaped(c, "'")}'` }));
    case "str": return strings.map((s) => ({ js: `"${escaped(s, '"')}"`, rust: tree.owned ? `String::from("${escaped(s, '"')}")` : `"${escaped(s, '"')}"` }));
    case "unit": return undefined;
    case "option": {
      const of = values(tree.of, small);
      return of && [{ js: "undefined", rust: `None::<${rustType(tree.of)}>` }, ...of.slice(0, 2).map((v) => ({ js: v.js, rust: `Some(${v.rust})` }))];
    }
    case "vec": {
      const of = values(tree.of, small);
      if (!of) return undefined;
      const list = (items: Given[]) => {
        const made = `Vec::<${rustType(tree.of)}>::from([${items.map((v) => v.rust).join(", ")}])`;
        return { js: `[${items.map((v) => v.js).join(", ")}]`, rust: tree.borrowed ? `&${made}[..]` : made };
      };
      return [list([]), list(of.slice(0, 1)), list(of.slice(0, 3))];
    }
    // A `&mut`'s JS is a box or the object it points at, as its `.d.ts`
    // says, declarations-conformance's to give (#2573): no value of these.
    case "ref": {
      if (tree.mutable) return undefined;
      const of = values(tree.of, small);
      return of?.map((v) => ({ js: v.js, rust: `&${v.rust}` }));
    }
    case "tuple": {
      const each = tree.items.map((item) => values(item, small));
      if (each.some((v) => v === undefined)) return undefined;
      const count = Math.max(...each.map((v) => v!.length));
      return Array.from({ length: count }, (_, i) => {
        const items = each.map((v) => v![i % v!.length]!);
        return { js: `[${items.map((v) => v.js).join(", ")}]`, rust: `(${items.map((v) => v.rust).join(", ")}${items.length === 1 ? "," : ""})` };
      });
    }
    case "box":
    case "fn":
    case "param":
    case "named":
    case "result":
    case "array":
    case "rc":
    case "cell":
    case "dyn":
    case "iterator":
    case "weak":
    case "non_zero":
    case "pointer":
    case "set":
    case "map":
    case "other":
      return undefined;
  }
}

/** Whether a result of `tree` is printed alike by both sides. */
export function printable(tree: Tree): boolean {
  switch (tree.kind) {
    case "option": case "vec": return printable(tree.of);
    // Of two or three, which `rustShow` prints.
    case "tuple": return (tree.items.length === 2 || tree.items.length === 3) && tree.items.every(printable);
    case "int": return tree.rust in ints;
    case "float": return tree.rust in floats;
    case "ref": return !tree.mutable && printable(tree.of);
    case "box": case "fn": case "param": case "named": case "result": case "array": case "rc": case "cell": case "dyn": case "iterator": case "weak": case "non_zero": case "pointer": case "set": case "map": case "other": return false;
    default: return true;
  }
}

/** The calls given a function of `params`: as many as its parameter of
 * the most values has, each parameter's values in turn, of small ones
 * alone where `small`; `undefined` where one has none. */
export function calls(params: Tree[], small = false): Given[][] | undefined {
  const each = params.map((param) => values(param, small));
  if (each.some((v) => v === undefined)) return undefined;
  const count = Math.min(8, Math.max(1, ...each.map((v) => v!.length)));
  return Array.from({ length: count }, (_, i) => each.map((v) => v![i % v!.length]!));
}

/** JS's printer, `show(value, tree)`, as source a harness embeds. */
export const jsShow = `function show(v, t) {
  switch (t.kind) {
    case "int": return typeof v === "bigint" || typeof v === "number" ? String(v) : "not-a-number(" + typeof v + ")";
    case "float": {
      const view = new DataView(new ArrayBuffer(8));
      if (t.rust === "f32") { view.setFloat32(0, v); return "f" + view.getUint32(0).toString(16).padStart(8, "0"); }
      view.setFloat64(0, v);
      return "f" + view.getBigUint64(0).toString(16).padStart(16, "0");
    }
    case "bool": return String(v);
    case "char": return typeof v === "string" ? "c" + v.codePointAt(0) : "not-a-char(" + typeof v + ")";
    case "str": return typeof v === "string" ? "s[" + [...v].map((c) => c.codePointAt(0)).join(", ") + "]" : "not-a-string(" + typeof v + ")";
    case "unit": return v === undefined ? "()" : "not-unit(" + typeof v + ")";
    case "option": return v === undefined ? "None" : v === null ? "null" : "Some(" + show(v, t.of) + ")";
    case "vec": return v != null && typeof v === "object" && typeof v.length === "number" ? "[" + Array.from(v, (x) => show(x, t.of)).join(", ") + "]" : "not-a-list(" + typeof v + ")";
    case "ref": return show(v, t.of);
    case "tuple": return Array.isArray(v) ? "(" + t.items.map((item, i) => show(v[i], item)).join(", ") + ")" : "not-a-tuple(" + typeof v + ")";
    default: return "unprintable";
  }
}`;

/** What an `Option`'s `None` is in a place (`OfferedNone`). */
export type NoneForm = "plain" | "present" | "null" | "null_optional" | "nested" | "held";

/** A struct's record as oracle B reads it: its fields, or its transparent
 * field's type, by its definition's key, `crate::path`. */
export type Shaped = {
  positional?: Tree[];
  unit?: { kind: "str" | "bool" | "undefined"; value?: string | boolean };
  fields?: { js: string; ty: Tree; none?: NoneForm; property?: Tree }[];
  transparent?: Tree;
  variants?: {
    form: "tagged" | "declared" | "untagged" | "keyed";
    key: string;
    variants: { rust: string; tag: { kind: "str" | "num" | "bool" | "null" | "undefined"; value?: string | number | boolean }; fields?: { js: string; ty: Tree; none?: NoneForm }[]; otherwise?: boolean }[];
  };
};

/** The guide's rows of its types table (docs/guide/interop/types.md) each
 * shape `conforms` checks is held to, each its Rust cell and its JS one,
 * which test/oracle-b.test.ts finds in the guide: the checker a mirror of
 * the guide's contract, never of the compiler's representation code, so a
 * value of the wrong shape fails though native's printout agrees (oracle
 * B, ADR 1204). */
export const shapeRows: [string, string][] = [
  ["`u8` `u16` `u32` `i8` `i16` `i32`", "number, wrapped after each operation"],
  ["`usize` `isize`", "number, 32 bits"],
  ["`u64` `i64` `u128` `i128`", "BigInt, `5n`"],
  ["`f64`, `f32`", "number (`f32` rounded by `Math.fround`)"],
  ["`bool`", "boolean"],
  ["`char`", "a string of one character"],
  ["`String`, `&str`, `Box<str>`", "string"],
  ["`()`", "`undefined`"],
  ["`Vec<T>`, `[T; N]`, `&[T]`, `VecDeque<T>`", "array"],
  ["tuple `(A, B)`", "array `[a, b]`"],
  ["struct with named fields", "object `{ x, y }`"],
  ["`#[repr(transparent)]` struct", "its field"],
  ["`Option<T>`", "the value, or `undefined`; a field's `None` left out where its struct is made, `point?: T`"],
  ["`#[rust_js::present]` `Option<T>` field", "the value, or `undefined`, its key written, `point: T \\| undefined`"],
  ["`#[rust_js::nullable]` `Option<T>`", "the value, or `null`"],
  ["`#[rust_js::nullable]` `#[rust_js::optional]` field", "absent until it's set, then the value or `null`"],
  ["`#[rust_js::nullable]` `Option<Option<T>>` field", "absent for `None`, `null` for `Some(None)`, else the value"],
  ["`#[rust_js::nullable]` field of an `Option` of what may be `undefined`, `()` or such an untagged enum", "`null` for `None`, else the value, `undefined` too"],
  ["`Box<T>`, `Rc<T>`, `Arc<T>`", "the value itself"],
  ["`&T`", "the value"],
  ["closure, `fn` pointer, `dyn Fn`", "function"],
  ["`Result<T, E>`", "`{ TAG: \"Ok\", _0 }`, `{ TAG: \"Err\", _0 }`"],
  ["an `Rc` whose count is read", "`{ value, strong, weak }`"],
  ["`Cell<T>`, `RefCell<T>`, `UnsafeCell<T>`", "`{ value }`, an `UnsafeCell`'s `get()` the cell itself; in a field, the property itself; a `RefCell` of an array, a `Map`, a `Set` or an object, that object"],
  ["`dyn Trait`", "`{ value, impl }`"],
  ["`NonZero<T>`", "its number, as `T`'s"],
  ["`Weak<T>`", "what its `Rc` is, the value itself or its counts' object; `null` of `Weak::new()`"],
  ["a raw pointer, `NonNull<T>`", "a handle, its referent its `value`; a raw one's `null` of `ptr::null()`"],
  ["an atomic, `AtomicU32`", "`{ value }`"],
  ["`HashMap`, `BTreeMap`", "`Map`"],
  ["`HashSet`, `BTreeSet`", "`Set`"],
  ["tuple struct", "array `[a, b]`"],
  ["unit struct", "`undefined`, or its `#[rust_js::name]` string"],
  ["`impl Iterator` given back, or std's iterator, `Chars`", "a JS iterator, spread by `[...it]`; of `impl DoubleEndedIterator`, `back()` its last item; of `impl ExactSizeIterator`, `len()` how many are left"],
  ["fieldless enum variant", "its name, `\"Red\"`; under a `#[repr(u8)]`, its number, its module's `const`, `Empty`"],
  ["enum variant with fields", "`{ TAG: \"Circle\", radius }`, `{ TAG: \"Square\", _0 }`"],
  ["`#[rust_js::tag = \"status\"]` enum", "`{ status: \"done\", value }`"],
  ["`#[rust_js::untagged]` enum", "its payload itself"],
];

/** Oracle B, as source a harness holds: `conforms(v, t, none, types,
 * count)`, the first way `v` isn't of `t`'s JS shape as `shapeRows`' rows
 * say, by its place, `.1[0]`, or `null`; `none`, its `Option`'s `None`
 * form; `types`, each struct's `Shaped` by its definition's key. Each
 * shape checked counts in `count.checked`, each it can't decide, an enum's,
 * a tuple struct's, a `&mut`'s, an `Option` of what may be `undefined`, in
 * `count.unchecked`: said, never passed unsaid. */
export const jsConforms = `function conforms(v, t, none, types, count, at = "", depth = 0, env = []) {
  const what = (x) => (x === null ? "null" : Array.isArray(x) ? "an array" : typeof x === "object" ? (x.constructor?.name ?? "an object") : typeof x);
  const bad = (said) => ("the value" + at) + ": " + what(v) + ", not " + said;
  const checked = () => { count.checked++; return null; };
  const unchecked = (why) => { count.unchecked++; if (count.why) count.why[why] = (count.why[why] ?? 0) + 1; return null; };
  // Past 1,000 places in: each object is held to a type once (below), so
  // this bounds only the recursion, a list's three places a node.
  if (depth > 1000) return unchecked("deep");
  // An object already held to this type, or being, a cycle's or one shared:
  // checked once.
  if (v !== null && typeof v === "object") {
    count.seen ??= new WeakMap();
    const held = count.seen.get(v) ?? new Set();
    if (held.has(t)) return null;
    held.add(t);
    count.seen.set(v, held);
  }
  const inner = (x, ty, place, form = "plain", within = env) => conforms(x, ty, form, types, count, at + place, depth + 1, within);
  // Of each field of a struct or a variant, its object's, as its form says.
  const fieldsOf = (fields, name, here) => {
    for (const field of fields) {
      const place = "." + field.js;
      const has = field.js in v;
      const form = field.none ?? "plain";
      // A cell field that's the property its cell holds: that property.
      const ty = field.property ?? field.ty;
      if (ty.kind === "option") {
        // A field's \`None\`: left out where it's known there, else a key of
        // \`undefined\` (ADR 0280), or its key written, as its form says.
        if (!has && (form === "plain" || form === "null_optional" || form === "nested")) { count.checked++; continue; }
        if (!has) return ("the value" + at) + place + ": no key, not its None's " + (form === "present" ? "undefined" : "null");
      } else if (!has) return ("the value" + at) + place + ": no key, not " + name + "'s field";
      const problem = inner(v[field.js], ty, place, form, here);
      if (problem) return problem;
    }
    return null;
  };
  // A tag as JS writes it, a variant's.
  const tagIs = (x, tag) => tag.kind === "null" ? x === null : tag.kind === "undefined" ? x === undefined : x === tag.value;
  // Whether a check of \`v\` finds no problem, counted apart: an untagged
  // enum's variant, tried in turn.
  // "yes", "no", or "unknown" where it can't say.
  const fits = (ty, here) => {
    const apart = { checked: 0, unchecked: 0 };
    if (conforms(v, ty, "plain", types, apart, at, depth + 1, here) !== null) return "no";
    return apart.unchecked === 0 ? "yes" : "unknown";
  };
  switch (t.kind) {
    case "int": {
      const wide = /^[iu](64|128)$/.test(t.rust);
      if (wide ? typeof v !== "bigint" : typeof v !== "number" || !Number.isInteger(v)) return bad(t.rust + "'s " + (wide ? "BigInt" : "integer number"));
      const bits = BigInt(/size$/.test(t.rust) ? 32 : Number(t.rust.slice(1)));
      const signed = t.rust[0] === "i";
      const n = BigInt(v);
      if (n < (signed ? -(1n << (bits - 1n)) : 0n) || n > (signed ? (1n << (bits - 1n)) - 1n : (1n << bits) - 1n)) return bad(t.rust + "'s range");
      return checked();
    }
    case "float":
      if (typeof v !== "number") return bad(t.rust + "'s number");
      if (t.rust === "f32" && !Number.isNaN(v) && Math.fround(v) !== v) return bad("an f32, rounded by Math.fround");
      return checked();
    case "bool": return typeof v === "boolean" ? checked() : bad("a boolean");
    case "char": return typeof v === "string" && [...v].length === 1 ? checked() : bad("a string of one character");
    case "str": return typeof v === "string" ? checked() : bad("a string");
    case "unit": return v === undefined ? checked() : bad("undefined");
    case "option": {
      const of = t.of;
      if (none === "nested") {
        if (of.kind !== "option") return unchecked("nested-not-option");
        if (v === undefined || v === null) return checked();
        return inner(v, of.of, "");
      }
      if (none === "held") return v === null ? checked() : inner(v, of, "");
      // A \`Some\` of what may itself be \`undefined\` is boxed, a form the guide's table doesn't give.
      if (of.kind === "option" || of.kind === "unit") return unchecked("option-of-undefined");
      if (none === "null") return v === null ? checked() : inner(v, of, "");
      if (none === "null_optional") return v === null || v === undefined ? checked() : inner(v, of, "");
      return v === undefined ? checked() : inner(v, of, "");
    }
    case "vec":
      if (!Array.isArray(v)) return bad("an array");
      for (const [i, x] of v.entries()) { const problem = inner(x, t.of, "[" + i + "]"); if (problem) return problem; }
      return checked();
    case "tuple":
      if (!Array.isArray(v) || v.length !== t.items.length) return bad("an array of " + t.items.length);
      for (const [i, item] of t.items.entries()) { const problem = inner(v[i], item, "[" + i + "]"); if (problem) return problem; }
      return checked();
    case "ref": return t.mutable ? unchecked("mut-ref") : inner(v, t.of, "", none);
    // A type parameter, of what its \`Named\` gives it, in that one's scope.
    case "param": {
      const given = env[t.index ?? -1];
      return given ? conforms(v, given.tree, none, types, count, at, depth + 1, given.env) : unchecked("param");
    }
    case "box": return inner(v, t.of, "", none);
    case "fn": return typeof v === "function" ? checked() : bad("a function");
    case "result": {
      if (v === null || typeof v !== "object" || (v.TAG !== "Ok" && v.TAG !== "Err")) return bad("a Result's { TAG: 'Ok' | 'Err', _0 }");
      return inner(v._0, v.TAG === "Ok" ? t.ok : t.err, "._0") ?? checked();
    }
    case "array":
      if (!Array.isArray(v) || v.length !== t.len) return bad("an array of " + t.len);
      for (const [i, x] of v.entries()) { const problem = inner(x, t.of, "[" + i + "]"); if (problem) return problem; }
      return checked();
    // An \`Rc\`, the value itself, or where it's counted, its counts' object.
    case "rc":
      if (!t.counted) return inner(v, t.of, "", none);
      if (v === null || typeof v !== "object" || typeof v.strong !== "number" || typeof v.weak !== "number" || !("value" in v)) return bad("a counted Rc's { value, strong, weak }");
      return inner(v.value, t.of, ".value") ?? checked();
    // A cell, \`{ value }\`, or a \`RefCell\` the object it holds, that object.
    case "cell":
      if (t.itself) return inner(v, t.of, "", none);
      if (v === null || typeof v !== "object" || !("value" in v)) return bad("a cell's { value }");
      return inner(v.value, t.of, ".value") ?? checked();
    // A \`dyn\` trait's pair, its value, \`undefined\` left out, and its impl.
    case "dyn":
      return v !== null && typeof v === "object" && v.impl !== null && typeof v.impl === "object" ? checked() : bad("a dyn's { value, impl }");
    // A \`Weak\`, as its \`Rc\` is, or \`Weak::new()\`'s \`null\`.
    case "weak":
      if (v === null) return checked();
      return inner(v, { kind: "rc", of: t.of, counted: t.counted }, "", none);
    case "non_zero": {
      const problem = inner(v, t.of, "", none);
      if (problem) return problem;
      return v === 0 || v === 0n ? bad("a NonZero's number, not 0") : checked();
    }
    // A handle whose \`value\` is its referent, never read here: a read may
    // expose its address.
    case "pointer":
      if (v === null && t.nullable) return checked();
      return v !== null && typeof v === "object" && "value" in v ? checked() : bad("a pointer's handle, of a value");
    case "set":
      if (!(v instanceof Set)) return bad("a Set");
      for (const [i, x] of [...v].entries()) { const problem = inner(x, t.of, "[" + i + "]"); if (problem) return problem; }
      return checked();
    case "map":
      if (!(v instanceof Map)) return bad("a Map");
      for (const [i, [k, x]] of [...v].entries()) {
        const problem = inner(k, t.key, ".keys[" + i + "]") ?? inner(x, t.value, ".get(" + i + ")");
        if (problem) return problem;
      }
      return checked();
    case "iterator":
      if (v === null || typeof v !== "object" || typeof v.next !== "function" || typeof v[Symbol.iterator] !== "function") return bad("a JS iterator");
      if (t.back && typeof v.back !== "function") return bad("a DoubleEndedIterator's iterator, of back()");
      if (t.len && typeof v.len !== "function") return bad("an ExactSizeIterator's iterator, of len()");
      return checked();
    case "named": {
      const shaped = types[t.crate + "::" + t.rust.join("::")];
      const name = t.rust.join("::");
      // Its own parameters, each of what it's given here.
      const here = (t.args ?? []).map((tree) => ({ tree, env }));
      if (shaped?.transparent) return inner(v, shaped.transparent, "", none, here);
      const e = shaped?.variants;
      if (e) {
        const isObject = v !== null && typeof v === "object" && !Array.isArray(v);
        if (e.form === "untagged") {
          // Its payload itself, or a variant's of no fields its literal: one
          // of them, each tried.
          const fit = e.variants.map((variant) => variant.fields?.length
            ? variant.fields.length === 1 ? fits(variant.fields[0].ty, here) : "unknown"
            : tagIs(v, variant.tag) ? "yes" : "no");
          if (fit.includes("yes")) return checked();
          return fit.includes("unknown") ? unchecked("untagged-undecided") : bad("one of " + name + "'s variants' payloads");
        }
        if (e.form === "keyed") {
          if (!isObject) return bad("an object of one of " + name + "'s variants' keys");
          const found = e.variants.find((variant) => (variant.fields ?? []).length > 0 && variant.fields.every((field) => field.js in v));
          if (!found) return unchecked("keyed-fieldless");
          return fieldsOf(found.fields, name + "::" + found.rust, here) ?? checked();
        }
        // Tagged: a variant of no fields its tag itself, but of a declared
        // tag's, an object of its tag; of fields, an object of its tag.
        const declared = e.form === "declared";
        const bare = e.variants.find((variant) => !declared && !(variant.fields ?? []).length && tagIs(v, variant.tag));
        if (bare) return checked();
        if (!isObject) return bad("one of " + name + "'s variants, its tag or an object of its " + e.key);
        const variant = e.variants.find((variant) => !variant.otherwise && (declared || (variant.fields ?? []).length > 0) && tagIs(v[e.key], variant.tag));
        if (!variant) {
          const otherwise = e.variants.find((variant) => variant.otherwise);
          if (otherwise?.fields?.length === 1) return inner(v, otherwise.fields[0].ty, "", "plain", here);
          return ("the value" + at) + "." + e.key + ": " + JSON.stringify(v[e.key]) + ", not a tag of " + name + "'s";
        }
        return fieldsOf(variant.fields ?? [], name + "::" + variant.rust, here) ?? checked();
      }
      // A tuple struct, its fields' array; a unit struct, its value.
      if (shaped?.positional?.length) {
        if (!Array.isArray(v) || v.length !== shaped.positional.length) return bad("an array of " + name + "'s " + shaped.positional.length + " fields");
        for (const [i, item] of shaped.positional.entries()) { const problem = inner(v[i], item, "[" + i + "]", "plain", here); if (problem) return problem; }
        return checked();
      }
      if (shaped?.unit) return tagIs(v, shaped.unit) ? checked() : bad(name + "'s value, " + (shaped.unit.kind === "undefined" ? "undefined" : JSON.stringify(shaped.unit.value)));
      if (!shaped?.fields?.length) return unchecked(shaped ? "named-no-fields" : "named-unrecorded:" + t.crate + "::" + name);
      if (v === null || typeof v !== "object" || Array.isArray(v)) return bad("an object of " + name + "'s fields");
      return fieldsOf(shaped.fields, name, here) ?? checked();
    }
    default: return unchecked(t.kind + (t.kind === "other" ? ":" + t.rust.replace(/<.*/, "<..>") : ""));
  }
}`;

/** Native Rust's printer, `Show`, as source a generated `main` begins with:
 * each line as JS's `show` prints it. */
export const rustShow = `#![allow(unused)]
trait Show {
    fn show(&self) -> String;
}
macro_rules! shown_as_text {
    ($($t:ty),*) => {$(impl Show for $t {
        fn show(&self) -> String {
            self.to_string()
        }
    })*};
}
shown_as_text!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool);
impl Show for f64 {
    fn show(&self) -> String {
        format!("f{:016x}", self.to_bits())
    }
}
impl Show for f32 {
    fn show(&self) -> String {
        format!("f{:08x}", self.to_bits())
    }
}
impl Show for char {
    fn show(&self) -> String {
        format!("c{}", *self as u32)
    }
}
impl Show for str {
    fn show(&self) -> String {
        format!("s[{}]", self.chars().map(|c| (c as u32).to_string()).collect::<Vec<_>>().join(", "))
    }
}
impl Show for String {
    fn show(&self) -> String {
        self.as_str().show()
    }
}
impl Show for () {
    fn show(&self) -> String {
        "()".to_string()
    }
}
impl<T: Show + ?Sized> Show for &T {
    fn show(&self) -> String {
        (**self).show()
    }
}
impl<T: Show> Show for Option<T> {
    fn show(&self) -> String {
        match self {
            None => "None".to_string(),
            Some(v) => format!("Some({})", v.show()),
        }
    }
}
impl<T: Show> Show for [T] {
    fn show(&self) -> String {
        format!("[{}]", self.iter().map(Show::show).collect::<Vec<_>>().join(", "))
    }
}
impl<T: Show> Show for Vec<T> {
    fn show(&self) -> String {
        self.as_slice().show()
    }
}
impl<A: Show, B: Show> Show for (A, B) {
    fn show(&self) -> String {
        format!("({}, {})", self.0.show(), self.1.show())
    }
}
impl<A: Show, B: Show, C: Show> Show for (A, B, C) {
    fn show(&self) -> String {
        format!("({}, {}, {})", self.0.show(), self.1.show(), self.2.show())
    }
}
/// What a call printed, or the panic it made, as JS's harness says it.
fn shown<T: Show>(call: impl FnOnce() -> T + std::panic::UnwindSafe) -> String {
    match std::panic::catch_unwind(call) {
        Ok(v) => v.show(),
        Err(payload) => match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
            (Some(said), _) => format!("panic: {said}"),
            (_, Some(said)) => format!("panic: {said}"),
            _ => "panic".to_string(),
        },
    }
}
`;
