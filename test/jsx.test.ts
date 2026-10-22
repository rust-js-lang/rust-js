// JSX's whole modules, kept as snapshots under test/snapshots/jsx (BLESS=1
// rewrites them), its grammar, components imported across modules, and what
// `jsx!` refuses; the rest of JSX is in the jsx-*.test.ts files by concept.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
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
  const normalize = (text: string) => text.replaceAll(dir + "/", "test/jsx/");
  expectSnapshot(dir, join(root, "test/snapshots/jsx", name), { normalize });
}

test("JSX supports components across modules, fragments, lists, conditions and spreads", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
use react::{JSX, jsx};
#[cfg(any())] mod missing;
#[cfg_attr(all(), path = "ui/card.rs")] mod card;
use card::Card as Panel;
pub struct Attrs { pub class_name: &'static str }
pub fn App() -> JSX::Element {
    let items: Vec<JSX::Element> = (0..3).map(|n| jsx! { <li key={n}>{n}</li> }).collect();
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
pub fn Spread() -> JSX::Element {
    let props = card::Props { title: "Spread", children: jsx! { <span>{"child"}</span> } };
    jsx! { <Panel {...props} /> }
}
pub fn Override() -> JSX::Element {
    let props = card::Props { title: "old", children: jsx! { <span /> } };
    jsx! { <Panel {...card::Props { title: "new", ..props }} /> }
}
`;
  const card = `use react::{JSX, jsx};
pub struct Props { pub title: &'static str, pub children: JSX::Element }
pub(crate) fn Card(p: Props) -> JSX::Element {
    jsx! { <section><h1>{p.title}</h1>{p.children}</section> }
}
`;
  const { dir, args } = compile(source, { "ui/card.rs": card });
  run(args);
  snapshot(dir, "modules");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('import { Card as Panel } from "./ui/card.jsx";');
  expect(code).toContain('<Panel title="Numbers">');
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
use react::{JSX, jsx};
use react::webapi::{AbortSignal, abort_controller};

pub struct BadgeProps {
    pub label: &'static str,
}

#[rust_js::link_name = "./badge.js#Badge"]
pub fn Badge(props: BadgeProps) -> JSX::Element {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "encodeURIComponent")]
#[allow(unused_variables)]
fn encode(text: &str) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
fn parse_int(text: &str) -> f64 {
    unreachable!()
}

pub fn App() -> JSX::Element {
    jsx! { <Badge label="new" /> }
}

pub fn values() -> (Vec<String>, Vec<f64>, Vec<bool>) {
    let encoded = vec!["a b", "c&d"].into_iter().map(encode).collect();
    // As \`.map\`'s own argument, \`parseInt\` would be given each index too.
    let numbers = vec!["10", "10", "10"].into_iter().map(parse_int).collect();
    let controller = abort_controller::new();
    controller.abort(None);
    let signals = vec![controller.signal(), abort_controller::new().signal()];
    let aborted = signals.into_iter().map(AbortSignal::aborted).collect();
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
use react::{JSX, jsx};
mod first;
mod second;
pub fn Card() -> JSX::Element { jsx! { <b>{"local"}</b> } }
pub fn View() -> JSX::Element {
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
    "first.rs": 'use react::{JSX, jsx}; pub fn Card() -> JSX::Element { jsx! { <b>{"first"}</b> } }',
    "second.rs": 'use react::{JSX, jsx}; pub fn Card() -> JSX::Element { jsx! { <b>{"second"}</b> } }',
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
use react::{JSX, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] #[allow(unused_variables)] fn record(n: i32) -> i32 { unreachable!() }
pub fn App() -> JSX::Element {
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

// A namespace import's component, \`Menu.Item\`, is its tag where it's
// written: JS never lets another module change an import, so what's made
// before it can't change what it reads.
test("a namespace import's component is a tag where it's written", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub mod Menu {
    use react::{JSX, ReactNode};
    pub struct PartProps<C> {
        pub children: C,
    }
    #[rust_js::link_name = "./menu.js#*.Root"]
    pub fn Root<C: ReactNode>(props: PartProps<C>) -> JSX::Element {
        unreachable!()
    }
    #[rust_js::link_name = "./menu.js#*.Item"]
    pub fn Item<C: ReactNode>(props: PartProps<C>) -> JSX::Element {
        unreachable!()
    }
}
fn base(p: &str) -> String {
    format!("/b{p}")
}
pub fn Nav() -> JSX::Element {
    jsx! {
        <Menu::Root>
            <Menu::Item>{base("/x")}</Menu::Item>
            <Menu::Item>{base("/y")}</Menu::Item>
        </Menu::Root>
    }
}
`;
  const { dir, args } = compile(source, {
    "menu.js": "export function Root({ children }) { return children; }\nexport function Item({ children }) { return children; }\n",
  });
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain("<Menu.Item>");
  expect(output).not.toContain("Component");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Nav())).toBe("/b/x/b/y");
});

