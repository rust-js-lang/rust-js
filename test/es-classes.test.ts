// An ES class (rd682): a `#[rust_js::class]` struct marked
// `#[rust_js::class(es)]` is `class Plugin { constructor(..) { .. }
// apply(..) { .. } }`, as React's ReactFlightWebpackPlugin.js writes
// `export default class ReactFlightWebpackPlugin`, where one unmarked is a
// constructor function and its prototype (ADR 0761). Each program is
// compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled in a fixture of `name`: its JS, its declarations, and where they are. */
function compiled(name: string, rust: string): { js: string; ts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, ...flags(dir)]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), ts: readFileSync(join(dir, "lib.d.ts"), "utf8"), dir };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

const plugin = `use std::cell::Cell;

thread_local! {
    static MADE: Plugin = Plugin::new("early");
}

/// A plugin, as webpack's are written.
#[cfg_attr(rust_js, rust_js::class(es, fields))]
pub struct Plugin {
    pub name: String,
    applied: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Plugin {
    pub const LIMIT: u32 = 3;

    pub fn new(name: &str) -> Self {
        println!("made {name}");
        Plugin {
            name: name.to_string(),
            applied: Cell::new(0),
        }
    }

    pub fn apply(&self, to: &str) -> String {
        self.applied.set(self.applied.get() + 1);
        format!("{} applied to {to}", self.name)
    }

    #[cfg_attr(rust_js, rust_js::getter)]
    pub fn count(&self) -> u32 {
        self.applied.get()
    }

    pub fn shout(text: &str) -> String {
        text.to_uppercase()
    }
}

/// The plugin its module made as it loaded, before its class's line.
pub fn early() -> String {
    MADE.with(|made| made.apply("early"))
}

js::export_default!(Plugin);
`;

