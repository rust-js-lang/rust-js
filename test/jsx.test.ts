import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup, renderToString } from "react-dom/server";
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

// Children whose JS needs no statements of its own read nothing ahead of a
// prop written before them: JSX reads its props, then its children, as Rust
// does. A `Link` whose props flatten an anchor's is such a child, as
// react.dev's `Breadcrumbs` has it.
// A list of children a chain collects: an array's `map`, each item given
// its index.
test("a list of children a chain collects is an array's map", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn List(items: Vec<&'static str>) -> JSX::Element {
    jsx! {
        <ul>
            {items.iter().enumerate().map(|(i, item)| jsx! { <li key={i}>{*item}</li> }).collect::<Vec<_>>()}
        </ul>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain("{items.map((item, i) => (\n");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.List(["a", "b"]))).toBe("<ul><li>a</li><li>b</li></ul>");
});

// An \`if\` given as a child, of an element or a value, is a conditional
// child, \`{link ? <a /> : author}\`, its test as it's written.
test("an if given as a child is a conditional child", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub fn Credit(author: &'static str, author_link: &'static str) -> JSX::Element {
    jsx! {
        <cite>
            {"Illustrated by "}
            {if !author_link.is_empty() {
                Box::new(jsx! { <a href={author_link}>{author}</a> }) as Box<dyn ReactNode>
            } else {
                Box::new(author)
            }}
        </cite>
    }
}
// Of elements more than two deep, and three ways, as react.dev's Link
// chooses its anchor: one conditional still, not a variable each sets.
pub fn Pick(href: &'static str) -> JSX::Element {
    jsx! {
        <>
            {if href.starts_with("https://") {
                jsx! { <p><b><i>{"out"}</i></b></p> }
            } else if href.starts_with("#") {
                jsx! { <p><b><u>{"here"}</u></b></p> }
            } else {
                jsx! { <p><b><s>{"in"}</s></b></p> }
            }}
        </>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain("{authorLink ? <a href={authorLink}>{author}</a> : author}");
  expect(output).not.toMatch(/\blet \w+;/);
  expect(output).toContain('{href.startsWith("https://") ? (');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Credit("R", "x"))).toBe('<cite>Illustrated by <a href="x">R</a></cite>');
  expect(renderToStaticMarkup(result.Credit("R", ""))).toBe("<cite>Illustrated by R</cite>");
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

// What an element reads, made before a branch in its tree, \`a || b\`'s, is
// read where it's written: its call runs first there as it ran first
// before.
test("what an element reads, made before a branch in its tree, is read where it's written", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
fn cn(a: &str, b: bool) -> String {
    format!("{a}{}", if b { " on" } else { "" })
}
pub fn Page(home: bool, blog: bool, content: &str) -> JSX::Element {
    jsx! {
        <div className={cn("page", home)}>
            {(home || blog).then(|| jsx! { <b>{"h"}</b> })}
            <p className={cn("text", blog)}>{content}</p>
        </div>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('<div className={cn("page", home)}>');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Page(true, false, "c"))).toBe('<div class="page on"><b>h</b><p class="text">c</p></div>');
});

// What an element reads that MIR makes ahead of it, \`&base(..)\`'s borrowed
// value, read where it's written past what does nothing before it there, a
// namespace's tag too: the tree is the order the program makes them in.
test("what an element reads past elements before it is read where it's written", async () => {
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
}
fn base(p: &str) -> String {
    format!("/b{p}")
}
pub struct LinkProps<'a> {
    pub href: &'a str,
}
pub fn Link(LinkProps { href }: LinkProps) -> JSX::Element {
    jsx! { <a href={href}>{"l"}</a> }
}
pub fn Nav() -> JSX::Element {
    jsx! {
        <Menu::Root>
            <Link href={&base("/x")} />
            <Link href={&base("/y")} />
        </Menu::Root>
    }
}
`;
  const { dir, args } = compile(source, {
    "menu.js": "export function Root({ children }) { return children; }\n",
  });
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('      <Link href={base("/x")} />\n');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Nav())).toBe('<a href="/b/x">l</a><a href="/b/y">l</a>');
});

// A let-else of what \`filter\` keeps, its \`else\` an element: a guard first,
// \`if (!href)\`, where the \`return\` is reached through what the component
// drops.
test("a let-else of what filter keeps, returning an element, is a guard first", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct LinkProps<'a> {
    pub href: Option<&'a str>,
    pub class_name: Option<&'a str>,
    pub items: Vec<&'a str>,
}
pub fn Link(LinkProps { href, class_name, items }: LinkProps) -> JSX::Element {
    let classes = "inline";
    let modified: Vec<String> = items.iter().map(|i| i.to_uppercase()).collect();
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return jsx! { <a href={href} className={class_name} /> };
    };
    jsx! {
        <>
            {if href.starts_with("https://") {
                jsx! { <b className={classes}>{href}{modified}</b> }
            } else {
                jsx! { <i>{href}</i> }
            }}
        </>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain("  if (!href) {\n    return <a href={href} className={className} />;\n  }\n");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Link({ href: "", items: [] }))).toBe('<a href=""></a>');
  expect(renderToStaticMarkup(result.Link({ href: "https://x", items: ["a"] }))).toBe('<b class="inline">https://xA</b>');
});

// A let-else of what \`filter\` keeps, then an early \`return\`: guards first, each
// \`return\` where it's written, whose value the branches make and MIR's
// one \`return\` gave back.
test("a let-else of what filter keeps, then an early return, are guards first", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
fn cn(a: &str, b: bool) -> String {
    format!("{a}{}", if b { " h" } else { "" })
}
pub struct FooterLinkProps<'a, C: ReactNode> {
    pub href: Option<&'a str>,
    pub children: C,
    pub is_header: bool,
}
pub fn FooterLink<C: ReactNode>(FooterLinkProps { href, children, is_header }: FooterLinkProps<C>) -> JSX::Element {
    let classes = cn("link", is_header);
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return jsx! { <div className={classes}>{children}</div> };
    };
    if href.starts_with("https://") {
        return jsx! {
            <div>
                <a href={href} className={classes.as_str()}>{children}</a>
            </div>
        };
    }
    jsx! {
        <div>
            <b title={href} className={classes.as_str()}>{children}</b>
        </div>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('  if (!href) {\n    return <div className={classes}>{children}</div>;\n  }\n  if (href.startsWith("https://")) {\n');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.FooterLink({ href: "https://x", children: "c", isHeader: true }))).toBe('<div><a href="https://x" class="link h">c</a></div>');
  expect(renderToStaticMarkup(result.FooterLink({ href: "", children: "c", isHeader: false }))).toBe('<div class="link">c</div>');
});

// What elements read, made before branches in their tree, each a call:
// each read where it's written, the last first, then the one before it.
test("what elements read before branches, each a call, is read where it's written", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
fn cn(a: &str, b: bool) -> String {
    format!("{a}{}", if b { " on" } else { "" })
}
pub fn Page(home: bool, blog: bool, content: &str) -> JSX::Element {
    jsx! {
        <div className={cn("page", home)}>
            {(home || blog).then(|| jsx! { <b>{"h"}</b> })}
            <div className={cn("cols", blog)}>
                {blog.then(|| jsx! { <i>{"s"}</i> })}
                <p className={cn("text", blog)}>{content}</p>
                <p className={cn("more", home)}>{content}</p>
            </div>
        </div>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('    <div className={cn("page", home)}>\n');
  expect(output).toContain('      <div className={cn("cols", blog)}>\n');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Page(true, false, "c"))).toBe('<div class="page on"><b>h</b><div class="cols"><p class="text">c</p><p class="more on">c</p></div></div>');
});

// A closure a \`let\` names, only a hook's function: written there, named by
// the \`let\` (ADR 0297).
test("a hook's closure a let names is a named function where it's given", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_effect, use_state};
pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state::<i32>(0);
    let tick = move || {
        set_count.set(count + 1);
    };
    use_effect(tick, Some(&[count]));
    jsx! { <b>{count}</b> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain("  useEffect(\n    function tick() {\n      setCount((count + 1) | 0);\n    },\n    [count],\n  );\n");
});

test("a prop before children that need no statements stays where it's written", () => {
  const source = `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::{JSX, Fragment, ReactNode, jsx};
// A \`Cell\` in it: its fields may change, so only what's needed is read first.
pub struct Crumb { pub title: String, pub seen: std::cell::Cell<bool> }
#[rust_js::link_name = "next/link#default"]
pub fn Link<C: ReactNode>(props: LinkProps<'_, C>) -> JSX::Element { unreachable!() }
#[derive(Default)]
pub struct LinkProps<'a, C> {
    pub href: &'a str,
    pub children: C,
    #[rust_js::flatten]
    pub anchor: AnchorHTMLAttributes<'a>,
    pub class_name: Option<&'a str>,
}
pub fn crumbs(crumbs: &[Crumb]) -> Vec<JSX::Element> {
    crumbs.iter().map(|crumb| jsx! {
        <Fragment key={crumb.title.as_str()}>
            <Link href={crumb.title.as_str()} className={Some("crumb")}>{crumb.title.as_str()}</Link>
        </Fragment>
    }).collect()
}
// Children that aren't simple, but need no statements: a table read by
// its variant, \`TITLES[kind]\` (ADR 0233).
#[derive(Clone, Copy)]
pub enum Kind {
    #[rust_js::name = "note"]
    Note,
    #[rust_js::name = "tip"]
    Tip,
    #[rust_js::name = "rsc"]
    Rsc,
}
pub struct Titles {
    pub note: &'static str,
    pub tip: &'static str,
    pub rsc: &'static str,
}
static TITLES: Titles = Titles { note: "Note", tip: "Tip", rsc: "RSC" };
pub fn titled(kind: Kind, crumb: &Crumb) -> JSX::Element {
    jsx! {
        <Fragment key={crumb.title.as_str()}>
            <b>{match kind { Kind::Note => TITLES.note, Kind::Tip => TITLES.tip, Kind::Rsc => TITLES.rsc }}</b>
        </Fragment>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("<Fragment key={crumb.title}>");
  expect(js).toContain("<b>{TITLES[kind]}</b>");
  expect(js).not.toContain("const key");
});

