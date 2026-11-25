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
import { buildReact, compiler, fixture, nativeBinary, programFlags, root, run, target } from "./support";
import { readdirSync } from "node:fs";
import { repoTypes, typeChecked } from "./type-checked";
import type { Tree } from "./generated-calls";

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

// What the compiler says a crate offers JS, its manifest's `offered_paths`
// (rust_js_program::offered, ADR 1204): each function's parameters, result
// and bounds, each struct's fields and whether it's sealed, each by its JS
// name, read of rustc's own answers, so no parse of the source decides a
// signature apart from the compiler's (#2573).
type Offered = {
  rust: string[];
  js: string[];
  kind: "function" | "const" | "static" | "namespace" | "type";
  params?: Tree[];
  ty?: Tree;
  fields?: { rust: string; js: string; ty: Tree; written: string }[];
  sealed?: boolean;
  unoffered_fields?: "symbol_keyed";
  type_params?: number;
  bounds?: { param: string; traits?: string[]; callable?: Tree; impls?: { ty: Tree; dictionary: string }[] }[];
};

// What a bound of no trait of the case's asks, a `Clone` or an `Ord`, of
// which a `u32` is one, given in its type parameter's place.
const stdBounds = new Set(["Clone", "Copy", "PartialOrd", "PartialEq", "Ord", "Eq", "Debug", "Sized", "MetaSized"]);

// The records' type, `tree`, as a value's type here, each type parameter
// its choice in `generics`; `null` of one it has none of.
function typeOf(tree: Tree, generics: Map<string, Ty | null>): Ty | null {
  const of = (inner: Tree) => typeOf(inner, generics);
  switch (tree.kind) {
    case "int":
    case "float":
      return { k: "num", n: tree.rust };
    case "bool":
      return { k: "bool" };
    // A `&str`, which the records hold as one form, a reference to a `str`
    // here.
    case "str":
      return tree.owned ? { k: "string" } : { k: "ref", mut: false, inner: { k: "str" } };
    case "unit":
      return { k: "tuple", items: [] };
    case "option": {
      const inner = of(tree.of);
      return inner && { k: "opt", inner };
    }
    case "vec": {
      const inner = of(tree.of);
      return inner && (tree.borrowed ? { k: "ref", mut: false, inner: { k: "slice", inner } } : { k: "vec", inner });
    }
    case "ref": {
      const inner = of(tree.of);
      return inner && { k: "ref", mut: tree.mutable === true, inner };
    }
    case "box": {
      const inner = of(tree.of);
      return inner && { k: "box", inner };
    }
    case "tuple": {
      const items = tree.items.map(of);
      return items.every((i) => i) ? { k: "tuple", items: items as Ty[] } : null;
    }
    case "fn": {
      const inputs = tree.inputs.map(of);
      const output = of(tree.output);
      return inputs.every((i) => i) && output ? { k: "fn", inputs: inputs as Ty[], output } : null;
    }
    case "param":
      return generics.get(tree.name) ?? null;
    case "named":
      return { k: "named", n: tree.rust.at(-1)! };
    // Any other by its name, which no value is made of but a `None` of an
    // `Option` of it.
    case "other":
      return { k: "named", n: tree.rust };
    default:
      return null;
  }
}

type Fn = { name: string; js: string; params: { name: string; ty: Ty }[]; output: Ty | null; dictionaries: string[] };

// Each function of the crate's root the records say it offers, of each
// choice of its type parameters: bound by a callable, that function; by
// no trait of the case's, a `u32`; by one of the case's, each impl rustc
// has of it in turn, a function of each, given that impl's dictionary by
// the name the records give it.
function functions(offered: Offered[]): Fn[] {
  const found: Fn[] = [];
  for (const path of offered.filter((p) => p.kind === "function" && p.rust.length === 1)) {
    const generics = new Map<string, Ty | null>();
    const ofTraits: [string, { ty: Tree; dictionary: string }[]][] = [];
    for (const bound of path.bounds ?? []) {
      if (bound.callable) generics.set(bound.param, typeOf(bound.callable, generics));
      else if ((bound.impls ?? []).length > 0) ofTraits.push([bound.param, bound.impls!]);
      else generics.set(bound.param, (bound.traits ?? []).every((t) => stdBounds.has(t.split("::").at(-1)!)) ? { k: "num", n: "u32" } : null);
    }
    let choices: { ty: Tree; dictionary: string }[][] = [[]];
    for (const [, of] of ofTraits) choices = choices.flatMap((chosen) => of.map((imp) => [...chosen, imp]));
    for (const chosen of choices) {
      ofTraits.forEach(([param], i) => generics.set(param, typeOf(chosen[i].ty, generics)));
      const params = (path.params ?? []).map((tree, i) => ({ name: `p${i}`, ty: typeOf(tree, generics) }));
      const output = path.ty ? typeOf(path.ty, generics) : { k: "tuple" as const, items: [] };
      if (params.length > 0 && params.every((p) => p.ty)) {
        found.push({ name: path.rust[0], js: path.js[0], params: params as Fn["params"], output, dictionaries: chosen.map((imp) => imp.dictionary) });
      }
    }
  }
  return found;
}

