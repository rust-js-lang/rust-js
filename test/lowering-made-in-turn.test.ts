// What's made in turn, a derived `Clone`'s or `Default`'s parts, and a
// clone of many items (#897, #902): where a later part's clone or
// `default()` may panic past a part made before it with a destructor,
// `$madeInTurn` or `$madeEach`, which drop what's made as it unwinds; where
// none may, the form it always had. `clone_may_panic` and
// `default_may_panic` decide, so a clone of `Copy` or `String` fields, or
// of a derived `Clone` of such, never takes the guarded form.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS. */
function compiled(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return readFileSync(join(dir, "lib.js"), "utf8");
}

const loud = `
pub struct Loud(pub u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        if self.0 == 0 {
            panic!("clone");
        }
        Loud(self.0)
    }
}

static MADE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

impl Default for Loud {
    fn default() -> Loud {
        let n = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if n == 1 {
            panic!("default");
        }
        Loud(n)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
`;

test("a clone or a default that can't panic keeps its form", () => {
  const js = compiled(
    "made-in-turn-plain",
    `#[derive(Clone, Default)]
pub struct Plain {
    pub n: u32,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Default)]
pub struct Outer {
    pub a: Plain,
    pub b: Plain,
}

pub fn grown(o: &Outer) -> Outer {
    let mut c = o.clone();
    c.a.tags.push("new".to_string());
    c
}

pub fn copies(v: &[Plain]) -> Vec<Plain> {
    v.to_vec()
}

pub fn made() -> (Outer, Plain) {
    Default::default()
}
`,
  );
  expect(js).not.toContain("$madeInTurn");
  expect(js).not.toContain("$madeEach");
  // Each part as it was: a spread, an array's \`slice()\`, its items' map.
  expect(js).toContain("a: { ...o.a, tags: o.a.tags.slice() }");
  expect(js).toContain("v.map((item) => ({ ...item, tags: item.tags.slice() }))");
});

test("a clone or a default that may panic past what it made drops it", () => {
  const js = compiled(
    "made-in-turn-guarded",
    `${loud}
#[derive(Clone, Default)]
pub struct Pair {
    pub a: Loud,
    pub n: u32,
    pub b: Loud,
}

pub fn copy(p: &Pair) -> Pair {
    p.clone()
}

pub fn copies(v: &[Loud]) -> Vec<Loud> {
    v.to_vec()
}

pub fn pair(t: &(Loud, Loud)) -> (Loud, Loud) {
    t.clone()
}

pub fn made() -> Pair {
    Pair::default()
}

#[derive(Clone)]
pub struct Counted {
    pub first: std::rc::Rc<Loud>,
    pub second: std::rc::Rc<Loud>,
}

pub fn share(c: &Counted) -> Counted {
    c.clone()
}

#[derive(Default)]
pub struct Lists {
    pub first: Vec<Loud>,
    pub second: Vec<Loud>,
}

pub fn lists() -> Lists {
    Lists::default()
}
`,
  );
  // Each part made before the last that may panic, given to \`made\` with
  // its drop; the last itself not, nor a number. Its lines joined, as
  // they're printed to fit.
  const flat = js.replace(/\s+/g, " ");
  expect(flat).toContain("$madeInTurn((made) => ({ ...p, a: made(loudClone_clone(p.a), loudDrop_drop), b: loudClone_clone(p.b), }))");
  expect(flat).toContain("$madeEach(v, (item) => loudClone_clone(item), loudDrop_drop)");
  expect(flat).toContain("$madeInTurn((made) => [made(loudClone_clone(t[0]), loudDrop_drop), loudClone_clone(t[1])])");
  expect(flat).toContain("$madeInTurn((made) => ({ a: made(loudDefault_default(), loudDrop_drop), n: 0, b: loudDefault_default(), }))");
  // An \`Rc\`'s clone only counts, and an empty \`Vec\`'s default makes
  // nothing: neither panics, so nothing before them is held.
  const body = (name: string) => flat.slice(flat.indexOf(`export function ${name}(`)).split("export function")[1];
  expect(body("share")).not.toContain("$madeInTurn");
  expect(body("lists")).not.toContain("$madeInTurn");
});
