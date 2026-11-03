// A class that extends a binding's class of a JS module (ADR 1041), as
// React's ReactFlightWebpackPlugin.js extends webpack's: `class
// ClientReferenceDependency extends ModuleDependency { get type() { return
// 'client-reference'; } }`. The binding crate is built as a
// `bindings = true` crate is, by rustc alone; each program is run in Node
// against a module of that class.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// webpack's own, whose `type` is a getter its subclasses override.
const moduleDependency = `export class ModuleDependency {
  constructor(request) {
    this.request = request;
  }
  get type() {
    return "unknown";
  }
}
`;

/** The binding crate `dep` of `rust`, in `dir`, and the JS module of its class. */
function bindings(dir: string, rust: string) {
  const module = join(dir, "node_modules", "webpackish");
  mkdirSync(module, { recursive: true });
  writeFileSync(join(module, "package.json"), `{ "name": "webpackish", "type": "module", "main": "index.js" }\n`);
  writeFileSync(join(module, "index.js"), moduleDependency);
  writeFileSync(join(dir, "dep.rs"), rust);
  run([compiler, "--rustc", "--edition=2024", "--crate-type=lib", "--crate-name=dep", "--emit=metadata", "--target=wasm32-unknown-unknown", join(dir, "dep.rs"), ...js, "-o", join(dir, "libdep.rmeta")]);
}

const app = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-name=app", "--crate-type=lib", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js];

// The binding: webpack's `ModuleDependency`, whose `get type()` a
// subclass writes and whose `get request` is its own.
const dependency = `#[cfg_attr(rust_js, rust_js::types = "webpackish#ModuleDependency")]
pub trait ModuleDependency: 'static {
    /// \`get type()\`.
    #[cfg_attr(rust_js, rust_js::getter)]
    fn r#type(&self) -> &'static str;

    #[cfg_attr(rust_js, rust_js::link_name = "get request")]
    fn request(&self) -> String {
        unreachable!()
    }
}
`;

const subclass = `use dep::ModuleDependency;

#[cfg_attr(rust_js, rust_js::class)]
pub struct ClientReferenceDependency {}

#[cfg_attr(rust_js, rust_js::class)]
impl ClientReferenceDependency {
    pub fn new(_request: &str) -> Self {
        ClientReferenceDependency {}
    }
}

impl ModuleDependency for ClientReferenceDependency {
    #[cfg_attr(rust_js, rust_js::getter)]
    fn r#type(&self) -> &'static str {
        println!("type read");
        "client-reference"
    }
}

fn noted(what: &str) -> &str {
    println!("{what} noted");
    what
}

/// Its type and request, read in Rust's order, the getter's body first.
pub fn kind(d: &ClientReferenceDependency) -> String {
    format!("{} {} {}", d.r#type(), noted("then"), d.request())
}
`;

// rd679: the getter a subclass writes is its class's getter, `get type()
// { .. }`, which webpack reads as `dependency.type`, not a method, and
// declared one; a Rust call of it reads it, its body run where Rust runs
// it, before what's called after it. rd680: it's declared extending its
// class, of no type arguments, not the trait's `Self`.
test("a getter of the binding's class a class extends is its class's getter", async () => {
  const dir = fixture("binding-subclass-getter");
  bindings(dir, dependency);
  writeFileSync(join(dir, "lib.rs"), subclass);
  run([compiler, ...app(dir)]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  const ts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(out).toContain('import { ModuleDependency } from "webpackish";');
  expect(out).toContain(`export class ClientReferenceDependency extends ModuleDependency {
  constructor(request) {
    super(request);
  }
  get type() {
    console.log("type read");
    return "client-reference";
  }
}`);
  expect(out).toContain('return `${d.type} ${noted("then")} ${d.request}`;');
  expect(ts).toContain("    get type(): string;\n");
  // rd680: of no type arguments, the trait's `Self` none of its class's.
  expect(ts).toContain("export declare class ClientReferenceDependency extends ModuleDependency {\n");
  const lib = await import(join(dir, "lib.js"));
  const base = (await import(join(dir, "node_modules", "webpackish", "index.js"))).ModuleDependency;
  const dependencyMade = new lib.ClientReferenceDependency("./client.js");
  expect(dependencyMade instanceof base).toBe(true);
  expect(typeof Object.getOwnPropertyDescriptor(lib.ClientReferenceDependency.prototype, "type")?.get).toBe("function");
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    expect(dependencyMade.type).toBe("client-reference");
    expect(lib.kind(dependencyMade)).toBe("client-reference then ./client.js");
  } finally {
    console.log = log;
  }
  // As native Rust: the getter's body runs before what's called after it.
  expect(logged).toEqual(["type read", "type read", "then noted"]);
});

