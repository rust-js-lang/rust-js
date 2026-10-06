import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, expectSnapshot, fixture, root, run, target } from "./support";
import { decodeMappings, lookup } from "./sourcemap";

beforeAll(buildReact, 600_000);

function compile(source: string, files: Record<string, string> = {}) {
  const dir = fixture("jsx-syntax");
  for (const [name, body] of Object.entries({ "lib.rs": source, ...files })) {
    const path = join(dir, name);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, body);
  }
  const args = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", join(dir, "manifest.json"),
    "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  return { dir, args };
}

// Keep the whole emitted module, including imports and formatting. Only the
// temporary source directory varies between runs; never normalize emitted code.
function snapshot(dir: string, name: string) {
  expectSnapshot(dir, join(root, "test/snapshots/jsx", name), {
    normalize: text => text.replaceAll(dir + "/", "test/jsx/"),
  });
}

test("JSX supports components across modules, fragments, lists, conditions and spreads", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
#[rust_js::camel_case]
const _: () = ();
use react::{Element, jsx};
#[cfg(any())] mod missing;
#[cfg_attr(all(), path = "ui/card.rs")] mod card;
use card::Card as Panel;
pub struct Attrs { pub class_name: &'static str }
pub fn App() -> Element {
    let items: Vec<Element> = (0..3).map(|n| jsx! { <li key={n}>{n}</li> }).collect();
    let attrs = Attrs { class_name: "list" };
    jsx! {
        <>
            <Panel title="Numbers">
                <ul {...attrs}>{items}</ul>
            </Panel>
            {if true { Some(jsx! { <p data-state="ready">{"done"}</p> }) } else { None }}
        </>
    }
}
pub fn Spread() -> Element {
    let props = card::Props { title: "Spread", children: jsx! { <span>{"child"}</span> } };
    jsx! { <Panel {...props} /> }
}
pub fn Override() -> Element {
    let props = card::Props { title: "old", children: jsx! { <span /> } };
    jsx! { <Panel {...card::Props { title: "new", ..props }} /> }
}
`;
  const card = `use react::{Element, jsx};
pub struct Props { pub title: &'static str, pub children: Element }
pub(crate) fn Card(p: Props) -> Element {
    jsx! { <section><h1>{p.title}</h1>{p.children}</section> }
}
`;
  const { dir, args } = compile(source, { "ui/card.rs": card });
  run(args);
  snapshot(dir, "modules");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('import { Card } from "./card.jsx";');
  expect(code).toContain('<Card title="Numbers">');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.App())).toBe('<section><h1>Numbers</h1><ul class="list"><li>0</li><li>1</li><li>2</li></ul></section><p data-state="ready">done</p>');
  expect(renderToStaticMarkup(result.Spread())).toBe('<section><h1>Spread</h1><span>child</span></section>');
  expect(renderToStaticMarkup(result.Override())).toBe('<section><h1>new</h1><span></span></section>');
  const manifest = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
  expect(manifest.sources).toContain(join(dir, "ui/card.rs"));
  expect(manifest.sources.some((s: string) => s.includes("jsx expansion"))).toBe(false);
});

// A package's component (an npm one: the pilot's toasts) is a binding used
// as a JSX tag, and any binding, a function, a method or a getter, can be
// a value, called with just its arguments. Found by the pilot.
test("a component a JS module exports is a JSX tag, and a binding is a value", async () => {
  const source = `#![allow(non_snake_case)]
use react::{Element, jsx};
use react::webapi::{abort_controller, abort_signal};

pub struct BadgeProps {
    pub label: &'static str,
}

#[rust_js::link_name = "./badge.js#Badge"]
pub fn Badge(props: BadgeProps) -> Element {
    unreachable!()
}

unsafe extern "Rust" {
    #[link_name = "encodeURIComponent"]
    safe fn encode(text: &str) -> String;
    #[link_name = "parseInt"]
    safe fn parse_int(text: &str) -> f64;
}

pub fn App() -> Element {
    jsx! { <Badge label="new" /> }
}

pub fn values() -> (Vec<String>, Vec<f64>, Vec<bool>) {
    let encoded = vec!["a b", "c&d"].into_iter().map(encode).collect();
    // As \`.map\`'s own argument, \`parseInt\` would be given each index too.
    let numbers = vec!["10", "10", "10"].into_iter().map(parse_int).collect();
    let controller = abort_controller::new();
    abort_controller::abort(controller);
    let signals = vec![abort_controller::signal(controller), abort_controller::signal(abort_controller::new())];
    let aborted = signals.into_iter().map(abort_signal::aborted).collect();
    (encoded, numbers, aborted)
}
`;
  const { dir, args } = compile(source, { "badge.js": 'import { createElement } from "react"; export function Badge({ label }) { return createElement("b", null, label); }\n' });
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('import { Badge } from "./badge.js";');
  expect(code).toContain('<Badge label="new" />');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.App())).toBe("<b>new</b>");
  expect(result.values()).toEqual([["a%20b", "c%26d"], [10, 10, 10], [true, false]]);
});

test("named component imports avoid local functions, nested parameters and duplicate exports", async () => {
  const source = `#![allow(non_snake_case)]
use react::{Element, jsx};
mod first;
mod second;
pub fn Card() -> Element { jsx! { <b>{"local"}</b> } }
pub fn View() -> Element {
    let render = |Card: i32| jsx! {
        <>
            <first::Card />
            <second::Card />
            <span>{Card}</span>
        </>
    };
    render(9)
}
`;
  const { dir, args } = compile(source, {
    "first.rs": 'use react::{Element, jsx}; pub fn Card() -> Element { jsx! { <b>{"first"}</b> } }',
    "second.rs": 'use react::{Element, jsx}; pub fn Card() -> Element { jsx! { <b>{"second"}</b> } }',
  });
  run(args);
  snapshot(dir, "import-collisions");
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('import { Card as Card$2 } from "./first.jsx";');
  expect(output).toContain('import { Card as Card$3 } from "./second.jsx";');
  expect(output).toContain("<Card$2 />");
  expect(output).toContain("<Card$3 />");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.View())).toBe("<b>first</b><b>second</b><span>9</span>");
  expect(renderToStaticMarkup(result.Card())).toBe("<b>local</b>");
  const lines = output.split("\n");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  for (const [generated, original] of [["Card$2", "<first::Card"], ["Card$3", "<second::Card"]]) {
    const line = lines.findIndex(l => l.includes(`<${generated}`));
    expect(lookup(decodeMappings(map.mappings), line, lines[line].indexOf(generated))?.srcLine)
      .toBe(source.split("\n").findIndex(l => l.includes(original)));
  }
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toBe(output);
});

test("JSX preserves evaluation order and maps tags and handler statements to their original lines", async () => {
  const source = `#![allow(non_snake_case)]
use react::{Element, jsx};
unsafe extern "Rust" {
    #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32;
}
pub fn App() -> Element {
    jsx! {
        <button
            title={record(1).to_string()}
            onClick={move |_| {
                record(5);
                record(6);
            }}
        >
            <span>{record(2)}</span>
            {record(3)}
        </button>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  snapshot(dir, "evaluation-order");
  const log: number[] = [];
  const previous = globalThis.record;
  globalThis.record = (n: number) => { log.push(n); return n; };
  try {
    const result = await import(join(dir, "lib.jsx"));
    const tree = result.App();
    expect(log).toEqual([1, 2, 3]);
    tree.props.onClick();
    expect(log).toEqual([1, 2, 3, 5, 6]);
  } finally { globalThis.record = previous; }
  const js = readFileSync(join(dir, "lib.jsx"), "utf8").split("\n");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  expect(map.sourcesContent).toEqual([source]);
  const segments = decodeMappings(map.mappings);
  for (const [generated, original] of [["button", "<button"], ["globalThis.record(5)", "record(5);"], ["globalThis.record(6)", "record(6);"], ["globalThis.record(2)", "<span>{record(2)}</span>"]]) {
    const line = js.findIndex(l => l.includes(generated));
    const hit = lookup(segments, line, js[line].indexOf(generated));
    expect(hit?.srcLine, generated).toBe(source.split("\n").findIndex(l => l.includes(original)));
  }
});

test("nested component JSX stays readable, contextually typed and mapped to the original Rust", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
#[rust_js::camel_case]
const _: () = ();
use react::{Element, jsx};
use std::rc::Rc;
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32); }
pub struct Props { pub title: &'static str, pub content: Element, pub on_submit: Option<Rc<dyn Fn()>> }
pub fn Card(p: Props) -> Element { jsx! { <section title={p.title}>{p.content}</section> } }
pub fn App(active: bool) -> Element {
    jsx! {
        <main>
            <Card
                title="Ready"
                content={if active {
                    jsx! { <button onClick={move |_| { record(7); }}>{"Save"}</button> }
                } else {
                    jsx! { <span>{"Waiting"}</span> }
                }}
                onSubmit={Some(Rc::new(move || record(8)))}
            />
        </main>
    }
}
pub fn Tokens() -> Element {
    jsx! { <Card
        title={stringify!(jsx! { untouched })}
        content={
            jsx! { <i /> }
            jsx! { <span /> }
        }
        onSubmit={None}
    /> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  snapshot(dir, "nested-expressions");
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain(`export function App(active) {
  return (
    <main>
      <Card
        title="Ready"
        content={
          active ? <button onClick={() => globalThis.record(7)}>Save</button> : <span>Waiting</span>
        }
        onSubmit={() => globalThis.record(8)}
      />
    </main>
  );
}`);
  const result = await import(join(dir, "lib.jsx"));
  const log: number[] = [];
  const previous = globalThis.record;
  globalThis.record = (n: number) => { log.push(n); };
  try {
    const card = result.App(true).props.children;
    expect(card.type).toBe(result.Card);
    card.props.content.props.onClick();
    card.props.onSubmit();
    expect(log).toEqual([7, 8]);
    expect(renderToStaticMarkup(result.App(false))).toBe('<main><section title="Ready"><span>Waiting</span></section></main>');
    expect(result.Tokens().props.title).toContain("jsx!");
    expect(result.Tokens().props.title).toContain("untouched");
  } finally { globalThis.record = previous; }
  const js = output.split("\n");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  expect(map.sourcesContent).toEqual([source]);
  const segments = decodeMappings(map.mappings);
  for (const [generated, original] of [["Card", "<Card"], ["button", "<button"], ["globalThis.record(7)", "record(7);"], ["globalThis.record(8)", "record(8)"]]) {
    const line = js.findIndex((l, i) => i > js.findIndex(l => l.includes("export function App")) && l.includes(generated));
    expect(lookup(segments, line, js[line].indexOf(generated))?.srcLine, generated)
      .toBe(source.split("\n").findIndex(l => l.includes(original)));
  }
});

test("JSX children have no twelve-sibling tuple limit", async () => {
  const {dir, args} = compile('use react::{Element, jsx}; pub fn View() -> Element { jsx! { <div>' + Array.from({length: 40}, (_, i) => `<span>{${i}}</span>`).join('') + '</div> } }');
  run(args);
  snapshot(dir, "many-children");
  const result = await import(join(dir, "lib.jsx"));
  expect(result.View().props.children).toHaveLength(40);
});

test("JSX uses SVG's tag and attribute spelling", async () => {
  const { dir, args } = compile(`use react::{Element, jsx};
pub fn View() -> Element {
    jsx! { <svg viewBox="0 0 10 10"><defs><linearGradient id="paint" /></defs></svg> }
}
`);
  run(args);
  snapshot(dir, "svg");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.View())).toBe('<svg viewBox="0 0 10 10"><defs><linearGradient id="paint"></linearGradient></defs></svg>');
});

test("component props, keys and children evaluate in source order without capturing names", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, jsx};
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32; }
pub struct Props { pub title: i32, pub children: i32 }
pub(crate) fn Card(p: Props) -> Element { jsx! { <div>{p.title}{p.children}</div> } }
pub fn App() -> Element {
    let __jsx0 = 3;
    jsx! { <Card title={record(1)} key={record(2)}>{record(__jsx0)}</Card> }
}
`);
  run(args);
  snapshot(dir, "keys");
  const log: number[] = [];
  const previous = globalThis.record;
  globalThis.record = (n: number) => { log.push(n); return n; };
  try {
    const result = await import(join(dir, "lib.jsx"));
    const tree = result.App();
    expect(log).toEqual([1, 2, 3]);
    expect(tree.key).toBe("2");
    expect(tree.props).toEqual({ title: 1, children: 3 });
  } finally { globalThis.record = previous; }
});

test("JSX loads nested modules using Rust's directory and cfg rules", async () => {
  const { dir, args } = compile(`use react::Element;
