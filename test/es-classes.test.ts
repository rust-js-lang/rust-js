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
#[cfg_attr(rust_js, rust_js::class(es))]
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

// Its constructor, methods, getter and static are the class's, its
// constant its prototype's after it; a thread-local made of it is made
// after the class, which isn't hoisted as a function is; its module's
// default is declared as it, `export default class Plugin`. JS sees an ES
// class: its methods not enumerable, a call without `new` a `TypeError`,
// and a JS subclass's `super(..)` runs its constructor.
test("a class whose struct says es is an ES class", () => {
  const { js, ts, dir } = compiled("es-class", plugin);
  expect(js).toContain(`/** A plugin, as webpack's are written. */
export default class Plugin {
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
const said = [early(), loud.apply("js"), loud.count, loud.limit, Plugin.shout("hi"), Object.keys(Plugin.prototype).join(), loud instanceof Plugin];
let thrown;
try { Plugin("called"); } catch (e) { thrown = e.constructor.name; }
console.log(JSON.stringify([...said, thrown]));`,
  );
  const ran = runSync([node ?? "node", join(dir, "run.mjs")], dir, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('made early\nmade loud\n["early applied to early","LOUD APPLIED TO JS",1,3,"HI","limit",true,"TypeError"]\n');
});

// What an ES class can't be is refused, saying what to write: one that
// extends a binding's class is one already; the form is its struct's, not
// its impl's; a word that's no form; and its prototype replaced, which JS
// doesn't let be written.
test("what an ES class is refused says what to write", () => {
  const unmarked = plugin.replace("js::export_default!(Plugin);\n", "");
  const cases: [string, string, string][] = [
    ["es-class-on-impl", unmarked.replace("#[cfg_attr(rust_js, rust_js::class)]\nimpl Plugin", "#[cfg_attr(rust_js, rust_js::class(es))]\nimpl Plugin"), "impl is marked `#[rust_js::class]` alone: its struct's attribute says its form, `#[rust_js::class(es)]`"],
    ["es-class-of-another-word", unmarked.replace("rust_js::class(es))]\npub struct", "rust_js::class(function))]\npub struct"), "is a constructor function, `#[rust_js::class]`, or an ES class, `#[rust_js::class(es)]`"],
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
