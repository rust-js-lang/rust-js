// An untagged enum (ADRs 0214, 1062): its payload, told apart by its
// runtime kind, a JS object by the class TypeScript names, and what JS
// can't tell apart refused, each compiled and run. Split from
// test/compiler.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildCompiler, buildReact, compiler, fixture, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(() => {
  buildCompiler();
  // The js and webapi crates, used from their metadata (ADR 0024).
  buildReact();
}, 600_000);

// ADR 0214: an untagged enum is its payload, TS's union: JS passes in what
// it has, each told apart by its runtime kind, and gets back the value.
test("an untagged enum is its payload, told apart by its runtime kind", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, "test/untagged.rs", "-o", join(target, "untagged.js"), ...withWeb]);
  const untagged = await import(join(target, "untagged.js"));
  expect(untagged.kind("a")).toBe("text a");
  expect(untagged.kind(1.5)).toBe("number 1.5");
  expect(untagged.kind(12n)).toBe("big 12");
  expect(untagged.kind(false)).toBe("flag false");
  expect(untagged.kind(["x", "y"])).toBe("names x+y");
  expect(untagged.kind((n: number) => n + 1)).toBe("step 2");
  expect(untagged.kind({ x: 1, y: 2 })).toBe("point 3");
  expect(untagged.mistyped(new TypeError("t"))).toBe("t");
  expect(untagged.text("a.png")).toBe("a.png");
  expect(untagged.numbers([1, 2])).toEqual([1, 2]);
  expect(untagged.shown(2.5)).toBe("2.5");
  expect([untagged.label("Recap"), untagged.label(["a", {}])]).toEqual(["Link for Recap", "Link for this heading"]);
  expect([untagged.isOther(["x"]), untagged.isOther(3), untagged.isOther("s")]).toEqual([true, true, false]);

  const js = withoutBoundary(await Bun.file(join(target, "untagged.js")).text());
  expect(js).not.toContain("instanceof");
  expect(js).toContain('  if (Array.isArray(src)) {\n    return `names ${src.join("+")}`;\n  }\n  switch (typeof src) {\n    case "object":\n');
  expect(js).toContain("export function text(s) {\n  return s;\n}");
  expect(js).toContain("String(n)");
  expect(js).toContain('return typeof value !== "string";');
});

// #391: the class an untagged enum's JS object variant is told by is the
// one TypeScript names it, a global's, `URL` of a `Url` declared
// `#[rust_js::types = "URL"]`, not of its Rust name; and a module's type,
// `./shapes.js#Chart`, which an `instanceof` would read unimported, is
// told apart as an object, as one that declares nothing is, never
// `instanceof Tag` of a `Tag` JS may not have. The js crate's global
// classes declare theirs, `js::RegExp`. A lone class is what the other
// kinds leave (ADR 1062), and one beside another class or an object is
// refused where it's told apart, naming its class, `instanceof URL`
// being `false` of another realm's (#555).
test("an untagged enum's JS object is tested by the class TypeScript names", async () => {
  const dir = fixture("untagged-declared-classes");
  writeFileSync(join(dir, "shapes.js"), "export class Chart {}\n");
  const declared = `use js::JsObject;
use std::marker::PhantomData;

#[cfg_attr(rust_js, rust_js::types = "URL")]
pub struct Url(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::types = "./shapes.js#Chart")]
pub struct Chart(PhantomData<JsObject>);

pub struct Tag(PhantomData<JsObject>);
`;
  writeFileSync(
    join(dir, "lib.rs"),
    `${declared}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Named<'a> {
    Tag(&'a Tag),
    Text(&'a str),
}

pub fn named(named: Named) -> String {
    match named {
        Named::Tag(_) => "tag".to_string(),
        Named::Text(text) => text.to_string(),
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Link<'a> {
    Url(&'a Url),
    Text(&'a str),
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Drawn<'a> {
    Chart(&'a Chart),
    Charts(Vec<&'a Chart>),
    Name(&'a str),
}

pub fn link(link: Link) -> String {
    match link {
        Link::Url(_) => "url".to_string(),
        Link::Text(text) => text.to_string(),
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Pattern<'a> {
    Regex(&'a js::RegExp),
    Text(&'a str),
}

pub fn pattern(pattern: Pattern) -> String {
    match pattern {
        Pattern::Regex(_) => "regex".to_string(),
        Pattern::Text(text) => text.to_string(),
    }
}

pub fn drawn(drawn: Drawn) -> String {
    match drawn {
        Drawn::Chart(_) => "chart".to_string(),
        Drawn::Charts(charts) => format!("{} charts", charts.len()),
        Drawn::Name(name) => name.to_string(),
    }
}
`,
  );
  const crates = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...crates]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).not.toContain("instanceof");
  expect(js).toContain('typeof drawn === "object" && !Array.isArray(drawn)');
  // A type that declares no class is an object, tested in its arm's place.
  expect(js).toContain('return typeof named === "object" ? "tag" : named;');
  const lib = await import(join(dir, "lib.js"));
  const { Chart } = await import(join(dir, "shapes.js"));
  expect([lib.link(new URL("https://a.b")), lib.link("x")]).toEqual(["url", "x"]);
  expect([lib.drawn(new Chart()), lib.drawn([new Chart(), new Chart()]), lib.drawn("pie")]).toEqual(["chart", "2 charts", "pie"]);
  expect([lib.named({ any: "tag" }), lib.named("x")]).toEqual(["tag", "x"]);
  expect([lib.pattern(/a/), lib.pattern("x")]).toEqual(["regex", "x"]);
  // Beside another class, the one TypeScript names.
  const refused = fixture("untagged-declared-classes-refused");
  writeFileSync(
    join(refused, "lib.rs"),
    `${declared}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Dated<'a> {
    Url(&'a Url),
    Date(&'a js::Date),
}

pub fn dated(dated: Dated) -> bool {
    matches!(dated, Dated::Url(_))
}
`,
  );
  const built = Bun.spawnSync([compiler, join(refused, "lib.rs"), "-o", join(refused, "lib.js"), ...crates]);
  expect(built.exitCode).not.toBe(0);
  expect(built.stderr.toString()).toContain("JS can't tell `Dated::Url`, a `URL`, from `Dated`'s other variants in every realm: `instanceof URL` is `false`");
});