// A props struct's companion writes its fields in the order they're
// declared, the flattened one where it is, `anchor` before `class_name`:
// a literal in order needs no `const`s, so an `if` of such a component is
// a conditional in its JSX, as react.dev's Link chooses its link (ADR 0213).
test("a component whose flattened props come before a field is a conditional in JSX", async () => {
  const source = `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::{JSX, ReactNode, jsx};
#[rust_js::link_name = "next/link#default"]
pub fn NextLink<C: ReactNode>(_props: LinkProps<'_, C>) -> JSX::Element { unreachable!() }
pub struct LinkProps<'a, C> {
    pub href: &'a str,
    pub children: C,
    #[rust_js::flatten]
    pub anchor: AnchorHTMLAttributes<'a>,
    pub class_name: Option<&'a str>,
}
pub fn Pick(href: &str) -> JSX::Element {
    jsx! {
        <>
            {if href.starts_with('#') {
                jsx! { <a href={href}>{"here"}</a> }
            } else {
                jsx! { <NextLink href={href} className={Some("link")} id={Some("x")}>{"there"}</NextLink> }
            }}
        </>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('{href.startsWith("#") ? (');
  expect(js).toContain('<NextLink href={href} className="link" id="x">');
  expect(js).not.toContain("let tmp");
});

// A child shown only if a test holds is `test && <b />`, as JSX writes it:
// a false test, and `null` or `undefined`, render nothing, as `undefined`
// does. react.dev's ConsoleBlock has `{level === 'warning' && <IconWarning />}`.
test("a child shown only if a test holds is the test && the child", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn Level(level: u32) -> JSX::Element {
    jsx! { <p>{(level == 1).then(|| jsx! { <b>{"warn"}</b> })}{"x"}</p> }
}
// A \`bool\` is one whatever its shape, a variable too, as react.dev's
// TeamMember has \`{isLead && <span>★</span>}\`.
pub fn Lead(group: &str) -> JSX::Element {
    let is_lead = group.ends_with('*');
    jsx! { <p>{is_lead.then(|| jsx! { <b>{"lead"}</b> })}</p> }
}
// Text and a number keep their != null, as "" and 0 would render;
// a test of text by its truthiness is \`!!href\`, a boolean; and any
// condition is one, a \`bool\`, a call's too.
pub fn Hash(href: &'static str) -> JSX::Element {
    jsx! { <p>{if href.starts_with('#') { Some(jsx! { <a href={href} /> }) } else { None }}</p> }
}
pub fn Named(name: Option<&'static str>, count: Option<u32>, href: Option<&'static str>) -> JSX::Element {
    jsx! {
        <p>
            {name.map(|n| jsx! { <b>{n}</b> })}
            {count.map(|c| jsx! { <i>{c}</i> })}
            {if let Some(h) = href.filter(|h| !h.is_empty()) { Some(jsx! { <a href={h} /> }) } else { None }}
        </p>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('{level === 1 && <b>warn</b>}');
  expect(jsx).toContain("{isLead && <b>lead</b>}");
  expect(jsx).toContain("{name != null && <b>{name}</b>}");
  expect(jsx).toContain("{count != null && <i>{count}</i>}");
  expect(jsx).not.toContain("{count &&");
  expect(jsx).not.toContain("{name &&");
  expect(jsx).toContain("{!!href && <a href={href} />}");
  expect(jsx).toContain('{href.startsWith("#") && <a href={href} />}');
  const { Level } = await import(join(dir, "lib.jsx"));
  expect([1, 2].map((level) => renderToStaticMarkup(Level(level)))).toEqual(["<p><b>warn</b>x</p>", "<p>x</p>"]);
  const { Lead } = await import(join(dir, "lib.jsx"));
  expect(["a*", "a"].map((group) => renderToStaticMarkup(Lead(group)))).toEqual(["<p><b>lead</b></p>", "<p></p>"]);
  const { Named } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(Named("", 0, "")), renderToStaticMarkup(Named(undefined, undefined, undefined))]).toEqual(["<p><b></b><i>0</i></p>", "<p></p>"]);
});

// A component whose props take what a DOM element takes is an `ElementType`,
// as @types/react's, whatever else its props have, so one field holds any of
// them, and a capitalized local of it is a tag: react.dev's ExpandableCallout
// keeps its icons in `variantMap` and renders `<variant.Icon className=.. />`
// (ADR 0234).
test("components of an element's props are an ElementType, rendered as a tag", async () => {
  const icons = `use react::attributes::SVGAttributes;
use react::{JSX, ElementProps, MemoExoticComponent, jsx, memo};
pub struct BadgeProps {
    #[rust_js::flatten]
    pub svg: SVGAttributes<'static>,
    pub size: Option<&'static str>,
}
impl ElementProps for BadgeProps {}
thread_local! {
    pub static IconNote: MemoExoticComponent<SVGAttributes<'static>> = memo(Note, None);
    pub static IconBadge: MemoExoticComponent<BadgeProps> = memo(Badge, None);
}
fn Note(props: SVGAttributes<'static>) -> JSX::Element {
    jsx! { <svg className={props.class_name} /> }
}
fn Badge(props: BadgeProps) -> JSX::Element {
    jsx! { <svg className={props.svg.class_name} width={props.size.unwrap_or("1em")} /> }
}
`;
  const { dir, args } = compile(`#![allow(non_snake_case)]
mod icons;
use icons::{IconBadge, IconNote};
use react::{JSX, ElementType, element_type, jsx};
pub struct Variant {
    pub title: &'static str,
    #[rust_js::name = "Icon"]
    pub icon: Option<ElementType>,
}
pub fn Callout(which: u32) -> JSX::Element {
    let icon = match which {
        0 => Some(element_type(&IconNote)),
        1 => Some(element_type(&IconBadge)),
        _ => None,
    };
    let variant = Variant { title: "t", icon };
    jsx! {
        <h3>
            {variant.icon.map(|Icon| jsx! { <Icon className={Some("inline")} /> })}
            {variant.title}
        </h3>
    }
}
// A \`static\`'s table holds one, as react.dev's \`variantMap\` does.
static PINNED: Variant = Variant { title: "p", icon: Some(element_type(&IconBadge)) };
pub fn Pinned() -> JSX::Element {
    jsx! { <h3>{PINNED.icon.map(|Icon| jsx! { <Icon className={Some("pin")} /> })}{PINNED.title}</h3> }
}
// Read from a table by a kind, as react.dev's ExpandableCallout reads its
// \`variantMap\` (ADR 0233): the icon read where it's rendered.
pub struct Variants {
    pub note: Variant,
    pub badge: Variant,
}
static VARIANTS: Variants = Variants {
    note: Variant { title: "n", icon: Some(element_type(&IconNote)) },
    badge: Variant { title: "b", icon: None },
};
#[derive(Clone, Copy)]
pub enum Kind {
    #[rust_js::name = "note"]
    Note,
    #[rust_js::name = "badge"]
    Badge,
}
pub fn Tabled(kind: Kind) -> JSX::Element {
    let variant = match kind {
        Kind::Note => &VARIANTS.note,
        Kind::Badge => &VARIANTS.badge,
    };
    jsx! {
        <h3>
            {variant.icon.map(|Icon| jsx! { <Icon className={Some("tabled")} /> })}
            {variant.title}
        </h3>
    }
}
// Its own props given by name, as a component's are: still one component.
pub fn Badged() -> JSX::Element {
    jsx! { <IconBadge className={Some("b")} size={Some("2em")} /> }
}
`, { "icons.rs": icons });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("icon = IconNote;");
  expect(jsx).toContain('{variant.Icon && <variant.Icon className="inline" />}');
  expect(jsx).toContain('{variant.Icon && <variant.Icon className="tabled" />}');
  const { Callout } = await import(join(dir, "lib.jsx"));
  expect([0, 1, 2].map((which) => renderToStaticMarkup(Callout(which)))).toEqual([
    '<h3><svg class="inline"></svg>t</h3>',
    '<h3><svg class="inline" width="1em"></svg>t</h3>',
    "<h3>t</h3>",
  ]);
  // A `static` of another module's components is made of them, which JS
  // imports first.
  expect(jsx).toContain('const PINNED = { title: "p", Icon: IconBadge };');
  const { Badged, Pinned, Tabled } = await import(join(dir, "lib.jsx"));
  expect(["note", "badge"].map((kind) => renderToStaticMarkup(Tabled(kind)))).toEqual(['<h3><svg class="tabled"></svg>n</h3>', "<h3>b</h3>"]);
  expect(renderToStaticMarkup(Pinned())).toBe('<h3><svg class="pin" width="1em"></svg>p</h3>');
  expect(renderToStaticMarkup(Badged())).toBe('<svg class="b" width="2em"></svg>');
  // Of its own module's, made after it in JS, it's refused, as a static
  // read before it's made is.
  const own = compile(`use react::{ElementType, MemoExoticComponent, element_type, jsx, memo};
${icons.replace(/^use .*\n/gm, "")}
static PINNED: Option<ElementType> = Some(element_type(&IconBadge));
`.replace("use react::{ElementType", "use react::attributes::SVGAttributes;\nuse react::{ElementProps, ElementType, JSX"));
  const refused = Bun.spawnSync(own.args, { cwd: own.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("statics of type")], refused.stderr.toString()).toEqual([false, true]);
});

// A component chosen as the page runs, `const Heading = isRecipes ? H4 : H2`,
// is a local of a function's type, `fn(HProps<..>) -> JSX::Element`, whose
// props `jsx!` builds by that type: `<Heading id=.. className=..>`, as
// react.dev's Challenges has it (ADR 0239).
test("a local of a component's function type is a tag given its props", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub struct HProps<'a, C: ReactNode> {
    pub id: Option<&'a str>,
    pub class_name: Option<&'a str>,
    pub children: C,
}
pub fn H2<C: ReactNode>(HProps { id, class_name, children }: HProps<C>) -> JSX::Element {
    jsx! { <h2 id={id} className={class_name}>{children}</h2> }
}
pub fn H4<C: ReactNode>(HProps { id, class_name, children }: HProps<C>) -> JSX::Element {
    jsx! { <h4 id={id} className={class_name}>{children}</h4> }
}
pub fn Title(small: bool) -> JSX::Element {
    let Heading: fn(HProps<'static, &'static str>) -> JSX::Element = if small { H4 } else { H2 };
    jsx! { <Heading key="h" id={Some("t")} className={Some("title")}>{"Hi"}</Heading> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const Heading = small ? H4 : H2;");
  expect(jsx).toContain('<Heading key="h" id="t" className="title">');
  const { Title } = await import(join(dir, "lib.jsx"));
  expect([true, false].map((small) => renderToStaticMarkup(Title(small)))).toEqual(['<h4 id="t" class="title">Hi</h4>', '<h2 id="t" class="title">Hi</h2>']);
});

// A component's children, each kept as `Children.forEach` gives it, for
// good, as react.dev's Challenges keeps them, each given a ref of its own,
// `useRef(kept.map(() => createRef()))`, as its Navigation gives each of its
// buttons one.
test("children kept from Children.forEach, each given a createRef", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::children::{self, Child};
use react::webapi::HTMLButtonElement;
use react::{JSX, ReactNode, create_ref, jsx, use_ref};
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub fn List<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let mut kept: Vec<Child<'static>> = Vec::new();
    children::for_each(&children, |child, _| {
        if let Some(child) = child {
            kept.push(child);
        }
    });
    let kept: &'static [Child<'static>] = Vec::leak(kept);
    let refs = use_ref(kept.iter().map(|_| create_ref::<&HTMLButtonElement>()).collect::<Vec<_>>());
    jsx! {
        <ul>
            {kept.iter().enumerate().map(|(i, &child)| jsx! {
                <li key={i}><button ref={refs.current()[i]}>{child}</button></li>
            }).collect::<Vec<_>>()}
        </ul>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("Children.forEach(children, (child) => {\n    if (child != null) {\n      kept.push(child);\n    }\n  });");
  expect(jsx).toContain("createRef()");
  const { List } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(List, null, "a", createElement("b", null, "x")))).toBe("<ul><li><button>a</button></li><li><button><b>x</b></button></li></ul>");
});

// What a closure took, read where it's inlined, leaves nothing behind: no
// `const captured = child.type`, as react.dev's prepareMDX tests each
// child's type against a thread-local's set.
test("a capture of an inlined closure leaves no const behind", async () => {
  const { dir, args } = compile(`use std::collections::HashSet;
use react::children::{self, Child};
use react::js::{self, Kind, Unknown, classify};
thread_local! {
    static TYPES: HashSet<&'static str> = HashSet::from(["h1", "Recap"]);
}
pub fn kinds(children: &'static Unknown) -> Vec<String> {
    let mut out = Vec::new();
    for child in children::to_array(children) {
        if let Child::Element(element) = child
            && let Kind::String(r#type) = classify(element.r#type())
            && TYPES.with(|types| types.contains(r#type))
        {
            out.push(if r#type == "Recap" { "recap".to_string() } else { r#type.to_uppercase() });
        }
    }
    out
}
`);
  run(args);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("captured");
  const { kinds } = await import(join(dir, "lib.js"));
  const { createElement } = await import("react");
  expect(kinds([createElement("h1", { key: "a" }), createElement("Recap", { key: "b" }), "text"])).toEqual(["H1", "recap"]);
});

// Each child with its index, `Children.forEach(children, (child, i) => ..)`,
// `None` of an empty one, `null`, which
// `forEach` gives where `toArray` leaves it out: react.dev's MDX keys its
// wrappers so, and skips what's empty.
test("children given with their index by Children.forEach", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::children;
use react::{JSX, ReactNode, jsx};
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub fn List<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let mut keys: Vec<String> = Vec::new();
    children::for_each(&children, |child, i| {
        // A child, never \`undefined\`, as a JS value of any shape.
        let typed = child.is_some_and(|child| react::js::get(react::js::unknown_of(child), "type").is_some());
        keys.push(format!("{}{i}{}", if child.is_some() { "k" } else { "n" }, if typed { "e" } else { "" }));
    });
    jsx! { <p>{keys.join(",")}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("Children.forEach(children, (child, i) => {");
  const { List } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(List, null, "a", null, "b"))).toBe("<p>k0,n1,k2</p>");
  expect(renderToStaticMarkup(createElement(List, null, createElement("b")))).toBe("<p>k0e</p>");
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

// A list whose callback has statements, and a handler of several, stay in
// their JSX, as a person writes them: oxfmt lays them out where they are
// (ADR 0218), as react.dev's `Breadcrumbs` maps its crumbs.
test("a callback of several statements stays in its JSX", () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub struct Crumb { pub title: String }
pub fn list(crumbs: &[Crumb]) -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! {
        <div onClick={move |_| { set_count.set(count + 1); set_count.set(count + 2); }}>
            {crumbs.iter().map(|crumb| {
                let t = crumb.title.as_str();
                if t.is_empty() { jsx! { <i /> } } else { jsx! { <b key={t}>{t}</b> } }
            }).collect::<Vec<_>>()}
        </div>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("{crumbs.map((crumb) => {");
  expect(js).toContain("onClick={() => {");
  expect(js).not.toContain("const items");
  expect(js).not.toContain("const onClick");
});

// A trait of no items, react's `Key`, is one a program may implement: its
// impl runs nothing, as an auto trait's doesn't. A fieldless variant is its
// name, which a list's `key` takes, as react.dev's `PageHeading` keys its tags.
test("a program implements a trait of no items, react's Key of its enum", () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
#[derive(Clone, Copy)]
pub enum Tag {
    #[rust_js::name = "new"]
    New,
    #[rust_js::name = "old"]
    Old,
}
impl react::Key for Tag {}
pub fn list(tags: &[Tag]) -> Vec<JSX::Element> {
    tags.iter().map(|tag| jsx! { <li key={*tag}>{"tag"}</li> }).collect()
}
`;
  const { dir, args } = compile(source);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("return tags.map((tag) => <li key={tag}>tag</li>);");
});

// A `key`, which `jsx!` gives last, is where it was written, as react.dev's
// NavigationBar writes `<Listbox.Option key={filePath} value={filePath}>`
// (ADR 0254), a component's as an element's.
test("a key is where it was written", () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct ItemProps {
    pub value: String,
}
#[rust_js::link_name = "./item.js#Item"]
pub fn Item(props: ItemProps) -> JSX::Element {
    unreachable!()
}
pub fn files(paths: &[String]) -> Vec<JSX::Element> {
    paths.iter().map(|path| jsx! { <li key={path.as_str()} className="file">{path.as_str()}</li> }).collect()
}
pub fn items(paths: &[String]) -> Vec<JSX::Element> {
    paths.iter().map(|path| jsx! { <Item key={path.as_str()} value={path.clone()} /> }).collect()
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('<li key={path} className="file">');
  expect(js).toContain("<Item key={path} value={path} />");
});

// A component's key that reads the same wherever it's read goes first with
// its children in place: it can't see what they do, nor they it, as
// react.dev's InlineTocItem writes `<LI key={node.item.url}>`. One the
// children change is still read after them.
test("a key that reads alike keeps a component's children in place", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub struct RowProps<C: ReactNode> {
    pub children: C,
}
pub fn Row<C: ReactNode>(RowProps { children }: RowProps<C>) -> JSX::Element {
    jsx! { <li>{children}</li> }
}
pub struct Entry {
    pub url: String,
}
pub fn rows(entries: &[Entry]) -> Vec<JSX::Element> {
    entries.iter().map(|entry| jsx! { <Row key={&entry.url}><b>{entry.url.as_str()}</b><i /></Row> }).collect()
}
pub struct Counter {
    pub count: u32,
}
fn bump(counter: &mut Counter) -> u32 {
    counter.count += 1;
    counter.count
}
pub fn bumped() -> JSX::Element {
    let mut counter = Counter { count: 0 };
    jsx! { <Row key={counter.count}>{bump(&mut counter)}</Row> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("<Row key={entry.url}>\n      <b>{entry.url}</b>");
  // JSX reads a key where it's written, before the children.
  expect(js).toContain("const children = bump(counter);\n  return <Row key={counter.count}>{children}</Row>;");
  const { bumped } = await import(join(dir, "lib.jsx"));
  expect(bumped().key).toBe("1");
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

// As react.dev's ExpandableExample has them: a Button's children, which its
// props declare first, read only a state's value, so a computed className
// after them needn't be read first; and an excerpt shown where it isn't
// empty is one expression, `excerpt && <div>..</div>`, as a person writes it.
test("JSX keeps children that read what never changes, and a filtered child, in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, event, jsx, use_state};
#[cfg_attr(rust_js, rust_js::link_name = "String")]
#[allow(unused_variables)]
fn cn(a: &str) -> String {
    unreachable!()
}
pub struct ButtonProps<'a, C: ReactNode> {
    pub children: C,
    pub class_name: Option<&'a str>,
    pub on_click: Option<Box<dyn Fn(&event::MouseEvent)>>,
}
pub fn Button<C: ReactNode>(ButtonProps { children, class_name, on_click }: ButtonProps<C>) -> JSX::Element {
    let _ = on_click;
    jsx! { <button className={class_name}>{children}</button> }
}
pub fn Example(excerpt: Option<&str>) -> JSX::Element {
    let (expanded, set_expanded) = use_state(false);
    jsx! {
        <div>
            <h5 className={cn("title")}>{"Example"}</h5>
            {excerpt.filter(|excerpt| !excerpt.is_empty()).map(|excerpt| jsx! { <p>{excerpt}</p> })}
            <Button className={Some(cn("button").as_str())} onClick={Some(Box::new(move |_| set_expanded.update(|e| !e)))}>
                <span>{if *expanded { "Hide" } else { "Show" }}</span>
            </Button>
        </div>
    }
}
pub fn title(name: Option<&str>) -> String {
    name.filter(|name| !name.is_empty()).unwrap_or("Error").to_string()
}
pub fn heading(code: Option<&str>) -> String {
    let shown = match code {
        Some(code) if !code.is_empty() => format!("Error #{code}"),
        _ => "Errors".to_string(),
    };
    shown
}
pub fn titled(code: Option<String>) -> String {
    let named = match code.as_deref() {
        Some(code) if !code.is_empty() => format!("Error #{code}"),
        _ => "Errors".to_string(),
    };
    named
}
pub fn both(a: bool, text: &str) -> u32 {
    if a && !text.is_empty() { 1 } else { 0 }
}
pub fn empty(text: &str) -> bool {
    text.is_empty()
}
pub fn positive(n: Option<i32>) -> i32 {
    n.filter(|n| *n > 0).unwrap_or(1)
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{!!excerpt && <p>{excerpt}</p>}");
  expect(jsx).toContain('<span>{expanded ? "Hide" : "Show"}</span>');
  expect(jsx).not.toContain("const children");
  expect(jsx).not.toContain("const className");
  // Text kept where it isn't empty, or a default, is `||` (ADR 0266).
  expect(jsx).toContain('return name || "Error";');
  // Text that isn't empty is JS's truthy text: `code ? .. : ..`, as
  // react.dev's errors page titles itself (ADR 0266).
  expect(jsx).toContain("const shown = code ? `Error #${code}` : \"Errors\";");
  expect(jsx).toContain("export function empty(text) {\n  return !text;\n}");
  expect(jsx).toContain("  if (a && text) {");
  // Of `as_deref()`, the option itself, matched where it is (ADR 0211).
  expect(jsx).toContain("const named = code ? `Error #${code}` : \"Errors\";");
  // A filter of another test, or a default, is that test.
  expect(jsx).toContain("return n != null && n > 0 ? n : 1;");
  const { Example, title } = await import(join(dir, "lib.jsx"));
  expect(["e", "", undefined].map((e) => renderToStaticMarkup(createElement(() => Example(e))))).toEqual([
    '<div><h5 class="title">Example</h5><p>e</p><button class="button"><span>Show</span></button></div>',
    '<div><h5 class="title">Example</h5><button class="button"><span>Show</span></button></div>',
    '<div><h5 class="title">Example</h5><button class="button"><span>Show</span></button></div>',
  ]);
  expect([title("Oops"), title(""), title(undefined)]).toEqual(["Oops", "Error", "Error"]);
  const { positive, heading, empty } = await import(join(dir, "lib.jsx"));
  expect([heading("1"), heading(""), heading(undefined)]).toEqual(["Error #1", "Errors", "Errors"]);
  expect([empty(""), empty("a")]).toEqual([true, false]);
  const { both } = await import(join(dir, "lib.jsx"));
  expect([both(true, "x"), both(true, ""), both(false, "x")]).toEqual([1, 0, 0]);
  expect([positive(3), positive(-2), positive(undefined)]).toEqual([3, 1, 1]);
});

// Each tag takes what @types/react's `JSX.IntrinsicElements` gives it: an
// `<a>` an `href`, a `<button>` `disabled`, every element a `title`, and a
// tag value, which may be any, anything; a `<div>`'s `href` is an error that
// says so (ADR 0228).
test("JSX tags take the attributes @types/react gives each", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, Tag, jsx};
#[derive(Clone, Copy)]
pub enum As {
    #[rust_js::name = "a"]
    A,
}
impl Tag for As {}
pub fn View(Comp: As) -> JSX::Element {
    jsx! {
        <div title="t">
            <a href="/a" target="_blank">{"a"}</a>
            <button disabled={true} type="button">{"b"}</button>
            <input value="v" placeholder="p" />
            <Comp href="/c">{"c"}</Comp>
        </div>
    }
}
`);
  run(args);
  const { View } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(View("a"))).toBe('<div title="t"><a href="/a" target="_blank">a</a><button disabled="" type="button">b</button><input placeholder="p" value="v"/><a href="/c">c</a></div>');
  for (const [wrong, says] of [
    ['<div href="/a" />', "`react::webapi::HTMLDivElement` takes no `href`"],
    ["<span disabled={true} />", "`react::webapi::HTMLSpanElement` takes no `disabled`"],
  ]) {
    const refused = compile(`use react::{JSX, jsx};\npub fn View() -> JSX::Element {\n    jsx! { ${wrong} }\n}\n`);
    const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
    expect([failed.exitCode === 0, failed.stderr.toString().includes(says)]).toEqual([false, true]);
  }
});

// An attribute @types/react types as a few strings, `referrerPolicy`'s
// `"no-referrer" | "origin" | ..`, a button's `type`, `aria-live`, takes a
// literal of them only, as TypeScript checks it: another is an error that
// says which it takes. One of any string, an `<input>`'s `type`, takes any.
test("JSX checks a literal of an attribute of a few strings", async () => {
  const { dir, args } = compile(`use react::{JSX, jsx};
pub fn View() -> JSX::Element {
    jsx! {
        <div aria-live="polite" draggable="true">
            <img referrerPolicy="no-referrer" crossOrigin="anonymous" src="a.png" />
            <button type="submit">{"Go"}</button>
            <input type="email" />
        </div>
    }
}
`);
  run(args);
  const { View } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(View())).toContain('<div aria-live="polite" draggable="true"><img referrerPolicy="no-referrer" crossorigin="anonymous" src="a.png"/><button type="submit">Go</button><input type="email"/></div>');
  for (const [wrong, says] of [
    ['<img referrerPolicy="no-referer" />', '`"no-referer"` isn\'t a `referrerPolicy`, which is one of "", "no-referrer"'],
    ['<button type="sumbit" />', '`"sumbit"` isn\'t a `type`, which is one of "button", "reset", "submit"'],
    ['<div aria-live="loud" />', '`"loud"` isn\'t a `aria-live`, which is one of "assertive", "off", "polite"'],
  ]) {
    const refused = compile(`use react::{JSX, jsx};\npub fn View() -> JSX::Element {\n    jsx! { ${wrong} }\n}\n`);
    const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
    expect([failed.exitCode === 0, failed.stderr.toString().includes(says)], failed.stderr.toString()).toEqual([false, true]);
  }
});

// Each attribute takes what @types/react types it as: `tabIndex` a number,
// `className` text, `width` either, `draggable` a `Booleanish`, a bool or
// its text; a component's flattened ones the same, of `NumberOrString` and
// `Booleanish`, which are their value in JS. Text for a number is an error.
test("JSX attributes take the values @types/react types them as", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};
pub struct PictureProps<'a> {
    #[rust_js::flatten]
    pub img: ImgHTMLAttributes<'a>,
}
pub fn Picture(PictureProps { img }: PictureProps) -> JSX::Element {
    jsx! { <img width={img.width} draggable={img.html.draggable} /> }
}
pub fn View() -> JSX::Element {
    jsx! {
        <div tabIndex={-1} className="c" draggable={true}>
            <img width={300} height="2em" draggable="false" />
            <Picture width={Some(64.0.into())} draggable={Some(true.into())} />
        </div>
    }
}
`);
  run(args);
  const { View } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(View())).toBe('<div tabindex="-1" class="c" draggable="true"><img width="300" height="2em" draggable="false"/><img width="64" draggable="true"/></div>');
  for (const [wrong, says] of [
    ['<div tabIndex={"x"} />', "is not a number"],
    ["<div className={3} />", "is not text"],
  ]) {
    const refused = compile(`use react::{JSX, jsx};\npub fn View() -> JSX::Element {\n    jsx! { ${wrong} }\n}\n`);
    const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
    expect([failed.exitCode === 0, failed.stderr.toString().includes(says)], failed.stderr.toString()).toEqual([false, true]);
  }
});

// An SVG tag is its SVG element, `<circle>` an `SVGCircleElement`, its ref's
// and its events' `currentTarget`; it takes `SVGAttributes`, as
// @types/react's `SVGProps` gives every SVG tag, and an HTML tag doesn't:
// `<div cx>` is an error, as an HTML-only global, `<circle hidden>`, is.
test("JSX SVG tags are their SVG elements, of SVG's attributes", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::webapi::SVGCircleElement;
use react::{JSX, RefObject, jsx};
pub fn Dot(dot: RefObject<Option<&'static SVGCircleElement>>) -> JSX::Element {
    jsx! {
        <svg viewBox="0 0 10 10" width="10">
            <circle
                ref={dot}
                cx="5"
                cy={5}
                r="4"
                className="dot"
                onClick={|e| {
                    let _: &SVGCircleElement = e.current_target();
                }} />
        </svg>
    }
}
`);
  run(args);
  const { Dot } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Dot({ current: null }))).toBe('<svg viewBox="0 0 10 10" width="10"><circle cx="5" cy="5" r="4" class="dot"></circle></svg>');
  for (const [wrong, says] of [
    ['<div cx="5" />', "`react::webapi::HTMLDivElement` takes no `cx`"],
    ["<circle hidden={true} />", "`react::webapi::SVGCircleElement` takes no `hidden`"],
  ]) {
    const refused = compile(`use react::{JSX, jsx};\npub fn View() -> JSX::Element {\n    jsx! { ${wrong} }\n}\n`);
    const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
    expect([failed.exitCode === 0, failed.stderr.toString().includes(says)], failed.stderr.toString()).toEqual([false, true]);
  }
});

// React DOM's own table leaves these out, as their spelling needs no warning;
// @types/react types them (ADR 0043).
test("JSX takes every attribute @types/react types", async () => {
  const { dir, args } = compile(`use react::{JSX, jsx};
pub fn View() -> JSX::Element {
    jsx! { <pre translate="no" slot="code"><img loading="lazy" decoding="async" src="a.png" /></pre> }
}
`);
  run(args);
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.View())).toBe('<pre translate="no" slot="code"><img loading="lazy" decoding="async" src="a.png"/></pre>');
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
  expect(code).toContain('<FORWARD label="forward" />');
  expect(code).toContain('<LAZY />');
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

// A context's `Consumer`, as @types/react has it, `<THEME.Consumer>`, of a
// function of its value, its children; and `createPortal`'s `ReactPortal`,
// an element, `ReactPortal extends ReactElement`, a component's result by
// `.element()`.
test("JSX reads a context by its Consumer, and a portal is a ReactPortal", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::dom::create_portal;
use react::webapi::HTMLElement;
use react::{Context, JSX, ReactPortal, create_context, jsx};
thread_local! {
    pub static THEME: Context<&'static str> = create_context("light");
}
pub fn Label() -> JSX::Element {
    jsx! { <THEME.Consumer>{|theme: &&str| jsx! { <b>{*theme}</b> }}</THEME.Consumer> }
}
pub fn Away(container: &'static HTMLElement) -> JSX::Element {
    let portal: &ReactPortal = create_portal(jsx! { <i>{"away"}</i> }, container, None);
    portal.element()
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<THEME.Consumer>{(theme) => <b>{theme}</b>}</THEME.Consumer>");
  expect(jsx).toContain('const portal = createPortal(<i>away</i>, container);\n  return portal;');
  const { Label, THEME } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect([
    renderToStaticMarkup(createElement(Label)),
    renderToStaticMarkup(createElement(THEME, { value: "dark" }, createElement(Label))),
  ]).toEqual(["<b>light</b>", "<b>dark</b>"]);
});

// react.dev's Toc gives its `IsInTocContext.Provider` a list of headings: a
// provider's children are any node, as a component's are.
test("JSX context providers take any node as children", async () => {
  const { dir, args } = compile(`#![deny(warnings)]
#![allow(non_snake_case)]
use react::{Context, JSX, create_context, jsx, use_context};
thread_local! { static THEME: Context<&'static str> = create_context("light"); }
fn Label() -> JSX::Element {
    let theme = use_context(&THEME);
    jsx! { <b>{*theme}</b> }
}
pub fn App() -> JSX::Element {
    let names = vec!["a", "b"];
    jsx! {
        <>
            <THEME value="dark">
                {names.iter().map(|name| jsx! { <i key={*name}>{*name}</i> }).collect::<Vec<_>>()}
                <Label />
            </THEME>
            <THEME.Provider value="legacy">
                {(!names.is_empty()).then(|| jsx! { <Label /> })}
            </THEME.Provider>
            <THEME.Provider value="text">{"plain"}</THEME.Provider>
        </>
    }
}
`);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<THEME.Provider value="text">plain</THEME.Provider>');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(result.App))).toBe("<i>a</i><i>b</i><b>dark</b><b>legacy</b>plain");
});

// ADR 0224: a tag's element reaches its handlers' events and its `ref`, as
// @types/react's `IntrinsicElements` gives them, while what JSX makes is an
// `JSX::Element` whatever its tag. A handler of any element's event is widened.
test("JSX gives a tag's element to its handlers and its ref", () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
use react::{JSX, event, jsx, use_ref, webapi};
pub struct ButtonProps {
    pub on_click: Box<dyn Fn(&event::MouseEvent)>,
}
pub fn Button(ButtonProps { on_click }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={event::MouseEvent::widen(on_click)}>{"Go"}</button> }
}
pub fn App(busy: bool) -> JSX::Element {
    let input = use_ref(None::<&'static webapi::HTMLInputElement>);
    let label = if busy { jsx! { <span>{"…"}</span> } } else { jsx! { <b>{"Save"}</b> } };
    jsx! {
        <form>
            <input ref={input} />
            <button onClick={|e| e.current_target().set_disabled(true)}>{label}</button>
            <details onToggle={|e| e.current_target().set_open(false)} />
        </form>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<button onClick={onClick}>Go</button>');
  expect(code).toContain("<input ref={input} />");
  expect(code).toContain("const label = busy ? <span>…</span> : <b>Save</b>;");
  expect(code).toContain("onClick={(e) => {\n          e.currentTarget.disabled = true;\n        }}");
  expect(code).toContain("e.currentTarget.open = false;");
  // An input's ref on a button, a handler of an input's event on it, and a
  // handler of any element's event not widened, are each rustc's error.
  for (const [wrong, written, error] of [
    ["<input ref={input} />", "<button ref={input} />", "the trait `react::webapi::IsA<react::webapi::HTMLInputElement>` is not implemented for `react::webapi::HTMLButtonElement`"],
    ["onClick={|e| e.current_target().set_disabled(true)}", "onClick={|e: &event::MouseEvent<webapi::HTMLInputElement>| { let _ = e; }}", "expected closure signature `for<'a> fn(&'a MouseEvent<react::webapi::HTMLButtonElement>) -> _`"],
    ["onClick={event::MouseEvent::widen(on_click)}", "onClick={on_click}", "found `dyn for<'a> std::ops::Fn(&'a react::event::MouseEvent)`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace(wrong, written));
    const result = Bun.spawnSync(args, { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
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
use react::{JSX, ForwardRefExoticComponent, ForwardedRef, RefObject, forward_ref, jsx, webapi};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] #[allow(unused_variables)] fn record(n: i32) -> i32 { unreachable!() }
pub struct Props { pub label: i32 }
pub fn Input(p: Props, reference: ForwardedRef<&'static webapi::Element>) -> JSX::Element {
    jsx! { <input ref={reference} tabIndex={p.label} /> }
}
thread_local! { static INPUT: ForwardRefExoticComponent<Props, &'static webapi::Element> = forward_ref(Input); }
pub fn Plain(reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
    jsx! { <INPUT ref={reference} label={1} /> }
}
pub fn App(reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
    jsx! { <INPUT ref={record(1); reference} label={record(2)} /> }
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
  writeFileSync(file, source.replace('pub fn App(reference: RefObject<Option<&\'static webapi::Element>>)', 'pub fn App(reference: RefObject<Option<i32>>)'));
  const invalid = Bun.spawnSync(args, { cwd: dir });
  expect(invalid.exitCode).not.toBe(0);
  expect(invalid.stderr.toString()).toContain('is not a ref of');
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toBe(output);
});

// What a hook takes and gives, by @types/react's names: a prop that's a
// `Reducer`, an `EffectCallback` or a `TransitionFunction` is given to its
// hook as it is, and `use_reducer` gives an `ActionDispatch`.
test("JSX hooks take @types/react's Reducer, EffectCallback and TransitionFunction", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{ActionDispatch, EffectCallback, JSX, Reducer, TransitionFunction, jsx, start_transition, use_effect, use_reducer};
pub struct CounterProps {
    pub reducer: Reducer<i32, i32>,
    pub effect: EffectCallback,
    pub later: Option<TransitionFunction>,
}
pub fn Counter(CounterProps { reducer, effect, later }: CounterProps) -> JSX::Element {
    let (count, _dispatch): (&i32, ActionDispatch<i32>) = use_reducer(reducer, 1);
    use_effect(effect, Some(&()));
    if let Some(later) = later {
        start_transition(later);
    }
    jsx! { <p>{*count}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("useReducer(reducer, 1)");
  expect(jsx).toContain("useEffect(effect, []);");
  expect(jsx).toContain("startTransition(later);");
  const { Counter } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Counter, { reducer: (s: number, a: number) => s + a, effect: () => {} }))).toBe("<p>1</p>");
});

// A component's props as an element's, @types/react's
// `DetailedHTMLProps<AnchorHTMLAttributes<HTMLAnchorElement>, HTMLAnchorElement>`,
// flattened: its `ref` a `Ref`, an object or a callback, each given as
// itself, `.into()`, and spread on to its `<a>` with the rest (ADR 0384).
test("a component's props flatten DetailedHTMLProps, its ref an object or a callback", async () => {
  const source = `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::webapi::HTMLAnchorElement;
use react::{DetailedHTMLProps, JSX, ReactNode, jsx, use_ref};
pub struct ExternalLinkProps<'a, C: ReactNode> {
    pub target: Option<&'a str>,
    pub children: Option<C>,
    #[rust_js::flatten]
    pub props: DetailedHTMLProps<AnchorHTMLAttributes<'a>, HTMLAnchorElement>,
}
pub fn ExternalLink<C: ReactNode>(ExternalLinkProps { target, children, props }: ExternalLinkProps<'_, C>) -> JSX::Element {
    jsx! { <a target={target.unwrap_or("_blank")} rel="noopener" {...props}>{children}</a> }
}
pub fn ByObject() -> JSX::Element {
    let anchor = use_ref::<Option<&'static HTMLAnchorElement>>(None);
    jsx! {
        <ExternalLink
            target={None}
            props={DetailedHTMLProps {
                r#ref: Some(anchor.into()),
                attributes: AnchorHTMLAttributes { href: Some("/a"), ..Default::default() },
            }}>
            {Some("A")}
        </ExternalLink>
    }
}
pub fn ByCallback() -> JSX::Element {
    jsx! {
        <ExternalLink
            target={None}
            props={DetailedHTMLProps {
                r#ref: Some((|_anchor: Option<&'static HTMLAnchorElement>| {}).into()),
                attributes: AnchorHTMLAttributes { href: Some("/b"), ..Default::default() },
            }}>
            {Some("B")}
        </ExternalLink>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function ExternalLink({ target, children, ...props }) {");
  expect(jsx).toContain('<ExternalLink ref={anchor} href="/a">');
  expect(jsx).toContain('<ExternalLink ref={(_anchor) => {}} href="/b">');
  const { ByObject, ByCallback } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(ByObject))).toBe('<a target="_blank" rel="noopener" href="/a">A</a>');
  expect(renderToStaticMarkup(createElement(ByCallback))).toBe('<a target="_blank" rel="noopener" href="/b">B</a>');
});