// A default import is named as the module's `use` renames it, `use
// next::link::Link as NextLink` is `import NextLink from "next/link"`, as
// react.dev's MDX `Link` has it, beside its own `Link`; a module that
// doesn't rename it keeps its name, and a `static`'s is camel case, as an
// asset's own name is (ADR 0202).
test("a default import is named as the module's use renames it", () => {
  const source = `#![allow(non_snake_case, non_upper_case_globals)]
pub mod next {
    use react::{JSX, ReactNode};
    #[rust_js::link_name = "next/link#default"]
    pub fn Link<C: ReactNode>(props: LinkProps<'_, C>) -> JSX::Element { unreachable!() }
    pub struct LinkProps<'a, C> {
        pub href: &'a str,
        pub children: C,
    }
}
pub mod crumbs {
    use crate::next::Link;
    use react::{JSX, jsx};
    pub fn Crumb() -> JSX::Element {
        jsx! { <Link href="/a">{"a"}</Link> }
    }
}
pub mod mdx {
    use crate::next::Link as NextLink;
    use react::{JSX, jsx};
    pub fn Link() -> JSX::Element {
        jsx! { <NextLink href="/b">{"b"}</NextLink> }
    }
}
pub mod assets {
    unsafe extern "Rust" {
        #[link_name = "./hero.png#default"]
        pub safe static hero_img: &'static str;
    }
}
pub mod banner {
    use crate::assets::hero_img as banner_img;
    pub fn banner() -> &'static str {
        banner_img
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const crumbs = readFileSync(join(dir, "crumbs.jsx"), "utf8");
  const mdx = readFileSync(join(dir, "mdx.jsx"), "utf8");
  expect(crumbs).toContain('import Link from "next/link";');
  expect(mdx).toContain('import NextLink from "next/link";');
  expect(mdx).toContain('<NextLink href="/b">');
  expect(readFileSync(join(dir, "banner.js"), "utf8")).toContain('import bannerImg from "./hero.png";');
});

test("nested component JSX stays readable, contextually typed and mapped to the original Rust", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
use react::{JSX, jsx};
use std::rc::Rc;
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] #[allow(unused_variables)] fn record(n: i32) { unreachable!() }
pub struct Props { pub title: &'static str, pub content: JSX::Element, pub on_submit: Option<Rc<dyn Fn()>> }
pub fn Card(p: Props) -> JSX::Element { jsx! { <section title={p.title}>{p.content}</section> } }
pub fn App(active: bool) -> JSX::Element {
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
pub fn Tokens() -> JSX::Element {
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
        onSubmit={() => {
          globalThis.record(8);
        }}
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
  const {dir, args} = compile('use react::{JSX, jsx}; pub fn View() -> JSX::Element { jsx! { <div>' + Array.from({length: 40}, (_, i) => `<span>{${i}}</span>`).join('') + '</div> } }');
  run(args);
  snapshot(dir, "many-children");
  const result = await import(join(dir, "lib.jsx"));
  expect(result.View().props.children).toHaveLength(40);
});

test("JSX uses SVG's tag and attribute spelling", async () => {
  const { dir, args } = compile(`use react::{JSX, jsx};
pub fn View() -> JSX::Element {
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
use react::{JSX, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] #[allow(unused_variables)] fn record(n: i32) -> i32 { unreachable!() }
pub struct Props { pub title: i32, pub children: i32 }
pub(crate) fn Card(p: Props) -> JSX::Element { jsx! { <div>{p.title}{p.children}</div> } }
pub fn App() -> JSX::Element {
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
  const { dir, args } = compile(`use react::JSX;
mod outer;
pub fn App() -> JSX::Element { outer::view() }
`, {
    "outer.rs": `use react::{JSX, jsx};
mod inner;
mod inline { pub mod leaf; }
#[cfg_attr(all(), path = "alternate.rs")] mod alternate;
pub fn view() -> JSX::Element { jsx! { <>{inner::view()}{inline::leaf::view()}{alternate::view()}</> } }
`,
    "outer/inner.rs": 'use react::{JSX, jsx}; pub fn view() -> JSX::Element { jsx! { <b>{"one"}</b> } }',
    "outer/inline/leaf.rs": 'use react::{JSX, jsx}; pub fn view() -> JSX::Element { jsx! { <i>{"two"}</i> } }',
    "alternate.rs": 'use react::{JSX, jsx}; pub fn view() -> JSX::Element { jsx! { <p>{"three"}</p> } }',
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
  // A DOM element's attributes may follow its spread, as JSX's do, in order.
  ["spread followed by attribute", '<Card {...Props { title: "x" }} title="y" />', 'put the props spread last'],
  ["multiple spreads", '<div {...Props { title: "x" }} {...Props { title: "y" }} />', 'only one props spread is supported'],
  ["duplicate children", '<Card title="x" children={1}>{2}</Card>', 'children were provided twice'],
  ["intrinsic generic", '<div::<i32> />', 'generic arguments belong on a function component'],
  ["HTML entity", '<p>&amp;</p>', 'literal or a Rust expression'],
  ["empty attribute expression", '<div title={} />', 'is not text'],
  ["bare text", '<p>Hello world</p>', 'literal or a Rust expression'],
  // An attribute is JSX's name, one spelling (ADR 0435).
  ["attribute in Rust's case", '<button on_click={move |_| ()} />', "`on_click` is `onClick` in JSX"],
  ["className in Rust's case", '<div class_name="x" />', "`class_name` is `className` in JSX"],
  ["aria attribute in Rust's case", '<div aria_label="x" />', "`aria_label` is `aria-label` in JSX"],
] as const) {
  test(`JSX ${name} reports the original source and preserves existing output`, () => {
    const {dir, args} = compile(`#![allow(non_snake_case)]\nuse react::{JSX, jsx};\npub struct Props { pub title: &'static str }\npub fn Card(p: Props) -> JSX::Element { jsx! { <div>{p.title}</div> } }\npub fn App() -> JSX::Element {\n    jsx! { ${body} }\n}`);
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

// ADR 0435: a component's prop is written in JSX as JSX names it, its
// field camelCase, and only so.
test("JSX names a component's prop as JSX does, and only so", () => {
  const source = (attr: string) => `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct BadgeProps { pub label_text: &'static str }
pub fn Badge(BadgeProps { label_text }: BadgeProps) -> JSX::Element { jsx! { <span>{label_text}</span> } }
pub fn App() -> JSX::Element {
    jsx! { <Badge ${attr}="new" /> }
}`;
  const good = compile(source("labelText"));
  run(good.args);
  const jsx = readFileSync(join(good.dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function Badge({ labelText }) {");
  expect(jsx).toContain('<Badge labelText="new" />');
  const bad = compile(source("label_text"));
  const result = Bun.spawnSync(bad.args, { cwd: bad.dir });
  expect([result.exitCode === 0, result.stderr.toString().includes("`label_text` is `labelText` in JSX")], result.stderr.toString()).toEqual([false, true]);
});

test("JSX covers generic functions, memo/lazy/forward-ref values and context providers", async () => {
  const { dir, args } = compile(`#![deny(warnings)]
#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
mod wrapped;
use wrapped::{MEMO as Cached, THEME as Theme};
pub struct Props<T> { pub value: T }
#[cfg_attr(all(), inline)]
pub fn Generic<T: ReactNode>(p: Props<T>) -> JSX::Element { jsx! { <span>{p.value}</span> } }
pub struct Children<T> { pub children: T }
pub fn Group<T: ReactNode>(p: Children<T>) -> JSX::Element { jsx! { <div>{p.children}</div> } }
pub fn Empty<T>() -> JSX::Element { jsx! { <i /> } }
pub fn App() -> JSX::Element {
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
    "wrapped.rs": `use react::{Context, JSX, ForwardRefExoticComponent, ForwardedRef, LazyExoticComponent, MemoExoticComponent, create_context, forward_ref, import_module, jsx, lazy, memo};
pub struct Props { pub label: &'static str }
pub fn Card(p: Props) -> JSX::Element { jsx! { <b>{p.label}</b> } }
pub fn Provider() -> JSX::Element { jsx! { <i /> } }
pub fn Input(p: Props, _: ForwardedRef<i32>) -> JSX::Element { jsx! { <b>{p.label}</b> } }
thread_local! {
    pub static MEMO: MemoExoticComponent<Props> = memo(Card, None);
    pub static LAZY: LazyExoticComponent<()> = lazy(|| import_module::<()>("./lazy.jsx"));
    pub static FORWARD: ForwardRefExoticComponent<Props, i32> = forward_ref(Input);
    pub static THEME: Context<&'static str> = create_context("light");
    #[cfg(any())] pub static DISABLED: MemoExoticComponent<Missing> = missing();
}
`,
  });
  run(args);
  snapshot(dir, "component-kinds");
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<Generic value={7} />');
  expect(code).toContain('<Generic value={8} />');
  expect(code).toContain('<Cached label="memo" />');
  expect(code).toContain('<Theme value="dark">');
  expect(code).toContain('<Theme.Provider value="legacy">');
  expect(code).toContain('<Forward label="forward" />');
  expect(code).toContain('<Lazy />');
  expect(code).not.toContain('component(');
  const source = readFileSync(join(dir, "lib.rs"), "utf8");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  const lines = code.split("\n");
  for (const [generated, original] of [["<Cached label=\"memo\"", "<Cached label=\"memo\""], ["<Theme.Provider", "<Theme.Provider"], ["<Generic value={8}", "<Generic::<i32>"]]) {
    const line = lines.findIndex(l => l.includes(generated));
    expect(lookup(decodeMappings(map.mappings), line, lines[line].indexOf(generated))?.srcLine)
      .toBe(source.split("\n").findIndex(l => l.includes(original)));
  }
});

test("JSX built-ins finish as elements and use one spelling for ref and form actions", () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx, use_ref, webapi};
pub fn App() -> JSX::Element {
    let object = use_ref(None::<&'static webapi::Element>);
    jsx! {
        <Profiler id="test" onRender={|_, _, _, _, _, _| ()}>
            <ViewTransition name="page">
                <form action={|_: &'static webapi::FormData| ()}>
                    <input ref={object} />
                    <input ref={|_: Option<&'static webapi::Element>| ()} />
                    <button formAction="/save" style={CSSProperties::new().color("red")}>{"Save"}</button>
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
  // A tag's builder is of its element (ADR 0224): `react::element` makes
  // it a `JSX::Element`, so what's refused is the builder, not its type.
  ["constructor", "react::element(react::html::div())"],
  ["import alias", "{ use react::html::div as make; react::element(make()) }"],
  ["function value", "{ let make = react::html::div; react::element(make()) }"],
  ["method", "jsx! { <div /> }.children(\"no\")"],
  ["UFCS", "react::JSX::Element::children(jsx! { <div /> }, \"no\")"],
  ["component", "react::component(Card, ())"],
  ["fragment", "react::fragment(())"],
  ["inside JSX expression", "jsx! { <div>{react::element(react::html::span())}</div> }"],
  ["inside component prop", "jsx! { <Wrapper content={react::element(react::html::span())} /> }"],
  ["inside closure", "jsx! { <button onClick={|_| { let _ = react::html::span(); }} /> }"],
  ["ordinary macro", "{ macro_rules! old { () => { react::element(react::html::span()) } } old!() }"],
] as const) {
  test(`direct element builder ${name} is rejected without replacing output`, () => {
    const { dir, args } = compile(`#![allow(non_snake_case, dead_code)]
use react::{JSX, jsx};
pub fn Card() -> JSX::Element { jsx! { <div /> } }
pub struct Props { pub content: JSX::Element }
pub fn Wrapper(p: Props) -> JSX::Element { p.content }
fn unused() -> JSX::Element { ${body} }
`);
    const file = join(dir, "lib.jsx");
    writeFileSync(file, "previous output");
    const result = Bun.spawnSync(args);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("element builders are compiler-only");
    expect(readFileSync(file, "utf8")).toBe("previous output");
  });
}

// A `jsx!` is found by the name it's imported as, one rule wherever it's
// written: a path to it, `react::jsx!`, is refused alike as an expression
// and among a macro's tokens, where it was expanded (the second
// architecture review), not left to a plain rustc's `__jsx`.
for (const [name, body] of [
  ["as an expression", "react::jsx! { <p>{\"b\"}</p> }"],
  ["in a macro's tokens", "{ let items: Vec<JSX::Element> = vec![react::jsx! { <li /> }]; jsx! { <ul>{items}</ul> } }"],
] as const) {
  test(`a jsx! by a path to it is refused ${name}`, () => {
    const { args } = compile(`#![allow(non_snake_case, dead_code)]
use react::{JSX, jsx};
pub fn App() -> JSX::Element { ${body} }
`);
    const result = Bun.spawnSync(args);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("`jsx!` is called by the name it's imported as");
  });
}

test("JSX context providers and refs work on React 18 while newer APIs stay gated", () => {
  const { dir, args } = compile(`#![deny(warnings)]
#![allow(non_snake_case)]
use react::{Context, JSX, create_context, jsx, webapi};
thread_local! { static THEME: Context<&'static str> = create_context("light"); }
pub fn App() -> JSX::Element {
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
  expect(code).toContain('<Theme.Provider value="dark">');
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
use react::{JSX, ForwardRefExoticComponent, ForwardedRef, RefObject, forward_ref, jsx, webapi};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] #[allow(unused_variables)] fn record(n: i32) -> i32 { unreachable!() }
pub struct Props { pub label: i32 }
pub fn Input(p: Props, reference: ForwardedRef<&'static webapi::Element>) -> JSX::Element {
    jsx! { <input ref={reference} tabIndex={p.label} /> }
}
thread_local! { static FORWARDED_INPUT: ForwardRefExoticComponent<Props, &'static webapi::Element> = forward_ref(Input); }
pub fn Plain(reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
    jsx! { <FORWARDED_INPUT ref={reference} label={1} /> }
}
pub fn App(reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
    jsx! { <FORWARDED_INPUT ref={record(1); reference} label={record(2)} /> }
}
pub struct NormalProps { pub r#ref: RefObject<Option<&'static webapi::Element>>, pub title: i32 }
pub fn Normal(p: NormalProps) -> JSX::Element { jsx! { <input ref={p.r#ref} tabIndex={p.title} /> } }
pub fn Ordinary(reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
    jsx! { <Normal title={record(3)} ref={record(4); reference} /> }
}
`);
  run(args);
  snapshot(dir, "refs");
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).not.toContain('checked_ref');
  expect(output).not.toContain('component(');
  expect(output).toContain('export function Plain(reference) {\n  return <ForwardedInput label={1} ref={reference} />;\n}');
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
  writeFileSync(file, source.replace('pub fn App(reference: RefObject<Option<&\'static webapi::Element>>)', 'pub fn App(reference: RefObject<Option<i32>>)'));
  const invalid = Bun.spawnSync(args, { cwd: dir });
  expect(invalid.exitCode).not.toBe(0);
  expect(invalid.stderr.toString()).toContain('is not a ref of');
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toBe(output);
});

// Grammar cases live together so their complete output is easy to review.
// API-specific behavior (hooks, mounting, async actions) stays in react.test.ts.
test("JSX grammar: literals, empty forms, Rust children and attribute expressions", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, inner_html, jsx};
pub fn Empty() -> JSX::Element { jsx! { <></> } }
pub fn Literals() -> JSX::Element {
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
pub fn Expressions(show: bool) -> JSX::Element {
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
pub fn Attributes() -> JSX::Element {
    jsx! {
        <>
            <button disabled title={let n = 2; n.to_string()} aria-label="Save" data-state="ready" tabIndex=3 />
            <input disabled={false} defaultValue="a" />
            <div style={CSSProperties::new().color("red")} dangerouslySetInnerHTML={inner_html("<b>raw</b>")} />
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

test("JSX grammar: spread precedence, children overrides, component paths and keyed fragments", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
mod ui;
use ui::Card as Panel;
pub struct Attrs { pub title: &'static str, pub class_name: &'static str }
pub fn App() -> JSX::Element {
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
    "ui.rs": `use react::{JSX, jsx};
pub struct Props { pub title: &'static str, pub children: JSX::Element }
pub fn Card(p: Props) -> JSX::Element { jsx! { <section title={p.title}>{p.children}</section> } }
pub fn Empty() -> JSX::Element { jsx! { <hr /> } }
`,
  });
  run(args);
  snapshot(dir, "spreads-and-paths");
  const result = await import(join(dir, "lib.jsx"));
  const tree = result.App();
  expect(tree.key).toBe("group");
  expect(renderToStaticMarkup(tree)).toBe('<div title="spread" class="card"></div><section title="panel"><b>new</b></section><section title="dot"><i></i></section><section title="path"><u></u></section><hr/>');
});

// A module whose component isn't its default, lazy-loaded as one, as
// react.dev's Search loads DocSearch's modal: `import(..).then((mod) =>
// ({ default: mod.DocSearchModal }))`, by `Promise::then_resolve` and
// `Module::of`.
test("a module's named component is loaded as a module's default", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use core::marker::PhantomData;
use js::{JsObject, Promise};
use react::{JSX, Module};

pub struct LabelProps<'a> {
    pub text: &'a str,
}

pub struct Labels(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::link_name = "import")]
#[allow(unused_variables)]
fn import_labels(specifier: &str) -> Promise<&'static Labels> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get Label")]
#[allow(unused_variables)]
fn label_of(this: &Labels) -> fn(LabelProps) -> JSX::Element {
    unreachable!()
}

pub fn load() -> Promise<Module<LabelProps<'static>>> {
    import_labels("./labels.js").then_resolve(|r#mod| Module::of(label_of(r#mod)))
}
`, { "labels.js": 'export function Label({ text }) { return text; }\n' });
  run([...args.slice(0, -2), "--extern", `js=${join(target, "libjs.rmeta")}`, ...args.slice(-2)]);
  const jsx = readFileSync(join(dir, "lib.js"), "utf8");
  expect(jsx).toContain('return import("./labels.js").then((mod) => ({ default: mod.Label }));');
  const lib = await import(join(dir, "lib.js"));
  const labels = await import(join(dir, "labels.js"));
  expect((await lib.load()).default).toBe(labels.Label);
});

// ADR 0439: a component is named as JSX names it, capitalized, and rust-js
// allows rustc's lint of it, so no crate writes `#![allow(non_snake_case)]`;
// a function that isn't one, `NotComponent`, is still flagged.
test("a component's capitalized name needs no allow", () => {
  const { dir, args } = compile(`use react::{JSX, jsx};
pub struct GreetingProps<'a> { pub name: &'a str }
pub fn Greeting(GreetingProps { name }: GreetingProps) -> JSX::Element {
    jsx! { <p>{"Hi "}{name}</p> }
}
pub fn App() -> JSX::Element {
    jsx! { <Greeting name="Ada" /> }
}
mod inner {
    use react::{JSX, jsx};
    pub fn Badge() -> JSX::Element { jsx! { <b>{"!"}</b> } }
}
pub fn NotComponent() -> u32 { 1 }
// A component that's a thread-local's value, a memo of the function it's
// made of, as \`memo(function Icon() {..})\`, or a context, a tag too.
thread_local! {
    pub static Icon: react::MemoExoticComponent<()> = react::memo({
        fn Icon() -> JSX::Element {
            jsx! { <i /> }
        }
        Icon
    }, None);
    pub static ThemeContext: react::Context<&'static str> = react::create_context("light");
    pub static notConstant: u32 = 1;
}
pub fn Themed() -> JSX::Element {
    jsx! { <ThemeContext value="dark"><Icon /></ThemeContext> }
}
pub fn read() -> u32 {
    notConstant.with(|n| *n)
}
// Its props private, as react.dev's DiagramGroup's interface isn't
// exported (ADR 0385): JSX builds them where the component is.
struct PanelProps { title: &'static str }
pub fn Panel(PanelProps { title }: PanelProps) -> JSX::Element {
    jsx! { <h2>{title}</h2> }
}
struct Hidden;
pub fn exposed(_: Hidden) -> u32 { 1 }
`);
  const built = Bun.spawnSync(args, { cwd: dir });
  const stderr = built.stderr.toString();
  const warned = [...stderr.matchAll(/function `(\w+)` should have a snake case name/g)].map((m) => m[1]);
  const constants = [...stderr.matchAll(/constant `(\w+)` should have an upper case name/g)].map((m) => m[1]);
  const private_ = [...stderr.matchAll(/type `(\w+)` is more private than the item `(\w+)`/g)].map((m) => m[2]);
  expect([built.exitCode, warned, constants, private_]).toEqual([0, ["NotComponent"], ["notConstant"], ["exposed"]]);
});

// ADR 0440: a binding's body never runs, so its parameters are unread,
// which rust-js allows; two modules' bindings of one JS name, each of its
// own Rust type, are no clash rustc flags; and a struct given a binding,
// here as a `&dyn Any`, has its fields read by JS, a struct in it too, and
// one given in what's made of it, `Some(&..)` (ADR 0443).
// Another function's unread parameter, and a struct JS isn't given, are
// still flagged.
test("a binding needs no allow", () => {
  const { dir, args } = compile(`mod a {
    #[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
    pub fn max(a: f64, b: f64) -> f64 { unreachable!() }
    unsafe extern "Rust" {
        #[link_name = "globalThis.scale"]
        pub safe static scale: &'static f64;
    }
}
mod b {
    unsafe extern "Rust" {
        #[link_name = "globalThis.scale"]
        pub safe static scale: &'static u32;
    }
}
pub fn unread(x: u32) -> u32 { 1 }
pub fn run() -> f64 { a::max(1.0, 2.0) + *a::scale + f64::from(*b::scale) }
struct Options { readonly: bool, inner: Inner }
struct Inner { depth: u32 }
struct Kept { unread: u32 }
#[cfg_attr(rust_js, rust_js::link_name = "configure")]
fn configure(options: &dyn std::any::Any) -> u32 { unreachable!() }
struct Maybe { given: u32 }
#[cfg_attr(rust_js, rust_js::link_name = "configure")]
fn configure_maybe(options: Option<&dyn std::any::Any>) -> u32 { unreachable!() }
fn keep(_: &Kept) -> u32 { 0 }
const fn sized<const N: u32>() -> u32 { N }
pub fn configured() -> u32 {
    // Given by its name, a struct in its field is read by JS too.
    let options = Options { readonly: true, inner: Inner { depth: 2 } };
    // A body in a body, a block given as a const argument, its own owner.
    keep(&Kept { unread: 1 }) + configure(&options) + configure_maybe(Some(&Maybe { given: 3 })) + sized::<{ sized::<1>() }>()
}
`);
  const built = Bun.spawnSync(args, { cwd: dir });
  const warned = built.stderr.toString().split("\n").filter((line) => line.startsWith("warning:") && !/warnings? emitted/.test(line));
  expect([built.exitCode, warned]).toEqual([0, ["warning: unused variable: `x`", "warning: field `unread` is never read"]]);
});