// #303: the js crate's types JS tells apart by what they are: a `Class`, a
// constructor, is a function, `typeof value === "function"`, of any
// signature; a `TimeoutId` or an `IntervalId`, a number in a browser and an
// object in Node, `number | object`, is refused, as nothing tells it from
// another number or object.
test("an untagged enum's js::Class is a function, and a timeout id is refused", async () => {
  buildReact();
  const dir = fixture("untagged-js-kinds");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::Class;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Made<'a> {
    Class(&'a Class),
    Text(&'a str),
}

pub fn is_class(made: Made<'_>) -> bool {
    matches!(made, Made::Class(_))
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain('return typeof made === "function";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.isClass(class Point {}), lib.isClass("Point")]).toEqual([true, false]);
  const timer = fixture("untagged-timeout-id");
  writeFileSync(
    join(timer, "lib.rs"),
    `use js::TimeoutId;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Timer<'a> {
    Id(&'a TimeoutId),
    Count(u32),
}

pub fn is_id(timer: Timer<'_>) -> bool {
    matches!(timer, Timer::Id(_))
}
`,
  );
  const refused = Bun.spawnSync([compiler, join(timer, "lib.rs"), "-o", join(timer, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("JS can't tell a `&'a js::TimeoutId` from another variant's value");
});

// #303: a `RegExpMatch` is an array with its own `index`, `input` and
// `groups`, `string[] & { .. }`, TypeScript's `RegExpExecArray`: an untagged
// variant of one is an array, `Array.isArray`, told apart from a string,
// and refused beside a `Vec`'s, another array; its `.d.ts` is the
// intersection, not `any`.
test("an untagged enum's js::RegExpMatch is an array", async () => {
  buildReact();
  const dir = fixture("untagged-regexp-match");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::RegExpMatch;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Found<'a> {
    Match(&'a RegExpMatch),
    Text(&'a str),
}

pub fn is_match(found: Found<'_>) -> bool {
    matches!(found, Found::Match(_))
}

pub fn first(m: &RegExpMatch) -> String {
    m.full_match()
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("return Array.isArray(found);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.isMatch(/b(c)/.exec("abc")), lib.isMatch("abc")]).toEqual([true, false]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(
    "export function first(m: string[] & {\n    index: number;\n    input: string;\n    groups?: {\n        [key: string]: string;\n    };\n}): string;",
  );
  const both = fixture("untagged-regexp-match-and-vec");
  writeFileSync(
    join(both, "lib.rs"),
    `use js::RegExpMatch;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Found<'a> {
    Match(&'a RegExpMatch),
    Words(Vec<String>),
}

pub fn is_match(found: Found<'_>) -> bool {
    matches!(found, Found::Match(_))
}
`,
  );
  const refused = Bun.spawnSync([compiler, join(both, "lib.rs"), "-o", join(both, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("`Words` and `Match` both hold an array, which JS can't tell apart in an untagged enum");
});
