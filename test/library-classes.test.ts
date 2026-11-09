// A library's trait of classes (rd283) is one in its consumer too (#1073):
// its library decided it of the impls it has, the ones in its own crate,
// and its consumer calls its methods as the library's prototype has them,
// implements it for a class of its own alone, and makes no `dyn` of it nor
// bounds a type parameter by it, as the library can't. Each program here
// prints what native Rust's does (`test/crates/pairs/library_trait_of_classes`
// runs one against it).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { compiler, fixture, root, run } from "./support";

/** `dep`, a library, and `app`, a crate that uses it, in `dir`: the app's
 * JS, or the compile's error where it's refused. */
function compiled(dir: string, dep: string, app: string): string {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir]);
  return readFileSync(join(dir, "app", "lib.js"), "utf8");
}

const render = readFileSync(join(root, "test", "crates", "pairs", "library_trait_of_classes", "dep.rs"), "utf8");

test("a consumer calls a library's trait of classes' methods on the prototype, and its own class's are its prototype's", () => {
  const dir = fixture("library-trait-of-classes");
  const app = compiled(dir, render, readFileSync(join(root, "test", "crates", "pairs", "library_trait_of_classes", "app.rs"), "utf8"));
  // No dictionary of the library's class, nor a copy of its methods.
  expect(app).not.toContain("rootRender");
  expect(app).toContain("  console.log(`${root.label()}`);\n  console.log(`${root.render(3)}`);\n");
  expect(app).toContain("function twice(item) {\n  return `${item.label()} ${item.render(1)}`;\n}");
  // The trait's default, its library's, the consumer's class's prototype's
  // method as the library's class has it.
  expect(app).toContain(
    "Leaf.prototype.render = function (times) {\n  let out = \"\";\n  for (let i = 0; i < times; i++) {\n    out += this.label();\n  }\n  return out;\n};",
  );
  expect(app).not.toContain("leafRender");
}, 300_000);

// A promise resolved with a library's class whose trait of classes has a
// `then` adopts it (#1071): `$settled` where it's given, as the crate's own.
test("a library's class whose trait of classes has a then is given as it is where it's awaited", async () => {
  const dir = fixture("library-trait-of-classes-thenable");
  const app = compiled(dir, `pub trait Settle {
    fn then(&self, n: u32) -> u32;
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Traited {
    value: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Traited {
    pub fn new(value: u32) -> Self {
        Traited { value }
    }
}

impl Settle for Traited {
    fn then(&self, n: u32) -> u32 {
        self.value + n
    }
}
`, `use dep::{Settle, Traited};

async fn traited(n: u32) -> Traited {
    Traited::new(n)
}

pub async fn report() -> String {
    let value = traited(3).await.then(1);
    format!("{value}")
}
`);
  expect(app).toContain("$unsettled(await traited(3)).then(1)");
  // One adopted never resolves: a second on, it's taken for hung.
  const hung = new Promise((resolve) => setTimeout(() => resolve("hung"), 1000));
  expect(await Promise.race([(await import(join(dir, "app", "lib.js"))).report(), hung])).toBe("4");
}, 300_000);

// What the library refuses of its own trait of classes, its consumer
// does too; and an impl of it for what's no class, which its library's
// code, its default's `this.label()`, would call on a prototype it hasn't.
test("a consumer makes no dyn of a library's trait of classes, bounds nothing by it, and implements it for a class alone", () => {
  const dir = fixture("library-trait-of-classes-refused");
  const refused = (app: string) => {
    try {
      compiled(dir, render, app);
    } catch (error) {
      return String(error);
    }
    return "compiled";
  };
  expect(refused(`use dep::Render;

pub fn main() {
    let items: Vec<Box<dyn Render>> = vec![Box::new(dep::root("a"))];
    println!("{}", items[0].render(2));
}
`)).toContain("rust-js: a `#[rust_js::class]` implements `Render`, a trait of classes, of no `dyn`");
  expect(refused(`use dep::Render;

pub fn twice<T: Render>(item: &T) -> String {
    item.render(2)
}
`)).toContain("rust-js: a `#[rust_js::class]` implements `Render`, a trait of classes, which bounds no type parameter");
  expect(refused(`use dep::Render;

pub struct Plain(pub u32);

impl Render for Plain {
    fn label(&self) -> String {
        format!("({})", self.0)
    }
}
`)).toContain(
    "rust-js: a `#[rust_js::class]` alone implements `dep::Render`, a library's trait of classes, whose methods are each class's prototype's: `Plain` is no class (ADR 0761)",
  );
}, 300_000);