mod outer;
pub fn App() -> Element { outer::view() }
`, {
    "outer.rs": `use react::{Element, jsx};
mod inner;
mod inline { pub mod leaf; }
#[cfg_attr(all(), path = "alternate.rs")] mod alternate;
pub fn view() -> Element { jsx! { <>{inner::view()}{inline::leaf::view()}{alternate::view()}</> } }
`,
    "outer/inner.rs": 'use react::{Element, jsx}; pub fn view() -> Element { jsx! { <b>{"one"}</b> } }',
    "outer/inline/leaf.rs": 'use react::{Element, jsx}; pub fn view() -> Element { jsx! { <i>{"two"}</i> } }',
    "alternate.rs": 'use react::{Element, jsx}; pub fn view() -> Element { jsx! { <p>{"three"}</p> } }',
  });
  run(args);
  snapshot(dir, "module-resolution");
  const result = await import(join(dir, "lib.js"));
  expect(renderToStaticMarkup(result.App())).toBe("<b>one</b><i>two</i><p>three</p>");
});

test("a disabled crate or module does not load or expand its contents", () => {
  for (const source of [
    '#![cfg(any())]\nmod missing; pub fn View() { jsx! { invalid } }',
    '#[cfg(any())] mod disabled { mod missing; pub fn view() { jsx! { invalid } } }',
    '#[cfg_attr(all(), path="disabled.rs")] mod disabled;',
  ]) {
    const { args } = compile(source, { "disabled.rs": '#![cfg(any())]\nmod missing;' });
    run(args);
  }
});

for (const [name, body, message] of [
  ["mismatched tag", '<button></div>', 'expected </button>'],
  ["missing closing tag", '<button>', 'missing closing tag'],
  ["invalid event", '<button onClick={123} />', 'expected a'],
  ["invalid prop type", '<button disabled={"wrong"} />', 'expected `bool`'],
  ["duplicate attribute", '<button disabled disabled />', 'duplicate attribute'],
  ["missing component prop", '<Card />', "missing prop `title`"],
  ["unknown component prop", '<Card nope="x" />', 'no field named'],
  ["invalid spread", '<div {...123} />', 'non-struct'],
  ["mixed component spread", '<Card title="x" {...Props { title: "y" }} />', "not JSX's `{...base}`"],
  ["adjacent roots", '<div /><span />', 'wrap adjacent JSX elements'],
  ["spread followed by attribute", '<div {...Props { title: "x" }} id="y" />', 'put the props spread last'],
  ["multiple spreads", '<div {...Props { title: "x" }} {...Props { title: "y" }} />', 'put the props spread last'],
  ["duplicate children", '<Card title="x" children={1}>{2}</Card>', 'children were provided twice'],
  ["intrinsic generic", '<div::<i32> />', 'generic arguments belong on a function component'],
  ["HTML entity", '<p>&amp;</p>', 'literal or a Rust expression'],
  ["empty attribute expression", '<div title={} />', 'Value'],
  ["bare text", '<p>Hello world</p>', 'literal or a Rust expression'],
] as const) {
  test(`JSX ${name} reports the original source and preserves existing output`, () => {
    const {dir, args} = compile(`#![allow(non_snake_case)]\nuse react::{Element, jsx};\npub struct Props { pub title: &'static str }\npub fn Card(p: Props) -> Element { jsx! { <div>{p.title}</div> } }\npub fn App() -> Element {\n    jsx! { ${body} }\n}`);
    const output = join(dir, "lib.jsx");
    writeFileSync(output, "previous output");
    const result = Bun.spawnSync(args, { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
    // Missing fields are diagnosed in the hygienic props constructor, with
    // the original invocation as a second label. Other errors point there directly.
    expect(result.stderr.toString()).toContain(name === "missing component prop" ? `6 |     jsx! { ${body} }` : "lib.rs:6:");
    expect(readFileSync(output, "utf8")).toBe("previous output");
    expect(existsSync(join(dir, "manifest.json"))).toBe(false);
  });
}