// A ref is given as a `RefObject` or a `RefCallback`, which a component's
// own prop takes, `impl IntoRef<&HTMLDivElement, M>`, and passes to its
// element as it is; one of another element is an error that says so.
test("JSX refs are @types/react's Ref and RefCallback", async () => {
  const source = `#![allow(non_snake_case)]
use react::webapi::{HTMLDivElement, HTMLElement};
use react::{IntoRef, JSX, RefCallback, RefObject, jsx};
pub fn Panel<M>(target: impl IntoRef<&'static HTMLDivElement, M>) -> JSX::Element {
    jsx! { <div ref={target} /> }
}
pub fn ByObject(target: RefObject<Option<&'static HTMLDivElement>>) -> JSX::Element {
    Panel(target)
}
pub fn ByCallback(target: RefCallback<&'static HTMLElement>) -> JSX::Element {
    jsx! { <section ref={target} /> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("return <div ref={target} />;");
  expect(jsx).toContain("return <section ref={target} />;");
  writeFileSync(join(dir, "lib.rs"), source.replace("RefObject<Option<&'static HTMLDivElement>>", "RefObject<Option<&'static HTMLElement>>"));
  const wrong = Bun.spawnSync(args, { cwd: dir });
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a ref of `&'static react::webapi::HTMLDivElement`")], wrong.stderr.toString()).toEqual([false, true]);
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

// A component's named props with the rest from a base, `{..Default::default()}`,
// as Rust's struct update has them: what a binding of a JS component with
// many optional props, `next/image`'s, is given (ADR 0192). `{...base}` with
// named props stays an error, as JSX's spread would override them.
test("JSX gives a component its named props and the rest from a base", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
#[derive(Default)]
pub struct Opts { pub title: Option<&'static str>, pub width: Option<u32> }
pub fn Card(o: Opts) -> JSX::Element { jsx! { <section title={o.title.unwrap_or("none")}>{o.width.unwrap_or(0)}</section> } }
// A JSX::Element's default is React's empty node, undefined, as next/link's
// children are given, or not.
#[derive(Default)]
pub struct Framed { pub title: Option<&'static str>, pub children: JSX::Element }
pub fn Frame(f: Framed) -> JSX::Element { jsx! { <div title={f.title.unwrap_or("bare")}>{f.children}</div> } }
// A child that might do something, which captured would be held in a variable.
fn kid() -> &'static str { "kid" }
pub fn App() -> JSX::Element {
    jsx! {
        <>
            <Card title={Some("named")} {..Default::default()} />
            <Card width={Some(3)} {..Opts { title: Some("base"), width: Some(9) }} />
            <Frame title={Some("t")} {..Default::default()}><b>{kid()}</b></Frame>
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

// A component's own flattened props passed on with a field of them set, as
// react.dev's Link gives `ExternalLink` its classes beside `{...props}`:
// `props={AnchorHTMLAttributes { html: HTMLAttributes { class_name, ..props.html
// }, ..props }}`. A flattened field read whole as a base is the object its
// parent is, `{...props}` (ADR 0213).
test("JSX passes on flattened props with a field of them set", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::{AnchorHTMLAttributes, HTMLAttributes};
use react::{JSX, ReactNode, jsx};
pub struct ExternalProps<'a, C: ReactNode> {
    pub href: Option<&'a str>,
    pub children: C,
    #[rust_js::flatten]
    pub props: AnchorHTMLAttributes<'a>,
}
pub fn External<C: ReactNode>(ExternalProps { href, children, props }: ExternalProps<C>) -> JSX::Element {
    jsx! { <a href={href} rel="noopener" {...props}>{children}</a> }
}
pub struct LinkProps<'a, C: ReactNode> {
    pub href: &'a str,
    pub children: C,
    #[rust_js::flatten]
    pub props: AnchorHTMLAttributes<'a>,
}
pub fn Link<C: ReactNode>(LinkProps { href, children, props }: LinkProps<C>) -> JSX::Element {
    jsx! {
        <External
            href={Some(href)}
            props={AnchorHTMLAttributes {
                html: HTMLAttributes { class_name: Some("link"), ..props.html },
                ..props
            }}>
            {children}
        </External>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<External href={href} {...props} className="link">');
  const { Link } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Link, { href: "/a", id: "x", target: "_self" }, "go")))
    .toBe('<a href="/a" rel="noopener" id="x" target="_self" class="link">go</a>');
  // What the update gives is after the rest, so it's what holds, as Rust's is.
  expect(renderToStaticMarkup(createElement(Link, { href: "/a", className: "theirs" }, "go")))
    .toBe('<a href="/a" rel="noopener" class="link">go</a>');
});

// A component's props are given as one flat list, `<ButtonLink href="/a"
// target={..} id={..}>`, its own fields and its flattened structs' alike,
// as JSX's caller writes them: what isn't given is left out, a field with a
// default or an `Option`, and one that's required is said (ADR 0213).
test("JSX takes a component's flattened props where they're written, none given left out", async () => {
  const flat = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
#[derive(Default)]
pub struct Html {
    pub id: Option<&'static str>,
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
pub fn ButtonLink<C: ReactNode + Default>(ButtonLinkProps { href, size, label, children, props }: ButtonLinkProps<C>) -> JSX::Element {
    let class = match size {
        Size::Md => "md",
        Size::Lg => "lg",
    };
    jsx! { <a href={href} data-size={class} aria-label={label} {...props}>{children}</a> }
}
`;
  const { dir, args } = compile(flat + `
pub fn App() -> JSX::Element {
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
pub fn Noted() -> JSX::Element {
    jsx! { <ButtonLink target={Some(note("t"))} href={note("h")}>{"N"}</ButtonLink> }
}
// Its own fields written out of its order: Rust makes \`href\` first.
fn big() -> Size {
    println!("big");
    Size::Lg
}
pub fn Sized() -> JSX::Element {
    jsx! { <ButtonLink size={big()} href={note("h")}>{"S"}</ButtonLink> }
}
// Its flattened struct given whole, a component's own passed on.
pub struct ForwardProps {
    pub href: &'static str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: Anchor,
}
pub fn Forward(ForwardProps { href, props }: ForwardProps) -> JSX::Element {
    jsx! { <ButtonLink href={href} props={props}>{"F"}</ButtonLink> }
}
// A variable is read as it is, beside props that do something, each read
// first, as they're written out of the struct's order.
pub fn Kept(h: &'static str) -> JSX::Element {
    jsx! { <ButtonLink target={Some(note("t"))} label={Some(note("l"))} href={h}>{"K"}</ButtonLink> }
}
// Its flattened struct made here, given whole, beside props that do
// something: each of its fields is read in its place.
pub fn Made() -> JSX::Element {
    jsx! { <ButtonLink label={Some(note("l"))} props={Anchor { target: Some(note("t")), html: Html { id: Some(note("i")), ..Default::default() }, ..Default::default() }} href={note("h")}>{"M"}</ButtonLink> }
}
// Children that do something, before its flattened struct in its fields'
// order: what's after them is read first, the struct's fields each in place.
pub fn Told(t: Option<&'static str>) -> JSX::Element {
    jsx! { <ButtonLink href="/t" target={t}>{note("c")}</ButtonLink> }
}
// Its flattened struct first, no children: Rust makes \`target\` first.
pub struct LinkyProps {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: Anchor,
    pub label: &'static str,
}
pub fn Linky(LinkyProps { anchor, label }: LinkyProps) -> JSX::Element {
    jsx! { <a title={label} {...anchor} /> }
}
pub fn Linked() -> JSX::Element {
    jsx! { <Linky label={note("l")} target={Some(note("t"))} /> }
}
// A prop made before one made by statements is read first, named as its
// field is.
pub fn Looped() -> JSX::Element {
    jsx! {
        <ButtonLink
            href={note("h")}
            label={Some({
                let mut n = 0;
                while n < 2 {
                    n += 1;
                }
                if n == 2 { "two" } else { "other" }
            })}>
            {"L"}
        </ButtonLink>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<ButtonLink target="_blank" href="/a" id="x" size="lg">');
  expect(jsx).toContain('<ButtonLink href="/b" className="c">');
  const { App, Noted, Linked, Sized, Forward } = await import(join(dir, "lib.jsx"));
  expect(jsx).toContain("<ButtonLink href={href} {...props}>");
  expect(jsx).toContain('export function Looped() {\n  const href = note("h");\n  let n = 0;');
  // A variable read as it is; a flattened struct made here, each field in
  // its place.
  expect(jsx).toContain('<ButtonLink target={note("t")} label={label} href={h}>');
  expect(jsx).toContain('<ButtonLink label={note("l")} target={note("t")} id={note("i")} href={href}>');
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Forward, { href: "/f", target: "_t", id: "i" }))).toBe('<a href="/f" data-size="md" target="_t" id="i">F</a>');
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
  expect(logged).toEqual(["h", "t", "t", "l", "h", "big"]);
  expect(jsx).toMatch(/<ButtonLink size=\{\w+\} href=\{\w+\}>/);
  expect(jsx).toContain("<Linky label={label} target={target} />");
  // MIR's first prop read in place, made after `label` as Rust makes it.
  expect(jsx).toContain('<ButtonLink target={note("t")} label={label} href={h}>');
  // Each made first is named as its field is, as a person names it.
  // Of these, only `href`, made before the rest, is named.
  expect(jsx).toContain('<ButtonLink label={note("l")} target={note("t")} id={note("i")} href={href}>');
  expect(jsx).toContain('<ButtonLink href="/t" target={t}>');
  // MIR reads `target` in place, made after `href`.
  expect(jsx).toMatch(/<ButtonLink target=\{note\("t"\)\} href=\{\w+\}>/);
  expect(renderToStaticMarkup(App())).toBe('<a href="/a" data-size="lg" target="_blank" id="x">A</a><a href="/b" data-size="md" class="c">B</a>');
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  // A required prop not given, and a name nothing has.
  refused(flat + `pub fn Missing() -> JSX::Element {\n    jsx! { <ButtonLink id={Some("x")}>{"A"}</ButtonLink> }\n}\n`, "missing prop \`href\`");
  refused(flat + `pub fn Both(props: Anchor) -> JSX::Element {\n    jsx! { <ButtonLink href="/a" props={props} target={Some("_t")}>{"A"}</ButtonLink> }\n}\n`, "not both");
  refused(flat + `pub fn Unknown() -> JSX::Element {\n    jsx! { <ButtonLink href="/a" colour={Some("red")}>{"A"}</ButtonLink> }\n}\n`, "colour");
});

// A props field's default, `#[rust_js::default]`, its type's, or
// `#[rust_js::default = "_self"]`, is where JS takes it, its destructuring's:
// `{ size = "md" }`, as React's `type = "primary"` is (ADR 0212).
test("JSX gives a props field's default where the props are taken apart", async () => {
  const chip = `#![allow(non_snake_case)]
use react::{JSX, jsx};
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
pub fn Chip(ChipProps { label, size, target }: ChipProps) -> JSX::Element {
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
  // A defaulted field read of props not taken apart where they're given
  // would have no default.
  const whole = compile(chip + "pub fn Whole(props: ChipProps) -> JSX::Element {\n    jsx! { <b>{props.target}</b> }\n}\n");
  const failed = Bun.spawnSync(whole.args, { cwd: whole.dir });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("taken apart where they're given")]);
  // A default that's made, not written, is said: a literal is JS's.
  const made = compile(chip + "pub struct MapProps {\n    #[cfg_attr(rust_js, rust_js::default)]\n    pub seen: std::collections::HashMap<u32, u32>,\n}\npub fn Seen(MapProps { seen }: MapProps) -> JSX::Element {\n    jsx! { <b>{seen.len()}</b> }\n}\n");
  const unmade = Bun.spawnSync(made.args, { cwd: made.dir });
  expect([unmade.exitCode === 0, unmade.stderr.toString()]).toEqual([false, expect.stringContaining("isn't a literal")]);
});

// A default the field says, of its own type: `= true` of a `bool`, as
// react.dev's Heading takes `isPageAnchor = true`, and `= 3` of a number.
test("JSX gives a props field the literal default it says, of its type", async () => {
  const anchor = `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct AnchorProps {
    pub label: &'static str,
    #[cfg_attr(rust_js, rust_js::default = true)]
    pub is_page_anchor: bool,
    #[cfg_attr(rust_js, rust_js::default = 3)]
    pub level: u32,
}
pub fn Anchor(AnchorProps { label, is_page_anchor, level }: AnchorProps) -> JSX::Element {
    jsx! { <h2 title={label} data-level={level}>{is_page_anchor.then_some("#")}</h2> }
}
`;
  const { dir, args } = compile(anchor);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function Anchor({ label, isPageAnchor = true, level = 3 }) {");
  const { Anchor } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Anchor, { label: "x" }))).toBe('<h2 title="x" data-level="3">#</h2>');
  expect(renderToStaticMarkup(createElement(Anchor, { label: "y", isPageAnchor: false, level: 4 }))).toBe('<h2 title="y" data-level="4"></h2>');
  // Taken whole, they're passed on as given, `<Anchor {...props} />`, as
  // react.dev's SandpackClient passes its props to SandpackRoot, and a field
  // with no default is read; the one taker that applies a default is the
  // component they're passed to. A field with one read of them would have
  // none where a JS caller left it out, so that's refused.
  const whole = compile(anchor + `pub fn Wrapper(props: AnchorProps) -> JSX::Element {
    let label = props.label;
    jsx! { <section title={label}><Anchor {..props} /></section> }
}
`);
  run(whole.args);
  const wholeJsx = readFileSync(join(whole.dir, "lib.jsx"), "utf8");
  expect(wholeJsx).toContain("export function Wrapper(props) {");
  expect(wholeJsx).toContain("<Anchor {...props} />");
  const { Wrapper } = await import(join(whole.dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(Wrapper, { label: "z" }))).toBe('<section title="z"><h2 title="z" data-level="3">#</h2></section>');
  const reads = compile(anchor + "pub fn Reads(props: AnchorProps) -> JSX::Element {\n    jsx! { <b>{props.level}</b> }\n}\n");
  const readFailed = Bun.spawnSync(reads.args, { cwd: reads.dir });
  expect([readFailed.exitCode === 0, readFailed.stderr.toString()]).toEqual([false, expect.stringContaining("props with a default taken whole")]);
  // One that isn't of the field's type is said.
  const wrong = compile(anchor.replace("rust_js::default = 3", 'rust_js::default = "3"'));
  const failed = Bun.spawnSync(wrong.args, { cwd: wrong.dir });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("isn't of the field's type")]);
});

// A props struct's flattened field, `#[rust_js::flatten]`, holds a struct
// whose fields are the component's own props, as TypeScript's
// `AnchorProps & ButtonLinkProps` has them: `...anchor` where they're
// taken apart, and in JSX each one given, an attribute (ADR 0204).
test("JSX gives a flattened struct's fields as a component's own props", async () => {
  const flattened = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
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
pub fn ButtonLink<C: ReactNode>(ButtonLinkProps { size, children, anchor }: ButtonLinkProps<C>) -> JSX::Element {
    let class = if size == Some("lg") { "big" } else { "small" };
    let target = anchor.target.unwrap_or("_self");
    jsx! { <a className={class} target={target} {...anchor}>{children}</a> }
}
`;
  const { dir, args } = compile(flattened + `
pub fn App() -> JSX::Element {
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
  // Made outside JSX, the props are one object too, a default's as well
  // (ADR 0204).
  const made = compile(flattened + "pub fn made() -> JSX::Element { ButtonLink(ButtonLinkProps { size: None, children: (), anchor: Anchor::default() }) }\npub fn defaulted() -> ButtonLinkProps<()> { ButtonLinkProps::default() }\n");
  run(made.args);
  const madeJs = readFileSync(join(made.dir, "lib.jsx"), "utf8");
  expect(madeJs).toContain("return ButtonLink({});");
  expect(madeJs).toContain("export function defaulted() {\n  return {};\n}");
  // What JS's props can't be is an error: the field read whole, two
  // rests, and a name both have.
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  refused(flattened + "pub fn read(p: ButtonLinkProps<()>) -> Option<&'static str> { let a = p.anchor; a.href }\n", "flattened props");
  refused(flattened + "pub fn matched(p: ButtonLinkProps<()>) -> Option<&'static str> { match p { ButtonLinkProps { anchor, .. } => anchor.href } }\n", "flattened props taken apart here");
  refused(flattened.replace("pub children: C,", "pub children: C,\n    pub rest: react::Rest,").replace("{ size, children, anchor }", "{ size, children, anchor, .. }"), "one rest");
});

// Children that do something stay in place before a flattened struct none
// of whose fields is given, made of constants, which they can't change, as
// react.dev's `<UL>{items.map(..)}</UL>` has them.
test("children stay in place before a flattened struct of nothing given", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::HTMLAttributes;
use react::webapi::HTMLUListElement;
use react::{JSX, ReactNode, jsx};
pub struct ElementProps<'a, C: ReactNode, T> {
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: HTMLAttributes<'a, T>,
}
pub fn UL<C: ReactNode>(p: ElementProps<C, HTMLUListElement>) -> JSX::Element {
    jsx! { <ul className="list" {...p} /> }
}
pub fn List(names: &[&'static str]) -> JSX::Element {
    jsx! { <UL>{names.iter().map(|name| jsx! { <li key={*name}>{*name}</li> }).collect::<Vec<_>>()}</UL> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx.includes("const children")).toBe(false);
  const { List } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(List(["a", "b"]))).toBe('<ul class="list"><li>a</li><li>b</li></ul>');
});

// A struct of two flattened fields is made as JS spreads two objects,
// `{ ...base, ...over }`, the second's keys over the first's, as react.dev's
// SandpackRoot gives Sandpack `{...template, ...files}`. JS's object has
// their keys mixed, so it's neither taken apart nor read through one.
test("a struct of two flattened fields is made as two spreads", async () => {
  const both = `#![allow(non_snake_case)]
#[derive(Default)]
pub struct Base {
    pub a: Option<u32>,
    pub b: Option<u32>,
}
#[derive(Default)]
pub struct Over {
    pub b: Option<u32>,
}
pub struct Both<'x> {
    pub size: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: &'x Base,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub over: &'x Over,
}
pub fn mixed(base: &'static Base, over: &'static Over) -> Both<'static> {
    Both { size: 3, base, over }
}
`;
  const made = compile(both);
  run(made.args);
  const js = readFileSync(join(made.dir, "lib.js"), "utf8");
  expect(js).toContain("return { size: 3, ...base, ...over };");
  const { mixed } = await import(join(made.dir, "lib.js"));
  expect(mixed({ a: 1, b: 2 }, { b: 5 })).toEqual({ size: 3, a: 1, b: 5 });
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  // A dictionary's keys spread as well, react.dev's `files`.
  const dictionary = compile(`pub struct Base {
    pub a: Option<u32>,
}
pub struct Over<'x> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: &'x Base,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub extra: &'x js::Dict<u32>,
}
pub fn over(base: &'static Base, extra: &'static js::Dict<u32>) -> Over<'static> {
    Over { base, extra }
}
`);
  run([...dictionary.args, "--extern", `js=${join(target, "libjs.rmeta")}`]);
  expect(readFileSync(join(dictionary.dir, "lib.js"), "utf8")).toContain("return { ...base, ...extra };");
  const { over } = await import(join(dictionary.dir, "lib.js"));
  expect(over({ a: 1 }, { a: 2, z: 3 })).toEqual({ a: 2, z: 3 });
  refused(both + "pub fn read(p: &Both) -> Option<u32> { p.base.a }\n", "more than one rest through one of them");
  refused(both + "pub fn taken(Both { size, base, over }: Both) -> u32 { size + base.a.unwrap_or(0) + over.b.unwrap_or(0) }\n", "more than one rest taken apart");
});

// Flattened structs chain, as TypeScript's interfaces extend one another,
// and a name its props have too is theirs, as TypeScript's `Omit` has it:
// what's taken apart besides holds none of it. A flattened field is read
// through, `props.html.title` is `props.title`, never whole (ADR 0205).
test("JSX gives a chain of flattened structs as props, a name the props have their own", async () => {
  const chained = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
#[derive(Default)]
pub struct Html {
    pub id: Option<&'static str>,
    pub title: Option<&'static str>,
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
    pub class_name: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: Anchor,
}
pub fn ButtonLink<C: ReactNode>(ButtonLinkProps { href, class_name, children, props }: ButtonLinkProps<C>) -> JSX::Element {
    let title = props.html.title.unwrap_or("none");
    jsx! { <a href={href} className={class_name.unwrap_or("x")} data-title={title} {...props}>{children}</a> }
}
// What it leaves, .., the rest does not hold.
pub fn Plain<C: ReactNode>(ButtonLinkProps { href, props, .. }: ButtonLinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} {...props} /> }
}
`;
  const { dir, args } = compile(chained + `
pub fn App() -> JSX::Element {
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
  refused(chained + `pub fn Both() -> JSX::Element {
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
use react::{JSX, jsx};
#[derive(Clone, Copy)]
pub struct TallyProps { pub n: u32, pub label: &'static str, pub children: u32 }
pub fn Tally(TallyProps { n, label, children }: TallyProps) -> JSX::Element {
    jsx! { <i title={label}>{n}{"/"}{children}</i> }
}
fn bump(t: &mut TallyProps) -> u32 {
    t.n += 1;
    5
}
pub fn App() -> JSX::Element {
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
use react::{JSX, ReactNode, jsx};
#[derive(Default)]
pub struct FrameProps<C> { pub title: Option<&'static str>, pub children: C, pub tags: Vec<&'static str>, pub id: Option<&'static str> }
pub fn Frame<C: ReactNode>(FrameProps { title, children, tags, id }: FrameProps<C>) -> JSX::Element {
    jsx! { <section title={title.unwrap_or("bare")} id={id} data-tags={tags.join(" ")}>{children}</section> }
}
pub struct CardProps<C> { pub title: &'static str, pub children: C }
pub fn Card<C: ReactNode + Default>(CardProps { title, children }: CardProps<C>) -> JSX::Element {
    jsx! { <Frame tags={vec!["card"]} title={Some(title)} {..Default::default()}>{children}</Frame> }
}
pub fn App() -> JSX::Element {
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
use react::{JSX, ReactNode, jsx};
pub struct P<C> { pub children: C }
pub fn Echo<C: ReactNode + Default>(P { children }: P<C>) -> JSX::Element {
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
use react::{JSX, jsx};
pub enum Size { S, Md }
pub struct P { pub class_name: Option<&'static str>, pub size: Option<Size>, pub title: Option<&'static str> }
// Of a value whose fields may change, \`seen\` a \`Cell\`: what's before a child
// whose statement only reads, \`const t = q.title\`, stays where it is.
pub struct Q { pub class_name: Option<&'static str>, pub title: Option<&'static str>, pub seen: std::cell::Cell<bool> }
pub fn Labeled(q: Q) -> JSX::Element {
    jsx! { <svg className={q.class_name}>{q.title.map(|t| jsx! { <title>{t}</title> })}</svg> }
}
pub fn Badge(p: P) -> JSX::Element {
    let class_name = p.class_name.unwrap_or("badge");
    jsx! {
        <svg
            className={class_name}
            id={class_name}
            width={if matches!(p.size, Some(Size::S)) { "12px" } else { "20px" }}
            height={if matches!(p.size, Some(Size::S)) { "12px" } else { "20px" }}
            viewBox="0 0 20 20"
            role={p.title}
        >
            {p.title.map(|title| jsx! { <title>{title}</title> })}
            <g fill="none"><path d="M0 0" /></g>
            {{
                let mut letters = 0;
                for _ in class_name.chars() {
                    letters += 1;
                }
                letters
            }}
        </svg>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // Its last child's statements come first, so what's before is read before
  // them, but a `const` already isn't copied.
  expect(jsx.match(/const (\w+)\$\d+ = \1;/)).toBe(null);
  expect(jsx).toContain("className={className}");
  // A field nothing changes reads the same after the child's statements.
  expect(jsx).toContain("role={p.title}");
  expect(jsx).toContain("<svg className={q.className}>");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Badge({ className: "c", size: "S", title: "t" }))).toBe('<svg class="c" id="c" width="12px" height="12px" viewBox="0 0 20 20" role="t"><title>t</title><g fill="none"><path d="M0 0"></path></g>1</svg>');
});

// A node told apart by what it is, `react::kind_of`: text, or anything else
// a node is, as react.dev's Heading labels its link `typeof children ===
// "string"` (ADR 0214).
test("JSX tells text children from any other node", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, ReactNodeKind, jsx, kind_of};
pub struct LabeledProps<C> {
    pub children: C,
}
pub fn Labeled<C: ReactNode>(LabeledProps { children }: LabeledProps<C>) -> JSX::Element {
    let mut label = "Link for this heading".to_string();
    if let ReactNodeKind::Text(text) = kind_of(&children) {
        label = format!("Link for {text}");
    }
    jsx! { <h2 title={label.as_str()}>{children}</h2> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('if (typeof children === "string") {');
  const { Labeled } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Labeled, { children: "Recap" }))).toBe('<h2 title="Link for Recap">Recap</h2>');
  expect(renderToStaticMarkup(createElement(Labeled, { children: ["A ", createElement("code", null, "b")] }))).toBe('<h2 title="Link for this heading">A <code>b</code></h2>');
});

// Children of a type parameter left out are React's empty node, none, as
// next/document's `<Head />` has them: of any props, whose type doesn't say.
test("JSX leaves out children of a type parameter", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
#[derive(Default)]
pub struct PanelProps<C> {
    pub children: C,
}
pub fn Panel<C: ReactNode>(PanelProps { children }: PanelProps<C>) -> JSX::Element {
    jsx! { <section>{children}</section> }
}
pub struct CardProps<'a, C> {
    pub title: &'a str,
    pub children: C,
}
pub fn Card<C: ReactNode>(CardProps { title, children }: CardProps<C>) -> JSX::Element {
    jsx! { <article title={title}>{children}</article> }
}
pub fn Page() -> JSX::Element {
    jsx! { <main><Panel /><Card title="t" /><Panel>{"x"}</Panel></main> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<Panel />');
  expect(jsx).toContain('<Card title="t" />');
  const { Page } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Page())).toBe('<main><section></section><article title="t"></article><section>x</section></main>');
});

// JSX among a `vec!`'s items is JSX, as among an array's: `vec!`'s tokens
// are a macro's, which rustc expands after JSX is read (ADR 0259).
test("JSX among a vec!'s items is elements", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn List() -> JSX::Element {
    let items = vec![jsx! { <li key="a">{"a"}</li> }, jsx! { <li key="b">{"b"}</li> }];
    jsx! { <ul>{items}</ul> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('const items = [<li key="a">a</li>, <li key="b">b</li>];');
  const { List } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(List())).toBe("<ul><li>a</li><li>b</li></ul>");
});

// A component looks inside its children as React's own API does, as
// react.dev's MDX components do: `Children.toArray`, an element told apart by
// `isValidElement`, its `type`, `props` and `key`, and `cloneElement`; and
// a list of them and more, `Child::element` of an element.
test("JSX components look inside their children with React's Children", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::Kind;
use react::children::{self, Child};
use react::{JSX, ReactNode, ReactNodeKind, clone_element, jsx, kind_of};
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub fn Kinds<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let mut kinds = Vec::new();
    for child in children::to_array(&children) {
        kinds.push(match child {
            Child::Text(text) => format!("text {text}"),
            Child::Number(n) => format!("number {n}"),
            Child::Element(element) => match js::classify(element.r#type()) {
                Kind::String(tag) => format!("<{tag}> {}", element.key().unwrap_or("")),
                _ => "component".to_string(),
            },
            Child::Other(_) => "other".to_string(),
        });
    }
    jsx! { <p>{kinds.join(", ")}</p> }
}
pub struct Linked<'a> {
    #[rust_js::name = "data-linked"]
    pub linked: &'a str,
}
pub fn Only<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    match kind_of(&children) {
        ReactNodeKind::Element(element) => clone_element(element, Linked { linked: "yes" }).element(),
        ReactNodeKind::Text(text) => jsx! { <i>{text}</i> },
        ReactNodeKind::List(_) | ReactNodeKind::Other(_) => jsx! { <b>{"other"}</b> },
    }
}
// Its children and one more, as react.dev's SandpackWithHTMLOutput adds
// files, of a node behind a reference too.
pub fn More(ListProps { children }: ListProps<&'static dyn ReactNode>) -> JSX::Element {
    let end = jsx! { <hr key="end" /> };
    let all = [children::to_array(children), vec![Child::element(end)]].concat();
    jsx! { <div>{all}</div> }
}
pub fn Id<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let id = match kind_of(&children) {
        ReactNodeKind::Element(element) => js::get(element.props(), "id").map(js::classify),
        _ => None,
    };
    jsx! { <p>{if let Some(Kind::String(id)) = id { id } else { "none" }}</p> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("Children.toArray(children)");
  expect(jsx).toContain("isValidElement(");
  expect(jsx).toContain("cloneElement(");
  expect(jsx).toContain("const all = [...Children.toArray(children), end];");
  const { Kinds, Only, Id, More } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const Component = () => null;
  expect(renderToStaticMarkup(createElement(Kinds, null, "a", 1, createElement("img", { key: "k" }), createElement(Component), null, true)))
    .toBe("<p>text a, number 1, &lt;img&gt; .$k, component</p>");
  expect(renderToStaticMarkup(createElement(Only, null, createElement("a", { href: "/" }, "x")))).toBe('<a href="/" data-linked="yes">x</a>');
  expect(renderToStaticMarkup(createElement(Only, null, "t"))).toBe("<i>t</i>");
  expect(renderToStaticMarkup(createElement(Only, null, "t", "u"))).toBe("<b>other</b>");
  expect(renderToStaticMarkup(createElement(Id, null, createElement("h4", { id: "deep" })))).toBe("<p>deep</p>");
  expect(renderToStaticMarkup(createElement(Id, null, "t"))).toBe("<p>none</p>");
  expect(renderToStaticMarkup(createElement(More, null, "a", createElement("b", null, "c")))).toBe("<div>a<b>c</b><hr/></div>");
});

// As react.dev's TerminalBlock reads its message, text or an element's text:
// a let chain of what's told apart by a node's kind is the `if`s a person
// writes, the node itself tested, no label to reach its `else`.
test("JSX components read their children's text in a let chain as a person writes it", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::{Kind, classify};
use react::{JSX, ReactNode, ReactNodeKind, jsx, kind_of};
pub struct BlockProps<C: ReactNode> {
    pub children: C,
}
pub fn Block<C: ReactNode>(BlockProps { children }: BlockProps<C>) -> JSX::Element {
    let message: String;
    if let ReactNodeKind::Text(text) = kind_of(&children) {
        message = text.to_string();
    } else if let ReactNodeKind::Element(element) = kind_of(&children)
        && let Some(inner) = js::get(element.props(), "children")
        && let Kind::String(text) = classify(inner)
    {
        message = text.to_string();
    } else {
        panic!("Expected plain text.");
    }
    jsx! { <pre>{message}</pre> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // The chain as it's written, one `else` for what fails it.
  expect(jsx).toContain(`  if (typeof children === "string") {
    message = children;
  } else if (isValidElement(children) && typeof children.props.children === "string") {
    const inner = children.props.children;
    message = inner;
  } else {
    throw new Error("Expected plain text.");
  }`);
  const { Block } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Block, null, "npm i"))).toBe("<pre>npm i</pre>");
  expect(renderToStaticMarkup(createElement(Block, null, createElement("code", null, "npm i")))).toBe("<pre>npm i</pre>");
  expect(() => renderToStaticMarkup(createElement(Block, null, createElement("code", null, createElement("b"))))).toThrow("Expected plain text.");
  expect(() => renderToStaticMarkup(createElement(Block, null, "a", "b"))).toThrow("Expected plain text.");
});

// An element is @types/react's `ReactElement<P>`: of props of a type, read
// by their fields, where a child's are taken as a component's,
// `ReactElement::<ItemProps>::unchecked_from(e)`, as TypeScript's
// `isValidElement<P>` takes them, unchecked. A `bigint`, an `i64`, and a
// promise of a node are children too, as React 19's `ReactNode` has them.
test("JSX elements have props of a type, and bigint and promise children", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::children::{self, Child};
use react::{JSX, ReactElement, ReactNode, jsx};
pub struct ItemProps<'a> {
    pub title: &'a str,
}
pub fn Item(ItemProps { title }: ItemProps) -> JSX::Element {
    jsx! { <li>{title}</li> }
}
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub fn Titles<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let titles: Vec<&str> = children::to_array(&children)
        .into_iter()
        .filter_map(|child| match child {
            Child::Element(element) => Some(ReactElement::<ItemProps>::unchecked_from(element).props().title),
            _ => None,
        })
        .collect();
    let big: i64 = 1 << 40;
    jsx! { <p>{titles.join(", ")}<b>{big}</b></p> }
}
pub fn Later(text: js::Promise<String>) -> JSX::Element {
    jsx! { <p>{text}</p> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("return child.props.title;");
  expect(jsx).toContain("return <p>{text}</p>;");
  const { Titles, Item } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Titles, null, createElement(Item, { title: "a" }), "x", createElement(Item, { title: "b" }))))
    .toBe("<p>a, b<b>1099511627776</b></p>");
});

