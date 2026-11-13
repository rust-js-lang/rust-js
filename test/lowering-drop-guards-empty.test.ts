// A guard whose `Drop` does nothing, its body written where it's dropped
// (ADRs 0608, 0705): its scope left nothing to drop, so none of it is
// written, as a person writes none (#1594). Split from
// lowering-drop-guards.test.ts by concept (#1334).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS. */
function compiled(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "main.rs"), rust);
  run([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js")]);
  return readFileSync(join(dir, "main.js"), "utf8");
}

test("a guard whose drop does nothing leaves no try of its scope", () => {
  const js = compiled("emptied-guard-scopes", `use std::mem;

struct Held<T>(T);

impl<T> Drop for Held<T> {
    fn drop(&mut self) {}
}

/// A guard whose drop does something, beside them: its scope stays.
struct Said(u32);

impl Drop for Said {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// May panic, so what's held as it's called is dropped as it unwinds.
fn risky(n: u32) -> u32 {
    if n > 5 {
        panic!("big");
    }
    n
}

fn main() {
    let mut a = Held(vec![1]);
    let mut b = Held(vec![2]);
    mem::swap(&mut a.0, &mut b.0);
    println!("{:?} {:?}", a.0, b.0);
    let said = Said(3);
    println!("{}", risky(std::hint::black_box(said.0)));
}
`);
  // Each held one's scope is gone, `Said`'s alone left.
  expect(js.match(/\$live\(\)/g)?.length ?? 0).toBe(1);
  expect(js.match(/\$unwindsOn\(/g)?.length ?? 0).toBe(1);
  expect(js).toContain("console.log(`drop ${said}`);");
});