test("JSX covers generic functions, memo/lazy/forward-ref values and context providers", async () => {
  const { dir, args } = compile(`#![deny(warnings)]
#![allow(non_snake_case)]
use react::{Element, Node, jsx};
mod wrapped;
use wrapped::{MEMO as Cached, THEME as Theme};
pub struct Props<T> { pub value: T }
#[cfg_attr(all(), inline)]
pub fn Generic<T: Node>(p: Props<T>) -> Element { jsx! { <span>{p.value}</span> } }
pub struct Children<T> { pub children: T }
pub fn Group<T: Node>(p: Children<T>) -> Element { jsx! { <div>{p.children}</div> } }
pub fn Empty<T>() -> Element { jsx! { <i /> } }
pub fn App() -> Element {
    let Selected = Generic::<i32>;
    jsx! {
        <>
            <Generic value={7} />
            <Generic::<i32> value={8} />
            <Generic::<&'static str> value="borrowed" />
            <Generic::<Vec<i32>> value={vec![9, 10]} />
            <Group::<Vec<i32>>>{vec![1]}</Group>
            <Selected {...Props { value: 11 }} />
            <Empty::<i32> />
            <Cached label="memo" />
            <wrapped::Provider />
            <wrapped::LAZY />
            <wrapped::FORWARD label="forward" />
            <Theme value="dark"><Cached label="child" /></Theme>
            <Theme.Provider value="legacy"><Cached label="old" /></Theme.Provider>
        </>
    }
}
`, {
    "wrapped.rs": `use react::{Context, Element, ForwardRef, Lazy, Memo, Ref, create_context, forward_ref, import_module, jsx, lazy, memo};
pub struct Props { pub label: &'static str }
pub fn Card(p: Props) -> Element { jsx! { <b>{p.label}</b> } }
pub fn Provider() -> Element { jsx! { <i /> } }
pub fn Input(p: Props, _: Ref<Option<i32>>) -> Element { jsx! { <b>{p.label}</b> } }
thread_local! {
    pub static MEMO: Memo<Props> = memo(Card);
    pub static LAZY: Lazy<()> = lazy(|| import_module::<()>("./lazy.jsx"));
    pub static FORWARD: ForwardRef<Props, i32> = forward_ref(Input);
    pub static THEME: Context<&'static str> = create_context("light");
    #[cfg(any())] pub static DISABLED: Memo<Missing> = missing();
}
`,
  });
  run(args);
  snapshot(dir, "component-kinds");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<Generic value={7} />');
  expect(code).toContain('<Generic value={8} />');
  expect(code).toContain('<MEMO label="memo" />');
  expect(code).toContain('<THEME value="dark">');
  expect(code).toContain('<THEME.Provider value="legacy">');
  expect(code).toContain('<FORWARD label="forward" />');
  expect(code).toContain('<LAZY />');
  expect(code).not.toContain('component(');
  const source = readFileSync(join(dir, "lib.rs"), "utf8");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  const lines = code.split("\n");
  for (const [generated, original] of [["<MEMO label=\"memo\"", "<Cached label=\"memo\""], ["<THEME.Provider", "<Theme.Provider"], ["<Generic value={8}", "<Generic::<i32>"]]) {
    const line = lines.findIndex(l => l.includes(generated));
    expect(lookup(decodeMappings(map.mappings), line, lines[line].indexOf(generated))?.srcLine)
      .toBe(source.split("\n").findIndex(l => l.includes(original)));
  }
});

