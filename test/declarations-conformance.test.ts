// Each exported function of a set of corpus cases, called from TypeScript
// as its declaration says (ADR 1091): a caller generated per function gives
// a value of each declared type, a `&mut`'s box or object as the `.d.ts`
// says it is, is type-checked by `tsc`, run in Node, and must print what
// the same calls print natively, each `&mut`'s value after it too. Where a
// declaration and its JS disagree, `tsc` refuses the caller, or its run
// differs from Rust's (#549, #559, #560, #619, #620). TypeScript 7 has no
// JS API, so the `.d.ts` is read as rust-js prints it.
//
// A function of a type parameter bound by a trait of the case's is called
// once for each impl of it, given that impl's dictionary as declared, and
// a `&mut` it gives is shown as declared, a box's `value` (#679).
//
// A function of a type the generator has no value for, a function a
// declaration leaves out (ADR 1077), or one of no parameters is skipped;
// each case must have at least one call. The nightly's suite runs it
// (`scripts/shard.ts`), and so does each mutation that names it.
//
// And each corpus case whose public functions or structs hold an `Option`,
// whose `.d.ts` says a `None` may be given as `null`, `undefined` or an
// absent key, `x?: T | null | undefined` (#822, ADR 1085's change 6): each
// function called once more for each of those its parameters' and their
// structs' fields' declarations admit, each `Option` given it where it's
// declared to take it, and what Rust is given of it, `None`, or of a
// nullable `Option<Option<T>>` field's `null`, `Some(None)` (ADR 1017). A
// `.d.ts` that admits what the lowering doesn't read as Rust's `None`, or a
// lowering that trusts a form the `.d.ts` doesn't promise, differs from
// Rust there. What each call gives is shown by its type, a `None` `null`
// whatever its form, so a key JS leaves out shows as Rust's `None` does.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, nativeBinary, root, run } from "./support";
import { readdirSync } from "node:fs";

const cases = [
  "declared_signatures",
  "nested_mut_self_through_a_dictionary",
  "let_chain_values",
  "switch_of_an_optional_bool",
  "or_pattern_paths_absorbed",
  "or_pattern_one_place",
  "match_arm_comments_kept",
  "cloned_option_of_a_type_parameter",
  "js_given_copies",
];

// A Rust type, as a signature writes it.
type Ty =
  | { k: "num"; n: string }
  | { k: "bool" }
  | { k: "str" }
  | { k: "string" }
  | { k: "ref"; mut: boolean; inner: Ty }
  | { k: "vec"; inner: Ty }
  | { k: "slice"; inner: Ty }
  | { k: "opt"; inner: Ty }
  | { k: "box"; inner: Ty }
  | { k: "tuple"; items: Ty[] }
  | { k: "fn"; inputs: Ty[]; output: Ty }
  | { k: "named"; n: string };

const numbers = ["u8", "u16", "u32", "usize", "i8", "i16", "i32", "isize", "f32", "f64"];

