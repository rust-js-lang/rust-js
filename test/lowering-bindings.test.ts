// What the compiler writes where a program meets JS: a binding's static
// properties, a call that throws, `js::Unknown`, `js::is_defined`, `on_load!`
// bodies, a handler's return and a global's const. Each test is a program of
// its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0024: `get X.y` of a function without a receiver reads a class's static
// property each time it's called, `Notification.permission`.
test("a binding reads a static property", async () => {
  const dir = fixture("static-getter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get Math.PI")]
fn pi() -> f64 {
    unreachable!()
}

pub fn read() -> f64 {
    pi() * 2.0
}

pub fn reader() -> fn() -> f64 {
    pi
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return Math.PI * 2;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.read(), lib.reader()()]).toEqual([Math.PI * 2, Math.PI]);
});

// ADR 0035: a JS call whose throw is an `Err`, its result ignored, is
// `try { call; } catch {}`, as a person writes it, not `$try(() => call)`.
test("an ignored result of a call that throws is try and an empty catch", async () => {
  const dir = fixture("ignored-try");
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.risky")]
#[allow(unused_variables)]
fn risky(n: f64) -> Result<f64, &'static JsError> {
    unreachable!()
}

pub fn ignored(n: f64) {
    let _ = risky(n);
}

pub fn twice(n: f64) {
    let _ = risky(n);
    _ = risky(n + 1.0);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$try");
  expect(js).toContain("export function ignored(n) {\n  try {\n    globalThis.risky(n);\n  } catch {}\n}");
  const lib = await import(join(dir, "lib.js"));
  const calls: number[] = [];
  (globalThis as any).risky = (n: number) => {
    calls.push(n);
    if (n < 0) throw new Error("negative");
    return n;
  };
  lib.ignored(-1);
  lib.twice(-2);
  expect(calls).toEqual([-1, -2, -1]);
});

// ADR 0637: a `match` of a throwing call's `Result` whose `Err` arm reads
// no error, or binds it, is `try { .. } catch { .. }` of the call, as
// React's `getComponentNameFromType` writes `try { .. init(payload) .. }
// catch (x) { return null; }`. Only the call is in the `try`, as only it is
// in Rust: the `Ok` arm's code is after it, its value a `let` the `try`
// sets, where it may throw and the `Err` arm leaves; in it, where it can't.
// Otherwise, `$try`.
test("a match of a throwing call's Result is try and catch", async () => {
  const dir = fixture("matched-try");
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.risky")]
#[allow(unused_variables)]
fn risky(n: f64) -> Result<f64, &'static JsError> {
    unreachable!()
}

fn checked(n: f64) -> f64 {
    if n > 100.0 {
        panic!("too big");
    }
    n * 2.0
}

pub fn returned(n: f64) -> Option<f64> {
    return match risky(n) {
        Ok(resolved) => Some(checked(resolved)),
        Err(_) => None,
    };
}

pub fn tail(n: f64) -> Option<f64> {
    match risky(n) {
        Ok(v) => Some(checked(v)),
        Err(_) => None,
    }
}

pub fn named(n: f64) -> String {
    match risky(n) {
        Ok(v) => format!("{}", checked(v)),
        Err(error) => error.message(),
    }
}

pub fn err_first(n: f64) -> f64 {
    match risky(n) {
        Err(_) => -1.0,
        Ok(v) => checked(v),
    }
}

pub fn kept(n: f64) -> f64 {
    match risky(n) {
        Ok(v) => v,
        Err(_) => -1.0,
    }
}

pub fn questioned(n: f64) -> Result<f64, &'static JsError> {
    let v = risky(n)?;
    Ok(checked(v))
}

pub fn shared(n: f64) -> f64 {
    let doubled = match risky(n) {
        Ok(v) => checked(v),
        Err(_) => 0.0,
    };
    doubled + 1.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "export function returned(n) {\n  let resolved;\n  try {\n    resolved = globalThis.risky(n);\n  } catch {\n    return;\n  }\n  return checked(resolved);\n}",
  );
  expect(js).toContain("export function tail(n) {\n  let v;\n  try {\n    v = globalThis.risky(n);\n  } catch {\n    return;\n  }\n  return checked(v);\n}");
  expect(js).toContain("  } catch (error) {\n    return error.message;\n  }\n  return $displayF64(checked(v));\n");
  expect(js).toContain("export function errFirst(n) {\n  let v;\n  try {\n    v = globalThis.risky(n);\n  } catch {\n    return -1;\n  }\n  return checked(v);\n}");
  expect(js).toContain("export function kept(n) {\n  try {\n    return globalThis.risky(n);\n  } catch {\n    return -1;\n  }\n}");
  // An `Err` arm that goes on, and an `Ok` arm that may throw; a `Result`
  // read otherwise, as `?` returns it: `$try`.
  expect(js).toContain("export function shared(n) {\n  const risky = $try(() => globalThis.risky(n));\n");
  expect(js).toContain("export function questioned(n) {\n  const result = $try(() => globalThis.risky(n));\n");
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).risky = (n: number) => {
    if (n < 0) throw new TypeError("negative");
    return n;
  };
  expect([lib.returned(2), lib.returned(-1), lib.tail(3), lib.tail(-1)]).toEqual([4, undefined, 6, undefined]);
  expect([lib.named(2), lib.named(-1), lib.kept(5), lib.kept(-5), lib.shared(2), lib.shared(-2)]).toEqual(["4", "negative", 5, -1, 5, 1]);
  // What the `Ok` arm throws isn't the `Err`'s: it's thrown on.
  expect([lib.errFirst(3), lib.errFirst(-3)]).toEqual([6, -1]);
  expect(lib.questioned(2)).toEqual({ TAG: "Ok", _0: 4 });
  expect(lib.questioned(-2)._0).toBeInstanceOf(TypeError);
  for (const f of [lib.returned, lib.tail, lib.named, lib.errFirst, lib.shared]) expect(() => f(101)).toThrow("too big");
});

