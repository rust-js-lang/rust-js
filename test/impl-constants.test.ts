// An inherent impl's constants (#498): the `pub` ones another crate or JS
// can name are properties of their type's object, `export const
// ReactiveFlags = { NONE: 0, .. }`, declared so, and each use is still the
// value where it's written, as TypeScript's `const enum` is (ADR 0410).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

function compile(name: string, source: string, crate = "js"): { js: string; dts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), source);
  const js = ["--", "--extern", `${crate}=${join(target, `lib${crate}.rmeta`)}`, "-L", target];
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dts: readFileSync(join(dir, "lib.d.ts"), "utf8"), dir };
}

const flags = `pub struct ReactiveFlags;

impl ReactiveFlags {
    pub const NONE: i32 = 0;
    pub const MUTABLE: i32 = 1;
    #[cfg_attr(rust_js, rust_js::name = "RecursedCheck")]
    pub const RECURSED_CHECK: i32 = 4;
    const PRIVATE: i32 = 8;
}

pub fn watched(flags: i32) -> bool {
    flags & (ReactiveFlags::MUTABLE | ReactiveFlags::RECURSED_CHECK | ReactiveFlags::PRIVATE) != ReactiveFlags::NONE
}
`;

test("a type's pub constants are its object's, each use its value", () => {
  const { js, dts } = compile("impl-constants-flags", flags);
  expect(js).toContain("export const ReactiveFlags = { NONE: 0, MUTABLE: 1, RecursedCheck: 4 };");
  expect(js).not.toContain("PRIVATE");
  expect(js).toContain("return (flags & (1 | 4 | 8)) !== 0;");
  expect(dts).toContain(
    "export const ReactiveFlags: {\n    readonly NONE: number;\n    readonly MUTABLE: number;\n    readonly RecursedCheck: number;\n};",
  );
});

test("a private type's constants, and a generic impl's, are no object", () => {
  const { js, dts } = compile(
    "impl-constants-private",
    "struct Limits;\n\nimpl Limits {\n    pub const MAX: u32 = 9;\n}\n\npub struct Wrapped<T>(pub T);\n\nimpl<T> Wrapped<T> {\n    pub const ONE: u32 = 1;\n}\n\npub fn max() -> u32 {\n    Limits::MAX + Wrapped::<u8>::ONE\n}\n",
  );
  expect(js).not.toContain("Limits");
  expect(js).not.toContain("Wrapped");
  expect(js).toContain("return 10;");
  expect(dts).not.toContain("Limits");
  expect(dts).not.toContain("export const Wrapped");
});

// Its object's `pub` methods are declared beside its constants, each of
// its receiver first, as the object's JS takes it, and of a generic impl
// its type parameters, `get<T>(wrapped: Wrapped<T>): T` (#507).
test("a type's object of constants and methods is declared whole", () => {
  const { js, dts, dir } = compile(
    "impl-constants-methods",
    `pub struct Counter {
    pub count: u32,
}

impl Counter {
    pub const START: u32 = 0;

    pub fn new() -> Counter {
        Counter { count: Counter::START }
    }

    pub fn tick(&mut self) {
        self.bump();
    }

    fn bump(&mut self) {
        self.count += 1;
    }
}

pub struct Wrapped<T> {
    pub value: T,
}

impl<T> Wrapped<T> {
    pub fn new(value: T) -> Self {
        Wrapped { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Wrapped<U> {
        Wrapped { value: f(self.value) }
    }
}
`,
  );
  expect(js).toContain("export const Counter = {\n  START: 0,\n\n  new() {\n    return { count: 0 };\n  },");
  expect(dts).toContain(
    "export const Counter: {\n    readonly START: number;\n    \"new\"(): Counter;\n    tick(counter: Counter): void;\n};",
  );
  expect(dts).not.toContain("bump");
  expect(dts).toContain(
    "export const Wrapped: {\n    \"new\"<T>(value: T): Wrapped<T>;\n    get<T>(wrapped: Wrapped<T>): T;\n    map<T, U>(wrapped: Wrapped<T>, f: (t: T) => U): Wrapped<U>;\n};",
  );
  // TypeScript calls it so.
  writeFileSync(
    join(dir, "use.ts"),
    'import { Counter, Wrapped } from "./lib.js";\n\nconst counter: Counter = Counter.new();\nCounter.tick(counter);\nconst n: number = Counter.START + counter.count;\nexport const text: string = Wrapped.get(Wrapped.map(Wrapped.new(n), (v) => `${v}`));\n',
  );
  const options = { module: "esnext", moduleResolution: "bundler", allowJs: true, strict: true, noEmit: true, skipLibCheck: false };
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({ compilerOptions: options, files: ["use.ts"] }));
  const tsc = join(root, "node_modules/typescript/bin/tsc");
  const checked = Bun.spawnSync([process.execPath, tsc, "-p", join(dir, "tsconfig.json")], { cwd: dir });
  expect(checked.stdout.toString()).toBe("");
});

