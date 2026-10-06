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
use react::{Element, Memo, Node, Rest, jsx, memo};

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

pub fn Tag(TagProps { variant, text, count }: TagProps) -> Element {
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

pub fn ExternalLink<C: Node>(LinkProps { href, children, rest }: LinkProps<C>) -> Element {
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

pub fn Button(ButtonProps { size, anchor }: ButtonProps) -> Element {
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

pub fn LinkButton(LinkButtonProps { href, class_name, linked }: LinkButtonProps) -> Element {
    jsx! { <a href={href} className={class_name} {...linked} /> }
}

// One name shadowed: Omit<Html, "title">.
pub struct TitledProps {
    pub title: &'static str,
    #[rust_js::flatten]
    pub html: Html,
}

pub fn Titled(TitledProps { title, html }: TitledProps) -> Element {
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

pub fn Download(DownloadProps { label, anchor }: DownloadProps) -> Element {
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

pub fn Crumbs(CrumbsProps { items, level }: CrumbsProps) -> Element {
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

pub fn Chip(ChipProps { label, count }: ChipProps) -> Element {
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

pub fn Card(CardProps { far }: CardProps) -> Element {
    jsx! { <b {...far} /> }
}

pub struct IconProps {
    pub class_name: Option<&'static str>,
}

fn Svg(props: IconProps) -> Element {
    jsx! { <svg className={props.class_name} /> }
}

thread_local! {
    pub static Icon: Memo<IconProps> = memo(Svg);
}

pub fn words(n: u64, flags: Vec<bool>) -> String {
    format!("{n} {}", flags.len())
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
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  // TypeScript prints them, the module's header first (ADR 0207).
  expect(declarations).toStartWith(`// Generated by rust-js from ${join(dir, "lib.rs")}. Do not edit.\n\nimport type`);
  for (const line of [
    'export type RouteTag = "foundation" | "advanced";',
    "export interface TagProps {\n    variant: RouteTag;\n    text?: string;\n    count: number;\n}",
    "export function Tag(props: TagProps): ReactNode;",
    "export interface LinkProps<C> {\n    href?: string;\n    children: C;\n    [prop: string]: unknown;\n}",
    "export function ExternalLink<C>(props: LinkProps<C>): ReactNode;",
    "export interface ButtonProps extends Anchor {\n    size?: string;\n}",
    "export interface CardProps {\n    [prop: string]: unknown;\n}",
    'import type { AnchorHTMLAttributes, NamedExoticComponent, ReactNode } from "react";\nimport type { Level, RouteItem } from "./routes.js";',
    "export interface CrumbsProps {\n    items: RouteItem[];\n    level: Level;\n}",
    "export interface ChipProps {\n    label: string;\n    count?: number;\n}",
    "export interface Linked extends Html {\n    href?: string;\n}",
    'export interface LinkButtonProps extends Omit<Linked, "href" | "className"> {\n    href: string;\n    className?: string;\n}',
    'export interface TitledProps extends Omit<Html, "title"> {\n    title: string;\n}',
    "export interface DownloadProps extends AnchorHTMLAttributes<HTMLAnchorElement> {\n    label: string;\n}",
    "export const Icon: NamedExoticComponent<IconProps>;",
    "export function words(n: bigint, flags: boolean[]): string;",
    "export type Size = string | number;",
    "export function width(size: Size): string;",
    "export type Items<T> = T[] | string;",
    "export function count(items: Items<number>): number;",
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
use react::attributes::AnchorHtmlAttributes;
use react::{Element, jsx};

pub struct ButtonLinkProps<'a> {
    pub href: &'a str,
    #[rust_js::name = "className"]
    pub class_name: Option<&'a str>,
    #[rust_js::flatten]
    pub props: AnchorHtmlAttributes<'a>,
}

pub fn ButtonLink(ButtonLinkProps { href, class_name, props }: ButtonLinkProps) -> Element {
    jsx! { <a href={href} className={class_name.unwrap_or("button")} {...props} /> }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { AnchorHTMLAttributes, ReactNode } from "react";');
  expect(declarations).toContain('export interface ButtonLinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href" | "className"> {');
  writeFileSync(join(dir, "use.tsx"), `import { ButtonLink } from "./lib.jsx";
export const ok = <ButtonLink href="/a" className="c" download="file" aria-label="A" onClick={(event) => event.currentTarget.href} />;
export const wrong = <ButtonLink href="/a" hrefLang={1} />;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.tsx(3,");
  const { ButtonLink } = await import(join(dir, "lib.jsx"));
  const { createElement } = await import("react");
  const { renderToStaticMarkup } = await import("react-dom/server");
  expect(renderToStaticMarkup(createElement(ButtonLink, { href: "/a", download: "f", className: "c" }))).toBe('<a href="/a" class="c" download="f"></a>');
});