// `cloneElement` makes an element, a child as any other, as react.dev's Link
// clones each `inlineCode` among its children and keeps the rest.
test("JSX components clone some of their children and keep the rest", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::children::{self, Child};
use react::{JSX, ReactNode, clone_element, jsx};
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub struct Marked {
    #[rust_js::name = "data-marked"]
    pub marked: &'static str,
}
pub fn Marks<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let marked: Vec<Child> = children::to_array(&children)
        .into_iter()
        .map(|child| match child {
            Child::Element(element) => Child::Element(clone_element(element, Marked { marked: "yes" })),
            child => child,
        })
        .collect();
    jsx! { <p>{marked}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('cloneElement(child, { "data-marked": "yes" })');
  const { Marks } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Marks, null, "a", createElement("b", null, "b"), 1)))
    .toBe('<p>a<b data-marked="yes">b</b>1</p>');
});

// Children that are a list, as JSX gives several, are that list, which a
// component takes apart, as react.dev's ExpandableExample renders its first
// child's `children` as its title and the rest as its body.
test("JSX components take apart a list of children", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, ReactNodeKind, jsx, kind_of};
pub struct ListProps<C: ReactNode> {
    pub children: C,
}
pub fn Titled<C: ReactNode>(ListProps { children }: ListProps<C>) -> JSX::Element {
    let ReactNodeKind::List(items) = kind_of(&children) else {
        panic!("expected a title and a body");
    };
    let Some(ReactNodeKind::Element(first)) = items.first() else {
        panic!("expected a title first");
    };
    jsx! {
        <section>
            <h4>{js::get(first.props(), "children")}</h4>
            <div>{&items[1..]}</div>
        </section>
    }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("Array.isArray(");
  const { Titled } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Titled, null, createElement("h4", null, "Title"), "a", createElement("b", null, "b"))))
    .toBe("<section><h4>Title</h4><div>a<b>b</b></div></section>");
  expect(() => renderToStaticMarkup(createElement(Titled, null, "only"))).toThrow("expected a title and a body");
});

// A DOM element's tag as a value, a `react::Tag`, is the tag JSX names by a
// capitalized parameter or `let` of its function, as react.dev's Heading
// renders `<Comp>` of `{ as: Comp = "div" }`.
test("JSX renders a tag that's a value, named by a capitalized local", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, Rest, jsx};
#[derive(Clone, Copy, Default)]
pub enum As {
    #[rust_js::name = "h1"]
    H1,
    #[rust_js::name = "h2"]
    H2,
    #[default]
    #[rust_js::name = "div"]
    Div,
}
impl react::Tag for As {}
pub struct HeadingProps<C> {
    #[rust_js::default]
    pub r#as: As,
    pub id: Option<&'static str>,
    pub children: C,
    pub rest: Rest,
}
pub fn Heading<C: ReactNode>(HeadingProps { r#as: Comp, id, children, rest }: HeadingProps<C>) -> JSX::Element {
    jsx! { <Comp id={id} {...rest} className="mdx-heading">{children}</Comp> }
}
pub fn Title() -> JSX::Element {
    jsx! { <Heading r#as={As::H2} id={Some("intro")}>{"Intro"}</Heading> }
}
// A \`let\` too.
pub fn Plain(level: u8) -> JSX::Element {
    let Comp = if level == 1 { As::H1 } else { As::Div };
    jsx! { <Comp>{"x"}</Comp> }
}
// Given its props whole, as a component is.
pub fn Bare(HeadingProps { r#as: Comp, rest, .. }: HeadingProps<()>) -> JSX::Element {
    jsx! { <Comp {...rest} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('export function Heading({ as: Comp = "div", id, children, ...rest }) {');
  expect(jsx).toContain("<Comp {...rest} />");
  expect(jsx).toContain('<Comp id={id} {...rest} className="mdx-heading">');
  expect(jsx).toContain('const Comp = level === 1 ? "h1" : "div";');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Title())).toBe('<h2 id="intro" class="mdx-heading">Intro</h2>');
  expect(renderToStaticMarkup(result.Heading({ children: "D", title: "t" }))).toBe('<div title="t" class="mdx-heading">D</div>');
  expect([renderToStaticMarkup(result.Plain(1)), renderToStaticMarkup(result.Plain(2))]).toEqual(["<h1>x</h1>", "<div>x</div>"]);
  expect(renderToStaticMarkup(result.Bare({ as: "h1", title: "t" }))).toBe('<h1 title="t"></h1>');
});

// A field of a Rust value nothing writes again reads the same after a later
// child's statement too, and a conditional of it, so they stay in place, as
// a person writes them: `width={p.size === "S" ? "12px" : "20px"}`. A JS
// object's getter, `n.textContent`, and a field through a `&mut` or a
// `Cell` may change, so each is read before what writes it (ADR 0218).
test("JSX keeps a field of what never changes in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::cell::Cell;
use react::webapi::Node;
use react::{JSX, jsx};
pub enum Size { S, Md }
pub struct P { pub size: Option<Size>, pub title: Option<&'static str> }
pub fn Badge(p: P) -> JSX::Element {
    jsx! {
        <svg width={if matches!(p.size, Some(Size::S)) { "12px" } else { "20px" }}>
            {p.title.map(|t| jsx! { <title>{t}</title> })}
            {{
                let mut n = 0;
                for _ in "ab".chars() {
                    n += 1;
                }
                n
            }}
        </svg>
    }
}
pub fn Live(n: &'static Node) -> JSX::Element {
    jsx! { <div><p>{n.text_content()}</p>{{ n.set_text_content("x"); 1 }}</div> }
}
pub struct N { pub n: i32 }
pub fn Edited(e: &mut N) -> JSX::Element {
    jsx! { <p>{e.n}{{ e.n = 5; 1 }}</p> }
}
pub fn Counter() -> JSX::Element {
    let count = Cell::new(0);
    jsx! { <div><p>{count.get()}</p>{{ count.set(5); 1 }}</div> }
}
pub struct Tally { pub count: Cell<i32> }
pub fn Tallied() -> JSX::Element {
    let tally = Tally { count: Cell::new(0) };
    jsx! { <div><p>{tally.count.get()}</p>{{ tally.count.set(5); 1 }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  const badge = jsx.slice(jsx.indexOf("export function Badge"), jsx.indexOf("export function Live"));
  expect([badge.includes("const width"), badge.includes("= p.title")]).toEqual([false, false]);
  expect(badge).toContain('<svg width={p.size === "S" ? "12px" : "20px"}>');
  expect(jsx).toMatch(/= n\.textContent;\n  n\.textContent = "x";/);
  expect(jsx).toMatch(/= count;\n  count = 5;/);
  const { Badge, Counter, Edited, Tallied } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Badge({ size: "S", title: "t" }))).toBe('<svg width="12px"><title>t</title>2</svg>');
  expect(renderToStaticMarkup(Counter())).toBe("<div><p>0</p>1</div>");
  expect(renderToStaticMarkup(Edited({ n: 0 }))).toBe("<p>01</p>");
  expect(renderToStaticMarkup(Tallied())).toBe("<div><p>0</p>1</div>");
});

// A comparison of what reads the same reads the same after a later child's
// statement too, so it stays in place, as react.dev's PageHeading tests its
// version (ADR 0218): before the statements of a later child, a loop.
test("JSX keeps a comparison of what reads the same in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub enum Version { Canary, Rc }
pub fn Heading(title: &'static str, version: Option<Version>, done: bool, status: Option<&'static str>) -> JSX::Element {
    jsx! {
        <h1>
            {title}
            {matches!(version, Some(Version::Canary)).then(|| jsx! { <i>{"canary"}</i> })}
            {(!matches!(version, Some(Version::Rc))).then(|| jsx! { <b>{"stable"}</b> })}
            {(!done).then(|| jsx! { <s>{"todo"}</s> })}
            {(done && status.is_some()).then(|| jsx! { <u>{"both"}</u> })}
            {(done || status.is_none()).then(|| jsx! { <q>{"either"}</q> })}
            {status.filter(|s| !s.is_empty()).map(|s| jsx! { <em>{s}</em> })}
            {{
                let mut letters = 0;
                for _ in title.chars() {
                    letters += 1;
                }
                letters
            }}
        </h1>
    }
}
// One of a variable a later child writes is read before the write.
pub fn Count() -> JSX::Element {
    let mut n = 0;
    jsx! { <p>{(n == 0).then(|| jsx! { <i>{"zero"}</i> })}{{ n += 1; n }}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  const heading = jsx.slice(0, jsx.indexOf("export function Count"));
  expect(heading).not.toContain("const condition");
  expect(jsx).toContain("const condition = n === 0;\n  n = (n + 1) | 0;");
  expect(jsx).toContain('{version === "Canary" && <i>canary</i>}');
  expect(jsx).toContain('{version !== "Rc" && <b>stable</b>}');
  expect(jsx).toContain("{!done && <s>todo</s>}");
  expect(jsx).toContain("{done && status != null && <u>both</u>}");
  expect(jsx).toContain("{(done || status == null) && <q>either</q>}");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Heading("T", "Canary", false, ""))).toBe("<h1>T<i>canary</i><b>stable</b><s>todo</s>1</h1>");
  expect(renderToStaticMarkup(result.Heading("T", "Rc", true, "new"))).toBe("<h1>T<u>both</u><q>either</q><em>new</em>1</h1>");
  expect(renderToStaticMarkup(result.Count())).toBe("<p><i>zero</i>1</p>");
});

// A component's props its struct doesn't name, a `react::Rest`: `...rest` of
// its destructured props, spread onto an element, as react.dev's
// ExternalLink takes its callers' `aria-label` (ADR 0195). A Rust caller's
// default gives none, and no `rest` prop.
test("JSX takes the props a component's struct doesn't name as ...rest", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, Rest, jsx};
#[derive(Default)]
pub struct LinkProps<C> { pub href: &'static str, pub rest: Rest, pub target: Option<&'static str>, pub children: C }
pub fn ExternalLink<C: ReactNode>(LinkProps { href, target, children, rest }: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} target={target.unwrap_or("_blank")} rel="noopener" {...rest}>{children}</a> }
}
pub fn Home() -> JSX::Element {
    jsx! { <ExternalLink href="/home" {..Default::default()}>{"Home"}</ExternalLink> }
}
#[derive(Default)]
pub struct SocialProps { pub name: &'static str, pub rest: Rest }
pub fn Social(SocialProps { name, rest }: SocialProps) -> JSX::Element {
    jsx! { <ExternalLink href="/social" rest={rest} {..Default::default()}>{name}</ExternalLink> }
}
// Props holding what has a destructor are taken apart where they're given
// too, { label, ...rest }, what's bound owned by the function.
pub struct Loud(pub &'static str);
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub struct BadgeProps { pub label: Loud, pub rest: Rest }
pub fn Badge(BadgeProps { label, rest }: BadgeProps) -> JSX::Element {
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
use react::{JSX, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> JSX::Element {
    jsx! { <a href={p.href} {...p.rest} /> }
}
`);
  const failed = Bun.spawnSync(read.args, { cwd: read.dir });
  expect([failed.exitCode === 0, failed.stderr.toString().includes("take it apart from them")]).toEqual([false, true]);
  // A `let` takes it apart as JS's destructuring does (ADR 0291).
  const later = compile(`#![allow(non_snake_case)]
use react::{JSX, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> JSX::Element {
    let P { href, rest } = p;
    jsx! { <a href={href} {...rest} /> }
}
`);
  const destructured = Bun.spawnSync(later.args, { cwd: later.dir });
  expect([destructured.exitCode, readFileSync(join(later.dir, "lib.jsx"), "utf8").includes("const { href, ...rest } = p;")]).toEqual([0, true]);
  // Nor is one taken apart anywhere else.
  const matched = compile(`#![allow(non_snake_case)]
use react::{JSX, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> JSX::Element {
    match p {
        P { href, rest } => jsx! { <a href={href} {...rest} /> },
    }
}
`);
  const refused = Bun.spawnSync(matched.args, { cwd: matched.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("a `Rest` of props taken apart here")]).toEqual([false, true]);
});

// A struct spread through a reference is the struct's spread, as react.dev's
// Image spreads `rest`, then reads its `src` after.
test("JSX spreads a struct through a reference as the struct", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};
pub struct ImageProps<'a> {
    pub alt: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub rest: ImgHTMLAttributes<'a>,
}
pub fn Image(ImageProps { alt, rest }: ImageProps) -> JSX::Element {
    jsx! { <img alt={alt} className="wide" {...&rest} src={rest.src.map(|src| format!("/base{src}"))} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain(`<img alt={alt} className="wide" {...rest} src=`);
  const { Image } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Image, { alt: "A", src: "/a.png", title: "T" }))).toBe('<link rel="preload" as="image" href="/base/a.png"/><img alt="A" class="wide" src="/base/a.png" title="T"/>');
});

// A `{" "}` the formatter makes text pairs with nothing, and a tag it breaks
// onto lines keeps its mappings in order, as react.dev's LanguageList and
// YouTubeIframe have them.
test("a tag broken onto lines keeps its mappings in order", () => {
  const source = `#![allow(non_snake_case)]
use react::attributes::IframeHTMLAttributes;
use react::{JSX, jsx};
fn first_more() -> &'static str {
    "z"
}
pub fn Item(first: JSX::Element, second: JSX::Element) -> JSX::Element {
    jsx! {
        <li>
            {first}{" "}{"— "}{second}
            <b className="bold">{"y"}</b>
            <i className="it">{first_more()}</i>
        </li>
    }
}
pub fn YouTubeIframe(props: IframeHTMLAttributes<'static>) -> JSX::Element {
    jsx! {
        <div className="relative h-0 overflow-hidden pt-[56.25%]">
            <iframe
                className="absolute inset-0 w-full h-full"
                frameBorder="0"
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                allowFullScreen={true}
                title="YouTube video player"
                {...props}
            />
        </div>
    }
}
pub fn After() -> JSX::Element {
    jsx! { <hr /> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8").split("\n");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  const segments = decodeMappings(map.mappings);
  for (const text of ["<li>", "<b", "<i", "allow=", "title=", "<hr"]) {
    const line = js.findIndex(l => l.includes(text));
    expect(lookup(segments, line, js[line].indexOf(text))?.srcLine, text).toBe(source.split("\n").findIndex(l => l.includes(text)));
  }
});

// Making an element runs nothing of the crate's, so a field's `RefCell`
// borrowed for a prop is borrowed only for a moment, and is its value (ADR
// 0362), as react.dev's InlineToc passes `items={root.children}`.
test("a field's RefCell borrowed for a prop is its value", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::cell::RefCell;
use std::rc::Rc;
use react::{JSX, jsx};
pub struct Node {
    pub name: &'static str,
    pub children: RefCell<Vec<Rc<Node>>>,
}
pub struct ItemsProps<'a> {
    pub items: &'a [Rc<Node>],
}
pub fn Items(ItemsProps { items }: ItemsProps) -> JSX::Element {
    jsx! {
        <ul>
            {items.iter().map(|node| jsx! {
                <li key={node.name}>
                    {node.name}
                    {if node.children.borrow().len() > 0 { Some(jsx! { <Items items={&node.children.borrow()} /> }) } else { None }}
                </li>
            }).collect::<Vec<_>>()}
        </ul>
    }
}
pub fn Tree() -> JSX::Element {
    let root = Rc::new(Node { name: "root", children: RefCell::new(vec![]) });
    let leaf = Rc::new(Node { name: "leaf", children: RefCell::new(vec![]) });
    root.children.borrow_mut().push(leaf);
    jsx! { <Items items={&root.children.borrow()} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes("$borrow"), jsx.includes("<Items items={node.children} />"), jsx.includes("<Items items={root.children} />")]).toEqual([false, true, true]);
  const { Tree } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(Tree))).toBe("<ul><li>leaf</li></ul>");
});

// Props whose children are declared `&'a dyn ReactNode`, any node, as
// TypeScript's `children: ReactNode`, or `Option` of one, as its
// `children?: ReactNode`, take JSX's children by reference, so
// one function, and a table's pointer to it, takes children of any type;
// a constant of a component's function type is a component JSX names, its
// props built by their companion where the component is, as react.dev's
// 404 page takes `{Intro, p: P}` from MDXComponents.
test("children declared any node are taken by reference, and a constant names a component", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub mod missing;
pub struct CardProps<'a> {
    pub title: Option<&'a str>,
    pub children: &'a dyn ReactNode,
}
pub fn Card(CardProps { title, children }: CardProps) -> JSX::Element {
    jsx! { <section title={title}>{children}</section> }
}
pub struct NoteProps<'a> {
    pub children: Option<&'a dyn ReactNode>,
}
pub fn Note(NoteProps { children }: NoteProps) -> JSX::Element {
    jsx! { <aside>{children}</aside> }
}
pub struct Table {
    pub card: for<'a> fn(CardProps<'a>) -> JSX::Element,
    pub note: for<'a> fn(NoteProps<'a>) -> JSX::Element,
}
pub const TABLE: Table = Table { card: Card, note: Note };
pub const SPARE: Table = Table { card: Card, note: Note };
const Picked: for<'a> fn(CardProps<'a>) -> JSX::Element = TABLE.card;
pub fn Page(name: &str) -> JSX::Element {
    jsx! {
        <Picked title={Some("t")}>
            <Card>{"Hello, "}{name}</Card>
            <Card><b>{"bold"}</b></Card>
            <Card />
            <Note>{"a"}{name}</Note>
            <Note />
            <Note><b />{if name.len() > 1 { Some(jsx! { <i /> }) } else { None }}</Note>
        </Picked>
    }
}
`, {
    "missing.rs": `use react::{JSX, jsx};
use super::{CardProps, NoteProps, SPARE, TABLE};
const Shown: for<'a> fn(CardProps<'a>) -> JSX::Element = TABLE.card;
const Aside: for<'a> fn(NoteProps<'a>) -> JSX::Element = TABLE.note;
const Spare: for<'a> fn(NoteProps<'a>) -> JSX::Element = SPARE.note;
pub fn Missing() -> JSX::Element {
    jsx! { <Shown title={None}><Aside>{"gone"}{"!"}</Aside><Spare /></Shown> }
}
`,
  });
  run(args);
  const missing = readFileSync(join(dir, "missing.jsx"), "utf8");
  // Two read of one object side by side are its destructuring, as JS writes it.
  expect(missing).toContain("const { card: Shown, note: Aside } = TABLE;\nconst Spare = SPARE.note;");
  const { Missing } = await import(join(dir, "missing.jsx"));
  expect(renderToStaticMarkup(Missing())).toBe("<section><aside>gone!</aside><aside></aside></section>");
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const Picked = TABLE.card;");
  expect(jsx).toContain('<Picked title="t">');
  expect(jsx).toContain("<Card>Hello, {name}</Card>");
  expect(jsx).toContain("<Note>a{name}</Note>\n      <Note />");
  expect(jsx).toContain("{$byteLen(name) > 1 && <i />}");
  const { Page } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Page("Ada"))).toBe('<section title="t"><section>Hello, Ada</section><section><b>bold</b></section><section></section><aside>aAda</aside><aside></aside><aside><b></b><i></i></aside></section>');
});

// A constant's thread-local component is the module's variable, which every
// use shares, as react.dev's MDXComponents table holds `pre: CodeBlock`, a
// memo.
test("a constant holds a thread-local by reference as its variable", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::thread::LocalKey;
use react::{JSX, MemoExoticComponent, jsx};
mod memoized;
use memoized::{Memoized, Props};
pub fn Plain(Props { name }: Props) -> JSX::Element {
    jsx! { <i>{name}</i> }
}
pub struct Table {
    pub plain: fn(Props) -> JSX::Element,
    pub memo: &'static LocalKey<MemoExoticComponent<Props>>,
}
pub const TABLE: Table = Table { plain: Plain, memo: &Memoized };
pub static MEMO: &LocalKey<MemoExoticComponent<Props>> = &Memoized;
`, {
    "memoized.rs": `use react::{JSX, MemoExoticComponent, jsx, memo};
pub struct Props {
    pub name: &'static str,
}
fn Bold(Props { name }: Props) -> JSX::Element {
    jsx! { <b>{name}</b> }
}
thread_local! {
    pub static Memoized: MemoExoticComponent<Props> = memo(Bold, None);
}
`,
  });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export const TABLE = { plain: Plain, memo: Memoized };");
  // A static of one by reference is the one variable too, which every use
  // shares, so the static may be.
  expect(jsx).toContain("export const MEMO = Memoized;");
  const { TABLE } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(TABLE.memo, { name: "m" }))).toBe("<b>m</b>");
});

// An async handler is the async function itself, as react.dev's
// `async function handleCopy()` is, where `spawn` in a closure would call
// one in place, `(async () => { .. })()`.
test("JSX takes an async event handler as the async function it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::Promise;
use react::webapi::HTMLButtonElement;
use react::{JSX, event, jsx, use_state};
#[cfg_attr(rust_js, rust_js::link_name = "Promise.resolve")]
#[allow(unused_variables)]
fn resolved() -> Promise<()> {
    unreachable!()
}
pub struct ButtonProps {
    pub on_click: Box<dyn Fn(&event::MouseEvent<HTMLButtonElement>)>,
}
pub fn Button(ButtonProps { on_click }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={on_click}>{"Copy"}</button> }
}
pub fn Copy() -> JSX::Element {
    let (copied, set_copied) = use_state(false);
    let handle_copy = event::MouseEvent::spawn(async move |_| {
        resolved().await;
        set_copied.set(true);
    });
    jsx! { <div><Button onClick={handle_copy} />{if *copied { "Copied" } else { "Copy" }}</div> }
}
pub fn Parsed(text: String) -> JSX::Element {
    let (copied, set_copied) = use_state(false);
    let handle_copy = event::MouseEvent::spawn(async move |_| {
        let Ok(n) = text.parse::<u32>() else {
            return;
        };
        resolved().await;
        set_copied.set(n == 1);
    });
    jsx! { <div><Button onClick={handle_copy} />{if *copied { "Copied" } else { "Copy" }}</div> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const handleCopy = async () => {\n    await Promise.resolve();\n    setCopied(true);\n  };");
  // One that may return early, as react.dev's does, takes no event either.
  expect(jsx.match(/const handleCopy = async \(\) => \{/g)?.length).toBe(2);
  expect(jsx).not.toContain("(async");
  const { Button } = await import(join(dir, "lib.jsx"));
  let clicked = false;
  const handler = async () => { clicked = true; };
  const button = Button({ onClick: handler });
  button.props.onClick();
  expect(clicked).toBe(true);
});

// A component's optional handler passed on to an element, as react.dev's
// Button does its onClick, is `onClick={onClick}` (ADR 0198): React
// ignores what a handler returns, and no handler does what one that calls
// nothing does.
test("JSX passes an optional event handler on as it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, event, jsx};
pub struct ButtonProps<C> { pub children: C, pub on_click: Option<Box<dyn Fn(&event::MouseEvent)>> }
pub fn Button<C: ReactNode>(ButtonProps { children, on_click }: ButtonProps<C>) -> JSX::Element {
    jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e.upcast()) }}>{children}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes("<button onClick={onClick}>"), jsx.includes("=>")]).toEqual([true, false]);
  const { Button } = await import(join(dir, "lib.jsx"));
  const handler = () => {};
  expect([Button({ children: "b", onClick: handler }).props.onClick === handler, Button({ children: "b" }).props.onClick]).toEqual([true, undefined]);
});

// A handler that only calls a function with nothing, `move |_| on_clear()`,
// is the function, `onClick={onClear}`, as react.dev's ClearButton has it
// (ADR 0198): React gives it the event, which a Rust function has no
// parameter for.
test("JSX passes a handler that calls a function with nothing as the function", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct ClearProps { pub on_clear: Box<dyn Fn()> }
pub fn Clear(ClearProps { on_clear }: ClearProps) -> JSX::Element {
    jsx! { <button onClick={move |_| on_clear()}>{"Clear"}</button> }
}
#[cfg_attr(rust_js, rust_js::link_name = "alert")]
fn alert() {
    unreachable!()
}
pub fn Alert() -> JSX::Element {
    jsx! { <button onClick={move |_| alert()}>{"Alert"}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<button onClick={onClear}>Clear</button>");
  // A binding's JS function keeps its arrow: `alert(event)` would show it.
  expect(jsx).toContain("<button onClick={() => alert()}>Alert</button>");
  const { Clear } = await import(join(dir, "lib.jsx"));
  const handler = () => {};
  expect(Clear({ onClear: handler }).props.onClick).toBe(handler);
});

// A handler prop is named as @types/react names it, `MouseEventHandler<T>`:
// `onClick?: MouseEventHandler<HTMLButtonElement>`, an `EventHandler` of
// its event, the element's, or any's by default.
test("JSX handler props are @types/react's EventHandler aliases", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::{ChangeEventHandler, EventHandler, MouseEvent, MouseEventHandler};
use react::webapi::{HTMLButtonElement, HTMLInputElement};
use react::{JSX, jsx};
pub struct FieldProps {
    pub on_click: Option<MouseEventHandler<HTMLButtonElement>>,
    pub on_change: ChangeEventHandler<HTMLInputElement>,
    pub on_any: MouseEventHandler,
    pub on_same: EventHandler<MouseEvent<HTMLButtonElement>>,
}
pub fn Field(FieldProps { on_click, on_change, on_any, on_same }: FieldProps) -> JSX::Element {
    let _ = (on_any, on_same);
    jsx! {
        <>
            <button onClick={move |e| if let Some(f) = &on_click { f(e) }}>{"Go"}</button>
            <input onChange={on_change} />
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<button onClick={onClick}>Go</button>");
  expect(jsx).toContain("<input onChange={onChange} />");
});

// A form's submit is @types/react's `SubmitEvent`, whose `submitter` is the
// button that sent it, and its native event the DOM's `SubmitEvent`; a
// `FormEvent` and an `InvalidEvent` are its names of a `SyntheticEvent`.
test("JSX onSubmit gets a SubmitEvent, of its submitter", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::{FormEvent, FormEventHandler, InvalidEvent, SubmitEventHandler};
use react::webapi::HTMLFormElement;
use react::{JSX, jsx};
pub struct FormProps {
    pub on_send: fn(String),
    pub on_reset: FormEventHandler<HTMLFormElement>,
    pub on_submitted: Option<SubmitEventHandler<HTMLFormElement>>,
}
pub fn checked(e: &InvalidEvent, f: &FormEvent) -> bool {
    e.is_default_prevented() || f.is_default_prevented()
}
pub fn Form(FormProps { on_send, on_reset, on_submitted }: FormProps) -> JSX::Element {
    let _ = on_submitted;
    jsx! {
        <form
            onSubmit={move |e| {
                e.prevent_default();
                let by = e.submitter().map(|b| b.title()).unwrap_or_default();
                on_send(format!("{by} {}", e.native_event().submitter().is_some()));
            }}
            onReset={on_reset} />
    }
}
`);
  run(args);
  const { Form } = await import(join(dir, "lib.jsx"));
  const sent: string[] = [];
  const { props } = Form({ onSend: (s: string) => sent.push(s), onReset: () => {} });
  const button = { title: "Send" };
  props.onSubmit({ preventDefault() {}, submitter: button, nativeEvent: { submitter: button } });
  props.onSubmit({ preventDefault() {}, submitter: null, nativeEvent: { submitter: null } });
  expect(sent).toEqual(["Send true", " false"]);
});

// `onInput` is @types/react's `InputEventHandler`, of an `InputEvent`, as
// `onBeforeInput` is; `onChange` alone is a `ChangeEvent`, of `value()`.
test("JSX onInput gets an InputEvent, as @types/react types it", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::InputEventHandler;
use react::webapi::HTMLInputElement;
use react::{JSX, jsx};
pub struct FieldProps {
    pub on_read: fn(String),
    pub on_typed: Option<InputEventHandler<HTMLInputElement>>,
}
pub fn Field(FieldProps { on_read, on_typed }: FieldProps) -> JSX::Element {
    let _ = on_typed;
    jsx! {
        <input
            onInput={move |e| on_read(format!(
                "{} {} {}",
                e.data().unwrap_or_default(),
                e.native_event().input_type(),
                e.current_target().value(),
            ))}
            onChange={move |e| on_read(e.value())} />
    }
}
`);
  run(args);
  const { Field } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Field({ onRead: (s: string) => read.push(s) });
  props.onInput({ data: "a", nativeEvent: { inputType: "insertText" }, currentTarget: { value: "ba" } });
  props.onChange({ target: { value: "ba" } });
  expect(read).toEqual(["a insertText ba", "ba"]);
});