function tokens(text: string): string[] {
  return text.match(/->|'[a-z_]+|[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*|[&<>()[\],+:]/g) ?? [];
}

function parseType(ts: string[], at = { i: 0 }): Ty {
  const t = ts[at.i++];
  if (t === "&") {
    while (ts[at.i]?.startsWith("'")) at.i++;
    const mut = ts[at.i] === "mut";
    if (mut) at.i++;
    return { k: "ref", mut, inner: parseType(ts, at) };
  }
  if (t === "(") {
    const items: Ty[] = [];
    while (ts[at.i] !== ")") {
      items.push(parseType(ts, at));
      if (ts[at.i] === ",") at.i++;
    }
    at.i++;
    return { k: "tuple", items };
  }
  if (t === "[") {
    const inner = parseType(ts, at);
    at.i++;
    return { k: "slice", inner };
  }
  if (t === "impl" || t === "dyn") {
    const name = ts[at.i++];
    if (!/^Fn(Mut|Once)?$/.test(name)) return { k: "named", n: `${t} ${name}` };
    return parseFnBound(ts, at);
  }
  if (numbers.includes(t)) return { k: "num", n: t };
  if (t === "bool") return { k: "bool" };
  if (t === "str") return { k: "str" };
  if (t === "String") return { k: "string" };
  if ((t === "Vec" || t === "Option" || t === "Box") && ts[at.i] === "<") {
    at.i++;
    const inner = parseType(ts, at);
    at.i++;
    return t === "Vec" ? { k: "vec", inner } : t === "Option" ? { k: "opt", inner } : { k: "box", inner };
  }
  return { k: "named", n: t };
}

// `Fn(A, B) -> R`, its name read.
function parseFnBound(ts: string[], at: { i: number }): Ty {
  at.i++;
  const inputs: Ty[] = [];
  while (ts[at.i] !== ")") {
    inputs.push(parseType(ts, at));
    if (ts[at.i] === ",") at.i++;
  }
  at.i++;
  let output: Ty = { k: "tuple", items: [] };
  if (ts[at.i] === "->") {
    at.i++;
    output = parseType(ts, at);
  }
  return { k: "fn", inputs, output };
}

// What's between `open` and its close, from `from`, and where it ends.
function enclosed(text: string, from: number, open: string, close: string): [string, number] {
  let depth = 0;
  for (let i = from; i < text.length; i++) {
    if (text.startsWith("->", i) || text.startsWith("=>", i)) {
      i++;
      continue;
    }
    if (text[i] === open) depth++;
    if (text[i] === close && --depth === 0) return [text.slice(from + 1, i), i + 1];
  }
  throw new Error(`no close of ${open} at ${from}`);
}

// Split at the commas of depth 0.
function split(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    if (text.startsWith("->", i) || text.startsWith("=>", i)) {
      i++;
      continue;
    }
    if ("<([{".includes(text[i])) depth++;
    if (">)]}".includes(text[i])) depth--;
    if (text[i] === "," && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map((p) => p.trim()).filter((p) => p !== "");
}

// A function, its type parameters replaced, and the dictionaries of the
// impls it's called with, by their JS names, `tallySlot`, which it's given
// after its arguments, as its declaration takes them.
type Fn = { name: string; params: { name: string; ty: Ty }[]; output: Ty | null; dictionaries: string[] };

type Impl = { ty: Ty; dictionary: string };

// Each impl of a trait of the case's, `impl Slot for Tally`, by its trait's
// name: the type it's of and its dictionary's JS name, `tallySlot`, which
// its module exports.
function impls(source: string): Map<string, Impl[]> {
  const found = new Map<string, Impl[]>();
  for (const m of source.matchAll(/^impl (\w+) for (\w+) \{/gm)) {
    if (!new RegExp(`^pub trait ${m[1]}\\b`, "m").test(source)) continue;
    const dictionary = `${m[2][0].toLowerCase()}${m[2].slice(1)}${m[1]}`;
    found.set(m[1], [...(found.get(m[1]) ?? []), { ty: parseType(tokens(m[2])), dictionary }]);
  }
  return found;
}

// Each top-level `pub fn`, its type parameters replaced: bound by `Fn`, its
// function; of no other bound but std's, a `u32`; bound by a trait of the
// case's, each of its impls' types in turn, a function of each, given that
// impl's dictionary.
function functions(source: string): Fn[] {
  const found: Fn[] = [];
  const implsOf = impls(source);
  for (const m of source.matchAll(/^pub fn (\w+)/gm)) {
    let at = m.index! + m[0].length;
    const generics: Record<string, Ty | null> = {};
    const ofTraits: [string, Impl[]][] = [];
    let bounds = "";
    if (source[at] === "<") {
      const [inside, end] = enclosed(source, at, "<", ">");
      bounds = inside;
      at = end;
    }
    const [params, afterParams] = enclosed(source, at, "(", ")");
    const rest = source.slice(afterParams, source.indexOf("{", afterParams));
    const [ret, where] = rest.split(/\bwhere\b/);
    for (const bound of split(`${bounds},${where ?? ""}`)) {
      const [name, ...said] = bound.split(":");
      const by = said.join(":").trim();
      const fnBound = /^(?:Fn|FnMut|FnOnce)\(/.exec(by);
      generics[name.trim()] = fnBound ? parseFnBound(tokens(by), { i: 1 }) : /^[A-Z][A-Za-z]*$/.test(name.trim()) && /^(?:(?:Clone|Copy|PartialOrd|PartialEq|Ord|Eq|Debug|'static)\s*\+?\s*)*$/.test(by) ? { k: "num", n: "u32" } : null;
      if (implsOf.has(by)) ofTraits.push([name.trim(), implsOf.get(by)!]);
    }
    const replace = (ty: Ty): Ty | null => {
      switch (ty.k) {
        case "named":
          return ty.n in generics ? generics[ty.n] : ty;
        case "ref":
        case "vec":
        case "slice":
        case "opt":
        case "box": {
          const inner = replace(ty.inner);
          return inner && { ...ty, inner };
        }
        case "tuple": {
          const items = ty.items.map(replace);
          return items.every((i) => i) ? { k: "tuple", items: items as Ty[] } : null;
        }
        case "fn": {
          const inputs = ty.inputs.map(replace);
          const output = replace(ty.output);
          return inputs.every((i) => i) && output ? { k: "fn", inputs: inputs as Ty[], output } : null;
        }
        default:
          return ty;
      }
    };
    // Each choice of an impl for each type parameter bound by a trait of the case's.
    let choices: Impl[][] = [[]];
    for (const [, of] of ofTraits) choices = choices.flatMap((chosen) => of.map((imp) => [...chosen, imp]));
    for (const chosen of choices) {
      ofTraits.forEach(([name], i) => {
        generics[name] = chosen[i].ty;
      });
      const typed = split(params).map((p) => {
        const colon = p.indexOf(":");
        return { name: p.slice(0, colon).replace(/^mut\s+/, "").trim(), ty: replace(parseType(tokens(p.slice(colon + 1)))) };
      });
      const arrow = ret.indexOf("->");
      const output = arrow < 0 ? { k: "tuple" as const, items: [] } : replace(parseType(tokens(ret.slice(arrow + 2))));
      const dictionaries = chosen.map((imp) => imp.dictionary);
      if (typed.every((p) => p.ty) && typed.length > 0) found.push({ name: m[1], params: typed as Fn["params"], output, dictionaries });
    }
  }
  return found;
}

type Struct = { name: string; fields: { name: string; ty: Ty }[] };

// Each struct of named fields, each of a type it has a value of.
function structs(source: string): Map<string, Struct> {
  const found = new Map<string, Struct>();
  const bare = source.replace(/#\[[^\]]*\]/g, "").replace(/\/\/[^\n]*/g, "");
  for (const m of bare.matchAll(/^(?:pub )?struct (\w+) \{/gm)) {
    const [body] = enclosed(bare, m.index! + m[0].length - 1, "{", "}");
    const fields = split(body).map((f) => {
      const [name, ...ty] = f.replace(/^pub(\([^)]*\))?\s+/, "").split(":");
      return { name: name.trim(), ty: parseType(tokens(ty.join(":"))) };
    });
    found.set(m[1], { name: m[1], fields });
  }
  return found;
}

const camel = (name: string) => name.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());

// How a declaration says a place takes a value: its type, and whether its
// key or parameter may be left out, `x?: T`.
type Decl = { ty: string; optional: boolean };

// A `None` as TypeScript gives it: `null`, `undefined`, or no key at all.
type NoneForm = "null" | "undefined" | "absent";
const noneForms: NoneForm[] = ["null", "undefined", "absent"];

// Whether a place declared `decl` admits `form` (#822).
function admits(decl: Decl | undefined, form: NoneForm): boolean {
  if (decl === undefined) return false;
  if (form === "null") return /(^|\|)\s*null\s*(\||$)/.test(decl.ty);
  if (form === "undefined") return decl.optional || /(^|\|)\s*undefined\s*(\||$)/.test(decl.ty);
  return decl.optional;
}

// What a value's made of beside its type: the `None` form a call gives its
// `Option`s, where the declaration of its place, `decl`, admits it; and the
// declared interfaces, each struct's fields' declarations.
type Ask = { form?: NoneForm; decl?: Decl; interfaces: Map<string, Map<string, Decl>>; depth?: number };

// A value of `ty`, in Rust and in TypeScript: the `seed`th, varied by call
// (`variant`); of an `Option` whose place admits the call's `None` form,
// that `None`, `absent` where it's no key or argument at all.
type Value = { rs: string; ts: string; absent?: boolean };
function value(ty: Ty, seed: number, variant: number, structsOf: Map<string, Struct>, ask: Ask): Value | null {
  const within = { ...ask, decl: undefined };
  switch (ty.k) {
    case "num": {
      const n = ty.n.startsWith("f") ? `${seed + variant + 0.5}` : `${seed + variant + 2}`;
      return { rs: `${n}${ty.n}`, ts: n };
    }
    case "bool": {
      const b = String((seed + variant) % 2 === 0);
      return { rs: b, ts: b };
    }
    case "str":
      return { rs: `"ab${seed}"`, ts: `"ab${seed}"` };
    case "string":
      return { rs: `String::from("ab${seed}")`, ts: `"ab${seed}"` };
    case "box": {
      const inner = value(ty.inner, seed, variant, structsOf, ask);
      return inner && { ...inner, rs: `Box::new(${inner.rs})` };
    }
    case "ref": {
      if (ty.mut) return null;
      const inner = value(ty.inner, seed, variant, structsOf, ask);
      return inner && { ...inner, rs: ty.inner.k === "str" ? inner.rs : `&${inner.rs}` };
    }
    case "vec":
    case "slice": {
      const items = [0, 1].map((i) => value(ty.inner, seed + i, variant, structsOf, within));
      if (items.some((i) => !i)) return null;
      const rs = items.map((i) => i!.rs).join(", ");
      return { rs: ty.k === "vec" ? `vec![${rs}]` : `[${rs}]`, ts: `[${items.map((i) => i!.ts).join(", ")}]` };
    }
    case "opt": {
      // Of a nullable `Option<Option<T>>`, ADR 1017's held form, `null` is
      // its `Some(None)`; else each form is `None`.
      if (ask.form !== undefined && admits(ask.decl, ask.form)) {
        const rs = ask.form === "null" && ty.inner.k === "opt" ? "Some(None)" : "None";
        return { rs, ts: ask.form === "null" ? "null" : "undefined", absent: ask.form === "absent" };
      }
      const inner = value(ty.inner, seed, variant, structsOf, within);
      if (inner) return { rs: `Some(${inner.rs})`, ts: inner.ts };
      // Of no value, a struct deep in itself, its `None`, as declared.
      const none = noneForms.find((f) => admits(ask.decl, f));
      if (none === undefined || ty.inner.k === "opt") return null;
      return { rs: "None", ts: none === "null" ? "null" : "undefined", absent: none === "absent" };
    }
    case "tuple": {
      const items = ty.items.map((t, i) => value(t, seed + i, variant, structsOf, within));
      if (items.some((i) => !i)) return null;
      return { rs: `(${items.map((i) => i!.rs).join(", ")}${items.length === 1 ? "," : ""})`, ts: `[${items.map((i) => i!.ts).join(", ")}]` };
    }
    case "fn": {
      const names = ty.inputs.map((_, i) => `a${i}`);
      const rsParams = ty.inputs.map((t, i) => `${names[i]}: ${rustType(t)}`).join(", ");
      const first = ty.inputs[0];
      const numeric = (t: Ty | undefined): boolean => !!t && (t.k === "num" || (t.k === "ref" && numeric(t.inner)));
      let body: Value | null;
      if (ty.output.k === "tuple" && ty.output.items.length === 0) body = { rs: "{}", ts: "{}" };
      else if (ty.output.k === "num" && numeric(first) && ty.output.n === (first.k === "ref" ? (first.inner as { n: string }).n : (first as { n: string }).n)) {
        const read = first.k === "ref" ? "*a0" : "a0";
        body = { rs: `${read} * 2 + 1`, ts: "a0 * 2 + 1" };
      } else body = value(ty.output, seed, variant, structsOf, within);
      return body && { rs: `|${rsParams}| ${body.rs}`, ts: `(${names.join(", ")}) => ${body.ts.startsWith("{") ? `(${body.ts})` : body.ts}`.replace("=> ({})", "=> {}") };
    }
    case "named": {
      const struct = structsOf.get(ty.n);
      // None of a struct that holds itself past a few levels: a list's
      // `next: Option<Box<Node>>` is `None` there, or no value.
      if (!struct || (ask.depth ?? 0) > 3) return null;
      // Each field as its interface declares it, a key left out where its
      // `None` is no key at all.
      const declaredFields = ask.interfaces.get(struct.name);
      const fields = struct.fields.map((f, i) => [f.name, value(f.ty, seed + i, variant, structsOf, { ...ask, decl: declaredFields?.get(camel(f.name)), depth: (ask.depth ?? 0) + 1 })] as const);
      if (fields.some(([, v]) => !v)) return null;
      return {
        rs: `${struct.name} { ${fields.map(([n, v]) => `${n}: ${v!.rs}`).join(", ")} }`,
        ts: `{ ${fields.filter(([, v]) => !v!.absent).map(([n, v]) => `${camel(n)}: ${v!.ts}`).join(", ")} }`,
      };
    }
  }
}

function rustType(ty: Ty): string {
  switch (ty.k) {
    case "num":
      return ty.n;
    case "bool":
      return "bool";
    case "str":
      return "str";
    case "string":
      return "String";
    case "ref":
      return `&${ty.mut ? "mut " : ""}${rustType(ty.inner)}`;
    case "vec":
      return `Vec<${rustType(ty.inner)}>`;
    case "slice":
      return `[${rustType(ty.inner)}]`;
    case "opt":
      return `Option<${rustType(ty.inner)}>`;
    case "box":
      return `Box<${rustType(ty.inner)}>`;
    case "tuple":
      return `(${ty.items.map(rustType).join(", ")}${ty.items.length === 1 ? "," : ""})`;
    case "fn":
      return "_";
    case "named":
      return ty.n;
  }
}

// Whether a value of `ty` is shown alike by Rust's `Show` and `JSON.stringify`.
function shown(ty: Ty, structsOf: Map<string, Struct>, within: Set<string> = new Set()): boolean {
  switch (ty.k) {
    case "ref":
    case "box":
    case "vec":
    case "slice":
    case "opt":
      return shown(ty.inner, structsOf, within) && !(ty.k === "opt" && ty.inner.k === "opt");
    case "tuple":
      return ty.items.length <= 6 && ty.items.every((t) => shown(t, structsOf, within));
    case "fn":
      return false;
    case "named":
      // Not a struct that holds itself, which a show written out field by
      // field never ends.
      if (within.has(ty.n)) return false;
      return structsOf.get(ty.n)?.fields.every((f) => shown(f.ty, structsOf, new Set([...within, ty.n]))) ?? false;
    default:
      return true;
  }
}

// What a `.d.ts` declares each exported function's parameters and result
// as, by its JS name: `export function name<T>(a: A, b?: B): R;`, as
// rust-js prints it (`typescript/`).
function declared(dts: string): Map<string, { params: Decl[]; returns: string }> {
  const found = new Map<string, { params: Decl[]; returns: string }>();
  for (const m of dts.matchAll(/^export function (\w+)/gm)) {
    let at = m.index! + m[0].length;
    if (dts[at] === "<") at = enclosed(dts, at, "<", ">")[1];
    const [params, end] = enclosed(dts, at, "(", ")");
    const from = end + dts.slice(end).match(/^:\s*/)![0].length;
    const returns = dts[from] === "{" ? dts.slice(from, enclosed(dts, from, "{", "}")[1]) : dts.slice(from, dts.indexOf(";", from));
    found.set(m[1], { params: split(params).map(decl), returns });
  }
  return found;
}

// A parameter's or a field's declaration, `name?: T`.
function decl(text: string): Decl {
  const colon = text.indexOf(":");
  return { ty: text.slice(colon + 1).trim().replace(/;$/, ""), optional: text.slice(0, colon).trim().endsWith("?") };
}

// Each interface a `.d.ts` declares, its fields' declarations by their JS
// names: `export interface Settings { width?: number | undefined; }`.
function interfaces(dts: string): Map<string, Map<string, Decl>> {
  const found = new Map<string, Map<string, Decl>>();
  for (const m of dts.matchAll(/^export interface (\w+)(?:<[^>]*>)? \{/gm)) {
    const [body] = enclosed(dts, m.index! + m[0].length - 1, "{", "}");
    const fields = new Map<string, Decl>();
    for (const line of body.split("\n").map((l) => l.trim())) {
      const field = /^(\w+)\??\s*:/.exec(line);
      if (field && !line.startsWith("//")) fields.set(field[1], decl(line));
    }
    found.set(m[1], fields);
  }
  return found;
}

// A TypeScript expression that shows `x`, of `ty`, as Rust's `Show` does:
// a `None` `null`, whatever its JS form, a struct each of its fields in
// order, so a key JS leaves out shows as Rust's `None` (#822).
function tsShow(ty: Ty, x: string, structsOf: Map<string, Struct>, depth = 0): string {
  const v = `e${depth}`;
  switch (ty.k) {
    case "ref":
    case "box":
      return tsShow(ty.inner, x, structsOf, depth);
    case "opt":
      return `(${x} == null ? "null" : ${tsShow(ty.inner, x, structsOf, depth)})`;
    case "vec":
    case "slice":
      return `"[" + ${x}.map((${v}) => ${tsShow(ty.inner, v, structsOf, depth + 1)}).join(",") + "]"`;
    case "tuple":
      return ty.items.length === 0 ? `"null"` : `"[" + [${ty.items.map((t, i) => tsShow(t, `${x}[${i}]`, structsOf, depth)).join(", ")}].join(",") + "]"`;
    case "named": {
      const fields = structsOf.get(ty.n)!.fields;
      return `"{" + [${fields.map((f) => `${JSON.stringify(`"${camel(f.name)}":`)} + ${tsShow(f.ty, `${x}.${camel(f.name)}`, structsOf, depth)}`).join(", ")}].join(",") + "}"`;
    }
    default:
      return `show(${x})`;
  }
}

// A declared type that's a box, `{ value: T }`, of a `&mut`.
const boxed = (declaredTy: Decl | string) => /^\{\s*value\s*:/.test(typeof declaredTy === "string" ? declaredTy : declaredTy.ty);

// The `None` forms some `Option` a call's parameters hold, or their
// structs' fields, is declared to take.
function formsOf(params: { ty: Ty }[], decls: Decl[], structsOf: Map<string, Struct>, declaredInterfaces: Map<string, Map<string, Decl>>): NoneForm[] {
  const admitted = (ty: Ty, d: Decl | undefined, form: NoneForm, depth: number): boolean => {
    if (ty.k === "ref" || ty.k === "box") return admitted(ty.inner, d, form, depth);
    if (ty.k === "opt") return admits(d, form);
    if (ty.k !== "named" || depth > 3) return false;
    const struct = structsOf.get(ty.n);
    const fields = declaredInterfaces.get(ty.n);
    return !!struct && !!fields && struct.fields.some((f) => admitted(f.ty, fields.get(camel(f.name)), form, depth + 1));
  };
  return noneForms.filter((form) => params.some((p, i) => admitted(p.ty, decls[i], form, 0)));
}

const harness = `trait Show {
    fn show(&self) -> String;
}
macro_rules! shown_as_display {
    ($($t:ty),*) => { $(impl Show for $t { fn show(&self) -> String { format!("{}", self) } })* };
}
shown_as_display!(u8, u16, u32, usize, i8, i16, i32, isize, f32, f64, bool);
impl Show for str {
    fn show(&self) -> String {
        format!("{:?}", self)
    }
}
impl Show for String {
    fn show(&self) -> String {
        self.as_str().show()
    }
}
impl<T: Show + ?Sized> Show for Box<T> {
    fn show(&self) -> String {
        (**self).show()
    }
}
impl<T: Show + ?Sized> Show for &T {
    fn show(&self) -> String {
        (**self).show()
    }
}
impl<T: Show> Show for [T] {
    fn show(&self) -> String {
        format!("[{}]", self.iter().map(|t| t.show()).collect::<Vec<_>>().join(","))
    }
}
impl<T: Show> Show for Vec<T> {
    fn show(&self) -> String {
        self.as_slice().show()
    }
}
impl<T: Show> Show for Option<T> {
    fn show(&self) -> String {
        self.as_ref().map_or("null".to_string(), |t| t.show())
    }
}
impl Show for () {
    fn show(&self) -> String {
        "null".to_string()
    }
}
impl<A: Show, B: Show> Show for (A, B) {
    fn show(&self) -> String {
        format!("[{},{}]", self.0.show(), self.1.show())
    }
}
macro_rules! shown_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Show),+> Show for ($($t,)+) {
            fn show(&self) -> String {
                format!("[{}]", [$(self.$i.show()),+].join(","))
            }
        }
    };
}
shown_tuple!(A 0, B 1, C 2);
shown_tuple!(A 0, B 1, C 2, D 3);
shown_tuple!(A 0, B 1, C 2, D 3, E 4);
shown_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
`;

// How many calls gave each `None` form, of every case run (#822).
const formCalls: Record<NoneForm, number> = { null: 0, undefined: 0, absent: 0 };
// And of which cases.
const formCases = new Set<string>();

// Each exported function of the corpus case `name` called from TypeScript
// as its `.d.ts` says, against Rust's calls: how many calls it made.
function conformance(name: string): number {
  const formsGiven: Record<NoneForm, number> = { null: 0, undefined: 0, absent: 0 };
  const file = join(root, "test/corpus", `${name}.rs`);
  const source = readFileSync(file, "utf8");
  const dir = fixture(`declarations-conformance-${name}`);
  writeFileSync(join(dir, "lib.rs"), `include!(${JSON.stringify(file)});\npub fn entry() {\n    main()\n}\n`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "-Awarnings"]);
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const params = declared(dts);
  const declaredInterfaces = interfaces(dts);
  const structsOf = structs(source);
  const rs: string[] = [];
  const tsLines: string[] = [];
  const imported = new Set<string>();
  for (const fn of functions(source)) {
    const jsName = camel(fn.name);
    const declaredFn = params.get(jsName);
    if (!declaredFn || declaredFn.params.length !== fn.params.length + fn.dictionaries.length || !fn.output || !shown(fn.output, structsOf)) continue;
    // Some, and again with each `bool` the other way; then each `None`
    // form its parameters' declarations admit (#822).
    const flips = fn.params.some((p) => p.ty.k === "opt" || p.ty.k === "bool" || (p.ty.k === "ref" && p.ty.inner.k === "opt"));
    const forms = formsOf(fn.params, declaredFn.params, structsOf, declaredInterfaces);
    const variants: { variant: number; form?: NoneForm }[] = [{ variant: 0 }, ...(flips ? [{ variant: 1 }] : []), ...forms.map((form) => ({ variant: 1, form }))];
    for (const { variant, form } of variants) {
      const rsArgs: string[] = [];
      const tsArgs: { ts: string; absent?: boolean }[] = [];
      const rsBefore: string[] = [];
      const tsBefore: string[] = [];
      const rsAfter: string[] = [];
      const tsAfter: string[] = [];
      let ok = true;
      fn.params.forEach((p, i) => {
        const local = `v${i}`;
        const pointee = p.ty.k === "ref" && p.ty.mut ? p.ty.inner : null;
        const ask: Ask = { form, decl: pointee ? undefined : declaredFn.params[i], interfaces: declaredInterfaces };
        const v = value(pointee ?? p.ty, i, variant, structsOf, ask);
        if (!v || (pointee && !shown(pointee, structsOf))) {
          ok = false;
          return;
        }
        if (!pointee) {
          rsArgs.push(v.rs);
          tsArgs.push(v);
          return;
        }
        // A `&mut`'s value is a box or the object, as its declaration says.
        const inBox = boxed(declaredFn.params[i]);
        rsBefore.push(`let mut ${local}: ${rustType(pointee)} = ${v.rs};`);
        rsArgs.push(`&mut ${local}`);
        rsAfter.push(`${local}.show()`);
        tsBefore.push(`const ${local} = ${inBox ? `{ value: ${v.ts} }` : v.ts};`);
        tsArgs.push({ ts: local });
        tsAfter.push(tsShow(pointee, `${local}${inBox ? ".value" : ""}`, structsOf));
      });
      if (!ok) continue;
      // An argument that's no argument at all is left out where none after
      // it is given, as JS leaves out an optional parameter; before one
      // that is, `undefined`, which its `?` admits too.
      while (tsArgs.at(-1)?.absent) tsArgs.pop();
      const given = tsArgs.map((a) => (a.absent ? "undefined" : a.ts));
      imported.add(jsName);
      // Each impl's dictionary, after the arguments, as declared.
      for (const dictionary of fn.dictionaries) imported.add(dictionary);
      given.push(...fn.dictionaries);
      // A `&mut` it gives, the box or the object, as declared.
      const outBoxed = fn.output.k === "ref" && fn.output.mut && boxed(declaredFn.returns);
      const result = tsShow(outBoxed && fn.output.k === "ref" ? fn.output.inner : fn.output, outBoxed ? "out.value" : "out", structsOf);
      const label = `${fn.name}#${variant}${form ? ` ${form}` : ""}${fn.dictionaries.map((d) => ` ${d}`).join("")}`;
      if (form !== undefined) formsGiven[form]++;
      rs.push(`    {\n        ${rsBefore.join("\n        ")}\n        let out = ${fn.name}(${rsArgs.join(", ")});\n        println!("{} {}", ${JSON.stringify(label)}, [out.show(), ${rsAfter.join(", ")}].join(" "));\n    }`);
      tsLines.push(`{\n  ${tsBefore.join("\n  ")}\n  const out = ${jsName}(${given.join(", ")});\n  console.log(${JSON.stringify(label)}, [${result}, ${tsAfter.join(", ")}].join(" "));\n}`);
    }
  }
  if (rs.length === 0) return 0;
  // Each struct shown by its fields, as its JS object's keys are.
  const shows = [...structsOf.values()]
    .filter((s) => shown({ k: "named", n: s.name }, structsOf))
    .map((s) => `impl Show for ${s.name} {\n    fn show(&self) -> String {\n        format!("{{${s.fields.map((f) => `\\"${camel(f.name)}\\":{}`).join(",")}}}", ${s.fields.map((f) => `self.${f.name}.show()`).join(", ")})\n    }\n}`);
  const native = `mod case {\n    include!(${JSON.stringify(file)});\n    ${harness}\n    ${shows.join("\n")}\n    pub fn conformance() {\n${rs.join("\n")}\n    }\n}\nfn main() {\n    case::conformance();\n}\n`;
  const built = nativeBinary(native, dir, ["--edition=2024", "-Coverflow-checks=off", "-Awarnings"]);
  if ("error" in built) throw new Error(`${name}: ${built.error}`);
  const expected = run([built.binary]);
  writeFileSync(
    join(dir, "caller.ts"),
    `import { ${[...imported].join(", ")} } from "./lib.js";\nconst show = (v: unknown): string => JSON.stringify(v === undefined ? null : v);\n${tsLines.join("\n")}\n`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", strict: true, allowJs: false, skipLibCheck: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["caller.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect([name, errors]).toEqual([name, []]);
  // The caller's JS beside the crate's, which `./lib.js` names.
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  const got = run(["node", join(dir, "caller.js")]);
  expect([name, got]).toEqual([name, expected]);
  for (const form of noneForms) formCalls[form] += formsGiven[form];
  if (noneForms.some((form) => formsGiven[form] > 0)) formCases.add(name);
  return rs.length;
}

test("each exported function's TypeScript caller type-checks and does what Rust does", () => {
  let calls = 0;
  for (const name of cases) {
    const made = conformance(name);
    expect([name, made > 0]).toEqual([name, true]);
    calls += made;
  }
  expect(calls).toBeGreaterThan(30);
}, 600_000);

// Each other corpus case whose public functions or structs hold an
// `Option` (#822): its calls of each `None` form its `.d.ts` admits, where
// the generator has a value of each of a function's parameters.
const optionCases = readdirSync(join(root, "test/corpus"))
  .filter((file) => file.endsWith(".rs"))
  .map((file) => file.slice(0, -".rs".length))
  .filter((name) => {
    if (cases.includes(name)) return false;
    const source = readFileSync(join(root, "test/corpus", `${name}.rs`), "utf8");
    // Not one rust-js refuses, whose refusal is its answer.
    return /^pub (fn|struct) /m.test(source) && source.includes("Option<") && !source.startsWith("//@ compile-fail");
  });

for (const name of optionCases) {
  test(`${name}: each None form its .d.ts admits does what Rust's None does`, () => {
    conformance(name);
  }, 300_000);
}

// The net isn't empty: each `None` form is given by some call, of many
// cases, once each case above has run.
test("each None form is given, of many cases", () => {
  console.log(`None forms given: ${JSON.stringify(formCalls)}, of ${formCases.size} cases`);
  for (const form of noneForms) expect([form, formCalls[form] > 0]).toEqual([form, true]);
  expect(formCases.size).toBeGreaterThan(10);
});
