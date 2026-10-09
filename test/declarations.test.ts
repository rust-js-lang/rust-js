// A crate's `declarations = true` (ADR 0196): a `.d.ts` beside each
// module's JS, which TypeScript that imports it is checked against, as
// react.dev's TypeScript is against the components ported to Rust.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

test("declarations type what a module exports, for TypeScript that imports it", () => {
  buildReact();
  const dir = fixture("declarations");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, MemoExoticComponent, ReactNode, Rest, jsx, memo};

pub enum RouteTag {
    #[rust_js::name = "foundation"]
    Foundation,
    #[rust_js::name = "advanced"]
    Advanced,
}

pub struct TagProps {
    pub variant: RouteTag,
    pub text: Option<&'static str>,
    pub count: u32,
}

pub fn Tag(TagProps { variant, text, count }: TagProps) -> JSX::Element {
    let name = match variant {
        RouteTag::Foundation => "Foundation",
        RouteTag::Advanced => "Advanced",
    };
    jsx! { <span title={text.unwrap_or(name)}>{count}</span> }
}

#[derive(Default)]
pub struct LinkProps<C> {
    pub href: Option<&'static str>,
    pub children: C,
    pub rest: Rest,
}

pub fn ExternalLink<C: ReactNode>(LinkProps { href, children, rest }: LinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} {...rest}>{children}</a> }
}

#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
}

// A flattened field's fields are its parent's: extends Anchor (ADR 0204).
pub struct ButtonProps {
    pub size: Option<&'static str>,
    #[rust_js::flatten]
    pub anchor: Anchor,
}

pub fn Button(ButtonProps { size, anchor }: ButtonProps) -> JSX::Element {
    jsx! { <a className={size} {...anchor} /> }
}

// A chain: what the props have too is theirs, TypeScript's Omit (ADR 0205).
#[derive(Default)]
pub struct Html {
    pub title: Option<&'static str>,
    #[rust_js::name = "className"]
    pub class_name: Option<&'static str>,
}

#[derive(Default)]
pub struct Linked {
    pub href: Option<&'static str>,
    #[rust_js::flatten]
    pub html: Html,
}

pub struct LinkButtonProps {
    pub href: &'static str,
    #[rust_js::name = "className"]
    pub class_name: Option<&'static str>,
    #[rust_js::flatten]
    pub linked: Linked,
}

pub fn LinkButton(LinkButtonProps { href, class_name, linked }: LinkButtonProps) -> JSX::Element {
    jsx! { <a href={href} className={class_name} {...linked} /> }
}

// One name shadowed: Omit<Html, "title">.
pub struct TitledProps {
    pub title: &'static str,
    #[rust_js::flatten]
    pub html: Html,
}

pub fn Titled(TitledProps { title, html }: TitledProps) -> JSX::Element {
    jsx! { <h1 title={title} {...html} /> }
}

// One React types, as its rust_js::types says: what the props extend.
#[rust_js::types = "react#AnchorHTMLAttributes<HTMLAnchorElement>"]
#[derive(Default)]
pub struct ReactAnchor {
    pub download: Option<&'static str>,
}

pub struct DownloadProps {
    pub label: &'static str,
    #[rust_js::flatten]
    pub anchor: ReactAnchor,
}

pub fn Download(DownloadProps { label, anchor }: DownloadProps) -> JSX::Element {
    jsx! { <a {...anchor}>{label}</a> }
}

// Another module's types, imported as types (ADR 0210).
pub mod routes {
    pub struct RouteItem {
        pub title: String,
        pub path: Option<String>,
    }

    pub enum Level {
        #[rust_js::name = "basic"]
        Basic,
        #[rust_js::name = "advanced"]
        Advanced,
    }

    pub fn root() -> RouteItem {
        RouteItem { title: "Home".to_string(), path: None }
    }
}

pub struct CrumbsProps {
    pub items: Vec<routes::RouteItem>,
    pub level: routes::Level,
}

pub fn Crumbs(CrumbsProps { items, level }: CrumbsProps) -> JSX::Element {
    let label = match level {
        routes::Level::Basic => "basic",
        routes::Level::Advanced => "advanced",
    };
    jsx! { <nav title={label}>{items.len()}</nav> }
}

// A field with a default is one a caller may leave out (ADR 0212).
pub struct ChipProps {
    pub label: &'static str,
    #[rust_js::default]
    pub count: u32,
}

