// A crate's `declarations = true` (ADR 0196): a `.d.ts` beside each
// module's JS, which TypeScript that imports it is checked against, as
// react.dev's TypeScript is against the components ported to Rust.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
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
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  for (const line of [
    'import type { NamedExoticComponent, ReactNode } from "react";',
    'export type RouteTag = "foundation" | "advanced";',
    "export interface TagProps {\n  variant: RouteTag;\n  text?: string;\n  count: number;\n}",
    "export function Tag(props: TagProps): ReactNode;",
    "export interface LinkProps<C> {\n  href?: string;\n  children: C;\n  [prop: string]: unknown;\n}",
    "export function ExternalLink<C>(props: LinkProps<C>): ReactNode;",
    "export interface ButtonProps extends Anchor {\n  size?: string;\n}",
    "export interface CardProps {\n  [prop: string]: unknown;\n}",
    "export const Icon: NamedExoticComponent<IconProps>;",
    "export function words(n: bigint, flags: boolean[]): string;",
  ]) {
    expect(declarations).toContain(line);
  }
  // TypeScript that uses them: optional props left out, a JS caller's own
  // passed on, a flattened struct's as its own, and wrong ones, the only errors.
  writeFileSync(join(dir, "use.tsx"), `import { Button, ExternalLink, Icon, Tag, words } from "./lib.jsx";
export const ok = [
  <Tag variant="advanced" count={2} />,
  <ExternalLink href="/a" aria-label="A">a</ExternalLink>,
  <Icon class_name="c" />,
  words(1n, [true]),
  <Button size="lg" href="/b" target="_blank" />,
];
export const wrong = <Tag variant="intermediate" count={2} />;
export const wrongHref = <Button href={1} />;
`);
  writeFileSync(join(dir, "tsconfig.json"), JSON.stringify({
    compilerOptions: { jsx: "react-jsx", strict: true, noEmit: true, module: "esnext", moduleResolution: "bundler", allowJs: true, skipLibCheck: true, typeRoots: [join(root, "node_modules/@types")] },
    files: ["use.tsx"],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(2);
  expect(errors[0]).toContain("use.tsx(9,");
  expect(errors[0]).toContain('"intermediate"');
  // A flattened struct's field is checked as the component's own.
  expect(errors[1]).toContain("use.tsx(10,");
});
