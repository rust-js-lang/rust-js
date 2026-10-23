// What JSX tags, attributes, children and components take, as @types/react
// types them: intrinsic elements and SVG, contexts and portals, React's
// `Children`, a tag that's a value, keys and nodes of any shape. Each test is
// a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
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
  expect(jsx).toContain("<Theme.Consumer>{(theme) => <b>{theme}</b>}</Theme.Consumer>");
  expect(jsx).toContain('const portal = createPortal(<i>away</i>, container);\n  return portal;');
  const { Label, Theme } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  expect([
    renderToStaticMarkup(createElement(Label)),
    renderToStaticMarkup(createElement(Theme, { value: "dark" }, createElement(Label))),
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
  expect(code).toContain('<Theme.Provider value="text">plain</Theme.Provider>');
  const result = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(result.App))).toBe("<i>a</i><i>b</i><b>dark</b><b>legacy</b>plain");
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
            Child::Element(element) => match js::classify(Some(element.r#type())) {
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
        ReactNodeKind::Element(element) => js::get(element.props(), "id"),
        _ => None,
    };
    jsx! { <p>{if let Kind::String(id) = js::classify(id) { id } else { "none" }}</p> }
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
        && let Kind::String(text) = classify(Some(inner))
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

// An element of what JS gives at run time, as react.dev's errors page
// revives its elements from JSON: its type, its key and its props each a
// `js::Unknown`, a wrapper's `Fragment` (ADR 0268).
test("an element's type, key and props may be JS values of any shape", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::{Kind, Unknown, classify};
use react::{ElementType, FRAGMENT, JSX, jsx};

pub fn revived(r#type: &'static Unknown, key: &'static Unknown, props: &'static Unknown) -> JSX::Element {
    let mut Type = ElementType::from_unknown(r#type);
    if matches!(classify(Some(r#type)), Kind::String("wrapper")) {
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
