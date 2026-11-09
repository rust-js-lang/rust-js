// What a promise would adopt, a thenable, given where a future resolves:
// boxed, `$settled`, and taken out where it's awaited, `$unsettled` (#172,
// ADR 0029). A value has a callable `then` where its JS has one: a field
// whose JS name is `then`, a class's or an object's method of that name,
// its own or a trait of classes' (#1071). Each awaited value here is what
// native Rust's `.await` gives, the lines its run prints
// (`target/probe/thenable2`'s programs, 2026-11-09); one adopted never
// resolves.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { compiler, fixture, run } from "./support";

test("a value whose JS has a callable `then` is given as it is where it's awaited", async () => {
  const dir = fixture("async-thenable-methods");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[rust_js::class]
pub struct Chunk {
    value: u32,
}

#[rust_js::class]
impl Chunk {
    pub fn new(value: u32) -> Self {
        Chunk { value }
    }

    pub fn then(&self, n: u32) -> u32 {
        self.value + n
    }
}

#[rust_js::object]
pub struct Own {
    value: u32,
}

#[rust_js::object]
impl Own {
    pub fn then(&self, n: u32) -> u32 {
        self.value + n
    }
}

pub trait Settle {
    fn then(&self, n: u32) -> u32;
}

#[rust_js::class]
pub struct Traited {
    value: u32,
}

#[rust_js::class]
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

pub struct Renamed {
    #[rust_js::name = "then"]
    pub next: fn(u32) -> u32,
}

fn double(n: u32) -> u32 {
    n * 2
}

async fn chunk(n: u32) -> Chunk {
    Chunk::new(n)
}

async fn maybe(n: u32) -> Option<Chunk> {
    if n > 0 { Some(Chunk::new(n)) } else { None }
}

async fn own(n: u32) -> Own {
    Own { value: n }
}

async fn traited(n: u32) -> Traited {
    Traited::new(n)
}

async fn renamed() -> Renamed {
    Renamed { next: double }
}

pub async fn report() -> String {
    let class = chunk(3).await.then(1);
    let option = maybe(3).await.map(|c| c.then(1));
    let block = async { Chunk::new(8) }.await.then(1);
    let ready = std::future::ready(Chunk::new(2)).await.then(1);
    let object = own(3).await.then(1);
    let trait_method = traited(3).await.then(1);
    let field = (renamed().await.next)(5);
    format!("{class} {option:?} {block} {ready} {object} {trait_method} {field}")
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("$unsettled(await chunk(3))");
  // One adopted never resolves: a second on, it's taken for hung.
  const hung = new Promise((resolve) => setTimeout(() => resolve("hung"), 1000));
  expect(await Promise.race([(await import(join(dir, "lib.js"))).report(), hung])).toBe("4 Some(4) 9 3 4 4 10");
});
