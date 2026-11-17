// A `pub fn` in a private module is its module's JS export, which JS may
// import and call with what the crate's own calls never give: its
// parameters aren't known from those calls alone (`only_called`, which
// asks `library::seen_outside`), as rustc's crate-level `is_exported` said
// they were, and a `u8`'s sum JS gave went unwrapped (#2025).
import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a private module's pub fn a JS caller imports is compiled for what JS gives, not the crate's calls alone", async () => {
  const dir = fixture("js-callers-of-modules");
  writeFileSync(join(dir, "lib.rs"), `mod m;

pub fn uses() -> (u8, u8) {
    (m::add(1, 2), m::sub(5, 3))
}
`);
  writeFileSync(join(dir, "m.rs"), `pub fn add(a: u8, b: u8) -> u8 {
    a + b
}

pub fn sub(a: u8, b: u8) -> u8 {
    a - b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib"]);
  const m = await import(join(dir, "m.js"));
  const lib = await import(join(dir, "lib.js"));
  expect([m.add(200, 100), m.sub(3, 5), lib.uses()]).toEqual([44, 254, [3, 2]]);
});