test("JSX built-ins finish as elements and use one spelling for ref and form actions", () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, Style, jsx, use_ref, webapi};
pub fn App() -> Element {
    let object = use_ref(None::<&'static webapi::Element>);
    jsx! {
        <Profiler id="test" onRender={|_, _, _, _, _, _| ()}>
            <ViewTransition name="page">
                <form action={|_: &'static webapi::FormData| ()}>
                    <input ref={object} />
                    <input ref={|_: Option<&'static webapi::Element>| ()} />
                    <button formAction="/save" style={Style::new().color("red")}>{"Save"}</button>
                </form>
            </ViewTransition>
            <Suspense />
            <Activity />
            <StrictMode />
        </Profiler>
    }
}
`);
  run(args);
  snapshot(dir, "builtins");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  for (const name of ["Profiler", "ViewTransition", "Suspense", "Activity", "StrictMode"]) expect(code).toContain(`<${name}`);
  expect(code).toContain('formAction="/save"');
  expect(code).not.toContain('actionFn');
  expect(code).not.toContain('refCallback');
});

for (const [name, body] of [
  ["constructor", "react::html::div()"],
  ["import alias", "{ use react::html::div as make; make() }"],
  ["function value", "{ let make = react::html::div; make() }"],
  ["method", "jsx! { <div /> }.children(\"no\")"],
  ["UFCS", "react::Element::children(jsx! { <div /> }, \"no\")"],
  ["component", "react::component(Card, ())"],
  ["fragment", "react::fragment(())"],
  ["inside JSX expression", "jsx! { <div>{react::html::span()}</div> }"],
  ["inside component prop", "jsx! { <Wrapper content={react::html::span()} /> }"],
  ["inside closure", "jsx! { <button onClick={|_| { let _ = react::html::span(); }} /> }"],
  ["ordinary macro", "{ macro_rules! old { () => { react::html::span() } } old!() }"],
] as const) {
  test(`direct element builder ${name} is rejected without replacing output`, () => {
    const { dir, args } = compile(`#![allow(non_snake_case, dead_code)]
use react::{Element, jsx};
pub fn Card() -> Element { jsx! { <div /> } }
pub struct Props { pub content: Element }
pub fn Wrapper(p: Props) -> Element { p.content }
fn unused() -> Element { ${body} }
`);
    const file = join(dir, "lib.jsx");
    writeFileSync(file, "previous output");
    const result = Bun.spawnSync(args);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("element builders are compiler-only");
    expect(readFileSync(file, "utf8")).toBe("previous output");
  });
}

test("JSX context providers and refs work on React 18 while newer APIs stay gated", () => {
  const { dir, args } = compile(`#![deny(warnings)]
#![allow(non_snake_case)]
use react::{Context, Element, create_context, jsx, webapi};
thread_local! { static THEME: Context<&'static str> = create_context("light"); }
pub fn App() -> Element {
    jsx! {
        <THEME.Provider value="dark">
            <Suspense key="body" fallback="loading">
                <form action="/save"><input ref={|_: Option<&'static webapi::Element>| ()} /></form>
            </Suspense>
        </THEME.Provider>
    }
}
`);
  const source = readFileSync(join(dir, "lib.rs"), "utf8");
  const metadata = join(dir, "libreact.rmeta");
  run(["react/build.sh", "-o", metadata, "--react", "18.2.0"]);
  const versionArgs = args.map(arg => arg === `react=${join(target, "libreact.rmeta")}` ? `react=${metadata}` : arg === target ? dir : arg);
  run(versionArgs);
  snapshot(dir, "react18");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<THEME.Provider value="dark">');
  expect(code).toContain('<Suspense key="body" fallback="loading">');
  for (const unsupported of [
    source.replaceAll("THEME.Provider", "THEME"),
    source.replace('action="/save"', "action={|_: &'static webapi::FormData| ()}"),
    source.replace('key="body" fallback="loading"', '').replaceAll('Suspense', 'Activity'),
  ]) {
    writeFileSync(join(dir, "lib.rs"), unsupported);
    expect(Bun.spawnSync(versionArgs, { cwd: dir }).exitCode).not.toBe(0);
    expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toBe(code);
  }
}, 30_000);

test("forwarded refs keep their handle type, evaluation order and handwritten JSX", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, ForwardRef, Ref, forward_ref, jsx, webapi};
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32; }
pub struct Props { pub label: i32 }
pub fn Input(p: Props, reference: Ref<Option<&'static webapi::Element>>) -> Element {
    jsx! { <input ref={reference} title={p.label} /> }
}
thread_local! { static INPUT: ForwardRef<Props, &'static webapi::Element> = forward_ref(Input); }
pub fn Plain(reference: Ref<Option<&'static webapi::Element>>) -> Element {
    jsx! { <INPUT ref={reference} label={1} /> }
}
pub fn App(reference: Ref<Option<&'static webapi::Element>>) -> Element {
    jsx! { <INPUT ref={record(1); reference} label={record(2)} /> }
}
pub struct NormalProps { pub r#ref: Ref<Option<&'static webapi::Element>>, pub title: i32 }
pub fn Normal(p: NormalProps) -> Element { jsx! { <input ref={p.r#ref} title={p.title} /> } }
pub fn Ordinary(reference: Ref<Option<&'static webapi::Element>>) -> Element {
    jsx! { <Normal title={record(3)} ref={record(4); reference} /> }
}
`);
  run(args);
  snapshot(dir, "refs");
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).not.toContain('checked_ref');
  expect(output).not.toContain('component(');
  expect(output).toContain('export function Plain(reference) {\n  return <INPUT label={1} ref={reference} />;\n}');
  const result = await import(join(dir, "lib.jsx"));
  const previous = globalThis.record;
  const calls: number[] = [];
  globalThis.record = (n: number) => { calls.push(n); return n; };
  try {
    const reference = { current: null };
    const tree = result.App(reference);
    expect(calls).toEqual([1, 2]);
    expect(tree.props.ref).toBe(reference);
    expect(tree.type.render(tree.props, tree.props.ref).props.ref).toBe(reference);
    result.Ordinary(reference);
    expect(calls).toEqual([1, 2, 3, 4]);
  } finally { globalThis.record = previous; }
  const file = join(dir, "lib.rs");
  const source = readFileSync(file, "utf8");
  writeFileSync(file, source.replace('pub fn App(reference: Ref<Option<&\'static webapi::Element>>)', 'pub fn App(reference: Ref<Option<i32>>)'));
  const invalid = Bun.spawnSync(args, { cwd: dir });
  expect(invalid.exitCode).not.toBe(0);
  expect(invalid.stderr.toString()).toContain('RefValue');
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toBe(output);
});

// Grammar cases live together so their complete output is easy to review.
// API-specific behavior (hooks, mounting, async actions) stays in react.test.ts.
test("JSX grammar: literals, empty forms, Rust children and attribute expressions", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, Style, inner_html, jsx};
pub fn Empty() -> Element { jsx! { <></> } }
pub fn Literals() -> Element {
    jsx! {
        <div>
            " leading "
            {"<&>"}
            {r#"raw "quote""#}
            {42}{1.5}{true}{false}{()}{None::<i32>}{Some(7)}
            {/* a comment contributes no child */}
            <span></span>
        </div>
    }
}
pub fn Expressions(show: bool) -> Element {
    let pair = (1, 2);
    let list = vec![3, 4];
    jsx! {
        <section>
            {pair}{list}
            {if show { Some(jsx! { <b>{"yes"}</b> }) } else { None }}
            {match show { true => "on", false => "off" }}
            {let n = 5; n + 1}
        </section>
    }
}
pub fn Attributes() -> Element {
    jsx! {
        <>
            <button disabled title={let n = 2; n.to_string()} aria-label="Save" data-state="ready" tabIndex=3 />
            <input disabled={false} defaultValue="a" />
            <div style={Style::new().color("red")} dangerouslySetInnerHTML={inner_html("<b>raw</b>")} />
        </>
    }
}
`);
  run(args);
  snapshot(dir, "grammar");
  const source = readFileSync(join(dir, "lib.rs"), "utf8").split("\n");
  const lines = readFileSync(join(dir, "lib.jsx"), "utf8").split("\n");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  for (const [generated, original] of [["const n = 5", "{let n = 5"], ["const n = 2", "title={let n = 2"]]) {
    const line = lines.findIndex(l => l.includes(generated));
    expect(line).toBeGreaterThanOrEqual(0);
    expect(lookup(decodeMappings(map.mappings), line, lines[line].indexOf("n ="))?.srcLine)
      .toBe(source.findIndex(l => l.includes(original)));
  }
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Empty())).toBe("");
  expect(renderToStaticMarkup(result.Literals())).toBe('<div> leading &lt;&amp;&gt;raw &quot;quote&quot;421.57<span></span></div>');
  expect(renderToStaticMarkup(result.Expressions(true))).toBe('<section>1234<b>yes</b>on6</section>');
  expect(renderToStaticMarkup(result.Expressions(false))).toBe('<section>1234off6</section>');
  expect(renderToStaticMarkup(result.Attributes())).toBe('<button disabled="" title="2" aria-label="Save" data-state="ready" tabindex="3"></button><input value="a"/><div style="color:red"><b>raw</b></div>');
});

// A component's named props with the rest from a base, `{..Default::default()}`,
// as Rust's struct update has them: what a binding of a JS component with
// many optional props, `next/image`'s, is given (ADR 0192). `{...base}` with
// named props stays an error, as JSX's spread would override them.
test("JSX gives a component its named props and the rest from a base", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, jsx};
#[derive(Default)]
pub struct Opts { pub title: Option<&'static str>, pub width: Option<u32> }
pub fn Card(o: Opts) -> Element { jsx! { <section title={o.title.unwrap_or("none")}>{o.width.unwrap_or(0)}</section> } }
// An Element's default is React's empty node, undefined, as next/link's
// children are given, or not.
#[derive(Default)]
pub struct Framed { pub title: Option<&'static str>, pub children: Element }
pub fn Frame(f: Framed) -> Element { jsx! { <div title={f.title.unwrap_or("bare")}>{f.children}</div> } }
pub fn App() -> Element {
    jsx! {
        <>
            <Card title={Some("named")} {..Default::default()} />
            <Card width={Some(3)} {..Opts { title: Some("base"), width: Some(9) }} />
            <Frame title={Some("t")} {..Default::default()}><b>{"kid"}</b></Frame>
            <Frame {..Default::default()} />
        </>
    }
}
`);
  run(args);
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.App())).toBe('<section title="named">0</section><section title="base">3</section><div title="t"><b>kid</b></div><div title="bare"></div>');
  // The JSX one writes: the base evaluated last, as Rust's is, with nothing
  // kept of it, and no children where none are given.
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes('<Frame title="t">'), jsx.includes("<Frame />"), jsx.includes("match")]).toEqual([true, true, false]);
});