// rd679: a getter's mark that can't apply is refused, saying what to
// write, never dropped: the trait's getter its impl doesn't mark, which
// would print a method; the impl's of what the trait has as a method,
// which would hide it; and the trait's of a method it binds. rd682: and
// `#[rust_js::class(es)]` of a class that's an ES class already, and #568's
// `class(es, fields)`, whose fields an accessor of the class it extends may
// answer.
test("a getter of the class a class extends is marked as the trait marks it", () => {
  const unmarked = subclass.replace("impl ModuleDependency for ClientReferenceDependency {\n    #[cfg_attr(rust_js, rust_js::getter)]\n", "impl ModuleDependency for ClientReferenceDependency {\n");
  const method = dependency.replace("    /// `get type()`.\n    #[cfg_attr(rust_js, rust_js::getter)]\n", "");
  const bound = dependency.replace('    #[cfg_attr(rust_js, rust_js::link_name = "get request")]\n', '    #[cfg_attr(rust_js, rust_js::link_name = "get request")]\n    #[cfg_attr(rust_js, rust_js::getter)]\n');
  const cases: [string, string, string, string][] = [
    ["binding-subclass-getter-unmarked", dependency, unmarked, "a `#[rust_js::class]` method is a getter where `ModuleDependency`'s `type` is one, `get type()`: mark it `#[cfg_attr(rust_js, rust_js::getter)]` as the trait's is"],
    ["binding-subclass-getter-of-a-method", method, subclass, "getter overrides a getter, and `ModuleDependency`'s `type` is a method"],
    ["binding-subclass-marked-es-fields", dependency, subclass.replace("#[cfg_attr(rust_js, rust_js::class)]\npub struct", "#[cfg_attr(rust_js, rust_js::class(es, fields))]\npub struct"), "that extends a binding's class is an ES class already: write it `#[rust_js::class]`"],
    ["binding-subclass-marked-es", dependency, subclass.replace("#[cfg_attr(rust_js, rust_js::class)]\npub struct", "#[cfg_attr(rust_js, rust_js::class(es))]\npub struct"), "that extends a binding's class is an ES class already: write it `#[rust_js::class]`"],
    ["binding-subclass-getter-bound", bound, subclass.replace("pub fn kind", "pub fn request_of(d: &ClientReferenceDependency) -> String {\n    d.request()\n}\n\npub fn kind").replace("impl ModuleDependency for ClientReferenceDependency {\n", "impl ModuleDependency for ClientReferenceDependency {\n    #[cfg_attr(rust_js, rust_js::getter)]\n    fn request(&self) -> String {\n        String::new()\n    }\n\n"), "getter of `ModuleDependency`'s is bound by its `link_name`, `\"get request\"`"],
  ];
  for (const [name, binding, rust, message] of cases) {
    const dir = fixture(name);
    bindings(dir, binding);
    writeFileSync(join(dir, "lib.rs"), rust);
    const ran = runSync([compiler, ...app(dir)], root, 60_000);
    expect(ran.code).not.toBe(0);
    expect(ran.stderr).toContain(message);
  }
});