type Struct = { name: string; fields: { name: string; js: string; ty: Ty; written: string }[]; sealed: boolean };

// Each struct of named fields the records say, by its name, each field by
// its Rust name, its JS name and its type; `sealed` as they say it.
function structs(offered: Offered[]): Map<string, Struct> {
  const found = new Map<string, Struct>();
  // Of the root's, of no type parameters: a generic one's maker would be
  // generic, which a caller's value can't choose for it.
  for (const path of offered.filter((p) => p.kind === "type" && p.rust.length === 1 && !p.type_params && (p.fields ?? []).length > 0)) {
    const fields = path.fields!.map((f) => ({ name: f.rust, js: f.js, ty: typeOf(f.ty, new Map()) ?? { k: "named" as const, n: f.written }, written: f.written }));
    found.set(path.rust.at(-1)!, { name: path.rust.at(-1)!, fields, sealed: path.sealed === true });
  }
  return found;
}

// A crate's records: `dir`'s `lib.rs` compiled with its manifest.
function records(dir: string, flags: string[] = []): Offered[] {
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", join(dir, "lib.manifest.json"), "--", "--edition=2024", "-Awarnings", ...flags]);
  return (JSON.parse(readFileSync(join(dir, "lib.manifest.json"), "utf8")).offered_paths ?? []) as Offered[];
}