// An event's `native_event` is the DOM's event it wraps, typed as
// @types/react types `nativeEvent`: a click's a `MouseEvent`, a pointer's a
// `PointerEvent`, a wheel's a `WheelEvent`, so its own fields are read.
test("JSX native_event is the DOM event of its kind", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct PadProps { pub on_read: fn(String) }
pub fn Pad(PadProps { on_read }: PadProps) -> JSX::Element {
    jsx! {
        <div
            onClick={move |e| on_read(e.native_event().client_x().to_string())}
            onPointerDown={move |e| on_read(e.native_event().pointer_type())}
            onWheel={move |e| on_read(e.native_event().delta_y().to_string())} />
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("e.nativeEvent.clientX");
  const { Pad } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Pad({ onRead: (s: string) => read.push(s) });
  props.onClick({ nativeEvent: { clientX: 3 } });
  props.onPointerDown({ nativeEvent: { pointerType: "pen" } });
  props.onWheel({ nativeEvent: { deltaY: 1.5 } });
  expect(read).toEqual(["3", "pen", "1.5"]);
});

// As react.dev's Button's `style={style}` of an optional prop: `None` none.
test("JSX passes an optional style on as it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct BoxProps { pub style: Option<CSSProperties> }
pub fn Panel(BoxProps { style }: BoxProps) -> JSX::Element {
    jsx! { <div style={style} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<div style={style} />");
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(Panel({ style: { color: "red" } })), renderToStaticMarkup(Panel({}))]).toEqual(['<div style="color:red"></div>', "<div></div>"]);
});

// A CSS property takes what @types/react's `CSSProperties`, csstype's
// `Properties<string | number>`, types it as: `width` a length, a number in
// pixels or text, `opacity` and `z_index` a number, `color` text only.
test("JSX styles take the values csstype types them as", async () => {
  const { dir, args } = compile(`use react::{CSSProperties, JSX, jsx};
pub fn Panel() -> JSX::Element {
    jsx! { <div style={CSSProperties::new().width(300).opacity(0.5).z_index(2).color("red").margin_top("1em")} /> }
}
`);
  run(args);
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Panel())).toBe('<div style="width:300px;opacity:0.5;z-index:2;color:red;margin-top:1em"></div>');
  const refused = compile(`use react::{CSSProperties, JSX, jsx};\npub fn Panel() -> JSX::Element {\n    jsx! { <div style={CSSProperties::new().color(3)} /> }\n}\n`);
  const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
  expect([failed.exitCode === 0, failed.stderr.toString().includes("is not text")], failed.stderr.toString()).toEqual([false, true]);
});

// A style spreads another over its own, as react.dev's console box writes
// `style={{width, height, ...customStyles}}`; `None` spreads nothing.
test("JSX styles spread another style over their own", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct BoxProps<'a> { pub width: &'a str, pub custom_styles: Option<CSSProperties> }
pub fn Panel(BoxProps { width, custom_styles }: BoxProps) -> JSX::Element {
    jsx! { <div style={CSSProperties::new().width(width).spread(custom_styles)} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<div style={{ width, ...customStyles }} />");
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(Panel({ width: "6px", customStyles: { width: "7px", color: "red" } })), renderToStaticMarkup(Panel({ width: "6px" }))])
    .toEqual(['<div style="width:7px;color:red"></div>', '<div style="width:6px"></div>']);
});

// A state's setter is a function, as @types/react's `Dispatch` is: given
// where a prop wants one, as react.dev's home page gives `setSearchText`,
// and `setHoverLine` to a memo'd CodeBlock, whose props are `'static`.
test("a state's setter is given as the function it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub struct InputProps<'a> { pub value: &'a str, pub on_change: &'a dyn Fn(String), pub on_clear: Option<&'static dyn Fn(String)> }
pub fn Input(InputProps { value, on_change, on_clear }: InputProps) -> JSX::Element {
    let _ = (on_change, on_clear);
    jsx! { <b>{value}</b> }
}
pub fn Search() -> JSX::Element {
    let (text, set_text) = use_state(String::from("a"));
    jsx! { <Input value={text.as_str()} onChange={&**set_text} onClear={Some(&**set_text)} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<Input value={text} onChange={setText} onClear={setText} />");
  const { Search } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(Search))).toBe("<b>a</b>");
});

// A style kept, in state say, is given by reference as it is, as react.dev's
// home page keeps `{ width, height, transform }`s for its hover overlays.
test("JSX styles are given by reference", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct OverlaysProps<'a> { pub styles: &'a [CSSProperties], pub first: Option<&'a CSSProperties> }
pub fn Overlays(OverlaysProps { styles, first }: OverlaysProps) -> JSX::Element {
    jsx! {
        <>
            <i style={first} />
            {styles.iter().map(|styles| jsx! { <b style={styles} /> }).collect::<Vec<_>>()}
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<b style={styles} />");
  expect(jsx).toContain("<i style={first} />");
  const { Overlays } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Overlays({ styles: [{ width: "1px" }, { width: "2px" }], first: { color: "red" } })))
    .toBe('<i style="color:red"></i><b style="width:1px"></b><b style="width:2px"></b>');
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
use react::{JSX, ReactNode, jsx};
pub struct CardProps<C> { pub title: &'static str, pub children: C }
pub fn Card<C: ReactNode>(CardProps { title, children }: CardProps<C>) -> JSX::Element {
    let heading = format!("{title}!");
    jsx! { <section title={heading}>{children}</section> }
}
`;
  const built = library(card);
  run(built.args);
  const jsx = readFileSync(join(built.dir, "lib.jsx"), "utf8");
  expect([jsx.includes("export function Card({ title, children }) {"), jsx.includes("drop")]).toEqual([true, false]);
  const given = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct HolderProps<T> { pub value: T }
pub fn Holder<T>(HolderProps { value }: HolderProps<T>) -> JSX::Element {
    let _kept = value;
    jsx! { <i /> }
}
pub struct Loud;
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub fn App() -> JSX::Element {
    jsx! { <Holder value={Loud} /> }
}
`);
  const refused = Bun.spawnSync(given.args, { cwd: given.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("component `Holder`'s `T` a type with a destructor")]).toEqual([false, true]);
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

// Adjacent text children are each a text node, as react.dev's Challenge
// has them, `{order} of{' '}{total}`: the first in braces, so JSX doesn't
// read the two as one text, which React would render as one node. Where the
// later has a space at an edge and isn't blank, and the earlier has none, the
// later is, as react.dev's 404 page writes `If this is a mistake{', '}`.
test("adjacent text children stay apart", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub fn Count(order: u32, total: u32) -> JSX::Element {
    jsx! { <p>{"Challenge"}{" "}{order}{" of"}{" "}{total}</p> }
}

pub fn Mistake(link: JSX::Element) -> JSX::Element {
    jsx! { <p>{"If this is a mistake"}{", "}{link}{", "}{"and we will fix it"}</p> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('{"Challenge"} {order}');
  expect(code).toContain('{" of"} {total}');
  expect(code).toContain('If this is a mistake{", "}');
  expect(code).toContain('{", "}and we will fix it');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToString(result.Count(1, 4))).toBe("<p>Challenge<!-- --> <!-- -->1<!-- --> of<!-- --> <!-- -->4</p>");
  expect(renderToString(result.Mistake(createElement("a")))).toBe("<p>If this is a mistake<!-- -->, <a></a>, <!-- -->and we will fix it</p>");
});

// A node of any type is `Box<dyn ReactNode>`, as @types/react's `ReactNode`
// is one type: react.dev's ErrorDecoder makes a list of text and links. It's
// the node itself, a string or an element, as React takes it.
test("a dyn ReactNode is the node itself", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};

fn urlify(text: &str) -> Vec<Box<dyn ReactNode>> {
    text.split('|')
        .enumerate()
        .map(|(i, part)| -> Box<dyn ReactNode> {
            if i % 2 == 1 {
                return Box::new(jsx! { <a key={i} href={part}>{part}</a> });
            }
            Box::new(part.to_string())
        })
        .collect()
}

fn Shown(node: &dyn ReactNode) -> JSX::Element {
    jsx! { <i>{node}</i> }
}

pub fn Message(text: &str) -> JSX::Element {
    jsx! { <b>{urlify(text)}{Shown(&"!")}</b> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain("return part;");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Message("see |https://x.dev| now"))).toBe('<b>see <a href="https://x.dev">https://x.dev</a> now<i>!</i></b>');
});

// A component generic in a callback, `F: Fn() + Copy`, takes its props apart
// as any other, as react.dev's Challenge does: a copy of a JS function is
// the function (ADR 0246).
test("props of an Fn type parameter are destructured", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct NextProps<F: Fn() + Copy + 'static> {
    pub total: usize,
    pub next: F,
}

pub fn Next<F: Fn() + Copy + 'static>(NextProps { total, next }: NextProps<F>) -> JSX::Element {
    jsx! { <button onClick={move |_| next()}>{total}</button> }
}

// A copy of a struct holding one is a spread, the function one of the rest.
#[derive(Clone, Copy)]
pub struct Counted<F: Fn() + Copy> {
    pub total: usize,
    pub next: F,
}

pub fn bumped<F: Fn() + Copy>(counted: Counted<F>) -> usize {
    let mut copy = counted;
    copy.total += 1;
    copy.total + counted.total
}

// A type changed in place, as react.dev's ChallengeContents is, which a
// type parameter could be but for its bound.
pub struct Tally {
    pub n: u32,
}

pub fn counted() -> u32 {
    let mut tally = Tally { n: 1 };
    tally.n += 1;
    tally.n
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function Next({ total, next }) {");
  expect(jsx).toContain("const copy = { ...counted };");
  const { Next, bumped } = await import(join(dir, "lib.jsx"));
  expect(bumped({ total: 1, next: () => {} })).toBe(3);
  let clicked = 0;
  const tree = Next({ total: 3, next: () => clicked++ });
  tree.props.onClick();
  expect([renderToStaticMarkup(tree), clicked]).toEqual(["<button>3</button>", 1]);
});

// A key may be none, as @types/react's `key?: Key | null`: react.dev's
// CodeDiagram gives each child's own, `key={child.key}`.
test("an element's key may be an Option", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub fn Keyed(keys: Vec<Option<&'static str>>) -> JSX::Element {
    jsx! { <ul>{keys.iter().map(|key| jsx! { <li key={*key}>{key.unwrap_or("none")}</li> }).collect::<Vec<_>>()}</ul> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<li key={key}>");
  const { Keyed } = await import(join(dir, "lib.jsx"));
  const tree = Keyed(["a", undefined]);
  expect(tree.props.children.map((li: any) => li.key)).toEqual(["a", null]);
  expect(renderToStaticMarkup(tree)).toBe("<ul><li>a</li><li>none</li></ul>");
});

// A key may be any JS number, as @types/react's `Key` has `number`:
// react.dev's home page keys its progress bar by a restart count.
test("an element's key may be a JS number", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub fn Restarted(restart_id: f64) -> JSX::Element {
    jsx! { <div key={restart_id}>{"bar"}</div> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<div key={restartId}>");
  const { Restarted } = await import(join(dir, "lib.jsx"));
  expect(Restarted(1.5).key).toBe("1.5");
});

// Props given whole through a reference, the rest of a child's own, are
// spread, then the props named, as react.dev's CodeDiagram gives
// CodeBlock a <pre>'s props, `{...child.props} noMargin={true}`: what the
// props hold is shared, and a key their type doesn't name passes too.
test("a component's props updated from a reference are spread", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

// Not \`Copy\`: the reference is captured, as the struct can't be moved out
// of it.
pub struct PanelProps<'a> {
    pub title: Option<&'a str>,
    pub wide: Option<bool>,
}

pub fn Panel(PanelProps { title, wide }: PanelProps) -> JSX::Element {
    jsx! { <p title={title} data-wide={wide.unwrap_or(false)}></p> }
}

pub fn Again(props: &'static PanelProps<'static>) -> JSX::Element {
    jsx! { <Panel wide={Some(true)} {..*props} /> }
}

// All of them so, as react.dev's home page gives each BlogCard a post,
// \`<BlogCard {...recentPosts[0]} />\`.
pub fn Alone(posts: &'static [PanelProps<'static>]) -> JSX::Element {
    jsx! { <Panel {..posts[0]} /> }
}

// Read through a reference no variable names, as react.dev's home page
// writes it, unchecked.
pub fn First(posts: &'static [PanelProps<'static>]) -> JSX::Element {
    jsx! { <Panel {..*unsafe { posts.get_unchecked(0) }} /> }
}

fn described(title: Option<&str>) -> String {
    format!("{}-panel", title.unwrap_or(""))
}

// Keyed first by a key that might do something, its props captured in
// order: still the props spread.
pub fn Keyed(props: &'static PanelProps<'static>) -> JSX::Element {
    jsx! { <Panel key={described(props.title)} wide={Some(true)} {..*props} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Panel {...props} wide />");
  expect(jsx).toContain("but the index is 0`);\n  }\n  return <Panel {...posts[0]} />;");
  expect(jsx).toContain("export function First(posts) {\n  return <Panel {...posts[0]} />;\n}");
  // The key MIR holds is named as it's given, `key`.
  expect(jsx).toContain("<Panel key={key} {...props} wide />");
  const { Again, Alone } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Alone([{ title: "a", wide: true }]))).toBe('<p title="a" data-wide="true"></p>');
  const tree = Again({ title: "t", wide: false, extra: 1 });
  expect(tree.props.extra).toBe(1);
  expect(renderToStaticMarkup(tree)).toBe('<p title="t" data-wide="true"></p>');
});

// A captured prop that reads only variables that never change, `Some(name)`,
// is read where it's given, not made a `const` of its own.
test("a captured prop that reads what never changes is read in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct PanelProps<'a> {
    pub title: Option<&'a str>,
    pub wide: Option<bool>,
}

pub fn Panel(PanelProps { title, wide }: PanelProps) -> JSX::Element {
    jsx! { <p title={title} data-wide={wide.unwrap_or(false)}></p> }
}

fn described(title: &str) -> String {
    format!("{title}-panel")
}

pub fn Named(name: &'static str) -> JSX::Element {
    jsx! { <Panel key={described(name)} title={Some(name)} wide={Some(true)} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // MIR's key read in place, what's after it doing nothing (ADR 0364).
  expect(jsx).toContain("<Panel key={described(name)} title={name} wide />");
  const { Named } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Named("n"))).toBe('<p title="n" data-wide="true"></p>');
});

// A component keyed first, its props captured in written order in a
// one-armed match, is the element itself, with no variable to hold it,
// as react.dev's PackageImport keys each CodeBlock, `key={i}`.
test("a keyed component captured in order is its element, held in nothing", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

#[derive(Clone, Copy)]
pub struct ItemProps<'a> {
    pub label: &'a str,
}

pub fn Item(ItemProps { label }: ItemProps) -> JSX::Element {
    jsx! { <li>{label}</li> }
}

pub fn List(items: &'static [ItemProps<'static>]) -> JSX::Element {
    let shown: Vec<Option<JSX::Element>> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if !item.label.is_empty() {
                let props = item;
                Some(jsx! { <Item key={i.to_string()} {..*props} /> })
            } else {
                None
            }
        })
        .collect();
    jsx! { <ul>{shown}</ul> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // MIR's key where it's written, read in place, before the spread.
  expect(jsx).toContain("return <Item key={String(i)} {...props} />;");
  expect(jsx).not.toContain("let tmp");
  const { List } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(List([{ label: "a" }, { label: "" }, { label: "b" }]))).toBe("<ul><li>a</li><li>b</li></ul>");
});

// An element of what JS gives at run time, as react.dev's errors page
// revives its elements from JSON: its type, its key and its props each a
// `js::Unknown`, a wrapper's `Fragment` (ADR 0268).
test("an element's type, key and props may be JS values of any shape", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::{Kind, Unknown, classify};
use react::{ElementType, FRAGMENT, JSX, jsx};

pub fn revived(r#type: &'static Unknown, key: &'static Unknown, props: &'static Unknown) -> JSX::Element {
    let mut Type = ElementType::from_unknown(r#type);
    if matches!(classify(r#type), Kind::String("wrapper")) {
        Type = FRAGMENT;
    }
    jsx! { <Type key={key} {...props} /> }
}

// A type that may be none, as JSON's may, shadowed as what JSX renders:
// the variable itself (ADR 0277).
pub fn retyped(Type: Option<&'static Unknown>) -> JSX::Element {
    let Type = ElementType::from_unknown(Type);
    jsx! { <Type /> }
}

// Props that may be none, \`undefined\`, which JS spreads as nothing, as a
// JSON element may have none.
pub fn maybe(props: Option<&'static Unknown>) -> JSX::Element {
    jsx! { <b {...props} /> }
}

// An element, or React's \`Fragment\`, as a JS value of any shape, as a JSON
// reviver gives one.
pub fn kept(wrapped: bool) -> &'static Unknown {
    if wrapped { js::unknown_of(FRAGMENT) } else { js::unknown_of(jsx! { <b>{"kept"}</b> }) }
}
`);
  run([...args, "--extern", `js=${join(target, "libjs.rmeta")}`]);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('let Type = type;\n  if (type === "wrapper") {\n    Type = Fragment;\n  }\n  return <Type key={key} {...props} />;');
  const { revived } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(revived("b", "k", { children: "bold", title: "t" }))).toBe('<b title="t">bold</b>');
  expect(renderToStaticMarkup(revived("wrapper", 0, { children: "plain" }))).toBe("plain");
  expect(revived("b", 7, {}).key).toBe("7");
  expect(jsx).toContain("return <b {...props} />;");
  expect(jsx).toContain("export function retyped(Type) {\n  return <Type />;\n}");
  const { retyped } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(retyped("hr"))).toBe("<hr/>");
  const { maybe } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(maybe(undefined)), renderToStaticMarkup(maybe({ title: "t" }))]).toEqual(["<b></b>", '<b title="t"></b>']);
  const { kept } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(kept(false))).toBe("<b>kept</b>");
  expect(kept(true)).toBe((await import("react")).Fragment);
});

// A component a `thread_local!` holds, `memo`'s, typed with its props'
// `'static` form, given props that borrow what the caller made: JS frees
// nothing, so they're alive while it renders, as react.dev's Page gives Seo
// the image it makes (ADR 0265).
test("a thread_local component takes props borrowing the caller's", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, MemoExoticComponent, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
}

fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
    jsx! { <b>{text}</b> }
}

thread_local! {
    pub static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
}

pub fn greet(name: &str) -> JSX::Element {
    let text = format!("Hi {name}");
    jsx! { <Memoized text={&text} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("return <Memoized text={text} />;");
  const { greet } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(greet("Ann"))).toBe("<b>Hi Ann</b>");
});

// A thread_local's memo of a component in another module takes its props
// as that component does, an `Option` not given left out, as react.dev's
// home page gives the memo'd CodeBlock only some of its props.
test("a thread_local component of another module's props leaves out what isn't given", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
pub mod label {
    use react::{JSX, jsx};
    pub struct LabelProps<'a> {
        pub text: &'a str,
        pub title: Option<&'a str>,
    }
    pub fn Label(LabelProps { text, title }: LabelProps) -> JSX::Element {
        jsx! { <b title={title}>{text}</b> }
    }
}
pub mod memoized {
    use super::label::{Label, LabelProps};
    use react::{MemoExoticComponent, memo};
    thread_local! {
        pub static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
    }
}
use memoized::Memoized;
use react::{JSX, jsx};

pub fn greet(name: &str) -> JSX::Element {
    let text = format!("Hi {name}");
    jsx! { <Memoized text={&text} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("return <Memoized text={text} />;");
  const { greet } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(greet("Ann"))).toBe("<b>Hi Ann</b>");
});

// Props a flattened struct holds, taken apart with the rest: its fields are
// the props' own in JS, `{ title, frame }`, as written, as next/router's
// `withRouter` gives react.dev's Seo its props and the router.
test("a flattened struct's pattern takes its fields apart with the props'", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Framed<'a> {
    #[rust_js::flatten]
    pub props: CardProps<'a>,
    pub frame: &'a str,
}

pub struct CardProps<'a> {
    pub title: &'a str,
    pub width: u32,
}

pub fn Card(Framed { props: CardProps { title, .. }, frame }: Framed) -> JSX::Element {
    jsx! { <section title={title} className={frame} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function Card({ title, frame }) {");
  const { Card } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Card({ title: "t", width: 1, frame: "f" }))).toBe('<section title="t" class="f"></section>');
});

// A child whose flattened prop is made by a call, as react.dev's TopNav
// gives its `Logo` classes inside next/link's `Link`: the call is made
// where it's given, as the defaults beside it do nothing, and the link's
// own flattened props, none given, are nothing.
test("a child's flattened prop made by a call is made in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::{AnchorHTMLAttributes, SVGAttributes};
use react::{JSX, ReactNode, jsx};

#[derive(Default)]
pub struct LinkProps<'a, C> {
    pub href: &'a str,
    #[rust_js::flatten]
    pub anchor: AnchorHTMLAttributes<'a>,
    pub children: C,
}

pub fn Link<C: ReactNode>(props: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={props.href}>{props.children}</a> }
}

#[derive(Default)]
pub struct LogoProps<'a> {
    #[rust_js::flatten]
    pub props: SVGAttributes<'a>,
}

pub fn Logo(LogoProps { props }: LogoProps) -> JSX::Element {
    jsx! { <svg {...props} /> }
}

fn classes() -> String {
    "logo".to_string()
}

pub fn Brand() -> JSX::Element {
    jsx! { <Link href="/"><Logo className={Some(classes().as_str())} /></Link> }
}

// One made by statements, which the link's defaults needn't be made before.
pub fn Counted() -> JSX::Element {
    jsx! {
        <Link href="/">
            <Logo className={Some({
                let mut n = 0;
                while n < 2 {
                    n += 1;
                }
                if n == 2 { "two" } else { "other" }
            })} />
        </Link>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('return (\n    <Link href="/">\n      <Logo className={classes()} />\n    </Link>\n  );');
  expect(jsx).not.toContain("anchor");
  const { Brand, Counted } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Brand())).toBe('<a href="/"><svg class="logo"></svg></a>');
  expect(renderToStaticMarkup(Counted())).toBe('<a href="/"><svg class="two"></svg></a>');
});

// A prop named as no JS variable can be, `data-platform`, destructured by
// its name quoted, as react.dev's TopNav gives its `Kbd` one.
test("a destructured prop of a hyphenated name is quoted", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct KbdProps<'a> {
    #[rust_js::name = "data-platform"]
    pub data_platform: Option<&'a str>,
}

pub fn Kbd(KbdProps { data_platform }: KbdProps) -> JSX::Element {
    jsx! { <kbd data-platform={data_platform}>{"K"}</kbd> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('export function Kbd({ "data-platform": dataPlatform }) {');
  const { Kbd } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Kbd({ "data-platform": "mac" }))).toBe('<kbd data-platform="mac">K</kbd>');
});

// An element made once, a module's constant, rendered wherever it's read,
// as react.dev's TopNav renders its icons: an element is never changed, so
// it's copied as it is.
test("a module's constant element is rendered where it's read", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

thread_local! {
    static star: JSX::Element = jsx! { <b>{"*"}</b> };
    static half: Option<JSX::Element> = Some(jsx! { <i>{"+"}</i> });
}

pub fn Rated() -> JSX::Element {
    jsx! { <p>{star.with(|icon| *icon)}{star.with(|icon| *icon)}{half.with(|icon| *icon)}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const star = <b>*</b>;");
  expect(jsx).toContain("const half = <i>+</i>;");
  expect(jsx).toContain("{star}\n      {star}\n      {half}");
  const { Rated } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Rated())).toBe("<p><b>*</b><b>*</b><i>+</i></p>");
});

// A key or ref that does nothing, a variable or `Some` of one, is given
// where JSX puts it with nothing captured in order, as react.dev's
// SidebarLink gives next/link its `ref={ref}` before its classes, which
// `cn` makes.
test("a key that does nothing needs nothing captured", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct TagProps<'a> {
    pub label: &'a str,
}

pub fn Tag(TagProps { label }: TagProps) -> JSX::Element {
    jsx! { <i>{label}</i> }
}

fn described(label: &str) -> &'static str {
    if label.is_empty() { "none" } else { "some" }
}

pub fn Tags(labels: Vec<&'static str>) -> JSX::Element {
    jsx! { <p>{labels.into_iter().map(|label| jsx! { <Tag key={label} label={described(label)} /> }).collect::<Vec<_>>()}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Tag key={label} label={described(label)} />");
  expect(jsx).not.toContain("const match");
  const { Tags } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Tags(["a", ""]))).toBe("<p><i>some</i><i>none</i></p>");
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

// A props field's default of a struct, an object of literals, is a
// `const` named by `#[rust_js::default(NAME)]`, as react.dev's Search
// takes `searchParameters = {hitsPerPage: 30, ..}` (ADR 0212).
test("a props field's default may be a const", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Parameters {
    pub hits_per_page: u32,
    pub highlighted: &'static [&'static str],
}

const DEFAULT_PARAMETERS: Parameters = Parameters { hits_per_page: 30, highlighted: &["content"] };

pub struct ResultsProps {
    #[rust_js::default(DEFAULT_PARAMETERS)]
    pub parameters: Parameters,
}

pub fn Results(ResultsProps { parameters }: ResultsProps) -> JSX::Element {
    jsx! { <p title={parameters.highlighted.join(",")}>{parameters.hits_per_page}</p> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('export function Results({ parameters = { hitsPerPage: 30, highlighted: ["content"] } }) {');
  const { Results } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Results({}))).toBe('<p title="content">30</p>');
});