// A component's props are given as one flat list, `<ButtonLink href="/a"
// target={..} id={..}>`, its own fields and its flattened structs' alike,
// as JSX's caller writes them: what isn't given is left out, a field with a
// default or an `Option`, and one that's required is said (ADR 0213).
test("JSX takes a component's flattened props where they're written, none given left out", async () => {
  const flat = `#![allow(non_snake_case)]
use react::{Element, Node, jsx};
#[derive(Default)]
pub struct Html {
    pub id: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'static str>,
}
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: Html,
}
#[derive(Default)]
pub enum Size {
    #[default]
    #[cfg_attr(rust_js, rust_js::name = "md")]
    Md,
    #[cfg_attr(rust_js, rust_js::name = "lg")]
    Lg,
}
pub struct ButtonLinkProps<C> {
    pub href: &'static str,
    #[cfg_attr(rust_js, rust_js::default)]
    pub size: Size,
    pub label: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: Anchor,
}
pub fn ButtonLink<C: Node + Default>(ButtonLinkProps { href, size, label, children, props }: ButtonLinkProps<C>) -> Element {
    let class = match size {
        Size::Md => "md",
        Size::Lg => "lg",
    };
    jsx! { <a href={href} data-size={class} aria-label={label} {...props}>{children}</a> }
}
`;
  const { dir, args } = compile(flat + `
pub fn App() -> Element {
    jsx! {
        <>
            <ButtonLink target={Some("_blank")} href="/a" id={Some("x")} size={Size::Lg}>{"A"}</ButtonLink>
            <ButtonLink href="/b" className={Some("c")}>{"B"}</ButtonLink>
        </>
    }
}
// What does something is made in Rust's order, the struct's, though the
// JSX writes it as the caller does.
fn note(text: &'static str) -> &'static str {
    println!("{text}");
    text
}
pub fn Noted() -> Element {
    jsx! { <ButtonLink target={Some(note("t"))} href={note("h")}>{"N"}</ButtonLink> }
}
// Its own fields written out of its order: Rust makes \`href\` first.
fn big() -> Size {
    println!("big");
    Size::Lg
}
pub fn Sized() -> Element {
    jsx! { <ButtonLink size={big()} href={note("h")}>{"S"}</ButtonLink> }
}
// A variable is read as it is, beside a prop that does something.
pub fn Kept(h: &'static str) -> Element {
    jsx! { <ButtonLink target={Some(note("t"))} href={h}>{"K"}</ButtonLink> }
}
// Its flattened struct first, no children: Rust makes \`target\` first.
pub struct LinkyProps {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: Anchor,
    pub label: &'static str,
}
pub fn Linky(LinkyProps { anchor, label }: LinkyProps) -> Element {
    jsx! { <a title={label} {...anchor} /> }
}
pub fn Linked() -> Element {
    jsx! { <Linky label={note("l")} target={Some(note("t"))} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<ButtonLink target="_blank" href="/a" id="x" size="lg">');
  expect(jsx).toContain('<ButtonLink href="/b" className="c">');
  const { App, Noted, Linked, Sized } = await import(join(dir, "lib.jsx"));
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    expect(renderToStaticMarkup(Noted())).toBe('<a href="h" data-size="md" target="t">N</a>');
  } finally {
    console.log = log;
  }
  console.log = (line: string) => logged.push(line);
  try {
    expect(renderToStaticMarkup(Linked())).toBe('<a title="l" target="t"></a>');
    expect(renderToStaticMarkup(Sized())).toBe('<a href="h" data-size="lg">S</a>');
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["h", "t", "l", "t", "h", "big"]);
  expect(jsx).toMatch(/<ButtonLink size=\{\w+\} href=\{\w+\}>/);
  expect(jsx).toContain("<Linky label={label} target={target} />");
  expect(jsx).toMatch(/<ButtonLink target=\{\w+\} href=\{h\}>/);
  expect(jsx).toMatch(/<ButtonLink target=\{\w+\} href=\{\w+\}>/);
  expect(renderToStaticMarkup(App())).toBe('<a href="/a" data-size="lg" target="_blank" id="x">A</a><a href="/b" data-size="md" class="c">B</a>');
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  // A required prop not given, and a name nothing has.
  refused(flat + `pub fn Missing() -> Element {\n    jsx! { <ButtonLink id={Some("x")}>{"A"}</ButtonLink> }\n}\n`, "missing prop \`href\`");
  refused(flat + `pub fn Unknown() -> Element {\n    jsx! { <ButtonLink href="/a" colour={Some("red")}>{"A"}</ButtonLink> }\n}\n`, "colour");
});

// A props field's default, `#[rust_js::default]`, its type's, or
// `#[rust_js::default = "_self"]`, is where JS takes it, its destructuring's:
// `{ size = "md" }`, as React's `type = "primary"` is (ADR 0212).
test("JSX gives a props field's default where the props are taken apart", async () => {
  const chip = `#![allow(non_snake_case)]
use react::{Element, jsx};
#[derive(Default)]
pub enum Size {
    #[default]
    #[cfg_attr(rust_js, rust_js::name = "md")]
    Md,
    #[cfg_attr(rust_js, rust_js::name = "lg")]
    Lg,
}
pub struct ChipProps {
    pub label: &'static str,
    #[cfg_attr(rust_js, rust_js::default)]
    pub size: Size,
    #[cfg_attr(rust_js, rust_js::default = "_self")]
    pub target: &'static str,
}
pub fn Chip(ChipProps { label, size, target }: ChipProps) -> Element {
    let class = match size {
        Size::Md => "md",
        Size::Lg => "lg",
    };
    jsx! { <a className={class} target={target}>{label}</a> }
}
`;
  const { dir, args } = compile(chip);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('export function Chip({ label, size = "md", target = "_self" }) {');
  const { Chip } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Chip, { label: "x" }))).toBe('<a class="md" target="_self">x</a>');
  expect(renderToStaticMarkup(createElement(Chip, { label: "y", size: "lg", target: "_blank" }))).toBe('<a class="lg" target="_blank">y</a>');
  // Props not taken apart where they're given would have no default.
  const whole = compile(chip + "pub fn Whole(props: ChipProps) -> Element {\n    jsx! { <b>{props.label}</b> }\n}\n");
  const failed = Bun.spawnSync(whole.args, { cwd: whole.dir });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("taken apart where they're given")]);
  // A default that's made, not written, is said: a literal is JS's.
  const made = compile(chip + "pub struct MapProps {\n    #[cfg_attr(rust_js, rust_js::default)]\n    pub seen: std::collections::HashMap<u32, u32>,\n}\npub fn Seen(MapProps { seen }: MapProps) -> Element {\n    jsx! { <b>{seen.len()}</b> }\n}\n");
  const unmade = Bun.spawnSync(made.args, { cwd: made.dir });
  expect([unmade.exitCode === 0, unmade.stderr.toString()]).toEqual([false, expect.stringContaining("isn't a literal")]);
});

