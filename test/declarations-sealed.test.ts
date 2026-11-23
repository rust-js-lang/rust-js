// What a crate's declarations say of a tuple struct no caller outside the
// crate can make, one with a field that isn't `pub`, `Email(String)`, which
// only its crate's checks make (#2356, ADR 1187): an interface branded by a
// `unique symbol` of its module's own, as std's opaque types are (ADR 1149),
// not `any`. TypeScript then takes one only of what rust-js gave, as Rust
// takes one only of what the crate made.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = es2022;

test("a tuple struct of a private field is declared opaque, branded by its own symbol", async () => {
  const dir = fixture("declarations-sealed");
  writeFileSync(join(dir, "lib.rs"), `pub mod mail;

use mail::Email;

/// A name that isn't blank.
pub struct Name(String);

impl Name {
    pub fn new(text: &str) -> Option<Name> {
        (!text.trim().is_empty()).then(|| Name(text.to_string()))
    }
}

pub fn welcome(name: &Name, email: &Email) -> String {
    format!("Welcome, {}! We'll write to {}.", name.0, email.as_str())
}
`);
  writeFileSync(join(dir, "mail.rs"), `/// An email address, which only \`parse\` makes.
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn parse(text: &str) -> Result<Email, String> {
    if text.contains('@') { Ok(Email(text.to_string())) } else { Err("an email has an @".to_string()) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare const $nameBrand: unique symbol;");
  expect(declarations).toContain("/** A name that isn't blank. */\nexport interface Name {\n    readonly [$nameBrand]: never;\n}");
  expect(declarations).toContain('import type { Email } from "./mail.js";');
  expect(declarations).toContain("export function welcome(name: Name, email: Email): string;");
  expect(declarations).not.toContain("any");
  const mail = readFileSync(join(dir, "mail.d.ts"), "utf8");
  expect(mail).toContain("declare const $emailBrand: unique symbol;");
  expect(mail).toContain("export interface Email {\n    readonly [$emailBrand]: never;\n}");
  expect(mail).not.toContain("export declare const $emailBrand");
  expect(mail).not.toContain("any");
  writeFileSync(join(dir, "use.ts"), `import { Name, welcome } from "./lib.js";
import { parse } from "./mail.js";
const email = parse("ada@example.com");
const name = Name.new("Ada");
export const sent: string | undefined = email.TAG === "Ok" && name != null ? welcome(name, email._0) : undefined;
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { Name, welcome } from "./lib.js";
import type { Email } from "./mail.js";
const name = Name.new("Ada")!;
welcome(name, "not an email");
welcome(name, ["not an email"]);
welcome(name, {});
welcome(name, "not an email" as Email);
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([
    expect.stringContaining("error TS2345: Argument of type 'string' is not assignable to parameter of type 'Email'"),
    expect.stringContaining("error TS2741: Property '[$emailBrand]' is missing in type 'string[]' but required in type 'Email'"),
    expect.stringContaining("error TS2741: Property '[$emailBrand]' is missing in type '{}' but required in type 'Email'"),
    expect.stringContaining("error TS2352: Conversion of type 'string' to type 'Email' may be a mistake"),
  ]);
  const lib = await import(join(dir, "lib.js"));
  const { parse } = await import(join(dir, "mail.js"));
  expect(lib.welcome(lib.Name.new("Ada"), parse("ada@example.com")._0)).toBe("Welcome, Ada! We'll write to ada@example.com.");
});

test("a tuple struct its crate's callers can make, a JS handle, a binding's type or a generic one is declared as before", () => {
  const dir = fixture("declarations-sealed-open");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::marker::PhantomData;

pub struct Open(pub String);

/// A JS \`Map\`, its native body the entries it holds.
#[cfg_attr(rust_js, rust_js::types = "Map<string, number>")]
pub struct Scores(RefCell<Vec<(String, u32)>>);

pub struct Handle(PhantomData<()>);

pub struct Id<T>(u64, PhantomData<T>);

#[repr(transparent)]
pub struct Meters(f64);

pub fn open(o: &Open) -> usize { o.0.len() }
pub fn handle(_h: &Handle) {}
pub fn id(i: &Id<u8>) -> u64 { i.0 }
pub fn meters(m: Meters) -> f64 { m.0 }
pub fn kept(s: &Scores) -> &Scores { s }
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).not.toContain("unique symbol");
  expect(declarations).toContain("export type Meters = number;");
  expect(declarations).toContain("export function kept(s: Map<string, number>): Map<string, number>;");
});

test("a sealed tuple struct's brand meets no export of its module, a function named as the brand was", () => {
  // `pub fn name_brand` is `nameBrand` in JS, which the brand was named
  // before, a duplicate identifier to TypeScript; `$nameBrand` is a name no
  // Rust item's JS name takes.
  const dir = fixture("declarations-sealed-brand-named");
  writeFileSync(join(dir, "lib.rs"), `pub struct Name(String);

impl Name {
    pub fn new(text: &str) -> Name {
        Name(text.to_string())
    }
}

pub fn name_brand() -> u32 {
    1
}

pub fn show(name: &Name) -> String {
    name.0.clone()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function nameBrand(): number;");
  writeFileSync(join(dir, "use.ts"), `import { Name, nameBrand, show } from "./lib.js";
export const shown: string = show(Name.new("Ada")) + nameBrand();
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
});
