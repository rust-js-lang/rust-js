// `#[rust_js::require]` on a `use` of a library's function (rd684, ADR
// 0305, amended): React's FizzTestUtils reads `require('react').createElement`
// where it calls it, so a test's `jest.resetModules()` gives each call the
// module it loaded. The import the `use` would make is read through
// `require(module)` at each read in that module, never imported; another
// module's `use` of it imports it as before. On a `use` of anything else,
// a compile error that says what to write.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const dep = `pub mod element {
    pub fn create_element(tag: &str) -> String {
        format!("<{tag}>")
    }

    pub fn on_something() -> bool {
        js::this().is_some()
    }
}

pub use element::create_element;

#[cfg_attr(rust_js, rust_js::link_name = "get node:path#sep")]
pub fn sep() -> String {
    unreachable!()
}
`;

/** The library `dep`, compiled, and a consumer of it, `app`, of `files`,
 * its root first: what compiling the consumer printed. */
function compiled(dir: string, files: Record<string, string>) {
  mkdirSync(join(dir, "dep"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"), { recursive: true });
  for (const [name, source] of Object.entries(files)) writeFileSync(join(dir, "app", name), source);
  return runSync([compiler, join(dir, "app", Object.keys(files)[0]), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js], dir, 120_000);
}

test("a use marked require reads the library's function through require where it's called", async () => {
  const dir = fixture("required-use");
  const ran = compiled(dir, {
    "lib.rs": `pub mod plain;

#[cfg_attr(rust_js, rust_js::require)]
use dep::create_element;
#[cfg_attr(rust_js, rust_js::require)]
use dep::element::on_something;
use dep::element;

pub fn tag() -> String {
    println!("tag");
    "div".to_string()
}

pub fn made() -> String {
    create_element(&tag())
}

pub fn twice() -> String {
    create_element("a") + &create_element("b")
}

/// Called with no \`this\`, not the module's exports.
pub fn called() -> bool {
    on_something()
}

/// Named through its module, read through \`require\` too: the module
/// reads it one way.
pub fn through() -> String {
    element::create_element("t")
}

/// A local named \`require\` takes another name.
pub fn shadowed() -> String {
    let require = String::from("r");
    create_element(&require)
}
`,
    "plain.rs": "use dep::create_element;\n\npub fn imported() -> String {\n    create_element(\"p\")\n}\n",
  });
  expect(ran.stderr).toBe("");
  expect(ran.code).toBe(0);
  const out = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(out).not.toMatch(/^import /m);
  expect(out).toContain('return require("../dep/element.js").createElement(tag());');
  expect(out).toContain('require("../dep/element.js").createElement("a") +\n    require("../dep/element.js").createElement("b")');
  expect(out).toContain('return (0, require("../dep/element.js").onSomething)();');
  expect(out).toContain('return require("../dep/element.js").createElement("t");');
  expect(out).toMatch(/const (require\$1|require_) = "r";\n  return require\("..\/dep\/element.js"\).createElement\(require(\$1|_)\);/);
  // Another module's \`use\` of it, unmarked, imports it.
  expect(readFileSync(join(dir, "app", "plain.js"), "utf8")).toContain('import { createElement } from "../dep/element.js";');
  // The library's module, each load of it counted, which a test's
  // `jest.resetModules()` loads anew: loaded where it's called, not where
  // the consumer loads, and anew at each call once its cache is cleared.
  const element = join(dir, "dep", "element.js");
  writeFileSync(element, `globalThis.loads = (globalThis.loads ?? 0) + 1;
const load = globalThis.loads;
globalThis.loaded.push(load);
export function createElement(tag) {
  return \`<\${tag}\${load}>\`;
}
export function onSomething() {
  return this != null;
}
`);
  const scope = globalThis as Record<string, unknown>;
  scope.loaded = [];
  scope.loads = 0;
  const app = await import(join(dir, "app", "lib.js"));
  expect(scope.loaded).toEqual([]);
  expect(app.made()).toBe("<div1>");
  delete require.cache[element];
  expect(app.twice()).toBe("<a2><b2>");
  expect(app.called()).toBe(false);
  expect([app.through(), app.shadowed()]).toEqual(["<t2>", "<r2>"]);
  expect(scope.loaded).toEqual([1, 2]);
});

test("require on a use of what isn't a library's function is refused", () => {
  const dir = fixture("required-use-refused");
  const refused = (source: string) => {
    const ran = compiled(dir, { "lib.rs": source });
    expect(ran.code).not.toBe(0);
    return ran.stderr;
  };
  expect(refused("#[cfg_attr(rust_js, rust_js::require)]\nuse dep::element::{create_element, on_something};\n\npub fn f() -> bool {\n    create_element(\"a\").is_empty() && on_something()\n}\n"))
    .toContain("`#[rust_js::require]` on a `use` names the `use` of one function");
  expect(refused("#[cfg_attr(rust_js, rust_js::require)]\npub use dep::create_element;\n"))
    .toContain("a `pub use` is the module's `export .. from`");
  expect(refused("#[cfg_attr(rust_js, rust_js::require)]\nuse dep::sep;\n\npub fn f() -> String {\n    sep()\n}\n"))
    .toContain("of a binding is the binding's own");
  expect(refused("mod inner {\n    pub fn f() -> u32 {\n        1\n    }\n}\n\n#[cfg_attr(rust_js, rust_js::require)]\nuse inner::f;\n\npub fn g() -> u32 {\n    f()\n}\n"))
    .toContain("the crate's own modules load with it");
  expect(refused("#[cfg_attr(rust_js, rust_js::require)]\nuse dep::element;\n\npub fn f() -> String {\n    element::create_element(\"a\")\n}\n"))
    .toContain("is of a `use` of another crate's function");
});