// ADR 0660: its `unwrap_or(d)` returned, `r.TAG === "Ok" ? r._0 : d`, is
// the `if` of its arms, each returned: `try { return f(); } catch {
// return d; }`. Held, it's `$try`'s.
test("a returned unwrap_or of a throwing call is try and catch", async () => {
  const dir = fixture("unwrap-or-try");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn text(value: Option<&Unknown>) -> String {
    js::concat_text(value).unwrap_or_default()
}

pub fn or(value: Option<&Unknown>) -> String {
    js::concat_text(value).unwrap_or("none".to_string())
}

pub fn ok(value: Option<&Unknown>) -> Option<String> {
    js::concat_text(value).ok()
}

pub fn held(value: Option<&Unknown>) -> String {
    let text = js::concat_text(value);
    let len = text.as_ref().map_or(0, |t| t.len());
    format!("{}{len}", text.unwrap_or_default())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function text(value) {\n  try {\n    return "" + value;\n  } catch {\n    return "";\n  }\n}');
  expect(js).toContain('export function or(value) {\n  try {\n    return "" + value;\n  } catch {\n    return "none";\n  }\n}');
  expect(js).toContain('export function ok(value) {\n  try {\n    return "" + value;\n  } catch {\n    return;\n  }\n}');
  expect(js).toContain("const text = $try(");
  const lib = await import(join(dir, "lib.js"));
  const symbol = Symbol("s");
  expect([lib.text(5), lib.text(symbol), lib.or(symbol), lib.ok("a"), lib.ok(symbol)]).toEqual(["5", "", "none", "a", undefined]);
  expect([lib.held("ab"), lib.held(symbol)]).toEqual(["ab2", "0"]);
});

// `set X.y` of a function without a receiver writes a global's property,
// `process.exitCode = 1`, as `get X.y` reads one.
test("a binding sets a static property", async () => {
  const dir = fixture("static-setter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get globalThis.counter")]
fn counter() -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set globalThis.counter")]
#[allow(unused_variables)]
fn set_counter(value: f64) {
    unreachable!()
}

pub fn bump() -> f64 {
    set_counter(counter() + 1.0);
    counter()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("globalThis.counter++;");
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).counter = 1;
  expect(lib.bump()).toBe(2);
});

// ADR 0285: `std::ptr::eq` of JS objects is whether they're one, `===`.
test("ptr::eq of JS objects is whether they're one", async () => {
  const dir = fixture("ptr-eq");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};

pub fn same(a: &Date, b: &Date) -> bool {
    std::ptr::eq(a, b)
}

pub fn made() -> (bool, bool) {
    let a = date::new(Some(&0.0));
    let b = date::new(Some(&0.0));
    (core::ptr::eq(a, a), std::ptr::eq(a as *const Date, b))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return a === b;");
  const lib = await import(join(dir, "mir.js"));
  const d = new Date(0);
  expect([lib.same(d, d), lib.same(d, new Date(0)), ...lib.made()]).toEqual([true, false, true, false]);
});

// ADR 0225: a JS value of unknown shape is a `js::Unknown`, as TypeScript's
// `unknown` and ReScript's are, which `classify` tells by `typeof`, and
// whose properties are read and set by name, as `obj[key]` is.
test("an unknown JS value is classified, and its properties read by name", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unknown");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Promise, Unknown, classify, json, object};
fn show(value: Option<&Unknown>) -> String {
    match classify(value) {
        Kind::Undefined | Kind::Null => "null".to_string(),
        Kind::String(text) => format!("'{text}'"),
        Kind::Number(n) => n.to_string(),
        Kind::BigInt(n) => format!("{n}n"),
        Kind::Bool(b) => b.to_string(),
        Kind::Symbol(symbol) => symbol.to_string(),
        Kind::Function(_) => "fn".to_string(),
        Kind::Array(items) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
        Kind::Object(fields) => format!(
            "{{{}}}",
            object::keys(fields).iter().map(|key| format!("{key}:{}", show(js::get(fields, key)))).collect::<Vec<_>>().join(",")
        ),
    }
}
pub fn parsed(text: &str) -> String {
    match json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn renamed(text: &str) -> String {
    match json::parse(text) {
        Ok(Some(value)) => {
            js::set(value, "name", "new");
            show(Some(value))
        }
        _ => "invalid".to_string(),
    }
}
pub fn shown(value: &Unknown) -> String {
    show(Some(value))
}
pub fn body(response: &webapi::Response) -> Promise<Option<&'static Unknown>> {
    response.json()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `classify` is the value, each variant told by `typeof`, a `switch` of
  // it, an array by `Array.isArray` before it (ADR 0668); a key that's a
  // variable is `[key]`, one written that's a name `.name`.
  expect(js).toContain('}\n  if (Array.isArray(value)) {\n    return `[${value.map(show).join(",")}]`;\n  }\n  switch (typeof value) {\n    case "string":');
  expect(js).toContain('    case "symbol":\n      return value.toString();\n    case "function":\n      return "fn";\n    case "object":');
  expect(js).toContain("Object.keys(value)");
  expect(js).toContain("show(value[key])");
  expect(js).toContain('  if (value$1.TAG === "Ok" && value$1._0 != null) {\n    const value = value$1._0;\n    value.name = "new";\n    return show(value);');
  expect(js).toContain("const value$1 = $try(() => JSON.parse(text));");
  expect(js).toContain("return response.json();");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("12"), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "12", "null", "invalid"]);
  expect(lib.renamed('{"name":"old","n":2}')).toBe("{name:'new',n:2}");
  expect([lib.shown(() => 1), lib.shown({ f: Math.max })]).toEqual(["fn", "{f:fn}"]);
});

