// A crate's declarations (ADR 0196), which `rust-js build` always writes
// (ADR 0412): a `.d.ts` beside each
// module's JS, which TypeScript that imports it is checked against, as
// react.dev's TypeScript is against the components ported to Rust.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

// The settings an app's TypeScript may have, each one that changes what a
// declaration means to it: a `.d.ts` means the same under every one, so an
// app's TypeScript isn't held to more, or less, than the Rust holds it to.
const settings: Record<string, object> = {
  bundler: { module: "esnext", moduleResolution: "bundler", allowJs: true },
  // What Vite's template has, and the strictest of TypeScript's checks that
  // read a declaration: an optional property takes `undefined` only where it
  // says so.
  strictest: { module: "esnext", moduleResolution: "bundler", verbatimModuleSyntax: true, isolatedModules: true, erasableSyntaxOnly: true, exactOptionalPropertyTypes: true },
  nodenext: { module: "nodenext", moduleResolution: "nodenext", allowJs: true },
};

/** TypeScript's errors in `file` of `dir`, which imports a module's JS, the same under each of `settings`; its declarations checked too, `skipLibCheck` off. */
function typeChecked(dir: string, file: string): string[] {
  const each = Object.entries(settings).map(([name, options]) => {
    const config = join(dir, `tsconfig.${name}.json`);
    writeFileSync(config, JSON.stringify({
      compilerOptions: { ...options, jsx: "react-jsx", strict: true, noEmit: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
      files: [file],
    }));
    const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
    return { name, errors: checked.stdout.toString().split("\n").filter((line) => line.includes("error TS")) };
  });
  const where = (errors: string[]) => errors.map((error) => error.slice(0, error.indexOf(":")));
  for (const { name, errors } of each.slice(1)) {
    expect({ [name]: where(errors) }).toEqual({ [name]: where(each[0].errors) });
  }
  return each[0].errors;
}

test("declarations type what a module exports, for TypeScript that imports it", () => {
  buildReact();
  const dir = fixture("declarations");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
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

// Of a reference too, as react.dev's runESLint spreads one.
pub struct Retitled {
    #[rust_js::flatten]
    pub html: &'static Html,
    pub title: u32,
}

pub fn retitled(html: &'static Html) -> Retitled {
    Retitled { html, title: 1 }
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
    pub static Icon: MemoExoticComponent<IconProps> = memo(Svg, None);
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

// A variant without fields, its name's string.
#[rust_js::untagged]
pub enum Fallback {
    Bool(bool),
    #[rust_js::name = "blocking"]
    Blocking,
}

pub fn blocks(fallback: Fallback) -> bool {
    matches!(fallback, Fallback::Blocking)
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

#[rust_js::tag = "pending"]
pub enum Status {
    #[rust_js::name = true]
    Pending { data: String },
    #[rust_js::name = false]
    Idle,
}

pub fn status(s: Status) -> bool {
    matches!(s, Status::Pending { .. })
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

pub struct Tally {
    pub count: std::cell::Cell<u32>,
}

pub struct Shelf {
    pub books: std::cell::RefCell<Vec<String>>,
}

pub fn shelved(shelf: &Shelf, book: String) -> usize {
    shelf.books.borrow_mut().push(book);
    shelf.books.borrow().len()
}

pub fn tallied(cell: &std::cell::Cell<u32>) -> u32 {
    cell.get()
}

pub fn fail(message: &str) -> ! {
    panic!("{message}")
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  // TypeScript prints them, the module's header first (ADR 0207).
  expect(declarations).toStartWith(`// Generated by rust-js from ${join(dir, "lib.rs")}. Do not edit.\n\nimport type`);
  for (const line of [
    'export type RouteTag = "foundation" | "advanced";',
    "export interface TagProps {\n    variant: RouteTag;\n    text?: string | undefined;\n    count: number;\n}",
    "export function Tag(props: TagProps): JSX.Element;",
    "export interface LinkProps<C> {\n    href?: string | undefined;\n    children: C;\n    [prop: string]: unknown;\n}",
    "export function ExternalLink(props: LinkProps<ReactNode>): JSX.Element;",
    "export interface ButtonProps extends Anchor {\n    size?: string | undefined;\n}",
    "export interface CardProps {\n    [prop: string]: unknown;\n}",
    'import type { AnchorHTMLAttributes, JSX, NamedExoticComponent, ReactNode } from "react";\nimport type { Level, RouteItem } from "./routes.js";',
    "export interface CrumbsProps {\n    items: RouteItem[];\n    level: Level;\n}",
    "export interface ChipProps {\n    label: string;\n    count?: number | undefined;\n}",
    "export interface Linked extends Html {\n    href?: string | undefined;\n}",
    'export interface LinkButtonProps extends Omit<Linked, "href" | "className"> {\n    href: string;\n    className?: string | undefined;\n}',
    'export interface TitledProps extends Omit<Html, "title"> {\n    title: string;\n}',
    'export interface Retitled extends Omit<Html, "title"> {\n    title: number;\n}',
    "export interface DownloadProps extends AnchorHTMLAttributes<HTMLAnchorElement> {\n    label: string;\n}",
    "export const Icon: NamedExoticComponent<IconProps>;",
    "export function words(n: bigint, flags: boolean[]): string;",
    "export function first(values: unknown[]): unknown | null | undefined;",
    "export function parsed(text: string): Json | null | undefined;",
    "export function names(numbers: {\n    [key: string]: number;\n}): string[];",
    "type Json = string | number | boolean | (Json | null | undefined)[] | {\n    [key: string]: Json | null | undefined;\n};",
    "export type Size = string | number;",
    'export type Fallback = boolean | "blocking";',
    "export function width(size: Size): string;",
    "export type Items<T> = T[] | string;",
    "export function count(items: Items<number>): number;",
    'export type Shape = "Empty" | {\n    TAG: "Circle";\n    _0: number;\n} | {\n    TAG: "Rect";\n    w: number;\n    h: number;\n};',
    'export type Settled = {\n    status: "fulfilled";\n    value: number;\n} | {\n    status: "pending";\n};',
    // A tagged enum's variants are objects of its tag, those of no fields too.
    'export type Light = {\n    kind: "on";\n} | {\n    kind: "Off";\n};',
    // A tag may be `true` or `false`.
    "export type Status = {\n    pending: true;\n    data: string;\n} | {\n    pending: false;\n};",
    // An `otherwise`, any other object, is the object it holds (ADR 0284).
    'export type Open = {\n    kind: "on";\n} | any;',
    // A `Cell` in a field is what it holds; one lent is `{ value }` (ADR 0288).
    "export interface Tally {\n    count: number;\n}",
    // So is a `RefCell` only ever borrowed for a moment (ADR 0362).
    "export interface Shelf {\n    books: string[];\n}",
    "export function tallied(cell: {\n    value: number;\n}): number;",
    // A function that never returns returns `never` (ADR 0386).
    "export function fail(message: string): never;",
  ]) {
    expect(declarations).toContain(line);
  }
  // TypeScript that uses them: optional props left out, a JS caller's own
  // passed on, a flattened struct's as its own, and wrong ones, the only errors.
  writeFileSync(join(dir, "use.tsx"), `import { Button, Chip, Crumbs, Download, ExternalLink, Icon, LinkButton, Tag, width, words } from "./lib.jsx";
export const ok = [
  <Tag variant="advanced" count={2} />,
  <ExternalLink href="/a" aria-label="A">a</ExternalLink>,
  <Icon className="c" />,
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
  const errors = typeChecked(dir, "use.tsx");
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
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
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

pub struct Meters(pub f64);

pub struct BadgeProps<'a> {
    #[rust_js::nullable]
    pub label: Option<&'a str>,
    pub title: Option<&'a str>,
    pub data: Option<&'a react::js::Unknown>,
    pub width: Option<Meters>,
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export const LanguagesContext: Context<LanguageItem[] | null | undefined>;");
  // A trailing `Option` is optional, left out by a caller (ADR 0443).
  expect(declarations).toContain("export function shown(name?: string | null | undefined): string;");
  expect(declarations).toContain("export interface BadgeProps {\n    label?: string | null | undefined;\n    title?: string | undefined;\n    data?: unknown;\n    width?: any;\n}");
  writeFileSync(join(dir, "use.tsx"), `import { LanguagesContext, shown, type BadgeProps } from "./lib.jsx";
const languages: { code: string }[] | null = null;
export const ok = [<LanguagesContext value={languages}>{shown(null)}</LanguagesContext>, shown(undefined), shown("a"), shown()];
export const badges: BadgeProps[] = [{ label: null }, {}, { label: "a", title: "t" }];
export const wrong: BadgeProps = { title: null };
export const maybe = (title: string | undefined): BadgeProps => ({ title, label: title });
`);
  const errors = typeChecked(dir, "use.tsx");
  // Only the field not marked nullable refuses null.
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(5,");
  const { shown } = await import(join(dir, "lib.jsx"));
  expect([shown(null), shown(undefined), shown("a")]).toEqual(["none", "none", "a"]);
});

// An enum of no variants has no values: TypeScript's `never`, chrono's
// `enum Void {}`, which was `type Void = ;`, no TypeScript at all.
test("an enum of no variants is declared never", () => {
  const dir = fixture("declarations-never");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), "pub enum Void {}\n\npub fn absurd(v: Void) -> u32 {\n    match v {}\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type Void = never;");
  writeFileSync(join(dir, "use.ts"), 'import { absurd, type Void } from "./lib.js";\nexport const never = (v: Void): number => absurd(v);\n');
  expect(typeChecked(dir, "use.ts")).toEqual([]);
});

// A function named as JS keeps a word apart, `new`, is declared by its JS
// name, `new$`, which its JS exports (ADR 0546).
test("declarations name a function as its JS does", () => {
  const dir = fixture("declarations-kept-word");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), "pub fn new() -> u32 {\n    1\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("export function new$()");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function new$(): number;");
  writeFileSync(join(dir, "use.ts"), 'import { new$ } from "./lib.js";\nexport const one: number = new$();\n');
  expect(typeChecked(dir, "use.ts")).toEqual([]);
});

// A style is React's `CSSProperties`, as a `<button>`'s `style` is, where it
// was `any`.
test("declarations type a style as React does", () => {
  buildReact();
  const dir = fixture("declarations-style");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};

pub struct BoxProps {
    pub style: Option<CSSProperties>,
}

pub fn Panel(BoxProps { style }: BoxProps) -> JSX::Element {
    jsx! { <div style={style.unwrap_or(CSSProperties::new())} /> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { CSSProperties, JSX } from "react";');
  expect(declarations).toContain("style?: CSSProperties | undefined;");
  writeFileSync(join(dir, "use.tsx"), `import { Panel } from "./lib.jsx";
export const ok = <Panel style={{ color: "red", fontSize: 12 }} />;
export const wrong = <Panel style={{ color: 1 }} />;
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
});

// A function is typed as Rust types it, react.dev's Button's `onClick` a
// `(event: MouseEvent<Element>) => void`, where it was `(...args: any[]) => any`.
test("declarations type a function by what it takes and gives", () => {
  buildReact();
  const dir = fixture("declarations-functions");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, event, jsx};

pub struct ButtonProps {
    pub on_click: Option<Box<dyn Fn(&event::MouseEvent)>>,
    pub format: fn(u32, u32) -> String,
}

pub fn Button(ButtonProps { on_click, format }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e.upcast()) }}>{format(1, 2)}</button> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { JSX, MouseEvent } from "react";');
  expect(declarations).toContain("onClick?: ((event: MouseEvent<Element>) => void) | undefined;");
  expect(declarations).toContain("format: (value: number, value2: number) => string;");
  writeFileSync(join(dir, "use.tsx"), `import { Button } from "./lib.jsx";
const format = (a: number, b: number) => \`\${a}/\${b}\`;
export const ok = <Button format={format} onClick={(event) => event.currentTarget.tagName} />;
export const wrong = <Button format={format} onClick={(event: number) => event} />;
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(4,");
});

// An `impl Trait` parameter of a trait TypeScript has no type for is
// `unknown`, as a person declares a value of any type, where it was rustc's
// name for it, `impl js::Defined + 'a`, which isn't TypeScript.
test("declarations give an impl Trait parameter of no TypeScript type unknown", () => {
  buildReact();
  const dir = fixture("declarations-impl-trait");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `pub fn size<'a>(value: &'a (impl js::Defined + 'a)) -> u32 {
    let _ = value;
    2
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function size(value: unknown): number;");
});

// A parameter named as a word JS keeps, `new`, is declared as the JS names
// it, `new$`: TypeScript reads no parameter `new`. Found by the pilot's
// `create(new: &NewContact)` once every build wrote declarations.
test("declarations name a parameter as its JS does", () => {
  const dir = fixture("declarations-reserved");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), "pub fn create(new: u32) -> u32 {\n    new + 1\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("export function create(new$) {");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function create(new$: number): number;");
});

// A type written as one of @types/react's aliases is declared by it, as a
// person writes it, `onClick?: MouseEventHandler<HTMLButtonElement>`, an
// argument left to its default left out, `MouseEventHandler`, where
// Rust's expanded type was spelled out, `(event: MouseEvent<..>) => void`.
// A `NodeList` is TypeScript's `NodeListOf` of its items' type.
test("declarations name @types/react's aliases as the Rust names them", () => {
  buildReact();
  const dir = fixture("declarations-aliases");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::event::{ChangeEventHandler, MouseEventHandler};
use react::webapi::{HTMLButtonElement, HTMLElement, HTMLInputElement, NodeList};
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
pub fn lists(elements: &NodeList<HTMLElement>, nodes: &NodeList) -> u32 {
    elements.length() + nodes.length()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("onClick?: MouseEventHandler<HTMLButtonElement> | undefined;");
  expect(declarations).toContain("onChange: ChangeEventHandler<HTMLInputElement>;");
  expect(declarations).toContain("onAny: MouseEventHandler;");
  expect(declarations).toContain("export function attach(target: RefCallback<HTMLElement>): number;");
  expect(declarations).toContain("export function hooks(reducer: Reducer<number, number>, effect: EffectCallback): number;");
  expect(declarations).toContain("export function lists(elements: NodeListOf<HTMLElement>, nodes: NodeListOf<Node>): number;");
  expect(declarations).toContain('import type { ChangeEventHandler, EffectCallback, JSX, MouseEventHandler, Reducer, RefCallback } from "react";');
});

// A component only `js::export_default!` exports is declared, not exported
// by its name, as react.dev's `function Recap() {..} export default Recap;`.
test("declarations declare a private default export", () => {
  buildReact();
  const dir = fixture("declarations-default");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
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
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare function Callout(props: CalloutProps): JSX.Element;\n\nexport default Callout;");
  writeFileSync(join(dir, "use.tsx"), `import Callout from "./lib.jsx";
export const ok = <Callout title="a" />;
export const wrong = <Callout title={1} />;
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
});

// A type alias is TypeScript's, \`export type Toc = TocItem[];\`, which
// TypeScript that imports it names, as react.dev's Toc imports \`Toc\`.
test("declarations name a type alias", () => {
  buildReact();
  const dir = fixture("declarations-alias");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `pub struct TocItem {
    pub url: String,
    pub depth: u32,
}
pub type Toc = Vec<TocItem>;
pub type Pair<T> = (T, T);

pub fn deepest(toc: &Toc) -> u32 {
    toc.iter().map(|item| item.depth).max().unwrap_or(0)
}

pub fn pairs(pair: Pair<Toc>) -> Vec<Toc> {
    vec![pair.0, pair.1]
}

pub struct Page {
    pub toc: Toc,
    pub sections: Option<Vec<Toc>>,
}

thread_local! {
    pub static TOC: react::Context<Toc> = react::create_context(Vec::new());
}

type Depth = u32;

pub fn depth(item: &TocItem) -> Depth {
    item.depth
}

pub mod pages;
`);
  writeFileSync(join(dir, "pages.rs"), `use crate::Toc;

pub fn first(toc: &Toc) -> Option<&str> {
    toc.first().map(|item| item.url.as_str())
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type Toc = TocItem[];");
  // Where the Rust names an alias, so does TypeScript, inside another type too.
  expect(declarations).toContain("export function deepest(toc: Toc): number;");
  expect(declarations).toContain("export function pairs(pair: Pair<Toc>): Toc[];");
  expect(declarations).toContain("toc: Toc;");
  expect(declarations).toContain("sections?: Toc[] | undefined;");
  expect(declarations).toContain("export const TOC: Context<Toc>;");
  // A private one, which isn't declared, is what it stands for.
  expect(declarations).toContain("export function depth(item: TocItem): number;");
  const pages = readFileSync(join(dir, "pages.d.ts"), "utf8");
  expect(pages).toContain("export function first(toc: Toc): string | null | undefined;");
  expect(pages).toMatch(/import type \{ Toc \} from "\.\/lib\.js";/);
  // TypeScript prints a tuple it's given across lines, as its printer does.
  expect(declarations).toContain("export type Pair<T> = [\n    T,\n    T\n];");
  writeFileSync(join(dir, "use.ts"), `import { deepest, type Pair, type Toc } from "./lib.js";
const toc: Toc = [{ url: "#a", depth: 2 }];
export const deep: number = deepest(toc);
export const pair: Pair<number> = [1, 2];
`);
  const errors = typeChecked(dir, "use.ts");
  expect(errors).toEqual([]);
});

// A doc comment is its declaration's JSDoc, a function's, a type's and a
// field's, which a TypeScript caller's editor shows, as react.dev's
// getRouteMeta documents RouteItem (ADR 0444).
test("declarations carry doc comments as JSDoc", () => {
  const dir = fixture("declarations-docs");
  writeFileSync(join(dir, "lib.rs"), `/// A route, in the sidebar.
pub struct RouteItem {
    /// Page title (for the sidebar)
    pub title: String,
    pub depth: u32,
    /// Path to page,
    /// if it has one.
    pub path: Option<String>,
}

/// Its title.
pub fn title(item: &RouteItem) -> &str {
    &item.title
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("/** A route, in the sidebar. */\nexport interface RouteItem {\n    /** Page title (for the sidebar) */\n    title: string;\n    depth: number;\n    /**\n     * Path to page,\n     * if it has one.\n     */\n    path?: string | undefined;\n}");
  expect(declarations).toContain("/** Its title. */\nexport function title(item: RouteItem): string;");
});

// A union is the object of the one field it holds (ADR 0406): to
// TypeScript, a union of one-field objects.
test("declarations type a union as the field it holds", () => {
  const dir = fixture("declarations-union");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub union Number {
    pub int: i32,
    pub float: f64,
}

pub fn make(n: i32) -> Number {
    Number { int: n }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type Number = {\n    int: number;\n} | {\n    float: number;\n};");
  writeFileSync(join(dir, "use.ts"), `import { make, type Number } from "./lib.js";
export const made: Number = make(1);
export const given: Number = { float: 0.5 };
`);
  const errors = typeChecked(dir, "use.ts");
  expect(errors).toEqual([]);
});

// A `pub use` of a module's function is TypeScript's `export .. from`,
// as react.dev's Challenges/index re-exports its Challenges.
test("declarations re-export what a module's pub use names", () => {
  buildReact();
  const dir = fixture("declarations-reexport");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
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
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export { helper, other as renamed } from "./inner.js";');
  writeFileSync(join(dir, "use.ts"), `import { helper, renamed } from "./lib.js";
export const sum: number = helper() + renamed();
export const wrong: string = helper();
`);
  const errors = typeChecked(dir, "use.ts");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.ts(3,");
});

// A `dyn ReactNode` is @types/react's `ReactNode`, as its trait says.
test("declarations type a dyn ReactNode as a ReactNode", () => {
  buildReact();
  const dir = fixture("declarations-dyn-node");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use react::ReactNode;

pub fn parts(text: &str) -> Vec<Box<dyn ReactNode>> {
    text.split('|').map(|part| -> Box<dyn ReactNode> { Box::new(part.to_string()) }).collect()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
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
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, non_upper_case_globals)]
use react::{JSX, MemoExoticComponent, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
}

fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
    jsx! { <b>{text}</b> }
}

thread_local! {
    static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
}

js::export_default!(Memoized);
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
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
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
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
    pub(crate) static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
}

js::export_default!(Memoized);
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
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
test("declarations where @rust-js/typescript isn't say to install @rust-js/build, which has it", () => {
  buildReact();
  const dir = mkdtempSync(join(tmpdir(), "rust-js-no-typescript-"));
  try {
    writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
    writeFileSync(join(dir, "lib.rs"), "pub fn answer(n: u32) -> u32 {\n    n + 1\n}\n");
    const { RUST_JS_TYPESCRIPT: _, ...env } = process.env;
    const failed = Bun.spawnSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { cwd: dir, env });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("install `@rust-js/build`, which has it")]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// A file compiled alone with no Cargo.toml above it has no crate directory
// but its own, where TypeScript is run (ADR 0207). Run nowhere, the error
// was "`node` can't be run: No such file or directory".
test("a file with no Cargo.toml above it has its declarations printed where it is", () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-no-manifest-"));
  try {
    writeFileSync(join(dir, "lib.rs"), "pub fn answer(n: u32) -> u32 {\n    n + 1\n}\n");
    const env = { ...process.env, RUST_JS_TYPESCRIPT: Bun.resolveSync("@rust-js/typescript", root) };
    const p = Bun.spawnSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { cwd: dir, env });
    expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
    expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function answer(n: number): number;");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// React's own element attributes, flattened (ADR 0208): the props are an
// anchor's, as TypeScript's `AnchorHTMLAttributes & ButtonLinkProps` is.
test("props that flatten React's attributes are typed as React types them", async () => {
  buildReact();
  const dir = fixture("declarations-attributes");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::{JSX, ReactNode, jsx};

pub struct ButtonLinkProps<'a, C: ReactNode> {
    pub href: &'a str,
    pub class_name: Option<&'a str>,
    pub children: C,
    #[rust_js::flatten]
    pub props: AnchorHTMLAttributes<'a>,
}

pub fn ButtonLink<C: ReactNode>(ButtonLinkProps { href, class_name, children, props }: ButtonLinkProps<C>) -> JSX::Element {
    jsx! { <a href={href} className={class_name.unwrap_or("button")} {...props}>{children}</a> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { AnchorHTMLAttributes, JSX, ReactNode } from "react";');
  expect(declarations).toContain('export interface ButtonLinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href" | "className"> {\n    href: string;\n    className?: string | undefined;\n    children: ReactNode;\n}');
  writeFileSync(join(dir, "use.tsx"), `import { ButtonLink } from "./lib.jsx";
export const ok = <ButtonLink href="/a" className="c" download="file" aria-label="A" onClick={(event) => event.currentTarget.href}>Go</ButtonLink>;
export const wrong = <ButtonLink href="/a" hrefLang={1}>Go</ButtonLink>;
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(declarations).toContain("export function ButtonLink(props: ButtonLinkProps): JSX.Element;");
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
  const { ButtonLink } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const { renderToStaticMarkup } = await import("react-dom/server");
  expect(renderToStaticMarkup(createElement(ButtonLink, { href: "/a", download: "f", className: "c" }))).toBe('<a href="/a" class="c" download="f"></a>');
});

// An element's props, @types/react's `DetailedHTMLProps`, flattened: react.dev's
// ExternalLink, whose TypeScript callers give it a ref, an object or a
// callback, of its `<a>`, and not of another element (ADR 0384).
test("props that flatten DetailedHTMLProps take a ref as TypeScript types it", async () => {
  buildReact();
  const dir = fixture("declarations-detailed");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::attributes::AnchorHTMLAttributes;
use react::webapi::HTMLAnchorElement;
use react::{DetailedHTMLProps, JSX, ReactNode, jsx};

pub struct ExternalLinkProps<'a, C: ReactNode> {
    pub target: Option<&'a str>,
    pub children: Option<C>,
    #[rust_js::flatten]
    pub props: DetailedHTMLProps<AnchorHTMLAttributes<'a>, HTMLAnchorElement>,
}

pub fn ExternalLink<C: ReactNode>(ExternalLinkProps { target, children, props }: ExternalLinkProps<'_, C>) -> JSX::Element {
    jsx! { <a target={target.unwrap_or("_blank")} rel="noopener" {...props}>{children}</a> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { AnchorHTMLAttributes, DetailedHTMLProps, JSX, ReactNode } from "react";');
  expect(declarations).toContain('export interface ExternalLinkProps extends Omit<DetailedHTMLProps<AnchorHTMLAttributes<HTMLAnchorElement>, HTMLAnchorElement>, "target"> {');
  writeFileSync(join(dir, "use.tsx"), `import { useRef } from "react";
import { ExternalLink } from "./lib.jsx";
export function Uses() {
  const anchor = useRef<HTMLAnchorElement>(null);
  const div = useRef<HTMLDivElement>(null);
  return <>
    <ExternalLink href="/a" ref={anchor}>A</ExternalLink>
    <ExternalLink href="/b" ref={(element) => { element?.focus(); }} aria-label="B">B</ExternalLink>
    <ExternalLink href="/c" ref={div}>C</ExternalLink>
  </>;
}
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors.length, errors.join("\n")).toBe(1);
  expect(errors[0]).toContain("use.tsx(9,");
});

// An SVG element's props, @types/react's `SVGProps`, as a component's whole
// props: react.dev's Logo, spread on to its `<svg>` (ADR 0384).
test("props that are SVGProps are typed as TypeScript types them", async () => {
  buildReact();
  const dir = fixture("declarations-svg-props");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::webapi::SVGSVGElement;
use react::{JSX, SVGProps, jsx};

pub fn Logo(props: SVGProps<'_, SVGSVGElement>) -> JSX::Element {
    jsx! { <svg viewBox="0 0 2 2" {...props}><circle r="1" /></svg> }
}

pub fn Plain() -> JSX::Element {
    jsx! { <Logo {..Default::default()} /> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain('<svg viewBox="0 0 2 2" {...props}>');
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function Logo(props: SVGProps<SVGSVGElement>): JSX.Element;");
  writeFileSync(join(dir, "use.tsx"), `import { useRef } from "react";
import { Logo } from "./lib.jsx";
export function Uses() {
  const svg = useRef<SVGSVGElement>(null);
  const div = useRef<HTMLDivElement>(null);
  return <>
    <Logo className="w-8" ref={svg} />
    <Logo ref={div} />
  </>;
}
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors.length, errors.join("\n")).toBe(1);
  expect(errors[0]).toContain("use.tsx(8,");
  const { Plain } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const { renderToStaticMarkup } = await import("react-dom/server");
  expect(renderToStaticMarkup(createElement(Plain))).toBe('<svg viewBox="0 0 2 2"><circle r="1"></circle></svg>');
});

// A module's private props struct, which its exported component names, is
// declared, not exported, before it, as TypeScript declares react.dev's
// DiagramGroup's `interface DiagramGroupProps`: not `any` (ADR 0385).
test("a private struct an exported function names is declared, not exported", () => {
  buildReact();
  const dir = fixture("declarations-private-props");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, private_interfaces)]
use react::{JSX, ReactNode, jsx};

struct GroupProps<C: ReactNode> {
    children: C,
}

pub fn Group<C: ReactNode>(GroupProps { children }: GroupProps<C>) -> JSX::Element {
    jsx! { <div>{children}</div> }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("interface GroupProps {\n    children: ReactNode;\n}\n\nexport function Group(props: GroupProps): JSX.Element;");
  expect(declarations).not.toContain("export interface GroupProps");
});

// React's attributes are of an element, as @types/react's are: react.dev's
// LoadingOverlay takes `React.HTMLAttributes<HTMLDivElement>`, whose
// handlers' `currentTarget` is the `<div>` (ADR 0208).
test("React's attributes are of the element they're given", () => {
  buildReact();
  const dir = fixture("declarations-attributes-element");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::attributes::HTMLAttributes;
use react::event::MouseEvent;
use react::webapi::HTMLDivElement;
use react::{JSX, jsx};

pub struct PanelProps<'a> {
    pub label: &'a str,
    #[rust_js::flatten]
    pub props: HTMLAttributes<'a, HTMLDivElement>,
}

pub fn Panel(PanelProps { label, props }: PanelProps) -> JSX::Element {
    jsx! { <div {...props}>{label}</div> }
}

pub fn clicked(event: &MouseEvent<HTMLDivElement>) -> String {
    event.current_target().align()
}

pub fn handler() -> HTMLAttributes<'static, HTMLDivElement> {
    HTMLAttributes {
        on_click: Some(Box::new(|event: &MouseEvent<HTMLDivElement>| {
            clicked(event);
        })),
        ..Default::default()
    }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export interface PanelProps extends HTMLAttributes<HTMLDivElement> {\n    label: string;\n}");
  writeFileSync(join(dir, "use.tsx"), `import { Panel } from "./lib.jsx";
export const ok = <Panel label="a" onClick={(event) => event.currentTarget.align} />;
`);
  const errors = typeChecked(dir, "use.tsx");
  expect(errors).toEqual([]);
});

// An `impl Fn(&Item, usize) -> R` parameter is the callback it says, as
// react.dev's toCommaSeparatedList declares `renderCallback: (item: Item,
// index: number) => React.ReactNode`: no `unknown`.
test("declarations type an impl Fn parameter as its function", () => {
  const dir = fixture("declarations-impl-fn");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), "pub fn listed<Item>(array: &[Item], render: impl Fn(&Item, usize) -> u32, done: impl Fn()) -> Vec<u32> {\n    done();\n    array.iter().enumerate().map(|(i, x)| render(x, i)).collect()\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function listed<Item>(array: Item[], render: (item: Item, value: number) => number, done: () => void): number[];");
});
