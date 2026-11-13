// What runs nothing is what lowering knew it to be, never a name (ADR
// 1138): a JS global's read lowering named, and a value's text, called
// where it may run code, as a format's arguments are evaluated first.
import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// A JS global's read, `Math.PI` of `to_degrees()`, which no call changes, as
// lowering named it (`Known::BuiltIn`, #1674): a value `{:?}` reads twice
// after it is held first, past it, never by the global's name, which a
// crate's item may share.
test("a value `{:?}` reads twice is held past a JS global's read", async () => {
  const dir = fixture("mir-applied-past-a-global");
  writeFileSync(join(dir, "lib.rs"), `fn parse(s: &str) -> Result<u8, String> { s.parse::<u8>().map_err(|_| s.to_string()) }
pub fn main() {
    let x = 1.5f64;
    println!("{} {:?}", x.to_degrees(), parse("x"));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const value = parse("x");');
  expect(js).not.toContain("((result) =>");
});