// ADR 0267: each `on_load!` body is a function's, its locals its own: two
// that bind one name are two variables at the module's top.
test("on_load bodies that bind one name each keep their own", async () => {
  const dir = fixture("on-load-scopes");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadSeen")] #[allow(unused_variables)] fn seen(n: u32) { unreachable!() }
fn pair() -> (u32, u32) {
    (1, 0)
}
js::on_load! {
    let (x, y) = pair();
    seen(x + y);
}
js::on_load! {
    let x = 2;
    seen(x);
}
pub fn ready() -> u32 {
    other::three()
}
pub mod other {
    js::on_load! {
        let x = 4;
        super::seen(x);
    }
    pub fn three() -> u32 {
        3
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("const [x, y] = pair();\nglobalThis.onLoadSeen((x + y) >>> 0);\nconst x$1 = 2;\nglobalThis.onLoadSeen(x$1);");
  // Another module's top is its own.
  expect(readFileSync(join(dir, "other.js"), "utf8")).toContain("const x = 4;");
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { onLoadSeen?: (n: number) => void };
  globals.onLoadSeen = (n) => seen.push(n);
  try {
    expect((await import(join(dir, "lib.js"))).ready()).toBe(3);
    expect(seen).toEqual([4, 1, 2]);
  } finally {
    delete globals.onLoadSeen;
  }
});

// ADR 0212: a props field's default, \`#[rust_js::default]\`, is where JS
// takes the props apart, \`{ className = "language-js" }\`: a
// field left out reads its default.
test("a props field's default is read where the props are taken apart", async () => {
  const dir = fixture("mir-prop-defaults");
  writeFileSync(join(dir, "lib.rs"), `pub struct CodeProps<'a> {
    #[rust_js::default = "language-js"]
    pub class_name: &'a str,
    #[rust_js::default = ""]
    pub children: &'a str,
    pub meta: Option<&'a str>,
}
pub fn describe(props: &CodeProps) -> String {
    let CodeProps { class_name, children: code, meta } = props;
    format!("{class_name}:{}:{}", code.len(), meta.unwrap_or("-"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  const { className = "language-js", children: code = "", meta } = props;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.describe({}), lib.describe({ className: "css", children: "ab", meta: "m" })]).toEqual(["language-js:0:-", "css:2:m"]);
});

// ADR 0364: an `on_load!` body from MIR, which ends a loop by its end, is
// its module's top all the same, where JS has no `return`: its `return` a
// `break` of a labeled block of it (ADR 0267).
test("an on_load body's loop ends at the module's top", async () => {
  const dir = fixture("on-load-loop");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadLooped")] #[allow(unused_variables)] fn seen(n: u32) { unreachable!() }
js::on_load! {
    let mut it = [1u32, 2, 3].iter();
    while let Some(&n) = it.next() {
        if n == 3 {
            return;
        }
        seen(n);
    }
    seen(4);
}
pub fn ready() -> u32 {
    1
}
`);
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { onLoadLooped?: (n: number) => void };
  globals.onLoadLooped = (n) => seen.push(n);
  try {
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    seen.length = 0;
    expect((await import(join(dir, "mir.js"))).ready()).toBe(1);
    expect(seen).toEqual([1, 2]);
  } finally {
    delete globals.onLoadLooped;
  }
});

// ADR 0031: a constant borrowed, `&XS`, is the one there is, as nothing can
// change it through a shared reference, a library's too (ADR 0100); one
// used by value, which may be, is copied.
test("a constant borrowed is the one there is, a library's too", async () => {
  const dir = fixture("borrowed-const");
  writeFileSync(join(dir, "lib.rs"), `pub mod files {
    pub const XS: [&str; 2] = ["a", "b"];
}

use files::XS;

pub fn has(x: &str) -> bool {
    XS.contains(&x)
}

pub fn changed() -> (&'static str, &'static str) {
    let mut copy = XS;
    copy[0] = "z";
    (copy[0], XS[0])
}

pub fn most(x: u32) -> bool {
    [x].contains(&u32::MAX)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return XS.includes(x);");
  expect(js).toContain("XS.slice()");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.has("b"), lib.has("c"), lib.changed(), lib.most(4294967295)]).toEqual([true, false, ["z", "a"], true]);
});

// An unread getter's read stays: a binding's getter may do something, as
// `el.offsetWidth` lays the page out again, which restarts an animation.
test("an unread getter of a binding is still read", async () => {
  const dir = fixture("unread-getter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get offsetWidth")] #[allow(unused_variables)] fn offset_width(this: &js::JsObject) -> f64 { unreachable!() }

pub fn reflow(el: &js::JsObject) {
    let _width = offset_width(el);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("el.offsetWidth");
  const lib = await import(join(dir, "lib.js"));
  let laidOut = 0;
  lib.reflow({ get offsetWidth() { laidOut += 1; return 10; } });
  expect(laidOut).toBe(1);
});

// ADR 0286: a component that renders nothing sometimes returns an
// `Option<JSX::Element>`, `undefined` where it's None, as react.dev's
// DownloadButton has `return null`, and `jsx!` takes it as it takes one
// that always renders, memo's too. React gives it no dictionary, a
// generic one's, as it gives none of a component (ADR 0201).
test("a component of an optional element is a component", async () => {
  const dir = fixture("optional-components");
  writeFileSync(join(dir, "lib.rs"), `use react::{JSX, MemoExoticComponent, ReactNode, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
    pub shown: bool,
}

pub fn Label(LabelProps { text, shown }: LabelProps) -> Option<JSX::Element> {
    if !shown {
        return None;
    }
    Some(jsx! { <b>{text}</b> })
}

thread_local! {
    pub static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
}

pub struct ShownProps<C> {
    pub children: C,
}

pub fn Shown<C: ReactNode + Default>(ShownProps { children }: ShownProps<C>) -> Option<JSX::Element> {
    Some(jsx! { <i>{children}</i> })
}

pub fn Page() -> JSX::Element {
    jsx! {
        <main>
            <Label text="hi" shown={true} />
            <Label text="no" shown={false} />
            <Memoized text="memo" shown={true} />
            <Shown>{"kid"}</Shown>
        </main>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('<Label text="hi" shown />');
  expect(js).toContain("    return;");
  expect(js).toContain("export function Shown({ children }) {");
  const { renderToStaticMarkup } = await import("react-dom/server");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page())).toBe("<main><b>hi</b><b>memo</b><i>kid</i></main>");
});

// ADR 0254: what jsx! captures for a key that might do something, an
// import it reads, react's `Fragment`, reads alike wherever it's read, as
// a binding of a module never changes: it's written in place, not
// `const match = Fragment`, as react.dev's NavigationBar gives Headless
// UI's `Listbox.Option` `as={Fragment}`.
test("an import jsx! captures is read in place", () => {
  const dir = fixture("captured-imports");
  writeFileSync(join(dir, "lib.rs"), `use react::{ElementType, FRAGMENT, JSX, jsx};

pub struct OptionProps {
    pub value: String,
    pub r#as: Option<ElementType>,
}

pub fn Choice(OptionProps { value, .. }: OptionProps) -> JSX::Element {
    jsx! { <li>{value}</li> }
}

pub fn Choices(items: Vec<String>) -> JSX::Element {
    jsx! {
        <ul>
            {items.iter().map(|item| jsx! { <Choice key={item.as_str()} value={item.clone()} r#as={Some(FRAGMENT)} /> }).collect::<Vec<_>>()}
        </ul>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("<Choice key={item} value={item} as={Fragment} />");
  expect(js).not.toContain("const match");
});

// ADR 0040: React ignores what a DOM element's handler returns, so one of a
// single call is `() => f()`; a component's callback may read it, so a
// callback that drops its value, `on_check`'s, keeps the statement there.
test("only a DOM element's handler returns its one call", async () => {
  const dir = fixture("handler-returns");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
fn parse_int(s: &str) -> f64 {
    unreachable!()
}

pub struct CustomProps {
    pub on_check: Box<dyn Fn()>,
}

pub fn Custom(CustomProps { on_check }: CustomProps) -> JSX::Element {
    jsx! { <b>{"x"}</b> }
}

pub fn Page() -> JSX::Element {
    jsx! {
        <div onClick={|_| { parse_int("7"); }}>
            <Custom onCheck={Box::new(|| { parse_int("7"); })} />
        </div>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('<div onClick={() => parseInt("7")}>');
  const lib = await import(join(dir, "lib.jsx"));
  const custom = lib.Page().props.children;
  expect(custom.props.onCheck()).toBeUndefined();
});

// ADR 0040: a component's callback of one call whose JS gives `undefined`
// anyway returns it, `onSubmit={() => submit(false)}`, as react.dev writes
// it: a function of the crate's that gives `()`, a closure of its own, or a
// binding that says so, `#[rust_js::returns_undefined]`, React's setter's.
// A binding that doesn't, `parseInt`, keeps its value dropped.
test("a component's callback returns a call that gives undefined", async () => {
  const dir = fixture("undefined-callbacks");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use std::rc::Rc;
use react::{JSX, jsx};

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
fn parse_int(s: &str) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
#[allow(unused_variables)]
fn record(n: i32) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
#[cfg_attr(rust_js, rust_js::returns_undefined)]
#[allow(unused_variables)]
fn noted(n: i32) {
    unreachable!()
}

pub struct CardProps {
    pub on_submit: Box<dyn Fn()>,
    pub on_next: Rc<dyn Fn()>,
    pub on_note: Option<Box<dyn Fn()>>,
    pub on_parse: Box<dyn Fn()>,
    pub on_record: Box<dyn Fn()>,
}

pub fn Card(CardProps { .. }: CardProps) -> JSX::Element {
    jsx! { <b>{"card"}</b> }
}

fn submit(flag: bool) {
    record(if flag { 1 } else { 2 })
}

pub fn Page() -> JSX::Element {
    let next = move || {
        record(3);
    };
    jsx! {
        <Card
            onSubmit={Box::new(|| submit(false))}
            onNext={Rc::new(move || next())}
            onNote={Some(Box::new(|| noted(4)))}
            onParse={Box::new(|| { parse_int("7"); })}
            onRecord={Box::new(|| record(5))}
        />
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("onSubmit={() => submit(false)}");
  expect(js).toContain("onNext={() => next()}");
  expect(js).toContain("onNote={() => globalThis.record(4)}");
  expect(js).toContain('onParse={() => {\n        parseInt("7");\n      }}');
  expect(js).toContain("onRecord={() => {\n        globalThis.record(5);\n      }}");
  const previous = globalThis.record;
  globalThis.record = () => 9;
  try {
    const { props } = (await import(join(dir, "lib.jsx"))).Page();
    expect([props.onSubmit(), props.onNext(), props.onParse(), props.onRecord()]).toEqual([undefined, undefined, undefined, undefined]);
  } finally { globalThis.record = previous; }
});

// ADR 0331: a binding's `&dyn Any` is any JS value, given the value, not
// the pair that downcasts it: of each in a `Vec` and a slice too, as
// react.dev's CodeBlock gives CodeMirror's `HighlightStyle.define` its specs.
test("a binding is given each dyn Any's value", async () => {
  const dir = fixture("any-to-js");
  writeFileSync(join(dir, "lib.rs"), `use std::any::Any;

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[allow(unused_variables)]
fn stringify_all(values: Vec<&dyn Any>) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[allow(unused_variables)]
fn stringify_slice(values: &[&dyn Any]) -> String {
    unreachable!()
}

struct Spec {
    tag: &'static str,
}

pub fn all() -> String {
    stringify_all(vec![&Spec { tag: "link" }, &3])
}

pub fn slice() -> String {
    let specs: [&dyn Any; 2] = [&Spec { tag: "strong" }, &"bold"];
    stringify_slice(&specs)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const { all, slice } = await import(join(dir, "lib.js"));
  expect(all()).toBe('[{"tag":"link"},3]');
  expect(slice()).toBe('[{"tag":"strong"},"bold"]');
});

// ADR 0102: `clearTimeout` and `clearInterval` take an id or `undefined`, as
// TypeScript types them, `number | undefined`: react.dev clears a timeout
// its listener may not have set, `clearTimeout(timeout)`.
test("a timer is cleared of an id or none", async () => {
  const dir = fixture("cleared-timers");
  writeFileSync(join(dir, "lib.rs"), `pub fn cleared(id: Option<&js::TimeoutId>) {
    js::clear_timeout(id);
}

pub fn stopped(id: Option<&js::IntervalId>) {
    js::clear_interval(id);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  clearTimeout(id);");
  expect(js).toContain("  clearInterval(id);");
  const lib = await import(join(dir, "lib.js"));
  let ran = false;
  const timer = setTimeout(() => { ran = true; }, 0);
  lib.cleared(undefined);
  lib.stopped(undefined);
  lib.cleared(timer);
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(ran).toBe(false);
});

// ADR 0622: a global binding a module names, as its default or by `pub
// use`, is the module's `const`, the JS function as it is, which JS calls
// with its own arguments: React's `const assign = Object.assign; export
// default assign;`.
test("a global binding a module names as its default or by pub use is its const", async () => {
  const dir = fixture("bound-globals");
  writeFileSync(join(dir, "lib.rs"), `pub mod assign;
pub mod keys;

mod bound {
    #[cfg_attr(rust_js, rust_js::link_name = "toUpperCase")]
    pub fn to_upper(this: &str) -> String {
        let _ = this;
        unreachable!()
    }
}
`);
  writeFileSync(join(dir, "assign.rs"), `use js::object::assign;

js::export_default!(assign);
`);
  writeFileSync(join(dir, "keys.rs"), `pub use crate::bound::to_upper;
pub use js::object::keys;
pub use js::object::keys as names;
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const assign = readFileSync(join(dir, "assign.js"), "utf8");
  expect(assign).toContain("\nconst assign = Object.assign;\n\nexport default assign;\n");
  const keys = readFileSync(join(dir, "keys.js"), "utf8");
  expect(keys).toContain("\nexport const keys = Object.keys;\nexport const names = Object.keys;\n");
  // A method's binding is no global to hold.
  expect(keys).not.toContain("toUpper");
  const merged = (await import(join(dir, "assign.js"))).default({}, { a: 1 }, null, { b: 2 });
  expect(merged).toEqual({ a: 1, b: 2 });
  expect((await import(join(dir, "keys.js"))).names({ x: 1 })).toEqual(["x"]);
});

// ADR 0633: a caller that names a binding through the module that holds it
// reads it through that module, as React's reconciler reads
// `shared/assign` and `shared/ReactSharedInternals`: imported by that
// module's export and read by that name, called with JS's arguments. A
// binding's static is a module's `const` as a global's function is, and a
// caller that names the binding itself reads it in place.
test("a binding named through the module that holds it is read through it", async () => {
  const dir = fixture("held-bindings");
  writeFileSync(join(dir, "lib.rs"), `pub mod assign;
pub mod direct;
pub mod internals;
pub mod keys;
pub mod names;
pub mod reconciler;

pub struct Internals(core::marker::PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "ReactSharedInternals")]
    #[link_name = "react#*.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE"]
    pub safe static REACT_SHARED_INTERNALS: &'static Internals;
}
`);
  writeFileSync(join(dir, "assign.rs"), `pub(crate) use js::object::assign;

js::export_default!(assign);
`);
  writeFileSync(join(dir, "internals.rs"), `pub(crate) use crate::REACT_SHARED_INTERNALS;

js::export_default!(REACT_SHARED_INTERNALS);
`);
  writeFileSync(join(dir, "keys.rs"), `pub use js::object::keys;
`);
  // Named through the module that holds it, re-exported from it.
  writeFileSync(join(dir, "names.rs"), `pub use crate::keys::keys;
pub use crate::keys::keys as names;
`);
  writeFileSync(join(dir, "reconciler.rs"), `use crate::Internals;
use crate::assign::assign;
use crate::internals::REACT_SHARED_INTERNALS;
use js::Unknown;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    assign(target, (source,))
}

pub fn listed(object: &Unknown) -> Vec<String> {
    crate::keys::keys(object)
}

pub fn merger() -> for<'a> fn(&'a Unknown, &'static [Option<&'static Unknown>]) -> &'a Unknown {
    assign
}

pub fn internals() -> &'static Internals {
    REACT_SHARED_INTERNALS
}
`);
  writeFileSync(join(dir, "direct.rs"), `use crate::Internals;
use js::Unknown;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    js::object::assign(target, (source,))
}

pub fn internals() -> &'static Internals {
    crate::REACT_SHARED_INTERNALS
}
`);
  // A stand-in for React, which shares its internals with its renderers.
  mkdirSync(join(dir, "node_modules/react"), { recursive: true });
  writeFileSync(join(dir, "node_modules/react/package.json"), '{ "name": "react", "type": "module", "main": "index.js" }\n');
  writeFileSync(join(dir, "node_modules/react/index.js"), "export const __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE = { H: null };\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const internals = readFileSync(join(dir, "internals.js"), "utf8");
  expect(internals).toContain('import * as react from "react";\n\nconst ReactSharedInternals = react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;\n\nexport default ReactSharedInternals;\n');
  const reconciler = readFileSync(join(dir, "reconciler.js"), "utf8");
  expect(reconciler).toContain('import assign from "./assign.js";\nimport ReactSharedInternals from "./internals.js";\nimport { keys } from "./keys.js";\n');
  expect(reconciler).toContain("  return assign(target, source);\n");
  expect(reconciler).toContain("  return keys(object);\n");
  expect(reconciler).toContain("  return (target, sources) => assign(target, ...sources);\n");
  expect(reconciler).toContain("  return ReactSharedInternals;\n");
  expect(reconciler).not.toContain("Object.");
  expect(reconciler).not.toContain('"react"');
  expect(readFileSync(join(dir, "names.js"), "utf8")).toContain('\nexport { keys, keys as names } from "./keys.js";\n');
  expect((await import(join(dir, "names.js"))).names({ z: 0 })).toEqual(["z"]);
  // Named itself, it's read in place.
  const direct = readFileSync(join(dir, "direct.js"), "utf8");
  expect(direct).toContain("  return Object.assign(target, source);\n");
  expect(direct).toContain("  return react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;\n");
  expect(direct).not.toContain("./");
  const lib = await import(join(dir, "reconciler.js"));
  expect(lib.merged({ a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
  expect(lib.merger()({}, [{ c: 3 }, null])).toEqual({ c: 3 });
  expect(lib.listed({ x: 1, y: 2 })).toEqual(["x", "y"]);
  const react = await import(join(dir, "node_modules/react/index.js"));
  expect(lib.internals()).toBe(react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE);
  expect((await import(join(dir, "direct.js"))).internals()).toBe(lib.internals());
});

// ADR 0588: a binding says it may throw by the result it's declared with,
// \`Promise<Result<..>>\`, not by what its generics are given: \`js::promise\`
// of a future that gives a \`Result\` is that promise, its \`Result\` the
// future's own, not a rejection settled.
test("a generic binding given a Result is not one that may throw", async () => {
  const dir = fixture("generic-binding-result");
  writeFileSync(join(dir, "lib.rs"), `pub async fn parse(text: String) -> Result<u32, String> {
    text.parse().map_err(|_| format!("not a number: {text}"))
}

pub async fn sum(a: String, b: String) -> Result<u32, String> {
    let (a, b) = js::promise::all2((js::promise(parse(a)), js::promise(parse(b)))).await;
    Ok(a? + b?)
}

#[rust_js::link_name = "this"]
fn same<T>(this: T) -> T {
    this
}

pub fn checked(n: u32) -> Result<u32, String> {
    same(n.checked_mul(2).ok_or_else(|| String::from("too big")))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$settle");
  const lib = await import(join(dir, "lib.js"));
  expect(await lib.sum("1", "2")).toEqual(await lib.parse("3"));
  expect(await lib.sum("1", "x")).toEqual(await lib.parse("x"));
  expect(js).not.toContain("$try");
  expect(lib.checked(2)).toEqual({ TAG: "Ok", _0: 4 });
});

// ADR 0618: \`Array.isArray\` and \`Object.getPrototypeOf\` are the js crate's,
// which a module holds as React's shared does, \`const isArrayImpl =
// Array.isArray\`.
test("Array.isArray and Object.getPrototypeOf are held as JS holds them", async () => {
  const dir = fixture("array-object-functions");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

thread_local! {
    static IS_ARRAY_IMPL: fn(Option<&Unknown>) -> bool = js::array::is_array;
    static GET_PROTOTYPE_OF: fn(Option<&Unknown>) -> Option<&'static Unknown> = js::object::get_prototype_of;
}

pub fn listed(a: Option<&Unknown>) -> bool {
    IS_ARRAY_IMPL.with(|is_array_impl| is_array_impl(a))
}

pub fn orphan(value: &Unknown) -> bool {
    GET_PROTOTYPE_OF.with(|get_prototype_of| get_prototype_of(Some(value))).is_none()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const isArrayImpl = Array.isArray;");
  expect(js).toContain("const getPrototypeOf = Object.getPrototypeOf;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.listed([1]), lib.listed({}), lib.listed(undefined), lib.orphan(Object.create(null)), lib.orphan({})]).toEqual([true, false, false, true, false]);
});

// ADR 0096: a static whose value is a function, its initializer lowered as
// code, imports a module's binding it reads, as a function's body does:
// `export const notify = toast;` and `import { toast } from "./toast.js"`.
test("a static of a module's binding imports it", async () => {
  const dir = fixture("static-of-binding");
  writeFileSync(join(dir, "toast.js"), "export const toast = () => 7;\n");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./toast.js#toast")]
fn toast() -> f64 {
    unreachable!()
}

pub static NOTIFY: fn() -> f64 = toast;

mod inner {
    pub const SHOWN: fn() -> f64 = super::toast;
}

pub fn shown() -> f64 {
    NOTIFY() + inner::SHOWN()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`import { toast } from "./toast.js";`);
  expect(js).toContain("export const NOTIFY = toast;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shown()).toBe(14);
});

// ADR 0615: \`Object.prototype.hasOwnProperty\`, a JS function of its own,
// called with the object it asks of, \`.call(object, key)\`, as React's
// shared hasOwnProperty keeps it, \`const hasOwnProperty =
// Object.prototype.hasOwnProperty\`.
test("Object.prototype.hasOwnProperty is called with its object", async () => {
  const dir = fixture("has-own-property");
  writeFileSync(join(dir, "lib.rs"), `use js::object::HasOwnProperty;

thread_local! {
    static HAS_OWN_PROPERTY: &'static HasOwnProperty = js::object::has_own_property;
}

pub fn owns(object: &js::Unknown, key: &str) -> bool {
    HAS_OWN_PROPERTY.with(|has_own_property| has_own_property.call(object, key))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const hasOwnProperty = Object.prototype.hasOwnProperty;");
  expect(js).toContain("  return hasOwnProperty.call(object, key);");
  const lib = await import(join(dir, "lib.js"));
  const own = Object.create({ inherited: 1 });
  own.mine = 2;
  own.hasOwnProperty = () => true;
  expect([lib.owns(own, "mine"), lib.owns(own, "inherited"), lib.owns(Object.create(null), "x")]).toEqual([true, false, false]);
});

// ADR 0612: two values of any shape are equal as JS's \`===\` has them,
// and two \`Option\`s of them as std's are (amended): both \`None\`,
// whichever of \`null\` and \`undefined\` each is, or \`===\` \`Some\`s.
test("js::Unknown's == is ===, of an Option's Somes", async () => {
  const dir = fixture("unknown-equality");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn same(a: &Unknown, b: &Unknown) -> bool {
    a == b
}

pub fn same_maybe(a: Option<&Unknown>, b: Option<&Unknown>) -> bool {
    a == b
}

pub fn is(a: Option<&Unknown>, b: &Unknown) -> bool {
    a == Some(b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function same(a, b) {\n  return a === b;\n}");
  expect(js).toContain("export function sameMaybe(a, b) {\n  return (a ?? null) === (b ?? null);\n}");
  // Of a \`Some\`, never \`null\` nor \`undefined\`, \`===\` alone answers the same.
  expect(js).toContain("export function is(a, b) {\n  return a === b;\n}");
  const lib = await import(join(dir, "lib.js"));
  const o = {};
  expect([lib.same(o, o), lib.same(o, {}), lib.same(NaN, NaN), lib.sameMaybe(null, undefined), lib.sameMaybe(undefined, undefined), lib.sameMaybe(o, o), lib.sameMaybe(o, null), lib.sameMaybe(1, "1")]).toEqual([true, false, false, true, true, true, false, false]);
});

// ADR 0931: two values of any shape are ordered as JS's \`<\` and \`>\` have
// them, equal as \`===\`, else not at all: \`partial_cmp\` is \`$partialCmp\`,
// and \`<=\` its \`<= 0\`, not JS's \`<=\`, which coerces \`"1" <= 1\` true.
test("js::Unknown's < is JS's, its partial_cmp $partialCmp", async () => {
  const dir = fixture("unknown-order");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn compare(a: &Unknown, b: &Unknown) -> i32 {
    if a == b {
        0
    } else if a < b {
        -1
    } else {
        1
    }
}

pub fn greater(a: &Unknown, b: &Unknown) -> bool {
    a > b
}

pub fn at_most(a: &Unknown, b: &Unknown) -> bool {
    a <= b
}

pub fn at_least(a: &Unknown, b: &Unknown) -> bool {
    a >= b
}

pub fn ordered(a: &Unknown, b: &Unknown) -> Option<std::cmp::Ordering> {
    a.partial_cmp(b)
}

pub fn not_less(a: &Unknown, b: &Unknown) -> bool {
    a.partial_cmp(b) != Some(std::cmp::Ordering::Less)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (a === b) {\n    return 0;\n  }\n  return a < b ? -1 : 1;\n");
  expect(js).toContain("  return a > b;\n");
  expect(js).toContain("  return $partialCmp(a, b) <= 0;\n");
  // Not \`!(a < b)\`, as of floats (ADR 0966): the helper may run a
  // \`valueOf\` twice.
  expect(js).toContain("  return $partialCmp(a, b) !== -1;\n");
  const lib = await import(join(dir, "lib.js"));
  const o = {};
  expect([lib.compare(1, 2), lib.compare("b", "a"), lib.compare(o, o), lib.compare("a", "ab")]).toEqual([-1, 1, 0, -1]);
  expect([lib.greater(2, 1), lib.greater(NaN, 1), lib.atMost("1", 1), lib.atMost(1, 1), lib.atLeast(o, {}), lib.atLeast(2, 1)]).toEqual([true, false, false, true, false, true]);
  expect([lib.ordered(1, 2), lib.ordered(NaN, NaN), lib.ordered(o, {}), lib.ordered("x", "x")]).toEqual([-1, undefined, undefined, 0]);
});

// ADR 0721: \`js::strict_equals(a, b)\` is JS's \`a === b\` of any two values,
// \`Option\`s too, where a \`null\` isn't an \`undefined\`, as React's
// \`Object.is\` polyfill asks; \`==\` stays Rust's (ADR 0612).
test("js::strict_equals is JS's ===, of a null and an undefined too", async () => {
  const dir = fixture("strict-equals");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsObject, Unknown};

pub fn same(a: Option<&Unknown>, b: Option<&Unknown>) -> bool {
    js::strict_equals(a, b)
}

pub fn same_object(a: &JsObject, b: &Unknown) -> bool {
    js::strict_equals(a, b)
}

pub fn is_text(a: &Unknown, text: &str) -> bool {
    js::strict_equals(a, text)
}

pub fn is_number(a: Option<&Unknown>) -> bool {
    js::strict_equals(a, 1.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function same(a, b) {\n  return a === b;\n}");
  expect(js).toContain("export function sameObject(a, b) {\n  return a === b;\n}");
  expect(js).toContain("export function isText(a, text) {\n  return a === text;\n}");
  expect(js).toContain("export function isNumber(a) {\n  return a === 1;\n}");
  const lib = await import(join(dir, "lib.js"));
  const o = {};
  expect([lib.same(null, undefined), lib.same(null, null), lib.same(o, o), lib.same(o, {}), lib.same(NaN, NaN)]).toEqual([false, true, true, false, false]);
  expect([lib.sameObject(o, o), lib.isText("a", "a"), lib.isText(1, "1"), lib.isNumber(1), lib.isNumber(undefined)]).toEqual([true, true, false, true, false]);
});

// ADR 0624: `js::is_defined` of a path of bindings, each read from the
// one before, asks each part of it: React's canUseDOM, `typeof window !==
// "undefined" && typeof window.document !== "undefined" && typeof
// window.document.createElement !== "undefined"`. A step that isn't the
// one before's method is refused.
test("js::is_defined of a path of bindings asks each part", async () => {
  const dir = fixture("defined-paths");
  const source = (path: string) => `pub struct Page(std::marker::PhantomData<js::JsObject>);
pub struct Doc(std::marker::PhantomData<js::JsObject>);

impl Page {
    #[cfg_attr(rust_js, rust_js::link_name = "get doc")]
    pub fn doc(&self) -> &'static Doc {
        unreachable!()
    }
}

impl Doc {
    #[cfg_attr(rust_js, rust_js::link_name = "make")]
    pub fn make<T: Copy>(&self, tag: T) -> &'static js::JsObject {
        let _ = tag;
        unreachable!()
    }
}

unsafe extern "Rust" {
    #[link_name = "rustJsPage"]
    pub safe static PAGE: &'static Page;
}

pub fn can_make() -> bool {
    js::is_defined(${path})
}
`;
  writeFileSync(join(dir, "lib.rs"), source("(Page::doc, Doc::make::<u8>)"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return (\n    typeof rustJsPage !== "undefined" &&\n    typeof rustJsPage.doc !== "undefined" &&\n    typeof rustJsPage.doc.make !== "undefined"\n  );');
  const lib = await import(join(dir, "lib.js"));
  const page = globalThis as Record<string, unknown>;
  const asked = [lib.canMake()];
  page.rustJsPage = {};
  asked.push(lib.canMake());
  page.rustJsPage = { doc: {} };
  asked.push(lib.canMake());
  page.rustJsPage = { doc: { make: () => ({}) } };
  asked.push(lib.canMake());
  expect(asked).toEqual([false, false, false, true]);
  // A step that isn't a method of what the one before gives.
  writeFileSync(join(dir, "lib.rs"), source("(Page::doc, Page::doc)"));
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("a step isn't a method of what the one before gives")]);
});

// ADR 0597: whether the host has a binding's global is `js::is_defined`'s,
// `typeof X !== "undefined"`, of the binding its type names, through a
// temporary too: React's scheduler asks for `setImmediate`, then
// `MessageChannel`, then `setTimeout`.
test("js::is_defined is whether the host has a binding's global", async () => {
  const dir = fixture("defined-globals");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsPresentHook")]
pub fn present(n: f64) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new RustJsAbsentClass")]
pub fn absent() -> &'static js::JsObject {
    unreachable!()
}

pub fn called(n: f64) -> Option<f64> {
    js::is_defined(present).then(|| present(n))
}

pub fn made() -> bool {
    let make = absent;
    js::is_defined(make)
}

unsafe extern "Rust" {
    #[link_name = "rustJsAbsentValue"]
    pub safe static ABSENT_VALUE: &'static js::JsObject;
}

pub fn valued() -> bool {
    js::is_defined(ABSENT_VALUE)
}

// A static of a path, each part asked (ADR 0624), as React asks
// \`Symbol.toStringTag\`.
pub fn tagged() -> bool {
    js::is_defined(js::symbol::to_string_tag)
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsTools.hook")]
pub fn tool_hook(n: f64) -> f64 {
    unreachable!()
}

pub struct Clock(std::marker::PhantomData<js::JsObject>);

impl Clock {
    #[cfg_attr(rust_js, rust_js::link_name = "now")]
    pub fn now(&self) -> f64 {
        unreachable!()
    }
}

unsafe extern "Rust" {
    #[link_name = "rustJsClock"]
    pub safe static CLOCK: &'static Clock;
}

pub fn tooled() -> bool {
    js::is_defined(tool_hook)
}

pub fn timed() -> bool {
    js::is_defined(Clock::now)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('typeof rustJsPresentHook !== "undefined"');
  expect(js).toContain('typeof RustJsAbsentClass !== "undefined"');
  expect(js).toContain('return typeof rustJsAbsentValue !== "undefined";');
  expect(js).toContain('return typeof Symbol !== "undefined" && typeof Symbol.toStringTag !== "undefined";');
  // Each part of a path, and a method of a global's type of that global.
  expect(js).toContain('return typeof rustJsTools !== "undefined" && typeof rustJsTools.hook !== "undefined";');
  expect(js).toContain('return typeof rustJsClock !== "undefined" && typeof rustJsClock.now !== "undefined";');
  (globalThis as Record<string, unknown>).rustJsPresentHook = (n: number) => n + 1;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.called(1), lib.made(), lib.valued(), lib.tooled(), lib.timed()]).toEqual([2, false, false, false, false]);
  (globalThis as Record<string, unknown>).rustJsTools = {};
  (globalThis as Record<string, unknown>).rustJsClock = { now: () => 1 };
  expect([lib.tooled(), lib.timed()]).toEqual([false, true]);
});

// ADR 0672: a `get x` binding's read is one a getter may answer, as
// `js::get`'s is, which the binding's lowering says of the expression it
// writes (`js::Known::Getter`): a let chain that tests and binds it reads it
// once.
test("a get binding's property a let chain tests and binds is read once", async () => {
  const dir = fixture("get-binding-read-once");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get url")]
#[allow(unused_variables)]
fn url(this: &js::JsObject) -> Option<String> {
    unreachable!()
}

pub fn long_url(page: &js::JsObject, ready: bool) -> String {
    if ready
        && let Some(found) = url(page)
        && found.len() > 3
    {
        return found;
    }
    String::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  let reads = 0;
  const page = (url: unknown) => ({
    get url() {
      reads += 1;
      return url;
    },
  });
  expect(lib.longUrl(page("/api/x"), true)).toBe("/api/x");
  expect(lib.longUrl(page("/a"), true)).toBe("");
  expect(lib.longUrl(page("/api/x"), false)).toBe("");
  expect(reads).toBe(2);
});

// ADR 0672: `js::get(request, "url")` reads the property once, as Rust
// calls it once, so a getter runs once. Joined into the `if` before it,
// `request != null && typeof request.url === "string"`, the read a `let`
// chain binds was read again for its binding, `const url = request.url;`,
// where React's `getIODescription` tests `value.request.url`: a getter ran
// twice. A test reads in place what's bound before it only where that's read
// once, or reads nothing but variables.
test("a property a let chain tests and binds is read once", async () => {
  const dir = fixture("property-read-once");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn requested_url(value: &Unknown) -> String {
    if let Some(request) = js::get(value, "request")
        && let Kind::String(url) = js::classify(js::get(request, "url"))
    {
        return url.to_string();
    }
    String::new()
}

pub fn has_url(value: &Unknown) -> bool {
    if let Some(request) = js::get(value, "request")
        && let Kind::String(_) = js::classify(js::get(request, "url"))
    {
        return true;
    }
    false
}

pub fn same(value: &Unknown, other: &Unknown) -> bool {
    if let Kind::String(a) = js::classify(js::get(value, "url"))
        && let Kind::String(b) = js::classify(js::get(other, "url"))
    {
        return a == b;
    }
    false
}

pub fn text_of(value: &Unknown) -> String {
    let text: String;
    if let Kind::String(own) = js::classify(Some(value)) {
        text = own.to_string();
    } else if let Kind::Object(object) = js::classify(Some(value))
        && let Some(inner) = js::get(object, "url")
        && let Kind::String(url) = js::classify(Some(inner))
    {
        text = url.to_string();
    } else {
        panic!("no text");
    }
    text
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function requestedUrl(value) {
  const request = value.request;
  if (request != null) {
    const url = request.url;
    if (typeof url === "string") {
      return url;
    }
  }
  return "";
}`);
  // Read once, in the test, which binds nothing: in place.
  expect(js).toContain(`if (request != null && typeof request.url === "string") {
    return true;
  }`);
  expect(js.match(/request\.url/g)?.length).toBe(2);
  const lib = await import(join(dir, "lib.js"));
  const reads: string[] = [];
  const counted = (name: string, url: unknown) => ({
    get request() {
      reads.push(`${name}.request`);
      return {
        get url() {
          reads.push(`${name}.url`);
          return url;
        },
      };
    },
    get url() {
      reads.push(`${name}.url`);
      return url;
    },
  });
  expect(lib.requestedUrl(counted("a", "/api"))).toBe("/api");
  expect(lib.requestedUrl(counted("b", 7))).toBe("");
  expect(lib.requestedUrl({ request: null })).toBe("");
  expect(lib.hasUrl(counted("c", "/api"))).toBe(true);
  expect(lib.same(counted("d", "/x"), counted("e", "/x"))).toBe(true);
  expect(reads).toEqual(["a.request", "a.url", "b.request", "b.url", "c.request", "c.url", "d.url", "e.url"]);
  // Where what fails either test goes on alike, the read first, where the
  // first holds, and one test of it: no label to reach the `else`.
  expect(js).toContain(`  } else {
    const inner = typeof value === "object" && !Array.isArray(value) ? value.url : undefined;
    if (typeof inner === "string") {
      text = inner;
    } else {`);
  expect(js).not.toContain("break");
  reads.length = 0;
  expect([lib.textOf("own"), lib.textOf(counted("f", "/f"))]).toEqual(["own", "/f"]);
  expect(() => lib.textOf(counted("g", 7))).toThrow("no text");
  expect(() => lib.textOf(["x"])).toThrow("no text");
  expect(reads).toEqual(["f.url", "g.url"]);
});

// ADR 0870: React's `createTask`, a thread-local of a let chain's value,
// `if __DEV__ && let Some(create_task) = console::create_task() { .. } else
// { .. }`: the getter read once, where `__DEV__` holds, then a conditional
// of it, each the module's `const`, not a closure called where it's made
// of a `let tmp` its branches set.
test("a thread-local of a let chain's value is the module's consts", async () => {
  const dir = fixture("chain-value-thread-local");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{ConsoleTask, console};

const DEV: bool = true;

type CreateTask = dyn Fn(&str) -> Option<&'static ConsoleTask>;

#[cfg_attr(rust_js, rust_js::nullable)]
fn no_task(_name: &str) -> Option<&'static ConsoleTask> {
    None
}

thread_local! {
    static CREATE_TASK: Box<CreateTask> =
        // eslint-disable-next-line react-internal/no-production-logging
        if DEV && let Some(create_task) = console::create_task() {
            Box::new(move |name| Some(create_task(name)))
        } else {
            Box::new(no_task)
        };
}

pub fn task(name: &str) -> Option<&'static ConsoleTask> {
    CREATE_TASK.with(|create| create(name))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`// eslint-disable-next-line react-internal/no-production-logging
const createTask$1 = DEV ? console.createTask : undefined;

const createTask = createTask$1 != null ? (name) => createTask$1(name) : noTask;
`);
  expect(js).not.toContain("tmp");
  expect(js).not.toContain("(() =>");
  const reads: string[] = [];
  const own = Object.getOwnPropertyDescriptor(console, "createTask");
  Object.defineProperty(console, "createTask", {
    configurable: true,
    get() {
      reads.push("createTask");
      return (name: string) => ({ name });
    },
  });
  try {
    const lib = await import(join(dir, "lib.js"));
    expect([lib.task("a"), lib.task("b")]).toEqual([{ name: "a" }, { name: "b" }]);
    expect(reads).toEqual(["createTask"]);
    // Where the host has none: `noTask`'s `null`.
    Object.defineProperty(console, "createTask", { configurable: true, value: undefined });
    const none = await import(`${join(dir, "lib.js")}?none`);
    expect(none.task("a")).toBeNull();
  } finally {
    if (own) Object.defineProperty(console, "createTask", own);
    else delete (console as { createTask?: unknown }).createTask;
  }
});

// ADR 0870: a thread-local's `const` made the module's is named apart from
// what the module's other code reads, a global of a binding's too, which
// the `const` would otherwise be.
test("a thread-local of a let chain's value is the module's consts, named apart", async () => {
  const dir = fixture("chain-value-named-apart");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsLabel")]
fn label() -> u32 {
    unreachable!()
}

const DEV: bool = true;

thread_local! {
    static FIRST: usize = if DEV && let Some(rust_js_label) = "ab".find('b') { rust_js_label } else { 0 };
}

pub fn labeled() -> u32 {
    label()
}

pub fn first() -> usize {
    FIRST.with(|first| *first)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const rustJsLabel$1 = DEV ? $find("ab", "b") : undefined;\n`);
  expect(js).toContain(`const FIRST = rustJsLabel$1 ?? 0;\n`);
  const globals = globalThis as { rustJsLabel?: () => number };
  globals.rustJsLabel = () => 7;
  try {
    const lib = await import(join(dir, "lib.js"));
    expect([lib.labeled(), lib.first()]).toEqual([7, 1]);
  } finally {
    delete globals.rustJsLabel;
  }
});
