import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
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
  expectSnapshot(dir, join(root, "test/snapshots/jsx", name), {
    normalize: text => text.replaceAll(dir + "/", "test/jsx/"),
  });
}

test("JSX supports components across modules, fragments, lists, conditions and spreads", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
#[rust_js::camel_case]
const _: () = ();
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
use react::{JSX, jsx};
use react::webapi::{abort_controller, abort_signal};

pub struct BadgeProps {
    pub label: &'static str,
}

#[rust_js::link_name = "./badge.js#Badge"]
pub fn Badge(props: BadgeProps) -> JSX::Element {
    unreachable!()
}

unsafe extern "Rust" {
    #[link_name = "encodeURIComponent"]
    safe fn encode(text: &str) -> String;
    #[link_name = "parseInt"]
    safe fn parse_int(text: &str) -> f64;
}

pub fn App() -> JSX::Element {
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
unsafe extern "Rust" {
    #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32;
}
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
    #[rust_js::name = "className"]
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
    #[rust_js::name = "className"]
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
// a test of text by its truthiness keeps its conditional.
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
  expect(jsx).toContain("{href ? <a href={href} /> : undefined}");
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
    pub static IconNote: MemoExoticComponent<SVGAttributes<'static>> = memo(Note);
    pub static IconBadge: MemoExoticComponent<BadgeProps> = memo(Badge);
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
// Its own props given by name, as a component's are: still one component.
pub fn Badged() -> JSX::Element {
    jsx! { <IconBadge className={Some("b")} size={Some("2em")} /> }
}
`, { "icons.rs": icons });
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("icon = IconNote;");
  expect(jsx).toContain('{variant.Icon && <variant.Icon className="inline" />}');
  const { Callout } = await import(join(dir, "lib.jsx"));
  expect([0, 1, 2].map((which) => renderToStaticMarkup(Callout(which)))).toEqual([
    '<h3><svg class="inline"></svg>t</h3>',
    '<h3><svg class="inline" width="1em"></svg>t</h3>',
    "<h3>t</h3>",
  ]);
  // A `static` of another module's components is made of them, which JS
  // imports first.
  expect(jsx).toContain('const PINNED = { title: "p", Icon: IconBadge };');
  const { Badged, Pinned } = await import(join(dir, "lib.jsx"));
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
    #[rust_js::name = "className"]
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
  expect(jsx).toContain('<Heading id="t" className="title" key="h">');
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
    children::for_each(&children, |child| kept.push(child));
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
  expect(jsx).toContain("Children.forEach(children, (child) => {\n    kept.push(child);\n  });");
  expect(jsx).toContain("createRef()");
  const { List } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect(renderToStaticMarkup(createElement(List, null, "a", createElement("b", null, "x")))).toBe("<ul><li><button>a</button></li><li><button><b>x</b></button></li></ul>");
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

test("nested component JSX stays readable, contextually typed and mapped to the original Rust", async () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
#[rust_js::camel_case]
const _: () = ();
use react::{JSX, jsx};
use std::rc::Rc;
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32); }
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
unsafe extern "Rust" {
    #[link_name = "String"]
    safe fn cn(a: &str) -> String;
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
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('{excerpt != null && excerpt.length !== 0 && <p>{excerpt}</p>}');
  expect(jsx).toContain('<span>{expanded ? "Hide" : "Show"}</span>');
  expect(jsx).not.toContain("const children");
  expect(jsx).not.toContain("const className");
  expect(jsx).toContain('return name != null && name.length !== 0 ? name : "Error";');
  const { Example, title } = await import(join(dir, "lib.jsx"));
  expect(["e", "", undefined].map((e) => renderToStaticMarkup(createElement(() => Example(e))))).toEqual([
    '<div><h5 class="title">Example</h5><p>e</p><button class="button"><span>Show</span></button></div>',
    '<div><h5 class="title">Example</h5><button class="button"><span>Show</span></button></div>',
    '<div><h5 class="title">Example</h5><button class="button"><span>Show</span></button></div>',
  ]);
  expect([title("Oops"), title(""), title(undefined)]).toEqual(["Oops", "Error", "Error"]);
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
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32; }
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
    "wrapped.rs": `use react::{Context, JSX, ForwardRefExoticComponent, LazyExoticComponent, MemoExoticComponent, RefObject, create_context, forward_ref, import_module, jsx, lazy, memo};
pub struct Props { pub label: &'static str }
pub fn Card(p: Props) -> JSX::Element { jsx! { <b>{p.label}</b> } }
pub fn Provider() -> JSX::Element { jsx! { <i /> } }
pub fn Input(p: Props, _: RefObject<Option<i32>>) -> JSX::Element { jsx! { <b>{p.label}</b> } }
thread_local! {
    pub static MEMO: MemoExoticComponent<Props> = memo(Card);
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
    let portal: &ReactPortal = create_portal(jsx! { <i>{"away"}</i> }, container);
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
            <button onClick={|e| webapi::html_button_element::set_disabled(e.current_target(), true)}>{label}</button>
            <details onToggle={|e| webapi::html_details_element::set_open(e.current_target(), false)} />
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
    ["onClick={|e| webapi::html_button_element::set_disabled(e.current_target(), true)}", "onClick={|e: &event::MouseEvent<webapi::HTMLInputElement>| { let _ = e; }}", "expected closure signature `for<'a> fn(&'a MouseEvent<react::webapi::HTMLButtonElement>) -> _`"],
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
use react::{JSX, ForwardRefExoticComponent, RefObject, forward_ref, jsx, webapi};
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32) -> i32; }
pub struct Props { pub label: i32 }
pub fn Input(p: Props, reference: RefObject<Option<&'static webapi::Element>>) -> JSX::Element {
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
  expect(invalid.stderr.toString()).toContain('is not a `Ref` of');
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
    use_effect(effect, ());
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

// A ref is @types/react's `Ref<T>`, a `RefObject` or a `RefCallback`, which a
// component's own prop takes, `impl Ref<&HTMLDivElement, M>`, and passes to
// its element as it is; one of another element is an error that says so.
test("JSX refs are @types/react's Ref and RefCallback", async () => {
  const source = `#![allow(non_snake_case)]
use react::webapi::{HTMLDivElement, HTMLElement};
use react::{JSX, Ref, RefCallback, RefObject, jsx};
pub fn Panel<M>(target: impl Ref<&'static HTMLDivElement, M>) -> JSX::Element {
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
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a `Ref` of `&'static react::webapi::HTMLDivElement`")], wrong.stderr.toString()).toEqual([false, true]);
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
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain('<ButtonLink target="_blank" href="/a" id="x" size="lg">');
  expect(jsx).toContain('<ButtonLink href="/b" className="c">');
  const { App, Noted, Linked, Sized, Forward } = await import(join(dir, "lib.jsx"));
  expect(jsx).toContain("<ButtonLink href={href} {...props}>");
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
  expect(jsx).toContain("<ButtonLink target={target} label={label} href={h}>");
  // Each made first is named as its field is, as a person names it.
  expect(jsx).toContain("<ButtonLink label={label} target={target} id={id} href={href}>");
  expect(jsx).toContain('<ButtonLink href="/t" target={t}>');
  expect(jsx).toMatch(/<ButtonLink target=\{\w+\} href=\{\w+\}>/);
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
  // Props not taken apart where they're given would have no default.
  const whole = compile(chip + "pub fn Whole(props: ChipProps) -> JSX::Element {\n    jsx! { <b>{props.label}</b> }\n}\n");
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
#[rust_js::camel_case]
const _: () = ();
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
  // What JS's props can't be is an error: the field read whole, the
  // struct made anywhere but as JSX's props, two rests, and a name both have.
  const refused = (source: string, says: string) => {
    const c = compile(source);
    const failed = Bun.spawnSync(c.args, { cwd: c.dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(says)]);
  };
  refused(flattened + "pub fn read(p: ButtonLinkProps<()>) -> Option<&'static str> { let a = p.anchor; a.href }\n", "flattened props");
  refused(flattened + "pub fn made() -> JSX::Element { ButtonLink(ButtonLinkProps { size: None, children: (), anchor: Anchor::default() }) }\n", "made only as JSX");
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
use react::{JSX, ReactNode, jsx};
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
  expect(jsx).toContain("<svg className={q.class_name}>");
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Badge({ class_name: "c", size: "S", title: "t" }))).toBe('<svg class="c" id="c" width="12px" height="12px" viewBox="0 0 20 20"><title>t</title><g fill="none"><path d="M0 0"></path></g>1</svg>');
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

// A component looks inside its children as React's own API does, as
// react.dev's MDX components do: `Children.toArray`, an element told apart by
// `isValidElement`, its `type`, `props` and `key`, and `cloneElement`.
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
  const { Kinds, Only, Id } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const Component = () => null;
  expect(renderToStaticMarkup(createElement(Kinds, null, "a", 1, createElement("img", { key: "k" }), createElement(Component), null, true)))
    .toBe("<p>text a, number 1, &lt;img&gt; .$k, component</p>");
  expect(renderToStaticMarkup(createElement(Only, null, createElement("a", { href: "/" }, "x")))).toBe('<a href="/" data-linked="yes">x</a>');
  expect(renderToStaticMarkup(createElement(Only, null, "t"))).toBe("<i>t</i>");
  expect(renderToStaticMarkup(createElement(Only, null, "t", "u"))).toBe("<b>other</b>");
  expect(renderToStaticMarkup(createElement(Id, null, createElement("h4", { id: "deep" })))).toBe("<p>deep</p>");
  expect(renderToStaticMarkup(createElement(Id, null, "t"))).toBe("<p>none</p>");
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
  expect(jsx).toContain(`  if (typeof children === "string") {
    message = children;
  } else if (isValidElement(children)) {
    const inner = children.props.children;
    if (typeof inner === "string") {
      message = inner;
    } else {
      throw new Error("Expected plain text.");
    }
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
use react::webapi::{Node, node};
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
    jsx! { <div><p>{node::text_content(n)}</p>{{ node::set_text_content(n, "x"); 1 }}</div> }
}
pub struct N { pub n: i32 }
pub fn Edited(e: &mut N) -> JSX::Element {
    jsx! { <p>{e.n}{{ e.n = 5; 1 }}</p> }
}
pub fn Counter() -> JSX::Element {
    let count = Cell::new(0);
    jsx! { <div><p>{count.get()}</p>{{ count.set(5); 1 }}</div> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  const badge = jsx.slice(jsx.indexOf("export function Badge"), jsx.indexOf("export function Live"));
  expect([badge.includes("const width"), badge.includes("= p.title")]).toEqual([false, false]);
  expect(badge).toContain('<svg width={p.size === "S" ? "12px" : "20px"}>');
  expect(jsx).toMatch(/= n\.textContent;\n  n\.textContent = "x";/);
  expect(jsx).toMatch(/= count\.value;\n  count\.value = 5;/);
  const { Badge, Counter, Edited } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Badge({ size: "S", title: "t" }))).toBe('<svg width="12px"><title>t</title>2</svg>');
  expect(renderToStaticMarkup(Counter())).toBe("<div><p>0</p>1</div>");
  expect(renderToStaticMarkup(Edited({ n: 0 }))).toBe("<p>01</p>");
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
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(result.Heading("T", "Canary", false, ""))).toBe("<h1>T<i>canary</i><b>stable</b><s>todo</s>1</h1>");
  expect(renderToStaticMarkup(result.Heading("T", "Rc", true, "new"))).toBe("<h1>T<em>new</em>1</h1>");
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
  // Nor is one taken apart anywhere but where the props are given.
  const later = compile(`#![allow(non_snake_case)]
use react::{JSX, Rest, jsx};
pub struct P { pub href: &'static str, pub rest: Rest }
pub fn A(p: P) -> JSX::Element {
    let P { href, rest } = p;
    jsx! { <a href={href} {...rest} /> }
}
`);
  const refused = Bun.spawnSync(later.args, { cwd: later.dir });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("a `Rest` of props taken apart here")]).toEqual([false, true]);
});

// An async handler is the async function itself, as react.dev's
// `async function handleCopy()` is, where `spawn` in a closure would call
// one in place, `(async () => { .. })()`.
test("JSX takes an async event handler as the async function it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::Promise;
use react::webapi::HTMLButtonElement;
use react::{JSX, event, jsx, use_state};
unsafe extern "Rust" {
    #[link_name = "Promise.resolve"]
    safe fn resolved() -> Promise<()>;
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
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const handleCopy = async () => {\n    await Promise.resolve();\n    setCopied(true);\n  };");
  expect(jsx).not.toContain("(async");
  const { Button } = await import(join(dir, "lib.jsx"));
  let clicked = false;
  const handler = async () => { clicked = true; };
  const button = Button({ on_click: handler });
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
  expect([Button({ children: "b", on_click: handler }).props.onClick === handler, Button({ children: "b" }).props.onClick]).toEqual([true, undefined]);
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
use react::webapi::{HTMLFormElement, html_element, submit_event};
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
                let by = e.submitter().map(|b| html_element::title(b)).unwrap_or_default();
                on_send(format!("{by} {}", submit_event::submitter(e.native_event()).is_some()));
            }}
            onReset={on_reset} />
    }
}
`);
  run(args);
  const { Form } = await import(join(dir, "lib.jsx"));
  const sent: string[] = [];
  const { props } = Form({ on_send: (s: string) => sent.push(s), on_reset: () => {} });
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
use react::webapi::{HTMLInputElement, html_input_element, input_event};
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
                input_event::input_type(e.native_event()),
                html_input_element::value(e.current_target()),
            ))}
            onChange={move |e| on_read(e.value())} />
    }
}
`);
  run(args);
  const { Field } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Field({ on_read: (s: string) => read.push(s) });
  props.onInput({ data: "a", nativeEvent: { inputType: "insertText" }, currentTarget: { value: "ba" } });
  props.onChange({ target: { value: "ba" } });
  expect(read).toEqual(["a insertText ba", "ba"]);
});

// An event's `native_event` is the DOM's event it wraps, typed as
// @types/react types `nativeEvent`: a click's a `MouseEvent`, a pointer's a
// `PointerEvent`, a wheel's a `WheelEvent`, so its own fields are read.
test("JSX native_event is the DOM event of its kind", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::webapi::{mouse_event, pointer_event, wheel_event};
use react::{JSX, jsx};
pub struct PadProps { pub on_read: fn(String) }
pub fn Pad(PadProps { on_read }: PadProps) -> JSX::Element {
    jsx! {
        <div
            onClick={move |e| on_read(mouse_event::client_x(e.native_event()).to_string())}
            onPointerDown={move |e| on_read(pointer_event::pointer_type(e.native_event()))}
            onWheel={move |e| on_read(wheel_event::delta_y(e.native_event()).to_string())} />
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("e.nativeEvent.clientX");
  const { Pad } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Pad({ on_read: (s: string) => read.push(s) });
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
  expect([renderToStaticMarkup(Panel({ width: "6px", custom_styles: { width: "7px", color: "red" } })), renderToStaticMarkup(Panel({ width: "6px" }))])
    .toEqual(['<div style="width:7px;color:red"></div>', '<div style="width:6px"></div>']);
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
#[rust_js::camel_case]
const _: () = ();
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
// read the two as one text, which React would render as one node.
test("adjacent text children stay apart", async () => {
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub fn Count(order: u32, total: u32) -> JSX::Element {
    jsx! { <p>{"Challenge"}{" "}{order}{" of"}{" "}{total}</p> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('{"Challenge"} {order}');
  expect(code).toContain('{" of"} {total}');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToString(result.Count(1, 4))).toBe("<p>Challenge<!-- --> <!-- -->1<!-- --> of<!-- --> <!-- -->4</p>");
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
  const { Next } = await import(join(dir, "lib.jsx"));
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

// Props given whole through a reference, the rest of a child's own, are
// spread, then the props named, as react.dev's CodeDiagram gives
// CodeBlock a <pre>'s props, `{...child.props} noMargin={true}`: what the
// props hold is shared, and a key their type doesn't name passes too.
test("a component's props updated from a reference are spread", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};

#[derive(Clone, Copy)]
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
  expect(jsx).toContain("<Panel {...props} wide key={match} />");
  const { Again } = await import(join(dir, "lib.jsx"));
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
  expect(jsx).toContain("<Panel title={name} wide key={match} />");
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
  expect(jsx).toContain("return <Item {...props} key={match} />;");
  expect(jsx).not.toContain("let tmp");
  const { List } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(List([{ label: "a" }, { label: "" }, { label: "b" }]))).toBe("<ul><li>a</li><li>b</li></ul>");
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
  expect(jsx).toContain("<Tag label={described(label)} key={label} />");
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

unsafe extern "Rust" {
    #[link_name = "import"]
    safe fn import_labels(specifier: &str) -> Promise<&'static Labels>;
    #[link_name = "get Label"]
    safe fn label_of(this: &Labels) -> fn(LabelProps) -> JSX::Element;
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
  expect(jsx).toContain('export function Results({ parameters = { hits_per_page: 30, highlighted: ["content"] } }) {');
  const { Results } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Results({}))).toBe('<p title="content">30</p>');
});