// A props struct's flattened field, `#[rust_js::flatten]`, holds a struct
// whose fields are the component's own props, as TypeScript's
// `AnchorProps & ButtonLinkProps` has them: `...anchor` where they're
// taken apart, and in JSX each one given, an attribute (ADR 0204).
test("JSX gives a flattened struct's fields as a component's own props", async () => {
  const flattened = `#![allow(non_snake_case)]
use react::{Element, Node, jsx};
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::name = "aria-label")]
    pub aria_label: Option<&'static str>,
}
#[derive(Default)]
pub struct ButtonLinkProps<C> {
    pub size: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: Anchor,
}
pub fn ButtonLink<C: Node>(ButtonLinkProps { size, children, anchor }: ButtonLinkProps<C>) -> Element {
    let class = if size == Some("lg") { "big" } else { "small" };
    let target = anchor.target.unwrap_or("_self");
    jsx! { <a className={class} target={target} {...anchor}>{children}</a> }
}
`;
  const { dir, args } = compile(flattened + `
pub fn App() -> Element {
    jsx! {
        <ButtonLink
            size={Some("lg")}
            anchor={Anchor { href: Some("/learn"), aria_label: Some("Learn"), ..Default::default() }}
            {..Default::default()}
        >
            {"Learn"}
        </ButtonLink>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function ButtonLink({ size, children, ...anchor }) {");
  expect(jsx).toContain('<ButtonLink size="lg" href="/learn" aria-label="Learn">');
  const { App, ButtonLink } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(App())).toBe('<a class="big" target="_self" href="/learn" aria-label="Learn">Learn</a>');
  expect(renderToStaticMarkup(createElement(ButtonLink, { href: "/x", target: "_blank" }, "x"))).toBe('<a class="small" target="_blank" href="/x">x</a>');
  // What JS's props can't be is an error: the field read whole, the
  // struct made anywhere but as JSX's props, two rests, and a name both have.
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  refused(flattened + "pub fn read(p: ButtonLinkProps<()>) -> Option<&'static str> { let a = p.anchor; a.href }\n", "flattened props");
  refused(flattened + "pub fn made() -> Element { ButtonLink(ButtonLinkProps { size: None, children: (), anchor: Anchor::default() }) }\n", "made only as JSX");
  refused(flattened + "pub fn defaulted() -> ButtonLinkProps<()> { ButtonLinkProps::default() }\n", "made only as JSX");
  refused(flattened + "pub fn matched(p: ButtonLinkProps<()>) -> Option<&'static str> { match p { ButtonLinkProps { anchor, .. } => anchor.href } }\n", "flattened props taken apart here");
  refused(flattened.replace("pub children: C,", "pub children: C,\n    pub rest: react::Rest,").replace("{ size, children, anchor }", "{ size, children, anchor, .. }"), "one rest");
});

// Flattened structs chain, as TypeScript's interfaces extend one another,
// and a name its props have too is theirs, as TypeScript's `Omit` has it:
// what's taken apart besides holds none of it. A flattened field is read
// through, `props.html.title` is `props.title`, never whole (ADR 0205).
test("JSX gives a chain of flattened structs as props, a name the props have their own", async () => {
  const chained = `#![allow(non_snake_case)]
use react::{Element, Node, jsx};
#[derive(Default)]
pub struct Html {
    pub id: Option<&'static str>,
    pub title: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'static str>,
}
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: Html,
}
#[derive(Default)]
pub struct ButtonLinkProps<C> {
    pub href: &'static str,
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: Anchor,
}
pub fn ButtonLink<C: Node>(ButtonLinkProps { href, class_name, children, props }: ButtonLinkProps<C>) -> Element {
    let title = props.html.title.unwrap_or("none");
    jsx! { <a href={href} className={class_name.unwrap_or("x")} data-title={title} {...props}>{children}</a> }
}
// What it leaves, .., the rest does not hold.
pub fn Plain<C: Node>(ButtonLinkProps { href, props, .. }: ButtonLinkProps<C>) -> Element {
    jsx! { <a href={href} {...props} /> }
}
`;
  const { dir, args } = compile(chained + `
pub fn App() -> Element {
    jsx! {
        <ButtonLink
            href="/a"
            props={Anchor { target: Some("_blank"), html: Html { id: Some("i"), ..Default::default() }, ..Default::default() }}
            {..Default::default()}
        >
            {"A"}
        </ButtonLink>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function ButtonLink({ href, className, children, ...props }) {");
  expect(jsx).toContain('const title = props.title ?? "none";');
  expect(jsx).toContain('<ButtonLink href="/a" target="_blank" id="i">');
  expect(jsx).toContain("export function Plain({ href, className: _className, children: _children, ...props }) {");
  const { App, ButtonLink, Plain } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(App())).toBe('<a href="/a" class="x" data-title="none" target="_blank" id="i">A</a>');
  expect(renderToStaticMarkup(createElement(ButtonLink, { href: "/b", className: "c", title: "T", rel: "r" }, "b"))).toBe('<a href="/b" class="c" data-title="T" title="T" rel="r">b</a>');
  expect(renderToStaticMarkup(createElement(Plain, { href: "/p", className: "c", title: "t" }, "kid"))).toBe('<a href="/p" title="t"></a>');
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  // A Rust caller giving a name the props have, which would be lost.
  refused(chained + `pub fn Both() -> Element {
    jsx! { <ButtonLink href="/a" props={Anchor { href: Some("/b"), ..Default::default() }} {..Default::default()}>{"A"}</ButtonLink> }
}
`, "`href`, which the props have too");
  // The chain's field read whole.
  refused(chained.replace("let title = props.html.title.unwrap_or(\"none\");", "let html = &props.html;\n    let title = html.title.unwrap_or(\"none\");"), "reading flattened props");
});

// A component's props are as written, its children last, then what a
// base gives (ADR 0203). Rust makes the children before it reads the base,
// so children that change it, `bump(&mut base)`, are made first in JS too.
test("JSX gives a component its props as written, and children that change the base first", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, jsx};
#[derive(Clone, Copy)]
pub struct TallyProps { pub n: u32, pub label: &'static str, pub children: u32 }
pub fn Tally(TallyProps { n, label, children }: TallyProps) -> Element {
    jsx! { <i title={label}>{n}{"/"}{children}</i> }
}
fn bump(t: &mut TallyProps) -> u32 {
    t.n += 1;
    5
}
pub fn App() -> Element {
    let mut base = TallyProps { n: 1, label: "b", children: 0 };
    jsx! { <Tally label="first" {..base}>{bump(&mut base)}</Tally> }
}
`);
  run(args);
  const { App } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(App())).toBe('<i title="first">2/5</i>');
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx.indexOf("bump(")).toBeLessThan(jsx.indexOf("base.n"));
  expect(jsx).toContain('<Tally label="first"');
});

// A component generic over its children gives them on with the rest of a
// base, `{..Default::default()}`, which needs `C: Default`: react.dev's
// ButtonLink, of next/link. React gives a component no dictionary, so it
// takes none, and a default the update replaces, a `Node`'s, which does
// nothing, isn't made: no `base` is kept (ADR 0201).
test("JSX gives a generic component's children on with a base, and the component no dictionary", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, Node, jsx};
#[derive(Default)]
pub struct FrameProps<C> { pub title: Option<&'static str>, pub children: C, pub tags: Vec<&'static str>, pub id: Option<&'static str> }
pub fn Frame<C: Node>(FrameProps { title, children, tags, id }: FrameProps<C>) -> Element {
    jsx! { <section title={title.unwrap_or("bare")} id={id} data-tags={tags.join(" ")}>{children}</section> }
}
pub struct CardProps<C> { pub title: &'static str, pub children: C }
pub fn Card<C: Node + Default>(CardProps { title, children }: CardProps<C>) -> Element {
    jsx! { <Frame tags={vec!["card"]} title={Some(title)} {..Default::default()}>{children}</Frame> }
}
pub fn App() -> Element {
    jsx! { <Card title="t">{"kid"}<b>{"!"}</b></Card> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // Its props as written, as JSX's are, not in its struct's order (ADR 0203).
  expect(jsx).toContain("export function Card({ title, children }) {\n  return (\n    <Frame tags={[\"card\"]} title={title}>\n      {children}\n    </Frame>\n  );\n}");
  const { App, Card } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(App())).toBe('<section title="t" data-tags="card">kid<b>!</b></section>');
  expect(renderToStaticMarkup(createElement(Card, { title: "js" }, "child"))).toBe('<section title="js" data-tags="card">child</section>');
  // One that would use a dictionary, React's not giving it one, is an error.
  const uses = compile(`#![allow(non_snake_case)]
use react::{Element, Node, jsx};
pub struct P<C> { pub children: C }
pub fn Echo<C: Node + Default>(P { children }: P<C>) -> Element {
    let empty = C::default();
    jsx! { <p>{children}{empty}</p> }
}
`);
  const failed = Bun.spawnSync(uses.args, { cwd: uses.dir });
  expect([failed.exitCode === 0, failed.stderr.toString().includes("React gives a component no")]).toEqual([false, true]);
});

// Attributes are read before a child that needs a statement of its own, in
// Rust's order, each once (ADR 0194): one already a `const` isn't copied
// to another, `const className$1 = className`, as react.dev's IconCanary was.
test("JSX reads an attribute before a child once, not copied again", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, jsx};
pub enum Size { S, Md }
pub struct P { pub class_name: Option<&'static str>, pub size: Option<Size>, pub title: Option<&'static str> }
pub fn Badge(p: P) -> Element {
    jsx! {
        <svg
            className={p.class_name}
            width={if matches!(p.size, Some(Size::S)) { "12px" } else { "20px" }}
            height={if matches!(p.size, Some(Size::S)) { "12px" } else { "20px" }}
            viewBox="0 0 20 20"
        >
            {p.title.map(|title| jsx! { <title>{title}</title> })}
            <g fill="none"><path d="M0 0" /></g>
        </svg>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx.match(/const (\w+)\$\d+ = \1;/)).toBe(null);
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Badge({ class_name: "c", size: "S", title: "t" }))).toBe('<svg class="c" width="12px" height="12px" viewBox="0 0 20 20"><title>t</title><g fill="none"><path d="M0 0"></path></g></svg>');
});

// A component's props its struct doesn't name, a `react::Rest`: `...rest` of
// its destructured props, spread onto an element, as react.dev's
// ExternalLink takes its callers' `aria-label` (ADR 0195). A Rust caller's
// default gives none, and no `rest` prop.
test("JSX takes the props a component's struct doesn't name as ...rest", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, Node, Rest, jsx};
#[derive(Default)]
pub struct LinkProps<C> { pub href: &'static str, pub rest: Rest, pub target: Option<&'static str>, pub children: C }
pub fn ExternalLink<C: Node>(LinkProps { href, target, children, rest }: LinkProps<C>) -> Element {
    jsx! { <a href={href} target={target.unwrap_or("_blank")} rel="noopener" {...rest}>{children}</a> }
}
pub fn Home() -> Element {
    jsx! { <ExternalLink href="/home" {..Default::default()}>{"Home"}</ExternalLink> }
}
#[derive(Default)]
pub struct SocialProps { pub name: &'static str, pub rest: Rest }
pub fn Social(SocialProps { name, rest }: SocialProps) -> Element {
    jsx! { <ExternalLink href="/social" rest={rest} {..Default::default()}>{name}</ExternalLink> }
}
// Props holding what has a destructor are taken apart where they're given
// too, { label, ...rest }, what's bound owned by the function.
pub struct Loud(pub &'static str);
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub struct BadgeProps { pub label: Loud, pub rest: Rest }
pub fn Badge(BadgeProps { label, rest }: BadgeProps) -> Element {
    jsx! { <b {...rest}>{label.0}</b> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes("export function ExternalLink({ href, target, children, ...rest })"), jsx.includes("{...rest}"), jsx.includes("rest=")]).toEqual([true, true, false]);
  // One a Rust component passes on is spread too.
  expect(jsx).toContain('<ExternalLink href="/social" {...rest}>');
  expect(jsx).toContain("export function Badge({ label, ...rest }) {");
  const { ExternalLink, Home, Social, Badge } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(ExternalLink, { href: "/a", "aria-label": "A", className: "c" }, "a"))).toBe('<a href="/a" target="_blank" rel="noopener" aria-label="A" class="c">a</a>');
  expect(renderToStaticMarkup(Home())).toBe('<a href="/home" target="_blank" rel="noopener">Home</a>');
  expect(renderToStaticMarkup(createElement(Social, { name: "S", "aria-label": "B" }))).toBe('<a href="/social" target="_blank" rel="noopener" aria-label="B">S</a>');
  expect(renderToStaticMarkup(createElement(Badge, { label: ["L"], title: "T" }))).toBe('<b title="T">L</b>');
  // JS's props have no `rest`: one read as a field is an error, which says
  // to take it apart.
  const read = compile(`#![allow(non_snake_case)]
use react::{Element, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> Element {
    jsx! { <a href={p.href} {...p.rest} /> }
}
`);
  const failed = Bun.spawnSync(read.args, { cwd: read.dir });
  expect([failed.exitCode === 0, failed.stderr.toString().includes("take it apart from them")]).toEqual([false, true]);
  // Nor is one taken apart anywhere but where the props are given.
  const later = compile(`#![allow(non_snake_case)]
use react::{Element, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> Element {
    let P { href, rest } = p;
    jsx! { <a href={href} {...rest} /> }
}
`);
  const refused = Bun.spawnSync(later.args, { cwd: later.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("take them apart where they're given")]).toEqual([false, true]);
});

// A component's optional handler passed on to an element, as react.dev's
// Button does its onClick, is `onClick={onClick}` (ADR 0198): React
// ignores what a handler returns, and no handler does what one that calls
// nothing does.
test("JSX passes an optional event handler on as it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Element, Node, event, jsx};
pub struct ButtonProps<C> { pub children: C, pub on_click: Option<Box<dyn Fn(&event::Mouse)>> }
pub fn Button<C: Node>(ButtonProps { children, on_click }: ButtonProps<C>) -> Element {
    jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e) }}>{children}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes("<button onClick={onClick}>"), jsx.includes("=>")]).toEqual([true, false]);
  const { Button } = await import(join(dir, "lib.jsx"));
  const handler = () => {};
  expect([Button({ children: "b", on_click: handler }).props.onClick === handler, Button({ children: "b" }).props.onClick]).toEqual([true, undefined]);
});

// React calls a component with its props and a value of its own, never a
// drop: a component's type parameters take no destructor, in a library too,
// whose consumers' could (ADR 0199). One given a type with one is refused.
test("JSX components take no drop of their type parameters", async () => {
  const library = (source: string) => {
    const { dir, args } = compile(source);
    const at = args.indexOf("--");
    return { dir, args: [...args.slice(0, at), "--library", ...args.slice(at)] };
  };
  const card = `#![allow(non_snake_case)]
use react::{Element, Node, jsx};
pub struct CardProps<C> { pub title: &'static str, pub children: C }
pub fn Card<C: Node>(CardProps { title, children }: CardProps<C>) -> Element {
    let heading = format!("{title}!");
    jsx! { <section title={heading}>{children}</section> }
}
`;
  const built = library(card);
  run(built.args);
  const jsx = readFileSync(join(built.dir, "lib.jsx"), "utf8");
  expect([jsx.includes("export function Card({ title, children }) {"), jsx.includes("drop")]).toEqual([true, false]);
  const given = compile(`#![allow(non_snake_case)]
use react::{Element, jsx};
pub struct HolderProps<T> { pub value: T }
pub fn Holder<T>(HolderProps { value }: HolderProps<T>) -> Element {
    let _kept = value;
    jsx! { <i /> }
}
pub struct Loud;
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub fn App() -> Element {
    jsx! { <Holder value={Loud} /> }
}
`);
  const refused = Bun.spawnSync(given.args, { cwd: given.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("component `Holder`'s `T` a type with a destructor")]).toEqual([false, true]);
});

test("JSX grammar: spread precedence, children overrides, component paths and keyed fragments", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
#[rust_js::camel_case]
const _: () = ();
use react::{Element, jsx};
mod ui;
use ui::Card as Panel;
pub struct Attrs { pub title: &'static str, pub class_name: &'static str }
pub fn App() -> Element {
    let attrs = Attrs { title: "spread", class_name: "card" };
    let props = ui::Props { title: "panel", children: jsx! { <i>{"old"}</i> } };
    let Selected = ui::Empty;
    jsx! {
        <Fragment key="group">
            <div title="named" {...attrs} />
            <Panel {...props}><b>{"new"}</b></Panel>
            <ui.Card title="dot"><i /></ui.Card>
            <ui::Card title="path" children={jsx! { <u /> }} />
            <Selected {...()} />
        </Fragment>
    }
}
`, {
    "ui.rs": `use react::{Element, jsx};
pub struct Props { pub title: &'static str, pub children: Element }
pub fn Card(p: Props) -> Element { jsx! { <section title={p.title}>{p.children}</section> } }
pub fn Empty() -> Element { jsx! { <hr /> } }
`,
  });
  run(args);
  snapshot(dir, "spreads-and-paths");
  const result = await import(join(dir, "lib.jsx"));
  const tree = result.App();
  expect(tree.key).toBe("group");
  expect(renderToStaticMarkup(tree)).toBe('<div title="spread" class="card"></div><section title="panel"><b>new</b></section><section title="dot"><i></i></section><section title="path"><u></u></section><hr/>');
});