pub fn Chip(ChipProps { label, count }: ChipProps) -> JSX::Element {
    jsx! { <b title={label}>{count}</b> }
}

pub mod far {
    #[derive(Default)]
    pub struct Far {
        pub id: Option<&'static str>,
    }
}

// One of another module, which TypeScript here can't see, is a Rest's.
pub struct CardProps {
    #[rust_js::flatten]
    pub far: far::Far,
}

pub fn Card(CardProps { far }: CardProps) -> JSX::Element {
    jsx! { <b {...far} /> }
}

pub struct IconProps {
    pub class_name: Option<&'static str>,
}

fn Svg(props: IconProps) -> JSX::Element {
    jsx! { <svg className={props.class_name} /> }
}

thread_local! {
    pub static Icon: MemoExoticComponent<IconProps> = memo(Svg);
}

pub fn words(n: u64, flags: Vec<bool>) -> String {
    format!("{n} {}", flags.len())
}

// A value of unknown shape is TypeScript's unknown (ADR 0225).
pub fn first(values: Vec<&'static react::js::Unknown>) -> Option<&'static react::js::Unknown> {
    values.first().copied()
}

// JSON is TypeScript's union of what it may be, declared here as another
// crate's untagged enum is, and a dictionary a Record (ADR 0225).
pub fn parsed(text: &str) -> Option<react::js::Json<'static>> {
    react::js::Json::parse(text).ok().flatten()
}

pub fn names(numbers: &react::js::Dict<f64>) -> Vec<String> {
    react::js::dict::keys(numbers)
}

