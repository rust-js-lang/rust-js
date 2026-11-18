// The js crate's binding forms, written once: each is how a TypeScript
// shape is bound in Rust, which the hand-kept specs, `builtins/intl.ts`, and
// what's generated from TypeScript's own library, `builtins/library.ts`,
// both write by (#1904). One file is written per run, its lines in `out`.

import { needsJsName, snake } from "../scripts/rust-names";

export const page = (path: string) => `https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Intl/${path}`;
/** A JS value's Rust variant: `"best fit"` `BestFit`, `"2-digit"` `TwoDigit`. */
export const variant = (js: string) =>
  js.replace(/^2-/, "two-").split(/[^A-Za-z0-9]+/).map((w) => w[0].toUpperCase() + w.slice(1)).join("");

export const out: string[] = [];
export const line = (s = "") => out.push(s);
export const close = () => {
  if (out.at(-1) === "") out.pop();
};

export const stringUnions = new Map<string, string[]>();

/** An enum of a string union's values, each its JS. */
export const union = (name: string, doc: string, values: string[]) => {
  stringUnions.set(name, values);
  line(`/// ${doc}`);
  line(`#[derive(Clone, Copy, Debug, PartialEq, Eq)]`);
  line(`pub enum ${name} {`);
  for (const v of values) {
    line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(v)})]`);
    line(`    ${variant(v)},`);
  }
  line("}");
  line();
};

/** A field: its JS name, and its Rust type. */
export type Field = [js: string, ty: string];
export const field = ([js, ty]: Field) => {
  const name = snake(js);
  if (needsJsName(name, js)) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(js)})]`);
  line(`    pub ${name}: ${ty},`);
};
/** What's given: each field an `Option`, `None` not given, as JS reads
 * an `undefined` option. A field given as required isn't. */
/** The options structs that borrow text, `<'a>`. */
export const borrowing = new Set<string>();
export const options = (name: string, doc: string, fields: Field[], required: string[] = [], types: string | null = `Intl.${name}`) => {
  const lifetime = fields.some(([, ty]) => ty.includes("'a"));
  if (lifetime) borrowing.add(name);
  line(`/// ${doc}`);
  line(`/// A JS object of these fields, a \`None\` one not given${required.length ? "" : ": `..Default::default()` the rest"}.`);
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(types ?? anonymous(fields, required))})]`);
  if (!required.length) line("#[derive(Default)]");
  line(`pub struct ${name}${lifetime ? "<'a>" : ""} {`);
  for (const [js, ty] of fields) field([js, required.includes(js) ? ty : `Option<${ty}>`]);
  line("}");
  line();
};
/** An anonymous native record, from the same fields that write its Rust.
 * Intl has no names for SupportedLocalesOptions or ListFormat's part. */
export const anonymous = (fields: Field[], required: string[]) => "{ " + fields.map(([js, rust]) => {
  const optional = rust.startsWith("Option<");
  const type = optional ? rust.slice(7, -1) : rust;
  const union = stringUnions.get(type);
  const value = type === "String" || /^&(?:'\w+ )?str$/.test(type) ? "string"
    : union ? union.map(value => JSON.stringify(value)).join(" | ")
    : undefined;
  if (!value) throw new Error("anonymous Intl field type is not supported: " + rust);
  return js + (optional || !required.includes(js) ? "?" : "") + ": " + value;
}).join("; ") + " }";

/** What's read: a JS object's fields as it gives them. */
export const record = (name: string, doc: string, fields: Field[], types: string | null = `Intl.${name}`) => {
  line(`/// ${doc}`);
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(types ?? anonymous(fields, fields.map(([name]) => name)))})]`);
  line(`#[derive(Clone, Debug, PartialEq)]`);
  line(`pub struct ${name} {`);
  for (const f of fields) field(f);
  line("}");
  line();
};

/** A method: its doc, its JS, its Rust signature. */
export const method = (doc: string, js: string, signature: string, indent = "    ", attributes: string[] = []) => {
  line(`${indent}/// ${doc}`);
  line(`${indent}#[cfg_attr(rust_js, rust_js::link_name = ${JSON.stringify(js)})]`);
  for (const attribute of attributes) line(`${indent}#[cfg_attr(rust_js, rust_js::${attribute})]`);
  line(`${indent}pub fn ${signature} {`);
  line(`${indent}    unreachable!()`);
  line(`${indent}}`);
  line();
};

/** `name`'s class, which `js::class::<T>()` names, held: made by its
 * module's constructor (ADR 1049). */
export const classed = (name: string) => {
  line(`impl super::Classed for ${name} {`);
  line("    type Constructor = super::AnyConstructor;");
  line("}");
};


// Argument membership is written once here; sealed implementations and the
// public erased input trait are derived from it (ADRs 0229, 0443).
export const argument = (name: string, type: string, members: string[], lists = false) => {
  const module = snake(name);
  line(`mod ${module} {`);
  if (members.some(member => /Date|Locale|BigInt/.test(member))) line("    use super::*;");
  line("    pub trait Sealed {}");
  if (lists) line("    pub trait Element {}");
  for (const member of members) {
    line(`    impl Sealed for ${member} {}`);
    if (lists) line(`    impl Element for ${member} {}`);
  }
  if (lists) {
    for (const member of ["&[T]", "Vec<T>", "&Vec<T>"]) line(`    impl<T: Element> Sealed for ${member} {}`);
    for (const member of ["[T; N]", "&[T; N]"]) line(`    impl<T: Element, const N: usize> Sealed for ${member} {}`);
  }
  line("}");
  line(`/// A \`${type}\` input: each member as it is, without a wrapper.`);
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(type)})]`);
  line(`pub trait ${name}: ${module}::Sealed {}`);
  line(`impl<T: ${module}::Sealed> ${name} for T {}`);
  line();
};