// What a sealed struct's TypeScript caller has of one: its crate's own
// maker's, as another crate's caller would, which the harness adds to the
// crate, `pub fn make_clicks(n: u32) -> Clicks`, by the JS name the
// records give it; none of one whose fields aren't each owned.
const makerOf = (struct: Struct) => `make_${struct.name.replace(/[A-Z]/g, (c, i: number) => `${i ? "_" : ""}${c.toLowerCase()}`)}`;
const owned = (ty: Ty): boolean => ty.k !== "ref" && ty.k !== "fn" && ty.k !== "slice" && !("inner" in ty && !owned(ty.inner)) && !(ty.k === "tuple" && !ty.items.every(owned));
function makers(structsOf: Map<string, Struct>): string {
  return [...structsOf.values()]
    // Not of one of a lifetime, `Iter<'a>`, whose maker would need it.
    .filter((s) => s.sealed && s.fields.every((f) => owned(f.ty) && !/'(?!static\b)\w/.test(f.written)))
    .map((s) => `pub fn ${makerOf(s)}(${s.fields.map((f) => `${f.name}: ${f.written}`).join(", ")}) -> ${s.name} {\n    ${s.name} { ${s.fields.map((f) => f.name).join(", ")} }\n}\n`)
    .join("");
}

// The `.d.ts` read as text, its declarations' brackets and lists.
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
// Each maker's JS name, `make_clicks`'s, as the case's records give it.
const makerNames = new Map<string, string>();

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
      return { rs: `::std::string::String::from("ab${seed}")`, ts: `"ab${seed}"` };
    case "box": {
      const inner = value(ty.inner, seed, variant, structsOf, ask);
      return inner && { ...inner, rs: `::std::boxed::Box::new(${inner.rs})` };
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
      return { rs: ty.k === "vec" ? `::std::vec![${rs}]` : `[${rs}]`, ts: `[${items.map((i) => i!.ts).join(", ")}]` };
    }
    case "opt": {
      // Of a nullable `Option<Option<T>>`, ADR 1017's held form, `null` is
      // its `Some(None)`; else each form is `None`.
      if (ask.form !== undefined && admits(ask.decl, ask.form)) {
        const rs = ask.form === "null" && ty.inner.k === "opt" ? "::std::option::Option::Some(::std::option::Option::None)" : "::std::option::Option::None";
        return { rs, ts: ask.form === "null" ? "null" : "undefined", absent: ask.form === "absent" };
      }
      const inner = value(ty.inner, seed, variant, structsOf, within);
      if (inner) return { rs: `::std::option::Option::Some(${inner.rs})`, ts: inner.ts };
      // Of no value, a struct deep in itself, its `None`, as declared.
      const none = noneForms.find((f) => admits(ask.decl, f));
      if (none === undefined || ty.inner.k === "opt") return null;
      return { rs: "::std::option::Option::None", ts: none === "null" ? "null" : "undefined", absent: none === "absent" };
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
      const fields = struct.fields.map((f, i) => [f, value(f.ty, seed + i, variant, structsOf, { ...ask, decl: declaredFields?.get(f.js), depth: (ask.depth ?? 0) + 1 })] as const);
      if (fields.some(([, v]) => !v)) return null;
      if (struct.sealed) {
        if (!struct.fields.every((f) => owned(f.ty))) return null;
        return {
          rs: `${struct.name} { ${fields.map(([f, v]) => `${f.name}: ${v!.rs}`).join(", ")} }`,
          ts: `${makerNames.get(makerOf(struct)) ?? makerOf(struct)}(${fields.map(([, v]) => (v!.absent ? "undefined" : v!.ts)).join(", ")})`,
        };
      }
      return {
        rs: `${struct.name} { ${fields.map(([f, v]) => `${f.name}: ${v!.rs}`).join(", ")} }`,
        ts: `{ ${fields.filter(([, v]) => !v!.absent).map(([f, v]) => `${f.js}: ${v!.ts}`).join(", ")} }`,
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
      return "::std::string::String";
    case "ref":
      return `&${ty.mut ? "mut " : ""}${rustType(ty.inner)}`;
    case "vec":
      return `::std::vec::Vec<${rustType(ty.inner)}>`;
    case "slice":
      return `[${rustType(ty.inner)}]`;
    case "opt":
      return `::std::option::Option<${rustType(ty.inner)}>`;
    case "box":
      return `::std::boxed::Box<${rustType(ty.inner)}>`;
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
      const struct = structsOf.get(ty.n)!;
      // A sealed one's fields, which its opaque declaration doesn't name,
      // read of its JS to compare with Rust's (ADR 1187).
      const of = struct.sealed ? `(${x} as unknown as Record<string, any>)` : x;
      return `"{" + [${struct.fields.map((f) => `${JSON.stringify(`"${f.js}":`)} + ${tsShow(f.ty, `${of}.${f.js}`, structsOf, depth)}`).join(", ")}].join(",") + "}"`;
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
    return !!struct && !!fields && struct.fields.some((f) => admitted(f.ty, fields.get(f.js), form, depth + 1));
  };
  return noneForms.filter((form) => params.some((p, i) => admitted(p.ty, decls[i], form, 0)));
}

const harness = `trait Show {
    fn show(&self) -> ::std::string::String;
}
macro_rules! shown_as_display {
    ($($t:ty),*) => { $(impl Show for $t { fn show(&self) -> ::std::string::String { ::std::format!("{}", self) } })* };
}
shown_as_display!(u8, u16, u32, usize, i8, i16, i32, isize, f32, f64, bool);
impl Show for str {
    fn show(&self) -> ::std::string::String {
        ::std::format!("{:?}", self)
    }
}
impl Show for ::std::string::String {
    fn show(&self) -> ::std::string::String {
        self.as_str().show()
    }
}
impl<T: Show + ?::core::marker::Sized> Show for ::std::boxed::Box<T> {
    fn show(&self) -> ::std::string::String {
        (**self).show()
    }
}
impl<T: Show + ?::core::marker::Sized> Show for &T {
    fn show(&self) -> ::std::string::String {
        (**self).show()
    }
}
impl<T: Show> Show for [T] {
    fn show(&self) -> ::std::string::String {
        ::std::format!("[{}]", self.iter().map(|t| t.show()).collect::<::std::vec::Vec<_>>().join(","))
    }
}
impl<T: Show> Show for ::std::vec::Vec<T> {
    fn show(&self) -> ::std::string::String {
        self.as_slice().show()
    }
}
impl<T: Show> Show for ::std::option::Option<T> {
    fn show(&self) -> ::std::string::String {
        self.as_ref().map_or("null".to_string(), |t| t.show())
    }
}
impl Show for () {
    fn show(&self) -> ::std::string::String {
        "null".to_string()
    }
}
impl<A: Show, B: Show> Show for (A, B) {
    fn show(&self) -> ::std::string::String {
        ::std::format!("[{},{}]", self.0.show(), self.1.show())
    }
}
macro_rules! shown_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Show),+> Show for ($($t,)+) {
            fn show(&self) -> ::std::string::String {
                ::std::format!("[{}]", [$(self.$i.show()),+].join(","))
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
  const dir = fixture(`declarations-conformance-${name}`);
  // Its records, then again with its sealed structs' makers, which the
  // first's say: what it offers, by the compiler's answers (#2573).
  const crate = (more: string) => writeFileSync(join(dir, "lib.rs"), `include!(${JSON.stringify(file)});\npub fn entry() {\n    main()\n}\n${more}`);
  crate("");
  const structsOf = structs(records(dir));
  const made = makers(structsOf);
  crate(made);
  const offered = records(dir);
  const makersMade = new Set([...made.matchAll(/^pub fn (\w+)/gm)].map((m) => m[1]));
  makerNames.clear();
  for (const path of offered) if (path.kind === "function" && makersMade.has(path.rust[0])) makerNames.set(path.rust[0], path.js[0]);
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const params = declared(dts);
  const declaredInterfaces = interfaces(dts);
  const rs: string[] = [];
  const tsLines: string[] = [];
  const imported = new Set<string>();
  for (const fn of functions(offered).filter((fn) => !makersMade.has(fn.name) && fn.name !== "entry")) {
    const jsName = fn.js;
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
      // A sealed struct's maker its arguments call (`makers`).
      for (const maker of given.join(" ").matchAll(/\b(make[A-Z]\w*)\(/g)) imported.add(maker[1]!);
      // Each impl's dictionary, after the arguments, as declared.
      for (const dictionary of fn.dictionaries) imported.add(dictionary);
      given.push(...fn.dictionaries);
      // A `&mut` it gives, the box or the object, as declared.
      const outBoxed = fn.output.k === "ref" && fn.output.mut && boxed(declaredFn.returns);
      const result = tsShow(outBoxed && fn.output.k === "ref" ? fn.output.inner : fn.output, outBoxed ? "out.value" : "out", structsOf);
      const label = `${fn.name}#${variant}${form ? ` ${form}` : ""}${fn.dictionaries.map((d) => ` ${d}`).join("")}`;
      if (form !== undefined) formsGiven[form]++;
      rs.push(`    {\n        ${rsBefore.join("\n        ")}\n        let out = ${fn.name}(${rsArgs.join(", ")});\n        ::std::println!("{} {}", ${JSON.stringify(label)}, [out.show(), ${rsAfter.join(", ")}].join(" "));\n    }`);
      tsLines.push(`{\n  ${tsBefore.join("\n  ")}\n  const out = ${jsName}(${given.join(", ")});\n  console.log(${JSON.stringify(label)}, [${result}, ${tsAfter.join(", ")}].join(" "));\n}`);
    }
  }
  if (rs.length === 0) return 0;
  // Each struct shown by its fields, as its JS object's keys are.
  const shows = [...structsOf.values()]
    .filter((s) => shown({ k: "named", n: s.name }, structsOf))
    .map((s) => `impl Show for ${s.name} {\n    fn show(&self) -> ::std::string::String {\n        ::std::format!("{{${s.fields.map((f) => `\\"${f.js}\\":{}`).join(",")}}}", ${s.fields.map((f) => `self.${f.name}.show()`).join(", ")})\n    }\n}`);
  const native = `mod case {\n    include!(${JSON.stringify(file)});\n    ${harness}\n    ${shows.join("\n")}\n    pub fn conformance() {\n${rs.join("\n")}\n    }\n}\nfn main() {\n    case::conformance();\n}\n`;
  const built = nativeBinary(native, dir, [...programFlags("2024"), "-Awarnings"]);
  if ("error" in built) throw new Error(`${name}: ${built.error}`);
  const expected = run([built.binary]);
  writeFileSync(
    join(dir, "caller.ts"),
    `import { ${[...imported].join(", ")} } from "./lib.js";\nconst show = (v: unknown): string => JSON.stringify(v === undefined ? null : v);\n${tsLines.join("\n")}\n`,
  );
  const errors = typeChecked(dir, "caller.ts", { ...repoTypes, target: "es2022", allowJs: false, skipLibCheck: true, noEmit: false });
  expect([name, errors]).toEqual([name, []]);
  // The caller's JS beside the crate's, which `./lib.js` names.
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  const got = run(["node", join(dir, "caller.js")]);
  expect([name, got]).toEqual([name, expected]);
  for (const form of noneForms) formCalls[form] += formsGiven[form];
  if (noneForms.some((form) => formsGiven[form] > 0)) formCases.add(name);
  return rs.length;
}

