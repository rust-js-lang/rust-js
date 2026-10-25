// What `jsx!` writes of children and keys: a list as an array's map, a child
// shown if a test holds as `test && child`, `Children.forEach`, a key where
// it's written, and a hook's closure as a named function. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup, renderToString } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";

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

// Children whose JS needs no statements of its own read nothing ahead of a
// prop written before them: JSX reads its props, then its children, as Rust
// does. A `Link` whose props flatten an anchor's is such a child, as
// react.dev's `Breadcrumbs` has it.
// ADR 0641: a `concat!` of literals, a child's or a prop's, is its pieces
// joined by `+` in JSX too, `{"a " + "b"}`, as a `const` of one is.
test("a concat! child or prop is its pieces joined by +", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};

const HINT: &str = concat!("Try logging smaller ", "or more specific objects.");

pub fn Note() -> JSX::Element {
    jsx! {
        <p className={concat!("note ", "wide")} title={HINT}>
            {concat!("Omitted ", "by React. ")}
            {HINT}
        </p>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const output = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(output).toContain('const HINT = "Try logging smaller " + "or more specific objects.";');
  expect(output).toContain('    <p className={"note " + "wide"} title={HINT}>\n      {"Omitted " + "by React. "}\n      {HINT}\n    </p>');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Note())).toBe(
    '<p class="note wide" title="Try logging smaller or more specific objects.">Omitted by React. Try logging smaller or more specific objects.</p>',
  );
});

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

// A closure a \`let\` names, only a hook's function: written there, named by
// the \`let\` (ADR 0297).
test("a hook's closure a let names is a named function where it's given", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_effect, use_state};
pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state::<i32>(|| 0);
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
            && let Kind::String(r#type) = classify(Some(element.r#type()))
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
  // The type a pattern binds, a getter's read, read once there, not at
  // each use (ADR 0750).
  expect(js).toContain('out.push(type === "Recap" ? "recap" : type.toUpperCase());');
  // And read once, as Rust reads it, before the test of its kind (#189).
  expect(js).toContain('if (isValidElement(child)) {\n      const type = child.type;\n      if (typeof type === "string" && types.has(type)) {');
  expect(js.match(/child\.type/g)?.length).toBe(1);
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

// A list whose callback has statements, and a handler of several, stay in
// their JSX, as a person writes them: oxfmt lays them out where they are
// (ADR 0218), as react.dev's `Breadcrumbs` maps its crumbs.
test("a callback of several statements stays in its JSX", () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub struct Crumb { pub title: String }
pub fn list(crumbs: &[Crumb]) -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
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
    let (expanded, set_expanded) = use_state(|| false);
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
  expect(jsx).toContain("  return a && text ? 1 : 0;");
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
    let (n, set_n) = use_state(|| 0u32);
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

// ADR 0298, amended: JSX shows an Option of an object where it's not
// `null` nor `undefined`, `{error != null && <div>..</div>}` and
// `hidden={error == null}`, as a test of any `Option` is.
test("JSX tests an Option of an object against null", async () => {
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
  expect(jsx).toContain("hidden={error == null}");
  expect(jsx).toContain("{error != null && <b>{error.message}</b>}");
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
    let (show_hint, _set_show_hint) = use_state(|| true);
    jsx! { <div>{if *show_hint { hint } else { None }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<div>{showHint && hint}</div>");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.Hint, { hint: "h" }))).toBe("<div>h</div>");
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

// A prop inside a child shown where a value's truth holds, `js::truthy(label)`,
// is what it is there: `title="x"`, not `title={label ? "x" : undefined}`,
// as the test it's under says `label` is truthy.
test("a prop under a child's truth test is its value there", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::js::Unknown;
use react::{JSX, jsx};
pub fn Truthy(label: Option<&'static Unknown>) -> JSX::Element {
    jsx! { <div>{if react::js::truthy(label) { Some(jsx! { <b title={react::js::truthy(label).then_some("x")} /> }) } else { None }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('return <div>{!!label && <b title="x" />}</div>;');
  const lib = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(lib.Truthy("a")), renderToStaticMarkup(lib.Truthy(""))]).toEqual(['<div><b title="x"></b></div>', "<div></div>"]);
});