// An untagged enum is TypeScript's union of its payloads (ADR 0214).
#[rust_js::untagged]
pub enum Size {
    Named(&'static str),
    Pixels(f64),
}

pub fn width(size: Size) -> String {
    match size {
        Size::Named(name) => name.to_string(),
        Size::Pixels(n) => format!("{n}px"),
    }
}

// Of its type parameters too.
#[rust_js::untagged]
pub enum Items<T> {
    Many(Vec<T>),
    Label(&'static str),
}

pub fn count(items: Items<u32>) -> usize {
    match items {
        Items::Many(all) => all.len(),
        Items::Label(_) => 1,
    }
}

// An enum with fields is its tagged objects' union, a variant without its
// name (ADR 0033); a discriminated union's, each an object (ADR 0284).
pub enum Shape {
    Empty,
    Circle(f64),
    Rect { w: f64, h: f64 },
}

pub fn area(shape: Shape) -> f64 {
    match shape {
        Shape::Empty => 0.0,
        Shape::Circle(r) => r * r,
        Shape::Rect { w, h } => w * h,
    }
}

#[rust_js::tag = "status"]
pub enum Settled {
    #[rust_js::name = "fulfilled"]
    Fulfilled { value: u32 },
    #[rust_js::name = "pending"]
    Pending,
}

pub fn settled(s: Settled) -> u32 {
    match s {
        Settled::Fulfilled { value } => value,
        Settled::Pending => 0,
    }
}

#[rust_js::tag = "kind"]
pub enum Light {
    #[rust_js::name = "on"]
    On,
    Off,
}

pub fn lit(light: Light) -> bool {
    matches!(light, Light::On)
}

#[rust_js::tag = "kind"]
pub enum Open {
    #[rust_js::name = "on"]
    On,
    #[rust_js::otherwise]
    Other(&'static js::JsObject),
}

pub fn on(open: &Open) -> bool {
    matches!(open, Open::On)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  // TypeScript prints them, the module's header first (ADR 0207).
  expect(declarations).toStartWith(`// Generated by rust-js from ${join(dir, "lib.rs")}. Do not edit.\n\nimport type`);
  for (const line of [
    'export type RouteTag = "foundation" | "advanced";',
    "export interface TagProps {\n    variant: RouteTag;\n    text?: string;\n    count: number;\n}",
    "export function Tag(props: TagProps): JSX.Element;",
    "export interface LinkProps<C> {\n    href?: string;\n    children: C;\n    [prop: string]: unknown;\n}",
    "export function ExternalLink(props: LinkProps<ReactNode>): JSX.Element;",
    "export interface ButtonProps extends Anchor {\n    size?: string;\n}",
    "export interface CardProps {\n    [prop: string]: unknown;\n}",
    'import type { AnchorHTMLAttributes, JSX, NamedExoticComponent, ReactNode } from "react";\nimport type { Level, RouteItem } from "./routes.js";',
    "export interface CrumbsProps {\n    items: RouteItem[];\n    level: Level;\n}",
    "export interface ChipProps {\n    label: string;\n    count?: number;\n}",
    "export interface Linked extends Html {\n    href?: string;\n}",
    'export interface LinkButtonProps extends Omit<Linked, "href" | "className"> {\n    href: string;\n    className?: string;\n}',
    'export interface TitledProps extends Omit<Html, "title"> {\n    title: string;\n}',
    "export interface DownloadProps extends AnchorHTMLAttributes<HTMLAnchorElement> {\n    label: string;\n}",
    "export const Icon: NamedExoticComponent<IconProps>;",
    "export function words(n: bigint, flags: boolean[]): string;",
    "export function first(values: unknown[]): unknown | null | undefined;",
    "export function parsed(text: string): Json | null | undefined;",
    "export function names(numbers: {\n    [key: string]: number;\n}): string[];",
    "type Json = string | number | boolean | (Json | null | undefined)[] | {\n    [key: string]: Json | null | undefined;\n};",
    "export type Size = string | number;",
    "export function width(size: Size): string;",
    "export type Items<T> = T[] | string;",
    "export function count(items: Items<number>): number;",
    'export type Shape = "Empty" | {\n    TAG: "Circle";\n    _0: number;\n} | {\n    TAG: "Rect";\n    w: number;\n    h: number;\n};',
    'export type Settled = {\n    status: "fulfilled";\n    value: number;\n} | {\n    status: "pending";\n};',
    // A tagged enum's variants are objects of its tag, those of no fields too.
    'export type Light = {\n    kind: "on";\n} | {\n    kind: "Off";\n};',
    // An `otherwise`, any other object, is the object it holds (ADR 0284).
    'export type Open = {\n    kind: "on";\n} | any;',
  ]) {
    expect(declarations).toContain(line);
  }
  // TypeScript that uses them: optional props left out, a JS caller's own
  // passed on, a flattened struct's as its own, and wrong ones, the only errors.
  writeFileSync(join(dir, "use.tsx"), `import { Button, Chip, Crumbs, Download, ExternalLink, Icon, LinkButton, Tag, width, words } from "./lib.jsx";
export const ok = [
  <Tag variant="advanced" count={2} />,
  <ExternalLink href="/a" aria-label="A">a</ExternalLink>,
  <Icon class_name="c" />,
  words(1n, [true]),
  <Button size="lg" href="/b" target="_blank" />,
  <LinkButton href="/c" title="t" />,
  <Download label="d" download="file" referrerPolicy="no-referrer" />,
  <Crumbs items={[{ title: "Home" }, { title: "Learn", path: "/learn" }]} level="basic" />,
  <Chip label="c" />,
  width("lg"),
  width(3),
];
export const wrong = <Tag variant="intermediate" count={2} />;
export const wrongHref = <Button href={1} />;
export const wrongLevel = <Crumbs items={[]} level="expert" />;
export const wrongSize = width(true);
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    // Its declarations checked too, as TypeScript reads them (ADR 0207).
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(4);
  expect(errors[0]).toContain("use.tsx(15,");
  expect(errors[0]).toContain('"intermediate"');
  // A flattened struct's field is checked as the component's own.
  expect(errors[1]).toContain("use.tsx(16,");
  // Another module's type, as it declares it.
  expect(errors[2]).toContain("use.tsx(17,");
  // Not one of an untagged enum's kinds.
  expect(errors[3]).toContain("use.tsx(18,");
});

// An `Option` is `None` of JS's `null` too, read `!= null` (ADR 0030):
// declared so, a TypeScript caller gives one, as react.dev's Page gives its
// `LanguagesContext` `Languages | null`. A field of one is `?: T`, as
// TypeScript's own data has it, but one marked `#[rust_js::nullable]`,
// `?: T | null`, as react.dev's errors page gives its `errorMessage`.
test("declarations take JS's null for an Option", async () => {
  buildReact();
  const dir = fixture("declarations-null");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{Context, create_context};

pub struct LanguageItem {
    pub code: String,
}

thread_local! {
    pub static LanguagesContext: Context<Option<Vec<LanguageItem>>> = create_context(None);
}

pub fn shown(name: Option<&str>) -> String {
    name.unwrap_or("none").to_string()
}

pub struct BadgeProps<'a> {
    #[rust_js::nullable]
    pub label: Option<&'a str>,
    pub title: Option<&'a str>,
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export const LanguagesContext: Context<LanguageItem[] | null | undefined>;");
  expect(declarations).toContain("export function shown(name: string | null | undefined): string;");
  expect(declarations).toContain("export interface BadgeProps {\n    label?: string | null;\n    title?: string;\n}");
  writeFileSync(join(dir, "use.tsx"), `import { LanguagesContext, shown, type BadgeProps } from "./lib.jsx";
const languages: { code: string }[] | null = null;
export const ok = [<LanguagesContext value={languages}>{shown(null)}</LanguagesContext>, shown(undefined), shown("a")];
export const badges: BadgeProps[] = [{ label: null }, {}, { label: "a", title: "t" }];
export const wrong: BadgeProps = { title: null };
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  // Only the field not marked nullable refuses null.
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(5,");
  const { shown } = await import(join(dir, "lib.jsx"));
  expect([shown(null), shown(undefined), shown("a")]).toEqual(["none", "none", "a"]);
});

// A style is React's `CSSProperties`, as a `<button>`'s `style` is, where it
// was `any`.
test("declarations type a style as React does", () => {
  buildReact();
  const dir = fixture("declarations-style");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};

pub struct BoxProps {
    pub style: Option<CSSProperties>,
}

pub fn Panel(BoxProps { style }: BoxProps) -> JSX::Element {
    jsx! { <div style={style.unwrap_or(CSSProperties::new())} /> }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { CSSProperties, JSX } from "react";');
  expect(declarations).toContain("style?: CSSProperties;");
  writeFileSync(join(dir, "use.tsx"), `import { Panel } from "./lib.jsx";
export const ok = <Panel style={{ color: "red", fontSize: 12 }} />;
export const wrong = <Panel style={{ color: 1 }} />;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
});

// A function is typed as Rust types it, react.dev's Button's `onClick` a
// `(event: MouseEvent<Element>) => void`, where it was `(...args: any[]) => any`.
test("declarations type a function by what it takes and gives", () => {
  buildReact();
  const dir = fixture("declarations-functions");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, event, jsx};

pub struct ButtonProps {
    #[rust_js::name = "onClick"]
    pub on_click: Option<Box<dyn Fn(&event::MouseEvent)>>,
    pub format: fn(u32, u32) -> String,
}

pub fn Button(ButtonProps { on_click, format }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e.upcast()) }}>{format(1, 2)}</button> }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { JSX, MouseEvent } from "react";');
  expect(declarations).toContain("onClick?: (event: MouseEvent<Element>) => void;");
  expect(declarations).toContain("format: (value: number, value2: number) => string;");
  writeFileSync(join(dir, "use.tsx"), `import { Button } from "./lib.jsx";
const format = (a: number, b: number) => \`\${a}/\${b}\`;
export const ok = <Button format={format} onClick={(event) => event.currentTarget.tagName} />;
export const wrong = <Button format={format} onClick={(event: number) => event} />;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(4,");
});

// An `impl Trait` parameter of a trait TypeScript has no type for is
// `unknown`, as a person declares a value of any type, where it was rustc's
// name for it, `impl js::Defined + 'a`, which isn't TypeScript.
test("declarations give an impl Trait parameter of no TypeScript type unknown", () => {
  const dir = fixture("declarations-impl-trait");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `pub fn size<'a>(value: &'a (impl js::Defined + 'a)) -> u32 {
    let _ = value;
    2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function size(value: unknown): number;");
});

// A type written as one of @types/react's aliases is declared by it, as a
// person writes it, `on_click?: MouseEventHandler<HTMLButtonElement>`, an
// argument left to its default left out, `MouseEventHandler`, where
// Rust's expanded type was spelled out, `(event: MouseEvent<..>) => void`.
test("declarations name @types/react's aliases as the Rust names them", () => {
  buildReact();
  const dir = fixture("declarations-aliases");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::event::{ChangeEventHandler, MouseEventHandler};
use react::webapi::{HTMLButtonElement, HTMLElement, HTMLInputElement};
use react::{EffectCallback, JSX, Reducer, RefCallback, jsx};
pub struct ButtonProps {
    pub on_click: Option<MouseEventHandler<HTMLButtonElement>>,
    pub on_change: ChangeEventHandler<HTMLInputElement>,
    pub on_any: MouseEventHandler,
}
pub fn Button(ButtonProps { on_click, on_change, on_any }: ButtonProps) -> JSX::Element {
    let _ = (on_click, on_change, on_any);
    jsx! { <button /> }
}
pub fn attach(target: RefCallback<&'static HTMLElement>) -> u32 {
    let _ = target;
    1
}
pub fn hooks(reducer: Reducer<i32, i32>, effect: EffectCallback) -> u32 {
    let _ = (reducer, effect);
    2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("on_click?: MouseEventHandler<HTMLButtonElement>;");
  expect(declarations).toContain("on_change: ChangeEventHandler<HTMLInputElement>;");
  expect(declarations).toContain("on_any: MouseEventHandler;");
  expect(declarations).toContain("export function attach(target: RefCallback<HTMLElement>): number;");
  expect(declarations).toContain("export function hooks(reducer: Reducer<number, number>, effect: EffectCallback): number;");
  expect(declarations).toContain('import type { ChangeEventHandler, EffectCallback, JSX, MouseEventHandler, Reducer, RefCallback } from "react";');
});

// A component only `js::export_default!` exports is declared, not exported
// by its name, as react.dev's `function Recap() {..} export default Recap;`.
test("declarations declare a private default export", () => {
  buildReact();
  const dir = fixture("declarations-default");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct CalloutProps<'a> {
    pub title: &'a str,
}

fn Callout(CalloutProps { title }: CalloutProps) -> JSX::Element {
    jsx! { <b>{title}</b> }
}

js::export_default!(Callout);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare function Callout(props: CalloutProps): JSX.Element;\n\nexport default Callout;");
  writeFileSync(join(dir, "use.tsx"), `import Callout from "./lib.jsx";
export const ok = <Callout title="a" />;
export const wrong = <Callout title={1} />;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
});

// A type alias is TypeScript's, \`export type Toc = TocItem[];\`, which
// TypeScript that imports it names, as react.dev's Toc imports \`Toc\`.
test("declarations name a type alias", () => {
  buildReact();
  const dir = fixture("declarations-alias");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `pub struct TocItem {
    pub url: String,
    pub depth: u32,
}
pub type Toc = Vec<TocItem>;
pub type Pair<T> = (T, T);

pub fn deepest(toc: &Toc) -> u32 {
    toc.iter().map(|item| item.depth).max().unwrap_or(0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type Toc = TocItem[];");
  // TypeScript prints a tuple it's given across lines, as its printer does.
  expect(declarations).toContain("export type Pair<T> = [\n    T,\n    T\n];");
  writeFileSync(join(dir, "use.ts"), `import { deepest, type Pair, type Toc } from "./lib.js";
const toc: Toc = [{ url: "#a", depth: 2 }];
export const deep: number = deepest(toc);
export const pair: Pair<number> = [1, 2];
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false },
    files: ["use.ts"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  expect(checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"))).toEqual([]);
});

// A `pub use` of a module's function is TypeScript's `export .. from`,
// as react.dev's Challenges/index re-exports its Challenges.
test("declarations re-export what a module's pub use names", () => {
  buildReact();
  const dir = fixture("declarations-reexport");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `mod inner;
pub use inner::helper;
pub use inner::other as renamed;
`);
  writeFileSync(join(dir, "inner.rs"), `pub fn helper() -> u32 {
    1
}

pub fn other() -> u32 {
    2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export { helper, other as renamed } from "./inner.js";');
  writeFileSync(join(dir, "use.ts"), `import { helper, renamed } from "./lib.js";
export const sum: number = helper() + renamed();
export const wrong: string = helper();
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false },
    files: ["use.ts"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.ts(3,");
});

// A `dyn ReactNode` is @types/react's `ReactNode`, as its trait says.
test("declarations type a dyn ReactNode as a ReactNode", () => {
  buildReact();
  const dir = fixture("declarations-dyn-node");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `use react::ReactNode;

pub fn parts(text: &str) -> Vec<Box<dyn ReactNode>> {
    text.split('|').map(|part| -> Box<dyn ReactNode> { Box::new(part.to_string()) }).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function parts(text: string): ReactNode[];");
  expect(declarations).toContain('import type { ReactNode } from "react";');
});

// A memo'd component kept in a `thread_local!` is a module's default
// export too, as react.dev's CodeBlock/index has `export default
// memo(function CodeBlockWrapper ..)`.
test("declarations declare a thread-local default export", async () => {
  buildReact();
  const dir = fixture("declarations-default-static");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, non_upper_case_globals)]
use react::{JSX, MemoExoticComponent, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
}

fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
    jsx! { <b>{text}</b> }
}

thread_local! {
    static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label);
}

js::export_default!(Memoized);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("export default Memoized;");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare const Memoized: NamedExoticComponent<LabelProps>;\n\nexport default Memoized;");
  const { createElement } = await import("react");
  const { renderToStaticMarkup } = await import("react-dom/server");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(lib.default, { text: "hi" }))).toBe("<b>hi</b>");
});

// A module's default export, imported by another of its crate, is its
// default import, named as the `use` names it, as react.dev's CodeDiagram
// has `import CodeBlock from './CodeBlock'`, which exports nothing else.
test("a module's default export is imported as one, by its use's name", async () => {
  buildReact();
  const dir = fixture("default-import");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, non_upper_case_globals)]
mod label;

use label::Memoized as Label;
use react::{JSX, jsx};

pub fn Page() -> JSX::Element {
    jsx! { <main><Label text="hi" /></main> }
}
`);
  writeFileSync(join(dir, "label.rs"), `use react::{JSX, MemoExoticComponent, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
}

fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
    jsx! { <b>{text}</b> }
}

thread_local! {
    pub(crate) static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label);
}

js::export_default!(Memoized);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const page = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(page).toContain('import Label from "./label.jsx";');
  expect(page).toContain('<Label text="hi" />');
  const label = readFileSync(join(dir, "label.jsx"), "utf8");
  expect(label).toContain("const Memoized = memo(Label);");
  expect(label).not.toContain("export const Memoized");
  const { renderToStaticMarkup } = await import("react-dom/server");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page())).toBe("<main><b>hi</b></main>");
});