// A type's name is rustc's answer, not a reading of the source's: a case's
// own `String`, `Vec`, `Option` or `Box`, declared or imported, is its own
// type, not std's (#1875); std's, imported by name, is std's.
test("a case's own type of a std type's name is its own", () => {
  const dir = fixture("declarations-conformance-own-names");
  writeFileSync(join(dir, "lib.rs"), `mod own {
    pub struct String(pub u8);
    pub enum Option { A }
}
use own::{Option, String};
use std::collections::HashMap as Vec;
pub fn own_string(s: String) -> u8 { s.0 }
pub fn own_option(o: Option) -> u8 { match o { Option::A => 1 } }
pub fn map_named_vec(v: Vec<u8, u8>) -> usize { v.len() }
pub fn std_string(s: ::std::string::String, b: Box<::std::string::String>) -> usize { s.len() + b.len() }
`);
  const params = (name: string) => functions(records(dir)).find((fn) => fn.name === name)?.params.map((p) => p.ty);
  expect(params("own_string")).toEqual([{ k: "named", n: "String" }]);
  expect(params("own_option")).toEqual([{ k: "named", n: "Option" }]);
  expect(params("map_named_vec")?.map((ty) => ty.k)).toEqual(["named"]);
  expect(params("std_string")).toEqual([{ k: "string" }, { k: "box", inner: { k: "string" } }]);
});