// A generic impl's parameter TypeScript has a type for, `C: ReactNode`, is
// that type in each method, as in its struct's declaration.
test("a generic impl's parameter of a TypeScript type is that type in its methods", () => {
  const { dts } = compile(
    "impl-constants-erased",
    "use react::ReactNode;\n\npub struct Card<C: ReactNode> {\n    pub children: C,\n}\n\nimpl<C: ReactNode> Card<C> {\n    pub fn children(&self) -> &C {\n        &self.children\n    }\n}\n",
    "react",
  );
  expect(dts).toContain("export interface Card {\n    children: ReactNode;\n}");
  expect(dts).toContain("export const Card: {\n    children(card: Card): ReactNode;\n};");
});

// An impl in another module than its type's makes its object there, and is
// declared there, its type imported (ADR 0047).
test("an impl of another module's type is declared where its object is", () => {
  const { dir } = compile(
    "impl-constants-elsewhere",
    "pub mod shapes {\n    pub struct Square {\n        pub side: f64,\n    }\n\n    pub mod area {\n        impl super::Square {\n            pub fn area(&self) -> f64 {\n                self.side * self.side\n            }\n        }\n    }\n}\n\npub fn area(square: &shapes::Square) -> f64 {\n    square.area()\n}\n",
  );
  expect(readFileSync(join(dir, "shapes/area.js"), "utf8")).toContain("export const Square = {\n  area(square) {");
  expect(readFileSync(join(dir, "shapes.d.ts"), "utf8")).not.toContain("export const Square");
  expect(readFileSync(join(dir, "shapes/area.d.ts"), "utf8")).toContain(
    'import type { Square } from "../shapes.js";\n\nexport const Square: {\n    area(square: Square): number;\n};',
  );
});

test("a class's constant is its prototype's, no object's", () => {
  const { js, dts } = compile(
    "impl-constants-class",
    "#[cfg_attr(rust_js, rust_js::class)]\npub struct Component {\n    pub props: f64,\n}\n\nunsafe impl js::Defined for Component {}\nimpl js::Classed for Component {\n    type Constructor = js::AnyConstructor;\n}\n\n#[cfg_attr(rust_js, rust_js::class)]\nimpl Component {\n    pub const LIMIT: f64 = 1.0;\n\n    fn new(props: f64) -> Self {\n        Component { props }\n    }\n\n    pub fn doubled(&self) -> f64 {\n        self.props * 2.0\n    }\n}\n\npub fn made(props: f64) -> f64 {\n    Component::new(props).props + Component::LIMIT\n}\n",
  );
  expect(js).toContain("Component.prototype.limit = 1;");
  // Read as its prototype's where it's used, not as a plain type's value.
  expect(js).toContain("return new Component(props).props + Component.prototype.limit;");
  expect(dts).not.toContain("export const Component");
});

// A constant whose value no value tree says, semver's `VersionReq::STAR` of
// a `Vec`, is its initializer lowered as code, its type's object's, which
// a JS caller reads, and each use's (#513).
test("a type's pub constant of a value no value tree says is its initializer's", async () => {
  const { js, dir } = compile(
    "impl-constants-coded",
    `pub struct Req {
    pub items: Vec<u32>,
}

impl Req {
    pub const STAR: Self = Req { items: Vec::new() };
    pub const ANY: Option<String> = None;
}

pub fn star_len() -> usize {
    let mut star = Req::STAR;
    star.items.push(1);
    star.items.len() + Req::STAR.items.len()
}
`,
  );
  const lib = await import(join(dir, "lib.js"));
  expect(lib.Req.STAR.items).toEqual([]);
  expect(lib.starLen()).toBe(1);
  expect(js).toContain("STAR");
});