// rd678: a class a module exports as its default, as webpack's
// `webpack/lib/dependencies/ModuleDependency` exports `ModuleDependency`,
// is extended where it's imported as its default, `import ModuleDependency
// from ".."`, in the JS and in the declarations, `import type
// ModuleDependency from ".."`, as a type of a module's default is
// imported by the name of the Rust type it types, `WebRequest` of
// `Request#default`; `import type { default }` isn't TypeScript.
test("a binding's class that's its module's default is imported as its default", async () => {
  const dir = fixture("binding-subclass-default");
  const module = join(dir, "node_modules", "webpackish", "lib", "dependencies");
  mkdirSync(module, { recursive: true });
  writeFileSync(join(dir, "node_modules", "webpackish", "package.json"), `{ "name": "webpackish", "type": "module" }\n`);
  writeFileSync(join(module, "ModuleDependency.js"), moduleDependency.replace("export class", "export default class"));
  writeFileSync(join(module, "Request.js"), "export default class Request {}\n");
  writeFileSync(join(dir, "dep.rs"), `${dependency.replace("webpackish#ModuleDependency", "webpackish/lib/dependencies/ModuleDependency#default")}
/// A module's default of no methods, a type alone, named as its Rust is.
#[cfg_attr(rust_js, rust_js::types = "webpackish/lib/dependencies/Request#default")]
pub struct WebRequest(core::marker::PhantomData<js::JsObject>);
`);
  run([compiler, "--rustc", "--edition=2024", "--crate-type=lib", "--crate-name=dep", "--emit=metadata", "--target=wasm32-unknown-unknown", join(dir, "dep.rs"), ...js, "-o", join(dir, "libdep.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `${subclass}
/// Whether it's given a request.
pub fn given(_request: &dep::WebRequest) -> bool {
    true
}
`);
  run([compiler, ...app(dir)]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  const ts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(out).toContain('import ModuleDependency from "webpackish/lib/dependencies/ModuleDependency";');
  expect(out).toContain("export class ClientReferenceDependency extends ModuleDependency {");
  expect(ts).toContain('import type ModuleDependency from "webpackish/lib/dependencies/ModuleDependency";');
  expect(ts).toContain("export declare class ClientReferenceDependency extends ModuleDependency {\n");
  expect(ts).toContain('import type WebRequest from "webpackish/lib/dependencies/Request";');
  expect(ts).toContain("export function given(request: WebRequest): boolean;");
  expect(ts).not.toContain("{ default }");
  const lib = await import(join(dir, "lib.js"));
  const base = (await import(join(module, "ModuleDependency.js"))).default;
  expect(new lib.ClientReferenceDependency("./client.js") instanceof base).toBe(true);
});

// #568: the fields of a class that extends a binding's class are never
// declared (`class(es, fields)` is refused it, above), as the class it extends may have an accessor of a field's
// name on its prototype, which the constructor's assignment calls and a
// declaration, defined after `super(..)` returns, would hide: here
// webpack's class upper-cases what's set as its `tag`, and the subclass's
// constructor sets it.
test("a field of a class that extends a binding's class is set through its accessor, undeclared", async () => {
  const dir = fixture("binding-subclass-field-accessor");
  bindings(dir, dependency);
  writeFileSync(
    join(dir, "node_modules", "webpackish", "index.js"),
    moduleDependency.replace("  get type() {", "  set tag(given) {\n    this._tag = given.toUpperCase();\n  }\n  get tag() {\n    return this._tag;\n  }\n  get type() {"),
  );
  writeFileSync(
    join(dir, "lib.rs"),
    subclass
      .replace("pub struct ClientReferenceDependency {}", "pub struct ClientReferenceDependency {\n    pub tag: String,\n}")
      .replace("    pub fn new(_request: &str) -> Self {\n        ClientReferenceDependency {}", "    pub fn new(request: &str) -> Self {\n        ClientReferenceDependency {\n            tag: request.to_string(),\n        }"),
  );
  run([compiler, ...app(dir)]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("export class ClientReferenceDependency extends ModuleDependency {\n  constructor(request) {\n    super(request);\n    this.tag = request;\n  }");
  const lib = await import(join(dir, "lib.js"));
  expect(new lib.ClientReferenceDependency("client").tag).toBe("CLIENT");
});