// Its fields are declared, `name;`, as upstream's are, as it says,
// `class(es, fields)`, in the order its constructor sets them, which sets
// each before anything reads the instance (#568); its constructor, methods, getter and static are the
// class's, its constant its prototype's after it; a thread-local made of it is made
// after the class, which isn't hoisted as a function is; its module's
// default is declared as it, `export default class Plugin`. JS sees an ES
// class: its methods not enumerable, a call without `new` a `TypeError`,
// and a JS subclass's `super(..)` runs its constructor.
test("a class whose struct says es is an ES class", () => {
  const { js, ts, dir } = compiled("es-class", plugin);
  expect(js).toContain(`/** A plugin, as webpack's are written. */
export default class Plugin {
  name;
  applied;

  constructor(name) {
    console.log(\`made \${name}\`);
    this.name = name;
    this.applied = 0;
  }

  apply(to) {
    this.applied = (this.applied + 1) >>> 0;
    return \`\${this.name} applied to \${to}\`;
  }

  get count() {
    return this.applied;
  }

  static shout(text) {
    return text.toUpperCase();
  }
}

Plugin.prototype.limit = 3;

const made = new Plugin("early");
`);
  expect(js).not.toContain("export default Plugin;");
  expect(ts).toContain("declare class Plugin {");
  expect(ts).toContain("export default Plugin;");
  writeFileSync(
    join(dir, "run.mjs"),
    `import Plugin, { early } from "./lib.js";
class Loud extends Plugin {
  apply(to) { return super.apply(to).toUpperCase(); }
}
const loud = new Loud("loud");
const said = [early(), loud.apply("js"), loud.count, loud.limit, Plugin.shout("hi"), Object.keys(Plugin.prototype).join(), loud instanceof Plugin, Object.keys(loud).join()];
let thrown;
try { Plugin("called"); } catch (e) { thrown = e.constructor.name; }
console.log(JSON.stringify([...said, thrown]));`,
  );
  const ran = runSync([node ?? "node", join(dir, "run.mjs")], dir, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('made early\nmade loud\n["early applied to early","LOUD APPLIED TO JS",1,3,"HI","limit",true,"name,applied","TypeError"]\n');
});

// What an ES class can't be is refused, saying what to write: one that
// extends a binding's class is one already; the form is its struct's, not
// its impl's; a word that's no form; and its prototype replaced, which JS
// doesn't let be written.
test("what an ES class is refused says what to write", () => {
  const unmarked = plugin.replace("js::export_default!(Plugin);\n", "");
  const cases: [string, string, string][] = [
    ["es-class-on-impl", unmarked.replace("#[cfg_attr(rust_js, rust_js::class)]\nimpl Plugin", "#[cfg_attr(rust_js, rust_js::class(es, fields))]\nimpl Plugin"), "impl is marked `#[rust_js::class]` alone: its struct's attribute says its form, `#[rust_js::class(es)]`"],
    ["es-class-of-another-word", unmarked.replace("rust_js::class(es, fields))]\npub struct", "rust_js::class(function))]\npub struct"), "is a constructor function, `#[rust_js::class]`, an ES class, `#[rust_js::class(es)]`, or one that declares its fields, `#[rust_js::class(es, fields)]`"],
    [
      "es-class-prototype-replaced",
      `${unmarked}
js::on_load! {
    js::class::<Plugin>().set_prototype(js::object::create(None, None));
}

impl js::Classed for Plugin {
    type Constructor = js::AnyConstructor;
}
`,
      "replacing the prototype of `Plugin`, an ES class, `#[rust_js::class(es)]`, whose `prototype` JS doesn't let be written: a constructor function's is, `#[rust_js::class]`",
    ],
  ];
  for (const [name, rust, message] of cases) {
    expect(refused(name, rust)).toContain(message);
  }
});

// #568, the user's decision of 2026-11-04: an ES class declares its
// fields where it says so, `#[rust_js::class(es, fields)]`, as upstream's
// ReactFlightWebpackPlugin does and react-art's LinearGradient doesn't; one
// that doesn't, none. Here one of `class(es)` whose prototype the program
// gives a setter of `name`, which its constructor's assignment calls.
test("an ES class declares no fields unless it says so", () => {
  const { js, dir } = compiled("es-class-fields-unasked", `#[cfg_attr(rust_js, rust_js::class(es))]
pub struct Watched {
    pub name: String,
}

impl js::Classed for Watched {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl Watched {
    pub fn new(name: &str) -> Self {
        Watched { name: name.to_string() }
    }
}

js::on_load! {
    js::object::define_property(
        js::class::<Watched>().prototype(),
        "name",
        &js::PropertyDescriptor {
            set: Some(&|given| println!("set {}", js::string(given))),
            ..Default::default()
        },
    );
}
`);
  expect(js).toContain("export class Watched {\n  constructor(name) {");
  writeFileSync(join(dir, "run.mjs"), `import { Watched } from "./lib.js";\nnew Watched("w");`);
  const ran = runSync([node ?? "node", join(dir, "run.mjs")], dir, 30_000);
  expect([ran.stderr, ran.stdout]).toEqual(["", "set w\n"]);
});

// Where a declaration would change what the constructor does, `fields` is
// refused, saying why, never dropped: the program reads its prototype,
// where it may define a setter of a field's name, which the define hides
// from the assignment; its constructor leaves a field unset, its `None`
// left out, which a declaration makes present, `undefined`; a field no
// class declares, of no JS name; and `fields` of no ES class.
test("an ES class's fields that would change what it does are refused", () => {
  const cases: [string, string, string][] = [
    ["es-fields-prototype-read", `#[cfg_attr(rust_js, rust_js::class(es, fields))]
pub struct Watched {
    pub name: String,
}

impl js::Classed for Watched {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl Watched {
    pub fn new(name: &str) -> Self {
        Watched { name: name.to_string() }
    }
}

js::on_load! {
    js::object::define_property(
        js::class::<Watched>().prototype(),
        "name",
        &js::PropertyDescriptor {
            set: Some(&|given| println!("set {}", js::string(given))),
            ..Default::default()
        },
    );
}
`, "its prototype is read, `js::class::<T>().prototype()`"],
    ["es-fields-left-unset", `#[cfg_attr(rust_js, rust_js::class(es, fields))]
pub struct Labeled {
    pub label: Option<String>,
    pub size: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Labeled {
    pub fn new(size: u32) -> Self {
        Labeled { label: None, size }
    }
}
`, "its constructor leaves `label` unset"],
    ["es-fields-reserved", `#[cfg_attr(rust_js, rust_js::class(es, fields))]
pub struct Dashed {
    #[cfg_attr(rust_js, rust_js::name = "data-x")]
    pub data: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Dashed {
    pub fn new() -> Self {
        Dashed { data: 1 }
    }
}
`, "`data-x` is a field no class declares"],
    ["es-fields-of-no-es-class", `#[cfg_attr(rust_js, rust_js::class(fields))]
pub struct Labeled {
    pub label: Option<String>,
    pub size: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Labeled {
    pub fn new(size: u32) -> Self {
        Labeled { label: None, size }
    }
}
`, "declares its fields as an ES class, `#[rust_js::class(es, fields)]`"],
  ];
  for (const [name, rust, message] of cases) {
    expect(refused(name, rust)).toContain(message);
  }
});

// rd750: a class's field named `constructor` is refused, of either form:
// no TypeScript class declares one (TS18006), so its `.d.ts` couldn't be
// written, and `this.constructor = ..` hides its prototype's.
test("a class's field named constructor is refused, saying what to write", () => {
  for (const form of ["class", "class(es)"]) {
    const rust = `#[cfg_attr(rust_js, rust_js::${form})]
pub struct Built {
    pub constructor: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Built {
    pub fn new() -> Self {
        Built { constructor: 1 }
    }
}
`;
    expect(refused(`class-field-constructor-${form === "class" ? "function" : "es"}`, rust)).toContain(
      "has no field named `constructor`, which no TypeScript class declares and whose `this.constructor` hides its prototype's",
    );
  }
});

// rd731, the user's decision relayed by m6-react: an ES class's field
// declarations are apart from its first member by a blank line, as
// upstream's and Prettier's are, though the Rust writes the struct's
// fields and its `fn new` in other items; its doc comment after it.
test("an ES class's fields are apart from its first member", () => {
  const { js } = compiled(
    "es-class-fields-apart",
    `#[cfg_attr(rust_js, rust_js::class(es, fields))]
pub struct Sized {
    pub label: String,
    pub size: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Sized {
    /// Made of its size.
    pub fn new(size: u32) -> Self {
        let label = format!("{size}");
        Sized { label, size }
    }
}
`,
  );
  expect(js).toContain("export class Sized {\n  label;\n  size;\n\n  /** Made of its size. */\n  constructor(size) {");
});