// ADR 0293: building an element runs nothing (ADR 0040), so a `Cell` a
// `let` takes apart and JSX reads first is still read at once: react.dev's
// ErrorMessage, `const {message, title} = error;`.
test("JSX reading a Cell a let took apart reads its value", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::cell::Cell;
use react::{JSX, Rest, jsx};
pub struct Failure { pub title: Cell<Option<&'static str>>, pub message: String }
pub struct ErrorMessageProps<'a> { pub error: &'a Failure, pub props: Rest }
pub fn ErrorMessage(ErrorMessageProps { error, props }: ErrorMessageProps) -> JSX::Element {
    let Failure { message, title } = error;
    jsx! {
        <div className="error" {...props}>
            <h2 className="title">{title.get().filter(|title| !title.is_empty()).unwrap_or("Error")}</h2>
            <pre>{message.as_str()}</pre>
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const { message, title } = error;");
  expect(jsx).toContain('{title || "Error"}');
  const { ErrorMessage } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(ErrorMessage, { error: { title: "", message: "m" }, id: "e" })))
    .toBe('<div class="error" id="e"><h2 class="title">Error</h2><pre>m</pre></div>');
});

// ADR 0296: a function a block makes and gives, `{ fn Label(..) { .. } Label }`,
// is JS's named function expression, as react.dev memoizes its components:
// `export const IconCanary = memo(function IconCanary(..) { .. })`. Rust
// can't name a module's function and its static alike; the block can.
test("a function a block makes and gives is a named function expression", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case, non_upper_case_globals)]
use react::{JSX, MemoExoticComponent, jsx, memo};
pub struct LabelProps<'a> {
    pub text: &'a str,
}
thread_local! {
    pub static Label: MemoExoticComponent<LabelProps<'static>> = memo({
        fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
            jsx! { <b>{text}</b> }
        }
        Label
    }, None);
}
pub struct CountProps {
    pub n: u32,
}
thread_local! {
    // A function written in its body is its own, there (ADR 0308).
    pub static Shout: MemoExoticComponent<LabelProps<'static>> = memo({
        fn Shout(LabelProps { text }: LabelProps) -> JSX::Element {
            fn loud(text: &str) -> String {
                text.to_uppercase()
            }
            jsx! { <i>{loud(text)}</i> }
        }
        Shout
    }, None);
    // One reading what's named as it is, the static, is named apart.
    pub static Count: MemoExoticComponent<CountProps> = memo({
        fn Count(CountProps { n }: CountProps) -> JSX::Element {
            jsx! { <b>{n}{(n > 0).then(|| jsx! { <Count n={n - 1} /> })}</b> }
        }
        Count
    }, None);
}
// One that calls itself is named twice: a function of the module.
pub static FACT: fn(u32) -> u32 = {
    fn fact(n: u32) -> u32 {
        if n == 0 { 1 } else { n * fact(n - 1) }
    }
    fact
};
pub fn fact_of(n: u32) -> u32 {
    FACT(n)
}
// A generic one, given its dictionaries where it's named: the module's.
pub static DOUBLE: fn(u32) -> u32 = {
    fn double<T: std::ops::Add<Output = T> + Copy>(x: T) -> T {
        x + x
    }
    double::<u32>
};
pub fn doubled(n: u32) -> u32 {
    DOUBLE(n)
}
pub fn Page() -> JSX::Element {
    jsx! { <main><Label text="hi" /><Shout text="up" /><Count n={2} /></main> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export const Label = memo(function Label({ text }) {");
  expect(jsx).not.toContain("function Label$1");
  expect(jsx).toContain("export const Shout = memo(function Shout({ text }) {");
  expect(jsx).toContain("function Shout({ text }) {\n  function loud(text) {\n    return text.toUpperCase();\n  }\n  return <i>{loud(text)}</i>;");
  expect(jsx).toContain("export const Count = memo(function Count$1({ n }) {");
  expect(jsx).toContain("\nfunction fact(n) {");
  expect(jsx).toContain("\nfunction double(x, TAdd) {");
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(lib.Page()), lib.Label.type.name, lib.factOf(3), lib.doubled(4)])
    .toEqual(["<main><b>hi</b><i>UP</i><b>2<b>1<b>0</b></b></b></main>", "Label", 6, 8]);
});

// ADR 0297: a closure a `let` names and only a hook's function is is written
// there, named, as react.dev names its effects: `useEffect(function
// createBundler() { .. }, [])`. Given elsewhere, or used twice, it's a
// `const` as before.
test("a closure a let names for a hook is the hook's named function", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_callback, use_effect, use_memo, use_state};
pub fn Ticker() -> JSX::Element {
    let (n, set_n) = use_state(0u32);
    let startTicking = move || {
        set_n.set(1);
    };
    use_effect(startTicking, Some(&()));
    let computeDouble = move || n * 2;
    let double = use_memo(computeDouble, [n]);
    let handleClick = move || set_n.set(n + 1);
    let onClick = use_callback(handleClick, [n]);
    let twice = move || n;
    let both = twice() + twice();
    // One reading what it would be named is a const too.
    let step = 1u32;
    let step = move || set_n.set(step);
    use_effect(step, Some(&[n]));
    // One used besides is a const too.
    let report = move || n;
    let reported = use_memo(report, [n]);
    let again = report();
    jsx! { <button onClick={move |_| onClick()}>{*double + both + *reported + again}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("useEffect(function startTicking() {");
  expect(jsx).toMatch(/useMemo\(\s*function computeDouble\(\) \{/);
  expect(jsx).toMatch(/useCallback\(\s*function handleClick\(\) \{/);
  expect(jsx).toContain("const twice = () => n;");
  expect(jsx).toContain("const report = () => n;");
  expect(jsx).toMatch(/const step\$1 = \(\) => \{\n\s*setN\(step\);\n\s*\};\n\s*useEffect\(step\$1, \[n\]\);/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Ticker))).toBe("<button>0</button>");
});

// ADR 0298: JSX shows an Option of an object by its truth, as react.dev's
// Preview writes `{error && <div>..</div>}` and `visible={!error}`.
test("JSX tests an Option of an object by its truth", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct Failure {
    pub message: String,
}
pub struct ShownProps<'a> {
    pub error: Option<&'a Failure>,
}
pub fn Shown(ShownProps { error }: ShownProps) -> JSX::Element {
    jsx! {
        <div hidden={error.is_none()}>
            {error.map(|error| jsx! { <b>{error.message.as_str()}</b> })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("hidden={!error}");
  expect(jsx).toContain("{error && <b>{error.message}</b>}");
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(createElement(lib.Shown, { error: { message: "m" } })), renderToStaticMarkup(createElement(lib.Shown, {}))])
    .toEqual(["<div><b>m</b></div>", '<div hidden=""></div>']);
});

// ADR 0235: a component's child shown only if a test holds is `test &&
// <el />` too, as an element's is: react.dev's NavigationBar has
// `{isMultiFile && showDropdown && (<Listbox.Options>..)}` in a Listbox.
test("a component's child shown if a test holds is test && element", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub struct BoxedProps<C> {
    pub children: C,
}
pub fn Boxed<C: ReactNode>(BoxedProps { children }: BoxedProps<C>) -> JSX::Element {
    jsx! { <div>{children}</div> }
}
pub fn Shown(a: bool, b: bool) -> JSX::Element {
    jsx! { <Boxed>{(a && b).then(|| jsx! { <b>{"x"}</b> })}</Boxed> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Boxed>{a && b && <b>x</b>}</Boxed>");
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(lib.Shown(true, true)), renderToStaticMarkup(lib.Shown(true, false))]).toEqual(["<div><b>x</b></div>", "<div></div>"]);
});

// ADR 0364: a component of the crate's is its function, a prop not given is
// left out, and an attribute is read before a child that assigns what it
// read, but one that reads what never changes, which reads alike after.
test("an attribute is read before a child that assigns what it read", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct Props { pub label: &'static str, pub note: Option<&'static str> }
pub fn Tag(p: Props) -> JSX::Element {
    jsx! { <b>{p.label}{p.note}</b> }
}
pub fn App() -> JSX::Element {
    let mut n = "one";
    let tip = "en";
    jsx! { <div title={n} lang={tip}><Tag label="x" />{{ n = "five"; n }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<Tag label="x" />');
  expect(jsx).toContain("<div title={title} lang={tip}>");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.App())).toBe('<div title="one" lang="en"><b>x</b>five</div>');
});

// ADR 0364: a component's flattened props given in part are the attributes
// given, not what `jsx!`'s `..Default::default()` and its omitted props
// leave out, and the component takes them apart as its pattern does, the
// rest its `...props`.
test("flattened props given in part are the attributes given", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
#[derive(Default)]
pub struct Html {
    pub id: Option<&'static str>,
    pub class_name: Option<&'static str>,
}
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: Html,
}
pub struct LinkProps<C> {
    pub href: &'static str,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: Anchor,
}
pub fn Link<C: ReactNode>(LinkProps { href, children, props }: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} {...props}>{children}</a> }
}
pub fn App() -> JSX::Element {
    jsx! { <Link href="/b" className={Some("c")}>{"B"}</Link> }
}
// What does something is made in Rust's order, the struct's own \`href\`
// after the flattened \`target\`, which it's written after.
fn note(text: &'static str) -> &'static str {
    println!("{text}");
    text
}
pub fn Noted() -> JSX::Element {
    jsx! { <Link target={Some(note("_blank"))} href={note("/n")}>{"N"}</Link> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("export function Link({ href, children, ...props }) {");
  expect(jsx).toContain('<Link href="/b" className="c">');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.App())).toBe('<a href="/b" class="c">B</a>');
  expect(renderToStaticMarkup(lib.Noted())).toBe('<a href="/n" target="_blank">N</a>');
});

// ADR 0364: a static's component is its static, wherever its tag is read,
// as after a prop a branch makes.
test("a static component given a prop a branch makes is its static", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, MemoExoticComponent, jsx, memo};
pub struct P {
    pub n: u32,
}
thread_local! {
    pub static Count: MemoExoticComponent<P> = memo({
        fn Count(P { n }: P) -> JSX::Element {
            jsx! { <b>{n}</b> }
        }
        Count
    }, None);
}
pub fn App(big: bool) -> JSX::Element {
    jsx! { <Count n={if big { 10 } else { 1 }} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Count n=");
  const lib = await import(join(dir, "lib.jsx"));
  expect([true, false].map((big) => renderToStaticMarkup(lib.App(big)))).toEqual(["<b>10</b>", "<b>1</b>"]);
});

// ADR 0364: what's made before a prop a branch makes stays what it is: a
// style being given its fields, and a tag a closure reads of a local it
// shares.
test("a style and a tag made before a prop's branch stay what they are", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{CSSProperties, JSX, jsx};
pub struct HProps {
    pub id: Option<&'static str>,
    pub class_name: Option<&'static str>,
}
pub fn H2(HProps { id, class_name }: HProps) -> JSX::Element {
    jsx! { <h2 id={id} className={class_name}>{"H"}</h2> }
}
pub fn H4(HProps { id, class_name }: HProps) -> JSX::Element {
    jsx! { <h4 id={id} className={class_name}>{"H"}</h4> }
}
pub fn App(small: bool) -> JSX::Element {
    let Heading: fn(HProps) -> JSX::Element = if small { H4 } else { H2 };
    jsx! {
        <div style={CSSProperties::new().opacity(if small { 1 } else { 0 }).transition("t")}>
            {small.then(|| jsx! { <Heading id={Some("t")} className={Some(if small { "a" } else { "b" })} /> })}
        </div>
    }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<Heading ");
  const lib = await import(join(dir, "lib.jsx"));
  expect([true, false].map((small) => renderToStaticMarkup(lib.App(small)))).toEqual([
    '<div style="opacity:1;transition:t"><h4 id="t" class="a">H</h4></div>',
    '<div style="opacity:0;transition:t"></div>',
  ]);
});

// ADR 0364: a context's provider read before its children's branch is a
// tag, capitalized as JSX reads one.
test("a provider read before its children's branch is a tag", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Context, JSX, create_context, jsx};
thread_local! {
    pub static Inside: Context<bool> = create_context(false);
}
pub fn App(items: &[&'static str]) -> JSX::Element {
    jsx! {
        <Inside.Provider value={true}>
            {(!items.is_empty()).then(|| jsx! { <b>{items.len().to_string()}</b> })}
        </Inside.Provider>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("Inside.Provider");
  // No temporary held, `_14` or what it's named after, `value`.
  expect(jsx).not.toMatch(/\bconst (_\d|value\b)/);
  const lib = await import(join(dir, "lib.jsx"));
  expect([["a", "b"], []].map((items) => renderToStaticMarkup(lib.App(items)))).toEqual(["<b>2</b>", ""]);
});

// ADR 0364: an attribute read first, before what a child's branch makes,
// is read into a variable named as JS names one, a hyphenated name's too.
test("a hyphenated attribute read first is a variable of a JS name", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
fn label(step: u32) -> String {
    format!("step {step}")
}
pub fn App(step: u32, open: bool) -> JSX::Element {
    jsx! { <div data-step={step.to_string()}>{if open { "a" } else { "b" }}</div> }
}
pub fn Repeated(step: u32) -> JSX::Element {
    jsx! { <div data-step={step.to_string()}>{ let mut s = String::new(); for _ in 0..step { s.push('x'); } s }</div> }
}
pub fn Labelled(step: u32) -> JSX::Element {
    jsx! { <div data-label={label(step)}>{ let mut s = String::new(); for _ in 0..step { s.push('x'); } s }</div> }
}
`);
  run(args);
  // A call of the crate's own runs before the loop, as written: named as
  // JS names a variable, \`dataLabel\`.
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("const dataLabel = label(step);");
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(lib.App(2, true)), renderToStaticMarkup(lib.App(3, false))]).toEqual([
    '<div data-step="2">a</div>',
    '<div data-step="3">b</div>',
  ]);
  expect(renderToStaticMarkup(lib.Repeated(2))).toBe('<div data-step="2">xx</div>');
  expect(renderToStaticMarkup(lib.Labelled(2))).toBe('<div data-label="step 2">xx</div>');
});

// ADR 0235: a child shown only where a test holds, `if shown { hint } else
// { None }`, is `shown && hint`, as react.dev's Challenge shows its hint.
test("a child shown where a test holds is test && child", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn App(hint: Option<&'static str>, shown: bool) -> JSX::Element {
    jsx! { <div>{if shown { hint } else { None }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<div>{shown && hint}</div>");
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(lib.App("h", true)), renderToStaticMarkup(lib.App("h", false)), renderToStaticMarkup(lib.App(undefined, true))]).toEqual(["<div>h</div>", "<div></div>", "<div></div>"]);
});

// A ref's `current`, read first as what an index is set of, is read in
// place, where JS reads an assignment's target before its value: `??=`, as
// react.dev's CustomPreset counts a file's lines once.
test("a ref's object set by its key is read where it's set", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::dict;
use react::{JSX, jsx, use_ref};
pub fn Lines(file: &'static str, code: &'static str) -> JSX::Element {
    let lineCountRef = use_ref(dict::new::<usize>());
    if dict::get(lineCountRef.current(), file).is_none() {
        dict::set(lineCountRef.current(), file, code.split('\\n').count());
    }
    let n = *dict::get(lineCountRef.current(), file).unwrap();
    jsx! { <b>{n.to_string()}</b> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('  lineCountRef.current[file] ??= code.split("\\n").length;\n');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Lines("a", "x\ny")))).toBe("<b>2</b>");
});

// A `let` pattern of a struct through a reference, `use_context`'s, takes
// it apart where it's made, as react.dev's IllustrationBlock reads
// `{ isInBlock }` of its context.
test("a let pattern of a struct through a reference takes it apart", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{Context, JSX, create_context, jsx, use_context};
pub struct Value {
    pub is_in_block: bool,
}
thread_local! {
    pub static Ctx: Context<&'static Value> = create_context(&Value { is_in_block: false });
}
pub fn Block() -> JSX::Element {
    let Value { is_in_block } = use_context(&Ctx);
    jsx! { <b>{(!is_in_block).then(|| "x")}</b> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("  const { isInBlock } = useContext(Ctx);\n");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Block))).toBe("<b>x</b>");
});

// A `let` pattern of props with a flattened field takes the rest apart,
// `{ alt, ...rest }`, as react.dev's Image spreads what it isn't given:
// the rest has no `alt`.
test("a let pattern's flattened field is the rest of what it takes apart", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};
pub struct ImageProps<'a> {
    pub alt: Option<&'a str>,
    #[rust_js::flatten]
    pub rest: ImgHTMLAttributes<'a>,
}
pub fn Image(props: ImageProps) -> JSX::Element {
    let ImageProps { alt, rest } = props;
    jsx! { <img alt={alt} className="pic" {...&rest} /> }
}
// A field the pattern leaves, \`..\`, isn't the rest's, though the rest is
// bound first; nor is a \`Rest\`'s.
pub struct CaptionProps<'a> {
    pub title: Option<&'a str>,
    pub lang: Option<&'a str>,
    #[rust_js::flatten]
    pub rest: ImgHTMLAttributes<'a>,
}
pub fn Caption(props: CaptionProps) -> JSX::Element {
    let CaptionProps { rest, title, .. } = props;
    jsx! { <img title={title} {...&rest} /> }
}
pub fn Badge(CaptionProps { rest, title, .. }: CaptionProps) -> JSX::Element {
    jsx! { <img title={title} {...&rest} /> }
}
pub struct LinkProps {
    pub href: &'static str,
    pub title: &'static str,
    pub rest: react::Rest,
}
pub fn Link(p: LinkProps) -> JSX::Element {
    let LinkProps { href, rest, .. } = p;
    jsx! { <a href={href} {...rest} /> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("  const { alt, ...rest } = props;\n");
  // The one it leaves out is `_`, as a person writes it.
  expect(jsx).toContain("  const { title, lang: _, ...rest } = props;\n");
  expect(jsx).toContain("  const { href, title: _, ...rest } = p;\n");
  expect(jsx).toContain("export function Badge({ title, lang: _, ...rest }) {");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Image, { alt: "a", src: "/s.png" }))).toContain('<img alt="a" class="pic" src="/s.png"/>');
  expect(renderToStaticMarkup(createElement(lib.Caption, { title: "t", lang: "en", src: "/s.png" }))).toContain('<img title="t" src="/s.png"/>');
  expect(renderToStaticMarkup(createElement(lib.Link, { href: "/h", title: "t", id: "i" }))).toBe('<a href="/h" id="i"></a>');
});

// ADR 0364: a prop MIR holds in a temporary, a `String` borrowed for it,
// `image={if .. { format!(..) } else { .. }.as_str()}`, is read where it's
// written, past a field of a plain value read before it, `meta.tag`, which
// no call changes, as react.dev's Page gives Seo its image.
test("a prop held for a borrow is read where it's written, past a plain value's field", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct SeoProps<'a> {
    pub title: &'a str,
    pub tag: Option<&'a str>,
    pub image: &'a str,
}
pub fn Seo(SeoProps { title, tag, image }: SeoProps) -> JSX::Element {
    jsx! { <span title={title} lang={tag.unwrap_or_default()}>{image}</span> }
}
pub struct Meta {
    pub tag: Option<&'static str>,
}
pub fn Page(home: bool, meta: &Meta, title: &'static str, path: String) -> JSX::Element {
    jsx! {
        <div>
            <Seo
                title={title}
                tag={meta.tag}
                image={
                    if home || title.is_empty() {
                        format!("/og-{}.png", path)
                    } else {
                        format!("/og/{}.png", path.replace('/', "-"))
                    }
                    .as_str()
                }
            />
            {home.then(|| jsx! { <b>{"rss"}</b> })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // No temporary held, `_14` or what it's named after, `value`.
  expect(jsx).not.toMatch(/\bconst (_\d|value\b)/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page(false, { tag: "t" }, "T", "a/b"))).toBe('<div><span title="T" lang="t">/og/a-b.png</span></div>');
});

// ADR 0364: a prop MIR holds in a temporary is read where it's written past
// an element's tag, a static component of another module, `<Seo>`, which no
// call changes, as react.dev's Page gives Seo its image.
test("a prop held for a borrow is read where it's written, past an imported tag", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
mod seo {
    use react::{JSX, MemoExoticComponent, jsx, memo};
    pub struct SeoProps<'a> {
        pub title: &'a str,
        pub image: &'a str,
    }
    fn Inner(SeoProps { title, image }: SeoProps) -> JSX::Element {
        jsx! { <span title={title}>{image}</span> }
    }
    thread_local! {
        pub static Seo: MemoExoticComponent<SeoProps<'static>> = memo(Inner, None);
    }
}
use seo::{Seo, SeoProps};
pub fn Page(home: bool, title: &'static str, path: String) -> JSX::Element {
    jsx! {
        <div>
            <Seo
                title={title}
                image={
                    if home || title.is_empty() || path.starts_with("/errors") {
                        format!("/og-{}.png", path)
                    } else {
                        format!("/og/{}.png", path.replace('/', "-"))
                    }
                    .as_str()
                }
            />
            {home.then(|| jsx! { <b>{"rss"}</b> })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // No temporary held, `_14` or what it's named after, `image`.
  expect(jsx).not.toMatch(/\bconst (_\d|value\b|image\b)/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page(false, "T", "a/b"))).toBe('<div><span title="T">/og/a-b.png</span></div>');
});

// ADR 0364: what elements made ahead of a branch read, a call each, is read
// where it's written, past what reads a \`let\` no closure sets, as
// react.dev's Page gives Seo its image and its divs their \`cn(..)\`.
test("what elements made ahead of a branch read is read where it's written, past a let's read", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
mod seo {
    use react::{JSX, MemoExoticComponent, jsx, memo};
    pub struct SeoProps<'a> {
        pub title: &'a str,
        pub title_for_title_tag: Option<&'a str>,
        pub image: &'a str,
        pub search_order: Option<i32>,
    }
    fn Inner(SeoProps { title, title_for_title_tag, image, search_order }: SeoProps) -> JSX::Element {
        jsx! { <span title={title}>{title_for_title_tag}{image}{search_order}</span> }
    }
    thread_local! {
        pub static Seo: MemoExoticComponent<SeoProps<'static>> = memo(Inner, None);
    }
}
use seo::{Seo, SeoProps};
pub struct Meta<'a> {
    pub title_for_title_tag: Option<&'a str>,
}
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub fn Top() -> JSX::Element {
    jsx! { <nav>{"t"}</nav> }
}
pub fn Page(home: bool, blog: bool, title: &'static str, meta: &Meta<'static>, path: String, order: Option<i32>) -> JSX::Element {
    let mut has_columns = true;
    if home || blog {
        has_columns = false;
    }
    let mut search_order = None;
    if !home {
        search_order = order;
    }
    jsx! {
        <>
            <Seo
                title={title}
                titleForTitleTag={meta.title_for_title_tag}
                image={
                    if home || title.is_empty() || path.starts_with("/errors") {
                        format!("/og-{}.png", path)
                    } else {
                        format!("/og/{}.png", path.replace('/', "-"))
                    }
                    .as_str()
                }
                searchOrder={search_order}
            />
            {(home || blog).then(|| jsx! { <b>{"rss"}</b> })}
            <Top />
            <div className={cn(&[has_columns.then_some("grid")])}>
                <article>{"c"}</article>
                <div className={cn(&[Some("self"), home.then_some("wash")])}>
                    {(!home).then(|| jsx! { <hr /> })}
                    <div className={cn(&[Some("py"), home.then_some("pt")])}>{"f"}</div>
                </div>
            </div>
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toMatch(/\bconst (_\d|value\b|image\b|className)/);
  expect(jsx).toContain('      <div className={cn([hasColumns ? "grid" : undefined])}>\n');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page(false, false, "T", { titleForTitleTag: "tt" }, "a/b", 3))).toBe(
    '<span title="T">tt/og/a-b.png3</span><nav>t</nav><div class="grid"><article>c</article><div class="self"><hr/><div class="py">f</div></div></div>',
  );
});

// ADR 0364: a \`let\` a closure shares a reference to reads alike while it
// runs, so an element's \`key\` is read first, where it's written, as
// react.dev's TopNav gives SidebarRouteTree its key and its routeTree.
test("a shared capture of a let reads alike, past an element's key", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    Learn,
    Blog,
}
pub struct TreeProps<'a> {
    pub items: &'a [&'a str],
    pub crumbs: &'a [&'a str],
    pub expanded: bool,
}
pub fn Tree(TreeProps { items, crumbs, expanded }: TreeProps) -> JSX::Element {
    jsx! { <ul data-expanded={expanded}>{(items.len() + crumbs.len()).to_string()}</ul> }
}
pub struct ItemProps {
    pub active: bool,
}
pub fn Item(ItemProps { active }: ItemProps) -> JSX::Element {
    jsx! { <li>{active.to_string()}</li> }
}
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct TopNavProps<'a> {
    pub items: &'a [&'a str],
    pub crumbs: &'a [&'a str],
    pub section: Option<Section>,
}
pub fn TopNav(TopNavProps { mut items, crumbs, section }: TopNavProps) -> JSX::Element {
    let (&open, set_open) = use_state(true);
    if items.len() > 2 {
        items = &items[1..];
    }
    jsx! {
        <>
            <button onClick={move |_| set_open.set(!open)}>{"m"}</button>
            {open.then(|| jsx! {
                <aside className={cn(&[Some("a"), Some(if open { "block" } else { "hidden" })])}>
                    <Item active={section == Some(Section::Learn)} />
                    <Item active={section == Some(Section::Blog)} />
                    <Tree
                        key={if open { "mobile" } else { "desktop" }}
                        items={items}
                        crumbs={crumbs}
                        expanded={open}
                    />
                </aside>
            })}
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<Tree key={open ? "mobile" : "desktop"} items={items}');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.TopNav, { items: ["a", "b", "c"], crumbs: ["x"], section: "Learn" }))).toBe(
    '<button>m</button><aside class="a block"><li>true</li><li>false</li><ul data-expanded="true">3</ul></aside>',
  );
});

// ADR 0364: a struct update of a flattened props rest, `..props.html` and
// `..props`, made ahead of a branch, spreads the rest once: a rest nothing
// changes reads its properties alike later, so it isn't copied,
// `{...{ ...props }}`, as react.dev's Link gives ExternalLink its props.
test("a flattened rest updated ahead of a branch is spread once", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::{AnchorHTMLAttributes, HTMLAttributes};
use react::{JSX, ReactNode, jsx};
pub struct ExternalLinkProps<'a, C: ReactNode> {
    pub href: Option<&'a str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: AnchorHTMLAttributes<'a>,
}
pub fn ExternalLink<C: ReactNode>(ExternalLinkProps { href, children, props }: ExternalLinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} {...props}>{children}</a> }
}
pub struct OtherProps<'a> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: AnchorHTMLAttributes<'a>,
}
pub fn Other(OtherProps { anchor }: OtherProps) -> JSX::Element {
    jsx! { <a {...anchor} /> }
}
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct LinkProps<'a> {
    pub href: Option<&'a str>,
    pub class_name: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: AnchorHTMLAttributes<'a>,
}
pub fn Link(LinkProps { href, class_name, props }: LinkProps) -> JSX::Element {
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return jsx! { <a className={class_name} {...props} /> };
    };
    jsx! {
        <>
        {if href.starts_with("https://") {
            jsx! {
                <ExternalLink
                    href={Some(href)}
                    props={AnchorHTMLAttributes {
                        html: HTMLAttributes {
                            class_name: Some(cn(&[Some("a"), class_name]).as_str()),
                            ..props.html
                        },
                        ..props
                    }}>
                    {"x"}
                </ExternalLink>
            }
        } else {
            jsx! { <Other anchor={props} /> }
        }}
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<ExternalLink href={href} {...props} className={cn([\"a\", className])}>");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Link({ href: "https://x", className: "b", title: "t" }))).toBe("<a href=\"https://x\" title=\"t\" class=\"a b\">x</a>");
});

