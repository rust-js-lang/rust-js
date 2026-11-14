// A type's name to TypeScript, the one owner's, `declarations::type_name`
// (#1829, ADR 0196 amended): a class's its JS name, one a
// `#[rust_js::name]` names by it, else its Rust name, of its export and
// every reference to it.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";

/** TypeScript's errors of `file` in `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, `tsconfig-${file}.json`);
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, verbatimModuleSyntax: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

// A type a `#[rust_js::name]` names is declared by it, its export and
// every reference, of its module and another's (#1829), as the vercel UI
// port's `#[rust_js::name = "ISODate"] pub type IsoDate = String;` is
// upstream's `type ISODate = string`.
test("declarations name a type by its #[rust_js::name]", () => {
  const dir = fixture("declarations-named-type");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::name = "ISODate")]
pub type IsoDate = String;

#[cfg_attr(rust_js, rust_js::name = "DateParts")]
pub struct Parts {
    pub year: f64,
}

pub fn add_days(date: &IsoDate, days: f64) -> IsoDate {
    format!("{date}+{days}")
}

pub fn parts(year: f64) -> Parts {
    Parts { year }
}

pub mod week;
`);
  writeFileSync(join(dir, "week.rs"), `use crate::{IsoDate, Parts};

pub fn start(date: &IsoDate, of: &Parts) -> IsoDate {
    format!("{date}@{}", of.year)
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type ISODate = string;");
  expect(declarations).toContain("export interface DateParts {");
  expect(declarations).toContain("export function addDays(date: ISODate, days: number): ISODate;");
  expect(declarations).toContain("export function parts(year: number): DateParts;");
  expect(declarations).not.toMatch(/IsoDate|\bParts\b/);
  const week = readFileSync(join(dir, "week.d.ts"), "utf8");
  expect(week).toContain("import type { DateParts, ISODate } from \"./lib.js\";");
  expect(week).toContain("export function start(date: ISODate, of: DateParts): ISODate;");
  writeFileSync(join(dir, "use.ts"), `import { addDays, type ISODate } from "./lib.js";
import { start } from "./week.js";
const typed: ISODate = "2024-02-29";
export const later: string = start(addDays(typed, 1), { year: 2024 });
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
});

// Re-exported from the crate's root by that name too, `export type {
// ISODate } from "./plain_date.js";`, renamed by its `as` (#1829), as Codex's
// root-calendar.ts imports it from the root.
test("a type a #[rust_js::name] names is re-exported by it", () => {
  const dir = fixture("declarations-named-reexported");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `pub mod plain_date;

pub use plain_date::{IsoDate, iso_date};
pub use plain_date::IsoDate as Iso;
`);
  writeFileSync(join(dir, "plain_date.rs"), `#[cfg_attr(rust_js, rust_js::name = "ISODate")]
pub type IsoDate = String;

pub fn iso_date(year: f64, month: f64, day: f64) -> Option<IsoDate> {
    (month <= 12.0).then(|| format!("{year}-{month:02}-{day:02}"))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export type { ISODate } from "./plain_date.js";');
  expect(declarations).toContain('export type { ISODate as Iso } from "./plain_date.js";');
  expect(declarations).not.toContain("IsoDate");
  // Codex's root-calendar.ts, as written but its import.
  writeFileSync(join(dir, "root-calendar.ts"), `import {isoDate, type ISODate} from "./lib.js";
const value: ISODate = "2024-02-29";
const formatted: ISODate | null | undefined = isoDate(2024, 2, 29);
// @ts-expect-error ISO dates retain the string representation.
const invalid: ISODate = 20240229;
void [value, formatted, invalid];
`);
  expect(typeChecked(dir, "root-calendar.ts")).toEqual([]);
});
