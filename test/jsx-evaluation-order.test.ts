// What an element reads is read where it's written (ADR 0364): what MIR makes
// ahead of a branch, a prop held for a borrow, attributes before a child, and
// a key made before the children where it does something. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, root, run, target } from "./support";
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
  expect(jsx).toContain("const condition = n === 0;\n  n += 1;");
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
    let (&open, set_open) = use_state(|| true);
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
  expect(code).toContain("<Theme.Provider value=\"dark\">");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => result.App(["a"])))).toBe("<nav><h2>On this page</h2><i>a</i><b>dark</b></nav>");
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