// ADR 0364: what MIR copies of a dictionary that changes, spread into an
// object it makes ahead of a later read, `{ ...{ ...extra } }`, is spread
// itself, `{ ...base, ...extra }`: an object of spreads alone gives what they
// give, as react.dev's SandpackRoot gives Sandpack its files.
test("a spread of an object of spreads alone is those spreads", async () => {
  const made = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct Base {
    pub a: Option<u32>,
}
pub struct Over<'x> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: &'x Base,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub extra: &'x js::Dict<u32>,
}
unsafe impl js::Defined for Over<'_> {}
pub struct ShownProps<'x> {
    pub over: Option<Over<'x>>,
    pub count: usize,
}
pub fn Shown(ShownProps { over, count }: ShownProps) -> JSX::Element {
    let keys = ["a", "b", "z"].iter().filter(|k| over.as_ref().is_some_and(|o| js::has(js::unknown(o), **k))).count();
    jsx! { <b>{format!("{keys} {count}")}</b> }
}
fn made(given: &'static js::Dict<u32>) -> &'static js::Dict<u32> {
    given
}
fn counted(extra: &js::Dict<u32>) -> usize {
    js::dict::keys(extra).len()
}
pub fn App(base: &'static Base, given: &'static js::Dict<u32>) -> JSX::Element {
    let extra = made(given);
    js::dict::set(extra, "z", 3);
    let outer = |base: &'static Base| {
        let shown = |n: usize| jsx! { <div><Shown over={Some(Over { base, extra })} count={n + counted(extra)} /></div> };
        shown(1)
    };
    outer(base)
}
`);
  run([...made.args, "--extern", `js=${join(target, "libjs.rmeta")}`]);
  const jsx = readFileSync(join(made.dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Shown over={{ ...base, ...extra }}");
  const { App } = await import(join(made.dir, "lib.jsx"));
  expect(renderToStaticMarkup(App({ a: 1 }, { b: 2 }))).toBe("<div><b>3 3</b></div>");
});

// ADR 0364: a context's `Provider`, of a context of another module, is the
// element's tag where it's written, made ahead of a branch: React never
// changes it, as react.dev's Toc gives its headings `IsInTocContext`.
test("an imported context's provider is a tag read where it's written", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_context};
mod theme {
    use react::{Context, create_context};
    thread_local! { pub static THEME: Context<&'static str> = create_context("light"); }
}
use theme::THEME;
fn Label() -> JSX::Element {
    let theme = use_context(&THEME);
    jsx! { <b>{*theme}</b> }
}
pub fn App(names: Vec<&'static str>) -> JSX::Element {
    jsx! {
        <nav>
            {(!names.is_empty()).then(|| jsx! { <h2>{"On this page"}</h2> })}
            <THEME.Provider value="dark">
                {names.iter().map(|name| jsx! { <i key={*name}>{*name}</i> }).collect::<Vec<_>>()}
                <Label />
            </THEME.Provider>
        </nav>
    }
}
`);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain("<THEME.Provider value=\"dark\">");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => result.App(["a"])))).toBe("<nav><h2>On this page</h2><i>a</i><b>dark</b></nav>");
});

// ADR 0364: a component's `key` written first is first: MIR's prop values
// are placed where their fields are written, as react.dev's PageHeading
// keys each Tag.
test("a component's key goes where it's written", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct TagProps<'a> {
    pub variant: &'a str,
}
pub fn Tag(TagProps { variant }: TagProps) -> JSX::Element {
    jsx! { <b>{variant}</b> }
}
pub fn Tags(tags: Vec<&'static str>) -> JSX::Element {
    jsx! { <div>{tags.iter().map(|tag| jsx! { <Tag key={*tag} variant={*tag} /> }).collect::<Vec<_>>()}</div> }
}
pub fn One(section: Option<&'static str>) -> JSX::Element {
    jsx! { <Tag key={section} variant={"x"} /> }
}
`);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain("<Tag key={tag} variant={tag} />");
  expect(code).toContain('<Tag key={section} variant="x" />');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Tags(["a", "b"]))).toBe("<div><b>a</b><b>b</b></div>");
});

// ADR 0364: a prop MIR holds in a temporary is read where it's written past
// a sibling element before it, `<Search isOpen={show} />`, which reads
// only what no call changes, as react.dev's TopNav gives its logo classes.
test("a prop held for a borrow is read where it's written, past a sibling element", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct SearchProps {
    pub is_open: bool,
}
pub fn Search(SearchProps { is_open }: SearchProps) -> JSX::Element {
    jsx! { <i>{is_open.then_some("open")}</i> }
}
pub struct SeoProps<'a> {
    pub image: &'a str,
}
pub fn Seo(SeoProps { image }: SeoProps) -> JSX::Element {
    jsx! { <span>{image}</span> }
}
pub fn Page(home: bool, show: bool, path: String) -> JSX::Element {
    jsx! {
        <div>
            <Search isOpen={show} />
            <Seo
                image={
                    if home {
                        format!("/og-{}.png", path)
                    } else {
                        format!("/og/{}.png", path.replace('/', "-"))
                    }
                    .as_str()
                }
            />
            {home.then(|| jsx! { <b>{"rss"}</b> })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // No temporary held, `_14` or what it's named after, `value`.
  expect(jsx).not.toMatch(/\bconst (_\d|value\b)/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page(false, true, "a/b"))).toBe("<div><i>open</i><span>/og/a-b.png</span></div>");
});

// ADR 0364: an element MIR makes only where a test holds, ahead of a later
// child, `let c; if (error != null) { c = cn(..); }`, is made where it's
// written, `error != null && <div className={cn(..)}>`, and so are the props
// before it, as react.dev's Preview shows its error.
test("an element made where a test holds is read where it's written", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub fn Preview(error: Option<&'static str>, expanded: bool) -> JSX::Element {
    jsx! {
        <div className={cn(&[Some("p-0"), (!expanded).then_some("overflow-auto")])}>
            <iframe className={cn(&[Some("frame"), expanded.then_some("wide")])} />
            {error.map(|error| jsx! {
                <div className={cn(&[Some("z-50"), expanded.then_some("sticky")])}>{error}</div>
            })}
            <span className={cn(&[Some("overlay"), (!expanded).then_some("hidden")])} />
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toContain("const className");
  expect(jsx).not.toContain("let className");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Preview("bad", true))).toBe('<div class="p-0"><iframe class="frame wide"></iframe><div class="z-50 sticky">bad</div><span class="overlay"></span></div>');
});

// ADR 0364: as above, of an `Option` of an object, tested by its truth,
// `!!error` where MIR makes the value and `error &&` where it's shown.
test("an element made where an object's truth holds is read where it's written", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct Failure {
    pub message: String,
}
pub fn Preview(error: Option<&'static Failure>, expanded: bool) -> JSX::Element {
    jsx! {
        <div className={cn(&[Some("p-0"), (!expanded).then_some("overflow-auto")])}>
            {error.map(|error| jsx! {
                <div className={cn(&[Some("z-50"), expanded.then_some("sticky")])}>{error.message.as_str()}</div>
            })}
            <span className={cn(&[Some("overlay"), (!expanded).then_some("hidden")])} />
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toContain("const className");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Preview({ message: "bad" }, true))).toBe('<div class="p-0"><div class="z-50 sticky">bad</div><span class="overlay"></span></div>');
});

// ADR 0235: a child shown where a state's `bool` holds, `if *show_hint {
// hint } else { None }`, is `showHint && hint`: the state, read through its
// handle, is a `bool`, as react.dev's Challenge shows its hint.
test("a child shown where a state's bool holds is an and", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx, use_state};
pub struct HintProps<'a> {
    pub hint: Option<&'a dyn ReactNode>,
}
pub fn Hint(HintProps { hint }: HintProps) -> JSX::Element {
    let (show_hint, _set_show_hint) = use_state(true);
    jsx! { <div>{if *show_hint { hint } else { None }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<div>{showHint && hint}</div>");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Hint, { hint: "h" }))).toBe("<div>h</div>");
});

// ADR 0364: children MIR holds as a tuple, `const children = [" ", ..]`,
// read item by item, `{children[0]}{children[1]}`, are those items where
// they're read, of what does nothing: a `then` closure whose element they're
// in is that element, `!recipes && <Button>..`, as react.dev's Challenge
// shows its solution button.
test("children held as a tuple are read as their items", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
pub struct IconProps<'a> {
    pub class_name: Option<&'a str>,
}
pub fn Icon(IconProps { class_name }: IconProps) -> JSX::Element {
    jsx! { <i className={class_name.unwrap_or_default()} /> }
}
pub struct ButtonProps<'a, C: ReactNode> {
    pub children: C,
    pub on_click: Option<Box<dyn Fn()>>,
    pub active: Option<bool>,
    pub class_name: Option<&'a str>,
}
pub fn Button<C: ReactNode>(ButtonProps { children, on_click, active, class_name }: ButtonProps<C>) -> JSX::Element {
    let _ = on_click;
    jsx! { <button className={class_name.unwrap_or_default()} disabled={active.unwrap_or_default()}>{children}</button> }
}
pub fn Buttons(recipes: bool, shown: bool, toggle: &'static dyn Fn()) -> JSX::Element {
    let toggle_solution = move || toggle();
    jsx! {
        <div>
            {(!recipes).then(|| jsx! {
                <Button className={Some("me-2")} onClick={Some(Box::new(toggle_solution))} active={Some(shown)}>
                    <Icon className={Some("me-1.5")} />{" "}
                    {if shown { "Hide solution" } else { "Show solution" }}
                </Button>
            })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toContain("children[");
  expect(jsx).not.toContain("const value");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Buttons(false, true, () => {}))).toBe('<div><button class="me-2" disabled=""><i class="me-1.5"></i> Hide solution</button></div>');
});

// `&dyn Any` values given to a JS module's function, `define(vec![&a, &b])`,
// are the values, `HighlightStyle.define([a, b])`, no `{ value, impl }`
// pairs unwrapped, read in place past the import's member, which no call
// changes, as react.dev's CodeBlock defines its highlighting.
test("dyn values given to a JS module's function are the values", async () => {
  const { dir, args } = compile(`use std::any::Any;
pub struct Highlight;
#[rust_js::link_name = "./style.js#HighlightStyle.define"]
#[allow(unused_variables)]
fn define(specs: Vec<&dyn Any>) -> &'static Highlight {
    unreachable!()
}
struct ClassStyle {
    tag: &'static str,
    class: String,
}
struct FontStyle {
    tag: &'static str,
    font_style: &'static str,
}
fn token(name: &str) -> String {
    format!("token-{name}")
}
pub fn highlight() -> &'static Highlight {
    define(vec![
        &FontStyle { tag: "em", font_style: "italic" },
        &ClassStyle { tag: "kw", class: token("keyword") },
    ])
}
`, { "style.js": "export const HighlightStyle = { define: (specs) => specs };\n" });
  run(args);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return HighlightStyle.define([");
  expect(js).not.toContain("impl");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.highlight()).toEqual([{ tag: "em", fontStyle: "italic" }, { tag: "kw", class: "token-keyword" }]);
});

// ADR 0364: props MIR holds ahead of a later child's closure, `const value$3
// = cn(..)`, are read where they're written past what no call can change:
// the closure's making, a conditional of parameters, and a `let` no
// closure sets, `target`, as react.dev's SidebarLink gives its link
// classes.
test("props held ahead of a closure child are read where they're written", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct LinkProps<'a, C: ReactNode> {
    pub href: &'a str,
    pub target: Option<&'a str>,
    pub aria_current: Option<&'a str>,
    pub class_name: Option<&'a str>,
    pub children: C,
}
pub fn Link<C: ReactNode>(LinkProps { href, target, aria_current, class_name, children }: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} target={target.unwrap_or_default()} aria-current={aria_current.unwrap_or_default()} className={class_name.unwrap_or_default()}>{children}</a> }
}
pub fn SidebarLink(href: &'static str, title: &'static str, selected: bool, level: u32, external: bool, version: u32) -> JSX::Element {
    let mut target = None;
    if external {
        target = Some("_blank");
    }
    jsx! {
        <Link
            href={href}
            target={target}
            ariaCurrent={selected.then_some("page")}
            className={Some(cn(&[Some("p-2"), (level > 0).then_some("text-sm"), selected.then_some("text-link")]).as_str())}>
            <div>
                {title}{" "}
                {(version == 19).then(|| jsx! { <span>{"React 19"}</span> })}
                {(version == 20).then(|| jsx! { <b>{"Canary"}</b> })}
            </div>
            {(level > 1).then(|| {
                let arrow = cn(&[Some("arrow"), selected.then_some("open")]);
                jsx! { <span className={arrow} /> }
            })}
        </Link>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  // Read in place.
  expect(jsx).toContain("      target={target}\n      ariaCurrent={selected ? \"page\" : undefined}\n      className={cn([");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.SidebarLink("/a", "A", true, 2, true, 19))).toBe('<a href="/a" target="_blank" aria-current="page" class="p-2 text-sm text-link"><div>A <span>React 19</span></div><span class="arrow open"></span></a>');
});

// ADR 0364: react.dev's SidebarLink, its imports stood in for: the link's
// classes, `cn(..)`, held for a borrow, are read where they're written, past
// the props and children MIR holds after them, a `let` no closure sets,
// `target`, and tests of parameters, `version === "major" && <span>`.
test("react.dev's SidebarLink writes its classes in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::{Kind, Unknown, classify};
use react::attributes::AnchorHTMLAttributes;
use react::webapi::HTMLAnchorElement;
use react::{JSX, ReactNode, RefObject, jsx, use_effect, use_ref};
#[cfg_attr(rust_js, rust_js::link_name = "./cn.js#default")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[cfg_attr(rust_js, rust_js::skips_falsy)]
#[allow(unused_variables)]
pub fn cn(classes: &[Option<&str>]) -> String {
    unreachable!()
}
#[derive(Default)]
pub struct LinkProps<'a, C> {
    pub href: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: AnchorHTMLAttributes<'a>,
    pub replace: Option<bool>,
    pub scroll: Option<bool>,
    pub pass_href: Option<bool>,
    pub r#ref: Option<RefObject<Option<&'static HTMLAnchorElement>>>,
    pub class_name: Option<&'a str>,
    pub target: Option<&'a str>,
    pub rel: Option<&'a str>,
    pub id: Option<&'a str>,
    pub children: C,
}
#[rust_js::link_name = "./link.js#default"]
pub fn Link<C: ReactNode>(props: LinkProps<'_, C>) -> JSX::Element {
    unreachable!()
}
pub struct IconProps<'a> {
    pub title: Option<&'a str>,
    pub class_name: Option<&'a str>,
}
pub fn IconCanary(IconProps { title, class_name }: IconProps) -> JSX::Element {
    jsx! { <i title={title.unwrap_or_default()} className={class_name.unwrap_or_default()} /> }
}
pub fn IconExperimental(IconProps { title, class_name }: IconProps) -> JSX::Element {
    jsx! { <i title={title.unwrap_or_default()} className={class_name.unwrap_or_default()} /> }
}
#[derive(Clone, Copy, PartialEq)]
pub enum DisplayDirection {
    #[cfg_attr(rust_js, rust_js::name = "down")]
    Down,
    #[cfg_attr(rust_js, rust_js::name = "end")]
    End,
}
pub struct IconNavArrowProps {
    pub display_direction: Option<DisplayDirection>,
}
pub fn IconNavArrow(IconNavArrowProps { display_direction }: IconNavArrowProps) -> JSX::Element {
    jsx! { <b>{(display_direction == Some(DisplayDirection::Down)).then_some("v")}</b> }
}
#[derive(PartialEq)]
pub enum Version {
    #[cfg_attr(rust_js, rust_js::name = "major")]
    Major,
    #[cfg_attr(rust_js, rust_js::name = "canary")]
    Canary,
    #[cfg_attr(rust_js, rust_js::name = "experimental")]
    Experimental,
    #[cfg_attr(rust_js, rust_js::name = "rc")]
    Rc,
}
pub struct SidebarLinkProps<'a> {
    pub href: &'a str,
    #[cfg_attr(rust_js, rust_js::default)]
    pub selected: bool,
    pub title: &'a str,
    pub level: u32,
    pub version: Option<Version>,
    pub icon: Option<Box<dyn ReactNode>>,
    pub is_expanded: Option<bool>,
    pub hide_arrow: Option<bool>,
    pub is_pending: bool,
}

// Chrome's and Safari's, not every browser's.
#[cfg_attr(rust_js, rust_js::link_name = "get scrollIntoViewIfNeeded")]
#[allow(unused_variables)]
fn scroll_into_view_if_needed_of(this: &HTMLAnchorElement) -> &'static Unknown {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "scrollIntoViewIfNeeded")]
#[allow(unused_variables)]
fn scroll_into_view_if_needed(this: &HTMLAnchorElement) {
    unreachable!()
}

pub fn SidebarLink(
    SidebarLinkProps {
        href,
        selected,
        title,
        version,
        level,
        is_expanded,
        hide_arrow,
        is_pending,
        ..
    }: SidebarLinkProps,
) -> JSX::Element {
    let r#ref = use_ref::<Option<&'static HTMLAnchorElement>>(None);

    use_effect(
        move || {
            if selected && let Some(current) = r#ref.current() {
                if matches!(classify(scroll_into_view_if_needed_of(current)), Kind::Function(_)) {
                    scroll_into_view_if_needed(current);
                }
            }
        },
        Some(&(r#ref, selected)),
    );

    let mut target = "";
    if href.starts_with("https://") {
        target = "_blank";
    }
    jsx! {
        <Link
            href={href}
            ref={Some(r#ref)}
            title={Some(title)}
            target={Some(target)}
            passHref={Some(true)}
            aria-current={selected.then_some("page".into())}
            className={Some(cn(&[
                Some("p-2 pe-2 w-full rounded-none lg:rounded-e-2xl text-start hover:bg-gray-5 dark:hover:bg-gray-80 relative flex items-center justify-between"),
                (level > 0).then_some("text-sm ps-6"),
                (level < 2).then_some("ps-5"),
                (level == 0).then_some("text-base font-bold"),
                (level == 0 && !selected).then_some("text-primary dark:text-primary-dark"),
                (level > 0 && !selected).then_some("text-base text-secondary dark:text-secondary-dark"),
                selected.then_some("text-base text-link dark:text-link-dark bg-highlight dark:bg-highlight-dark border-blue-40 hover:bg-highlight hover:text-link dark:hover:bg-highlight-dark dark:hover:text-link-dark"),
                is_pending.then_some("dark:bg-gray-70 bg-gray-3 dark:hover:bg-gray-70 hover:bg-gray-3"),
            ]).as_str())}>
            // This here needs to be refactored ofc
            <div>
                {title}{" "}
                {(version == Some(Version::Major)).then(|| jsx! {
                    <span
                        title="- This feature is available in React 19 beta and the React canary channel"
                        className="text-xs px-1 ms-1 rounded bg-gray-10 dark:bg-gray-40 dark:bg-opacity-20 text-gray-40 dark:text-gray-40">
                        {"React 19"}
                    </span>
                })}
                {(version == Some(Version::Canary)).then(|| jsx! {
                    <IconCanary
                        title={Some(" - This feature is available in the latest Canary version of React")}
                        className={Some("ms-1 text-gray-30 dark:text-gray-60 inline-block w-3.5 h-3.5 align-[-3px]")}
                    />
                })}
                {(version == Some(Version::Experimental)).then(|| jsx! {
                    <IconExperimental
                        title={Some(" - This feature is available in the latest Experimental version of React")}
                        className={Some("ms-1 text-gray-30 dark:text-gray-60 inline-block w-3.5 h-3.5 align-[-3px]")}
                    />
                })}
                {(version == Some(Version::Rc)).then(|| jsx! {
                    <IconCanary
                        title={Some(" - This feature is available in the latest RC version")}
                        className={Some("ms-1 text-gray-30 dark:text-gray-60 inline-block w-3.5 h-3.5 align-[-3px]")}
                    />
                })}
            </div>

            {(is_expanded.is_some() && !hide_arrow.unwrap_or(false)).then(|| jsx! {
                <span
                    className={cn(&[
                        Some("pe-1"),
                        is_expanded.unwrap_or(false).then_some("text-link dark:text-link-dark"),
                        (!is_expanded.unwrap_or(false)).then_some("text-tertiary dark:text-tertiary-dark"),
                    ])}>
                    <IconNavArrow
                        displayDirection={Some(if is_expanded.unwrap_or(false) { DisplayDirection::Down } else { DisplayDirection::End })}
                    />
                </span>
            })}
        </Link>
    }
}
`, {
    "link.js": 'import { createElement } from "react"; export default function Link({ href, className, target, title, children, ...rest }) { return createElement("a", { href, className, target, title, "aria-current": rest["aria-current"] }, children); }\n',
    "cn.js": "export default (...classes) => classes.filter(Boolean).join(\" \");\n",
  });
  run([...args, "--extern", `js=${join(target, "libjs.rmeta")}`]);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toMatch(/const \w+ = cn\(/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.SidebarLink, { href: "https://x", selected: true, title: "T", level: 0, version: "major", isPending: false }))).toContain('<a href="https://x" class="p-2 pe-2');
});

// react.dev's Preview: its error's test, `!!error`, a negation of a state's
// value, reads the same after a later child's statement, its overlay's
// `&&`, so it's `error && (..)` in place, not `const condition = !!error`.
test("react.dev's Preview tests its error in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct ErrorType {
    pub message: String,
}
pub struct ErrorMessageProps<'a> {
    pub error: &'a ErrorType,
}
pub fn ErrorMessage(ErrorMessageProps { error }: ErrorMessageProps) -> JSX::Element {
    jsx! { <pre>{error.message.as_str()}</pre> }
}
pub struct LoadingOverlayProps {
    pub dependencies_loading: bool,
}
pub fn LoadingOverlay(LoadingOverlayProps { dependencies_loading }: LoadingOverlayProps) -> Option<JSX::Element> {
    dependencies_loading.then(|| jsx! { <p /> })
}
pub fn Preview(is_expanded: bool, rawError: Option<&'static ErrorType>) -> JSX::Element {
    let (bundlerIsReady, _) = use_state(false);
    let (iframeComputedHeight, _) = use_state::<Option<f64>>(None);
    let (error, _) = use_state(rawError);
    let error = *error;
    jsx! {
        <div>
            {error.map(|error| jsx! {
                <div className={cn(&[Some("z-50"), is_expanded.then_some("sticky top-8 ")])}>
                    <ErrorMessage error={error} />
                </div>
            })}
            <LoadingOverlay dependenciesLoading={!*bundlerIsReady && iframeComputedHeight.is_none()} />
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{error && (");
  expect(jsx).not.toContain("const condition");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Preview(true, { message: "boom" })))).toBe('<div><div class="z-50 sticky top-8 "><pre>boom</pre></div><p></p></div>');
  expect(renderToStaticMarkup(createElement(() => lib.Preview(false, undefined)))).toBe("<div><p></p></div>");
});

// react.dev's Page: its feed's test, `isHomePage || isBlogIndex`, shows an
// import, `Head`, which no later child's statement can change, so it's in
// place, not `const condition = ..` before them.
test("react.dev's Page tests its home and blog index in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct HeadProps<C: ReactNode> {
    pub children: C,
}
#[rust_js::link_name = "./head.js#default"]
pub fn Head<C: ReactNode>(_: HeadProps<C>) -> JSX::Element {
    unreachable!()
}
pub fn Page(cleaned_path: &str) -> JSX::Element {
    let is_home_page = cleaned_path == "/";
    let is_blog_index = cleaned_path == "/blog";
    jsx! {
        <>
            {(is_home_page || is_blog_index).then(|| jsx! {
                <Head>
                    <link rel="alternate" type="application/rss+xml" title="React Blog RSS Feed" href="/rss.xml" />
                </Head>
            })}
            <div className={cn(&[is_home_page.then_some("grid")])} />
        </>
    }
}
`, { "head.js": 'import { createElement } from "react"; export default function Head({ children }) { return createElement("head", null, children); }\n' });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{(isHomePage || isBlogIndex) && (");
  expect(jsx).not.toContain("const condition");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page("/blog"))).toBe('<head><link rel="alternate" type="application/rss+xml" title="React Blog RSS Feed" href="/rss.xml"/></head><div class=""></div>');
  expect(renderToStaticMarkup(lib.Page("/learn"))).toBe('<div class=""></div>');
});