// A `dyn Fn` is the function it is, its inputs and output rustc's, so a
// case's `Option<&dyn Fn(u32)>` is called with one (#2573).
test("a dyn Fn is read as the function it is", () => {
  const dir = fixture("declarations-conformance-dyn-fn");
  writeFileSync(join(dir, "lib.rs"), "pub fn call_dyn(f: &dyn Fn(u32) -> u32, g: Option<&dyn Fn(u8)>) -> u32 {\n    if let Some(g) = g {\n        g(2);\n    }\n    f(1)\n}\n");
  const [fn] = functions(records(dir)).filter((fn) => fn.name === "call_dyn");
  const u32: Ty = { k: "num", n: "u32" };
  expect(fn?.params.map((p) => p.ty)).toEqual([
    { k: "ref", mut: false, inner: { k: "fn", inputs: [u32], output: u32 } },
    { k: "opt", inner: { k: "ref", mut: false, inner: { k: "fn", inputs: [{ k: "num", n: "u8" }], output: { k: "tuple", items: [] } } } },
  ]);
});

// A struct of a field a JS symbol names (ADR 1051) says its fields aren't
// offered, and why, so no reader of `fields` takes its silence for none: its
// fields' key form is still to come (#2583).
test("a struct of a symbol-keyed field says its fields aren't offered", () => {
  const dir = fixture("declarations-conformance-symbol-keyed");
  writeFileSync(join(dir, "lib.rs"), `use js::Symbol;

thread_local! {
    static RESPONSE: &'static Symbol = js::symbol::new(None);
}

pub struct Reason {
    pub id: u32,
    #[cfg_attr(rust_js, rust_js::name(RESPONSE))]
    pub response: String,
}

pub struct Plain {
    pub id: u32,
}

pub fn reason(id: u32) -> Reason {
    Reason { id, response: String::new() }
}
`);
  // Given the `js` crate, which the bindings' build makes.
  buildReact();
  const types = records(dir, ["--crate-type", "lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]).filter((p) => p.kind === "type").map((p) => [p.rust.join("::"), p.unoffered_fields ?? null, (p.fields ?? []).map((f) => f.js)]);
  expect(types).toEqual([["Reason", "symbol_keyed", []], ["Plain", null, ["id"]]]);
}, 600_000);

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
