// What `jsx!` gives a component as its props: named props with the rest from a
// base, a field's default, flattened structs as a component's own props,
// `...rest`, and props spread through a reference. Each test is a program of
// its own, compiled and run.

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
  expect(jsx).toContain('<ExternalLink ref={(anchor) => {}} href="/b">');
  const { ByObject, ByCallback } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(ByObject))).toBe('<a target="_blank" rel="noopener" href="/a">A</a>');
  expect(renderToStaticMarkup(createElement(ByCallback))).toBe('<a target="_blank" rel="noopener" href="/b">B</a>');
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