// react.dev's NavigationBar: its options, a component shown if a test holds
// among Headless UI's `Listbox`'s children, are `test && <Listbox.Options>`
// in place (ADR 0235), not the component and its items held in `let`s
// before a later child's closure, `{!!condition && <Component>}`.
test("react.dev's NavigationBar shows its options in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::collections::HashMap;
use react::attributes::HtmlHTMLAttributes;
use react::{JSX, ReactNode, jsx, use_state};
pub struct ListboxProps<T, C> {
    pub value: Option<T>,
    pub children: C,
}
#[rust_js::link_name = "./listbox.js#Listbox"]
pub fn Listbox<T, C: ReactNode>(_: ListboxProps<T, C>) -> JSX::Element {
    unreachable!()
}
#[derive(Default)]
pub struct ListboxOptionsProps<'a, C> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: HtmlHTMLAttributes<'a>,
    pub children: C,
}
#[rust_js::link_name = "./listbox.js#Listbox.Options"]
pub fn ListboxOptions<C: ReactNode>(_: ListboxOptionsProps<'_, C>) -> JSX::Element {
    unreachable!()
}
pub struct PlaygroundProps<'a> {
    pub content: &'a str,
}
pub fn Playground(PlaygroundProps { content }: PlaygroundProps) -> JSX::Element {
    jsx! { <pre>{content}</pre> }
}
pub fn NavigationBar(activeFile: &'static str, visibleFiles: &'static [String], files: &'static HashMap<&'static str, String>) -> JSX::Element {
    let (showDropdown, _) = use_state(true);
    let isMultiFile = visibleFiles.len() > 1;
    jsx! {
        <div>
            <Listbox value={Some(activeFile.to_string())}>
                <span>{activeFile}</span>
                {(isMultiFile && *showDropdown).then(|| jsx! {
                    <ListboxOptions className={Some("absolute mt-0.5")}>
                        {visibleFiles.iter().map(|filePath| jsx! { <li key={filePath.as_str()}>{filePath.as_str()}</li> }).collect::<Vec<_>>()}
                    </ListboxOptions>
                })}
            </Listbox>
            {activeFile.ends_with(".tsx").then(|| jsx! { <Playground content={files.get(activeFile).map(|file| file.as_str()).unwrap_or("")} /> })}
        </div>
    }
}
`, { "listbox.js": 'import { createElement } from "react"; export function Listbox({ children }) { return createElement("div", null, children); } Listbox.Options = function Options({ className, children }) { return createElement("ul", { className }, children); };\n' });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('{isMultiFile && showDropdown && (\n          <Listbox.Options className="absolute mt-0.5">');
  expect(jsx).not.toContain("let Component");
  const lib = await import(join(dir, "lib.jsx"));
  const files = new Map([["App.tsx", "code"]]);
  expect(renderToStaticMarkup(createElement(() => lib.NavigationBar("App.tsx", ["App.tsx", "a.ts"], files)))).toBe('<div><div><span>App.tsx</span><ul class="absolute mt-0.5"><li>App.tsx</li><li>a.ts</li></ul></div><pre>code</pre></div>');
  expect(renderToStaticMarkup(createElement(() => lib.NavigationBar("a.ts", ["a.ts"], files)))).toBe("<div><div><span>a.ts</span></div></div>");
});

// react.dev's Diagram: its images' srcs, `withBasePath(..)` made before
// MIR's block for the caption's test, are read in place, and the test,
// `matches!(.., None | Some(Bottom))`, is `(.. == null || .. === "bottom") &&`,
// not `let tmp = true` set in an `if` before the JSX.
test("react.dev's Diagram writes its images' srcs and caption test in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct ImageProps<'a> {
    pub src: &'a str,
    pub alt: &'a str,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[rust_js::link_name = "./image.js#default"]
pub fn Image(_: ImageProps) -> JSX::Element {
    unreachable!()
}
pub fn withBasePath(path: &str) -> String {
    if path.starts_with('/') && !path.starts_with("//") {
        format!("/base{path}")
    } else {
        path.to_string()
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum CaptionPosition {
    #[cfg_attr(rust_js, rust_js::name = "top")]
    Top,
    #[cfg_attr(rust_js, rust_js::name = "bottom")]
    Bottom,
}
pub struct DiagramProps<'a> {
    pub name: &'a str,
    pub alt: &'a str,
    pub height: u32,
    pub width: u32,
    pub children: &'a str,
    pub caption_position: Option<CaptionPosition>,
}
struct CaptionProps<'a> {
    text: &'a str,
}
fn Caption(CaptionProps { text }: CaptionProps) -> JSX::Element {
    jsx! {
        <div className="w-full flex justify-center">
            <figcaption className="p-1 sm:p-2 mt-0 sm:mt-0 text-gray-40 text-base lg:text-lg text-center leading-tight table-caption max-w-lg">
                {text}
            </figcaption>
        </div>
    }
}
pub fn Diagram(
    DiagramProps {
        name,
        alt,
        height,
        width,
        children,
        caption_position,
    }: DiagramProps,
) -> JSX::Element {
    jsx! {
        <figure className="flex flex-col px-0 p-0 sm:p-10 first:mt-0 mt-10 sm:mt-0 justify-center items-center">
            {(caption_position == Some(CaptionPosition::Top)).then(|| jsx! { <Caption text={children} /> })}
            <div className="dark-image">
                <Image
                    src={withBasePath(&format!("/images/docs/diagrams/{name}.dark.png")).as_str()}
                    alt={alt}
                    height={Some(height)}
                    width={Some(width)}
                />
            </div>
            <div className="light-image">
                <Image
                    src={withBasePath(&format!("/images/docs/diagrams/{name}.png")).as_str()}
                    alt={alt}
                    height={Some(height)}
                    width={Some(width)}
                />
            </div>
            {matches!(caption_position, None | Some(CaptionPosition::Bottom)).then(|| jsx! { <Caption text={children} /> })}
        </figure>
    }
}
`, { "image.js": 'import { createElement } from "react"; export default function Image({ src, alt, width, height }) { return createElement("i", { "data-src": src, title: alt, "data-size": `${width}x${height}` }); }\n' });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("src={withBasePath(`/images/docs/diagrams/${name}.dark.png`)}");
  expect(jsx).toContain('{(captionPosition == null || captionPosition === "bottom") && <Caption text={children} />}');
  expect(jsx).not.toContain("let tmp");
  const lib = await import(join(dir, "lib.jsx"));
  const props = { name: "d", alt: "A", height: 1, width: 2, children: "C" };
  const caption = '<div class="w-full flex justify-center"><figcaption class="p-1 sm:p-2 mt-0 sm:mt-0 text-gray-40 text-base lg:text-lg text-center leading-tight table-caption max-w-lg">C</figcaption></div>';
  const images = '<div class="dark-image"><i data-src="/base/images/docs/diagrams/d.dark.png" title="A" data-size="2x1"></i></div><div class="light-image"><i data-src="/base/images/docs/diagrams/d.png" title="A" data-size="2x1"></i></div>';
  const figure = (inside: string) => `<figure class="flex flex-col px-0 p-0 sm:p-10 first:mt-0 mt-10 sm:mt-0 justify-center items-center">${inside}</figure>`;
  expect(renderToStaticMarkup(createElement(lib.Diagram, props))).toBe(figure(images + caption));
  expect(renderToStaticMarkup(createElement(lib.Diagram, { ...props, captionPosition: "top" }))).toBe(figure(caption + images));
});

// react.dev's SandboxShell: its expand button, shown if a test holds among
// Sandpack's `SandpackLayout`'s children, is `test && <button>` in place.
// The layout's props flatten a `<div>`'s attributes before `children`, so
// its companion must build them in that order: else `className` is built
// after the children, and the button's values are `let`s set before the JSX.
test("react.dev's SandboxShell shows its expand button in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::HtmlHTMLAttributes;
use react::{JSX, ReactNode, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
#[derive(Default)]
pub struct SandpackLayoutProps<'a, C> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: HtmlHTMLAttributes<'a>,
    pub children: C,
}
#[rust_js::link_name = "./sandpack.js#SandpackLayout"]
pub fn SandpackLayout<C: ReactNode>(_: SandpackLayoutProps<'_, C>) -> JSX::Element {
    unreachable!()
}
pub enum DisplayDirection {
    #[cfg_attr(rust_js, rust_js::name = "up")]
    Up,
    #[cfg_attr(rust_js, rust_js::name = "down")]
    Down,
}
pub struct IconChevronProps {
    pub class_name: Option<&'static str>,
    pub display_direction: Option<DisplayDirection>,
}
pub fn IconChevron(props: IconChevronProps) -> JSX::Element {
    let up = matches!(props.display_direction, Some(DisplayDirection::Up));
    jsx! { <svg className={cn(&[up.then_some("rotate-180"), props.class_name])} /> }
}
pub struct SandboxShellProps {
    pub is_expandable: bool,
}
pub fn SandboxShell(SandboxShellProps { is_expandable }: SandboxShellProps) -> JSX::Element {
    let (isExpanded, setIsExpanded) = use_state(false);
    jsx! {
        <SandpackLayout
            className={Some(&cn(&[
                (!(is_expandable || *isExpanded)).then_some("rounded-b-lg overflow-hidden"),
                (*isExpanded).then_some("sp-layout-expanded"),
            ]))}>
            {(is_expandable || *isExpanded).then(|| jsx! {
                <button
                    translate="yes"
                    className="sandpack-expand"
                    onClick={move |_| setIsExpanded.set(!*isExpanded)}>
                    <span className="flex">
                        <IconChevron
                            className={Some("inline")}
                            displayDirection={Some(if *isExpanded { DisplayDirection::Up } else { DisplayDirection::Down })}
                        />
                        {if *isExpanded { "Show less" } else { "Show more" }}
                    </span>
                </button>
            })}
        </SandpackLayout>
    }
}
`, { "sandpack.js": 'import { createElement } from "react"; export function SandpackLayout({ className, children }) { return createElement("div", { className }, children); }\n' });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{(isExpandable || isExpanded) && (\n        <button");
  expect(jsx).toContain('<IconChevron className="inline" displayDirection={isExpanded ? "up" : "down"} />\n            {isExpanded ? "Show less" : "Show more"}');
  expect(jsx).not.toContain("const condition");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.SandboxShell, { isExpandable: true }))).toBe('<div class=""><button translate="yes" class="sandpack-expand"><span class="flex"><svg class="inline"></svg>Show more</span></button></div>');
  expect(renderToStaticMarkup(createElement(lib.SandboxShell, { isExpandable: false }))).toBe('<div class="rounded-b-lg overflow-hidden"></div>');
});

// react.dev's InlineToc: a node's `RefCell` of children, only borrowed for a
// moment, to push, to count and to pass on, is the array itself, not a cell
// with `$borrow` and `$unborrow` around each use.
test("react.dev's InlineToc nests its headings in a field's array", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use react::{Context, JSX, ReactNode, create_context, jsx, use_context, use_memo};
pub struct TocItem {
    pub url: String,
    pub text: JSX::Element,
    pub depth: u32,
}
thread_local! {
    pub static TocContext: Context<&'static [TocItem]> = create_context(&[]);
    pub static IsInTocContext: Context<bool> = create_context(false);
}
pub struct ChildrenProps<C> {
    pub children: C,
}
fn UL<C: ReactNode>(ChildrenProps { children }: ChildrenProps<C>) -> JSX::Element {
    jsx! { <ul>{children}</ul> }
}
fn LI<C: ReactNode>(ChildrenProps { children }: ChildrenProps<C>) -> JSX::Element {
    jsx! { <li>{children}</li> }
}
pub struct LinkProps<'a, C> {
    pub href: Option<&'a str>,
    pub children: C,
}
fn Link<C: ReactNode>(LinkProps { href, children }: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={href.unwrap_or_default()}>{children}</a> }
}
pub struct NestedTocNode {
    pub item: Option<&'static TocItem>,
    pub children: RefCell<Vec<Rc<NestedTocNode>>>,
}
fn calculateNestedToc(toc: &'static [TocItem]) -> Rc<NestedTocNode> {
    let mut currentAncestors: HashMap<u32, Rc<NestedTocNode>> = HashMap::new();
    let root = Rc::new(NestedTocNode {
        item: None,
        children: RefCell::new(vec![]),
    });
    let startIndex = 1; // Skip "Overview"
    for i in startIndex..toc.len() {
        let item = &toc[i];
        let currentParent = currentAncestors
            .get(&(item.depth - 1))
            .cloned()
            .unwrap_or_else(|| root.clone());
        let node = Rc::new(NestedTocNode {
            item: Some(item),
            children: RefCell::new(vec![]),
        });
        currentParent.children.borrow_mut().push(node.clone());
        currentAncestors.insert(item.depth, node);
    }
    root
}
pub fn InlineToc() -> Option<JSX::Element> {
    let toc = use_context(&TocContext);
    let root = use_memo(move || calculateNestedToc(toc), (toc,));
    if root.children.borrow().len() < 2 {
        return None;
    }
    Some(jsx! {
        <IsInTocContext value={true}>
            <InlineTocItem items={&root.children.borrow()} />
        </IsInTocContext>
    })
}
pub struct InlineTocItemProps<'a> {
    pub items: &'a [Rc<NestedTocNode>],
}
fn InlineTocItem(InlineTocItemProps { items }: InlineTocItemProps) -> JSX::Element {
    jsx! {
        <UL>
            {items.iter().map(|node| {
                // Only the root has no item.
                let item = node.item.unwrap();
                jsx! {
                    <LI key={&item.url}>
                        <Link href={Some(item.url.as_str())}>{item.text}</Link>
                        {if node.children.borrow().len() > 0 { Some(jsx! { <InlineTocItem items={&node.children.borrow()} /> }) } else { None }}
                    </LI>
                }
            }).collect::<Vec<_>>()}
        </UL>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const root = { children: [] };");
  expect(jsx).toContain("currentParent.children.push(node);");
  expect(jsx).toContain("root.children.length < 2");
  expect(jsx).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.jsx"));
  const toc = [["#o", "Overview", 2], ["#a", "A", 2], ["#b", "B", 3], ["#c", "C", 2]].map(([url, text, depth]) => ({ url, text, depth }));
  const render = (items: object[]) => renderToStaticMarkup(createElement(lib.TocContext, { value: items }, createElement(lib.InlineToc)));
  expect(render(toc)).toBe('<ul><li><a href="#a">A</a><ul><li><a href="#b">B</a></li></ul></li><li><a href="#c">C</a></li></ul>');
  expect(render(toc.slice(0, 3))).toBe("");
});

// ADR 0040: a component's handler of one call whose JS gives `undefined`,
// held by MIR ahead of a borrow's call, is `() => f()` where it's read in
// place, as react.dev's ExpandableExample toggles.
test("a held handler of one call is a concise arrow", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::webapi::MouseEvent;
use react::{JSX, ReactNode, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct ButtonProps<'a, C: ReactNode> {
    pub children: C,
    pub on_click: Option<Box<dyn Fn(&MouseEvent)>>,
    pub class_name: Option<&'a str>,
}
pub fn Button<C: ReactNode>(ButtonProps { children, on_click, class_name }: ButtonProps<C>) -> JSX::Element {
    let _ = on_click;
    jsx! { <button className={class_name.unwrap_or_default()}>{children}</button> }
}
pub fn Example(deep: bool) -> JSX::Element {
    let (expanded, set_expanded) = use_state(false);
    jsx! {
        <Button
            className={Some(cn(&[deep.then_some("deep"), (!deep).then_some("plain")]).as_str())}
            onClick={Some(Box::new(move |_| set_expanded.update(|current| !current)))}>
            <span>{if *expanded { "Hide" } else { "Show" }}</span>
        </Button>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("onClick={() => setExpanded((current) => !current)}");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Example(true)))).toBe('<button class="deep"><span>Show</span></button>');
});

// A variant's test function, `isValidElement(child)`, gives a `bool`, so a
// pattern's test of it is no `!!(..)`, as react.dev's CodeDiagram keeps its
// images.
test("a variant's test function is a bool in a test", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::children::{self, Child};
use react::{JSX, ReactElement, ReactNode, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "get type")]
#[allow(unused_variables)]
fn tag(this: &ReactElement) -> Option<&'static str> {
    unreachable!()
}
pub struct DiagramProps<C: ReactNode> {
    pub children: C,
}
pub fn Diagram<C: ReactNode>(DiagramProps { children }: DiagramProps<C>) -> JSX::Element {
    let images: Vec<Child> = children::to_array(&children)
        .into_iter()
        .filter(|child| matches!(child, Child::Element(element) if tag(element) == Some("img")))
        .collect();
    jsx! { <section>{images}</section> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('(child) => isValidElement(child) && child.type === "img"');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Diagram, null, createElement("img"), "text", createElement("b")))).toBe("<section><img/></section>");
});

// ADR 0364: classes MIR holds ahead of a child's branch, `cn(.., variant.textColor)`,
// are read where they're written past what does nothing, a static's
// fields picked by a test, as react.dev's ExpandableCallout gives its
// variant's colors.
test("classes held ahead of a branch past a static's fields are read in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "./cn.js#default")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[cfg_attr(rust_js, rust_js::skips_falsy)]
#[allow(unused_variables)]
fn cn(classes: &[Option<&str>]) -> String {
    unreachable!()
}
pub struct Variant {
    pub title: &'static str,
    pub icon: Option<&'static str>,
    pub container_classes: &'static str,
    pub text_color: &'static str,
}
pub struct VariantMap {
    pub note: Variant,
    pub wip: Variant,
}
static variantMap: VariantMap = VariantMap {
    note: Variant { title: "Note", icon: Some("i"), container_classes: "bg-green", text_color: "text-green" },
    wip: Variant { title: "WIP", icon: None, container_classes: "bg-yellow", text_color: "text-yellow" },
};
pub struct CalloutProps<C: ReactNode> {
    pub children: C,
    pub wip: bool,
}
pub fn Callout<C: ReactNode>(CalloutProps { children, wip }: CalloutProps<C>) -> JSX::Element {
    let variant = if wip { &variantMap.wip } else { &variantMap.note };
    jsx! {
        <div className={cn(&[Some("callout"), Some(variant.container_classes)])}>
            <h3 className={cn(&[Some("title"), Some(variant.text_color)])}>
                {variant.icon.map(|icon| jsx! { <i className={cn(&[Some("icon"), Some(variant.text_color)])}>{icon}</i> })}
                {variant.title}
            </h3>
            <div>{children}</div>
        </div>
    }
}
`, { "cn.js": "export default (...classes) => classes.filter(Boolean).join(\" \");\n" });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toContain("const className");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Callout, { wip: false }, "x"))).toBe('<div class="callout bg-green"><h3 class="title text-green"><i class="icon text-green">i</i>Note</h3><div>x</div></div>');
});

// ADR 0364: a variant's binding of a place nothing changes, `element` of
// `let Child::Element(element) = child else { .. }`, is that place, `child`,
// read where it's used, as react.dev's Challenges parses its children.
test("an element a let-else binds is read as the child it binds", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::Unknown;
use react::children::{self, Child};
use react::{JSX, ReactNode, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "get mdxName")]
#[allow(unused_variables)]
fn mdx_name(this: &Unknown) -> Option<&'static str> {
    unreachable!()
}
pub fn Kinds<C: ReactNode>(children: C) -> JSX::Element {
    let mut hints = 0;
    let mut solutions = Vec::new();
    let mut content = Vec::new();
    children::for_each(&children, |child, _| {
        let Some(child) = child else {
            return;
        };
        let Child::Element(element) = child else {
            content.push(child);
            return;
        };
        match mdx_name(element.r#type()) {
            Some("Solution") => {
                solutions.push(std::mem::take(&mut content));
            }
            Some("Hint") => {
                hints += 1;
            }
            Some("h4") => {
                hints += 2;
            }
            _ => {
                content.push(child);
            }
        }
    });
    jsx! { <p data-hints={hints} data-solutions={solutions.len()}>{content}</p> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("if (!isValidElement(child)) {\n      content.push(child);\n      return;\n    }");
  const lib = await import(join(dir, "lib.jsx"));
  const Hint = Object.assign(() => null, { mdxName: "Hint" });
  const element = createElement(Hint, { key: "h" });
  expect(renderToStaticMarkup(lib.Kinds(["a", element, "b"]))).toBe('<p data-hints="1" data-solutions="0">ab</p>');
  const Solution = Object.assign(() => null, { mdxName: "Solution" });
  const solution = createElement(Solution, { key: "s" });
  expect(renderToStaticMarkup(lib.Kinds(["a", solution, "b"]))).toBe('<p data-hints="0" data-solutions="1">b</p>');
});

// ADR 0364: what MIR holds ahead of the children before it, `showToc &&
// toc.length !== 0`, is read where it's written, past their calls: what
// does nothing, of a `let` no closure sets, which no call changes, as
// react.dev's Page shows its Toc.
test("a test of a let held ahead of calls is read in place", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct TocItem {
    pub text: JSX::Element,
}
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub fn Page(toc: &'static [TocItem], home: bool) -> JSX::Element {
    let mut show_toc = true;
    if home {
        show_toc = false;
    }
    jsx! {
        <div className={cn(&[home.then_some("home")])}>
            <p className={cn(&[Some("p")])} />
            {(show_toc && !toc.is_empty()).then(|| jsx! { <span>{toc.len()}</span> })}
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{showToc && toc.length !== 0 && <span>");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page([{ text: null }], false))).toBe('<div class=""><p class="p"></p><span>1</span></div>');
});

// ADR 0364: a child chosen of elements, `open ? <IconClose /> : <IconHamburger
// />`, makes nothing a call changes, so classes MIR holds for a later
// child, `cn(..)` of a flattened prop, are read past it where they're
// written, as react.dev's TopNav its menu button's icon and its logo's.
test("a chosen element child is read past by later held classes", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::attributes::SVGAttributes;
use react::{JSX, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
#[derive(Default)]
pub struct LogoProps<'a> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: SVGAttributes<'a>,
}
pub fn Logo(LogoProps { props }: LogoProps) -> JSX::Element {
    jsx! { <svg {..props} /> }
}
pub fn IconClose() -> JSX::Element {
    jsx! { <b /> }
}
pub fn IconHamburger() -> JSX::Element {
    jsx! { <i /> }
}
pub fn Nav() -> JSX::Element {
    let (&open, set_open) = use_state(false);
    jsx! {
        <div className={cn(&[Some(if open { "wide" } else { "narrow" })])}>
            <button onClick={move |_| set_open.set(!open)} className={cn(&[Some("menu"), open.then_some("open")])}>
                {if open { jsx! { <IconClose /> } } else { jsx! { <IconHamburger /> } }}
            </button>
            <Logo className={Some(cn(&[Some("logo")]).as_str())} />
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).not.toMatch(/const (className|value|_\d+) =/);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Nav))).toBe('<div class="narrow"><button class="menu"><i></i></button><svg class="logo"></svg></div>');
});

// ADR 0364: a variable the program names, `activeCode`, stays named where
// MIR holds the prop it's moved into ahead of a child spreading props,
// `const code = activeCode`: the prop reads the variable, as react.dev's
// Sandpack gives its fallback the active code.
test("a named variable moved into a held prop keeps its name", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, MemoExoticComponent, Suspense, jsx, memo};
pub struct Snip {
    pub code: String,
}
pub struct GlimmerProps {
    pub code: String,
}
fn Glimmer(GlimmerProps { code }: GlimmerProps) -> JSX::Element {
    jsx! { <pre>{code.as_str()}</pre> }
}
pub struct ClientProps {
    pub snips: &'static [Snip],
    pub active: &'static [usize],
}
fn Root(props: ClientProps) -> JSX::Element {
    jsx! { <p>{props.active.len()}</p> }
}
thread_local! {
    pub static Client: MemoExoticComponent<ClientProps> = memo({
        fn ClientWrapper(props: ClientProps) -> JSX::Element {
            let activeCode;
            if props.active.is_empty() {
                activeCode = props.snips.get(0).unwrap().code.clone();
            } else {
                activeCode = props.snips.get(props.active[0]).unwrap().code.clone();
            }
            jsx! {
                <Suspense fallback={jsx! { <Glimmer code={activeCode} /> }}>
                    <Root {..props} />
                </Suspense>
            }
        }
        ClientWrapper
    }, None);
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<Glimmer code={activeCode} />");
  const lib = await import(join(dir, "lib.jsx"));
  const props = { snips: [{ code: "a" }, { code: "b" }], active: [1] };
  expect(renderToStaticMarkup(createElement(lib.Client, props))).toBe("<p>1</p>");
});

// ADR 0364: what MIR holds for a prop or a child, made before a later one's
// statements, is named after the prop, `const href = made("h")`, or a
// child's `children`, not `value`.
test("a value held for a prop is named after it", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
fn made(s: &'static str) -> &'static str {
    s
}
pub fn Looped() -> JSX::Element {
    jsx! {
        <a
            href={made("h")}
            title={{
                let mut n = 0;
                while n < 2 {
                    n += 1;
                }
                if n == 2 { "two" } else { "other" }
            }}>
            {made("L")}
            {{
                let mut m = 0;
                while m < 2 {
                    m += 1;
                }
                m
            }}
        </a>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('const href = made("h");');
  // A child's too.
  expect(jsx).toContain('const children = made("L");');
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Looped())).toBe('<a href="h" title="two">L2</a>');
});

// ADR 0364: JS's JSX runtime makes a `key` after the children, so a key
// that does something is made first, held, in source order: what's read in
// it waits on the children too.
test("a key that does something is made before the children", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.recordKey")] #[allow(unused_variables)] fn record(n: i32) -> i32 { unreachable!() }
pub struct Props { pub title: i32, pub children: i32 }
pub(crate) fn Card(p: Props) -> JSX::Element { jsx! { <div>{p.title}{p.children}</div> } }
pub fn App() -> JSX::Element {
    let three = 3;
    jsx! { <Card title={record(1)} key={record(2)}>{record(three)}</Card> }
}
pub fn Chosen(c: bool) -> JSX::Element {
    jsx! { <Card title={record(1)} key={record(2)}>{if c { record(3) } else { record(4) }}</Card> }
}
`);
  run(args);
  const log: number[] = [];
  (globalThis as any).recordKey = (n: number) => (log.push(n), n);
  const lib = await import(join(dir, "lib.jsx"));
  const tree = lib.App();
  expect(log).toEqual([1, 2, 3]);
  expect([tree.key, tree.props]).toEqual(["2", { title: 1, children: 3 }]);
  log.length = 0;
  const chosen = lib.Chosen(false);
  expect(log).toEqual([1, 2, 4]);
  expect(chosen.key).toBe("2");
});

// ADR 0364: what an element's attributes and children read, a state's
// value and a prop, is read where they're written, as react.dev's
// ExpandableExample writes its Button: not each held first, its children
// then read apart, `children[0]`.
test("JSX attributes and children read in place around a borrowed one", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().cloned().collect::<Vec<_>>().join(" ")
}
pub struct ButtonProps<'a, C: ReactNode> {
    pub children: C,
    pub class_name: Option<&'a str>,
    pub on_click: Option<Box<dyn Fn(u32)>>,
}
pub fn Button<C: ReactNode>(ButtonProps { children, class_name, .. }: ButtonProps<C>) -> JSX::Element {
    jsx! { <button className={class_name}>{children}</button> }
}
pub fn App(deep: bool) -> JSX::Element {
    let (is_expanded, set_is_expanded) = use_state(false);
    jsx! {
        <details open={*is_expanded} className={cn(&[Some("my-12"), deep.then_some("purple")])}>
            <Button
                className={Some(cn(&[deep.then_some("bg"), Some("x")]).as_str())}
                onClick={Some(Box::new(move |_| set_is_expanded.update(|current| !current)))}>
                <span>{if *is_expanded { "up" } else { "down" }}</span>
                {if *is_expanded { "Hide" } else { "Show" }}
            </Button>
        </details>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain(`<details open={isExpanded} className={cn(["my-12", deep ? "purple" : undefined])}>`);
  expect(jsx).toContain(`        <span>{isExpanded ? "up" : "down"}</span>\n`);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.App(true)))).toBe('<details class="my-12 purple"><button class="bg x"><span>down</span>Show</button></details>');
});

// ADR 0364: what's made before a labeled block, a `matches!`'s, and read
// after it is declared where it's read, as react.dev's Diagram has its
// images' sources before its caption's test: not inside the block.
test("JSX reads what's made before a matches! child's block", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
#[derive(Clone, Copy, PartialEq)]
pub enum CaptionPosition {
    #[cfg_attr(rust_js, rust_js::name = "top")]
    Top,
    #[cfg_attr(rust_js, rust_js::name = "bottom")]
    Bottom,
}
fn with_base(path: &str) -> String {
    format!("/base{path}")
}
pub struct ImageProps<'a> {
    pub src: &'a str,
}
pub fn Image(ImageProps { src }: ImageProps) -> JSX::Element {
    jsx! { <b>{src}</b> }
}
pub fn Diagram(name: &str, caption_position: Option<CaptionPosition>) -> JSX::Element {
    jsx! {
        <figure>
            {(caption_position == Some(CaptionPosition::Top)).then(|| jsx! { <i>{"top"}</i> })}
            <Image src={with_base(&format!("/{name}.dark.png")).as_str()} />
            <Image src={with_base(&format!("/{name}.png")).as_str()} />
            {matches!(caption_position, None | Some(CaptionPosition::Bottom)).then(|| jsx! { <i>{"bottom"}</i> })}
        </figure>
    }
}
`);
  run(args);
  const lib = await import(join(dir, "lib.jsx"));
  expect([undefined, "top", "bottom"].map((at) => renderToStaticMarkup(lib.Diagram("d", at)))).toEqual([
    '<figure><b>/base/d.dark.png</b><b>/base/d.png</b><i>bottom</i></figure>',
    '<figure><i>top</i><b>/base/d.dark.png</b><b>/base/d.png</b></figure>',
    '<figure><b>/base/d.dark.png</b><b>/base/d.png</b><i>bottom</i></figure>',
  ]);
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