// TypeScript prints them (ADR 0207), through @rust-js/typescript, which a
// crate without it is told to add, its build failing.
test("declarations where @rust-js/typescript isn't say to add it", () => {
  buildReact();
  const dir = mkdtempSync(join(tmpdir(), "rust-js-no-typescript-"));
  try {
    writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
    writeFileSync(join(dir, "lib.rs"), "pub fn answer(n: u32) -> u32 {\n    n + 1\n}\n");
    const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { cwd: dir });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("npm install --save-dev @rust-js/typescript")]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// React's own element attributes, flattened (ADR 0208): the props are an
// anchor's, as TypeScript's `AnchorHTMLAttributes & ButtonLinkProps` is.
test("props that flatten React's attributes are typed as React types them", async () => {
  buildReact();
  const dir = fixture("declarations-attributes");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::{JSX, ReactNode, jsx};

pub struct ButtonLinkProps<'a, C: ReactNode> {
    pub href: &'a str,
    #[rust_js::name = "className"]
    pub class_name: Option<&'a str>,
    pub children: C,
    #[rust_js::flatten]
    pub props: AnchorHTMLAttributes<'a>,
}

pub fn ButtonLink<C: ReactNode>(ButtonLinkProps { href, class_name, children, props }: ButtonLinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} className={class_name.unwrap_or("button")} {...props}>{children}</a> }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { AnchorHTMLAttributes, JSX, ReactNode } from "react";');
  expect(declarations).toContain('export interface ButtonLinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href" | "className"> {\n    href: string;\n    className?: string;\n    children: ReactNode;\n}');
  writeFileSync(join(dir, "use.tsx"), `import { ButtonLink } from "./lib.jsx";
export const ok = <ButtonLink href="/a" className="c" download="file" aria-label="A" onClick={(event) => event.currentTarget.href}>Go</ButtonLink>;
export const wrong = <ButtonLink href="/a" hrefLang={1}>Go</ButtonLink>;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(declarations).toContain("export function ButtonLink(props: ButtonLinkProps): JSX.Element;");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
  const { ButtonLink } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const { renderToStaticMarkup } = await import("react-dom/server");
  expect(renderToStaticMarkup(createElement(ButtonLink, { href: "/a", download: "f", className: "c" }))).toBe('<a href="/a" class="c" download="f"></a>');
});

// An `impl Fn(&Item, usize) -> R` parameter is the callback it says, as
// react.dev's toCommaSeparatedList declares `renderCallback: (item: Item,
// index: number) => React.ReactNode`: no `unknown`.
test("declarations type an impl Fn parameter as its function", () => {
  const dir = fixture("declarations-impl-fn");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), "pub fn listed<Item>(array: &[Item], render: impl Fn(&Item, usize) -> u32, done: impl Fn()) -> Vec<u32> {\n    done();\n    array.iter().enumerate().map(|(i, x)| render(x, i)).collect()\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function listed<Item>(array: Item[], render: (item: Item, value: number) => number, done: () => void): number[];");
});
