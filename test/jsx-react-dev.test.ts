// react.dev's components as JSX writes them, SidebarLink, Preview, Page,
// NavigationBar, Diagram, SandboxShell and InlineToc, and what MIR holds ahead
// of their children read in place. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, root, run, target } from "./support";

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
pub fn cn(classes: impl js::Rest<dyn js::IntoUnknown>) -> String {
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
fn scroll_into_view_if_needed_of(this: &HTMLAnchorElement) -> Option<&'static Unknown> {
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
            className={Some(cn((
                Some("p-2 pe-2 w-full rounded-none lg:rounded-e-2xl text-start hover:bg-gray-5 dark:hover:bg-gray-80 relative flex items-center justify-between"),
                (level > 0).then_some("text-sm ps-6"),
                (level < 2).then_some("ps-5"),
                (level == 0).then_some("text-base font-bold"),
                (level == 0 && !selected).then_some("text-primary dark:text-primary-dark"),
                (level > 0 && !selected).then_some("text-base text-secondary dark:text-secondary-dark"),
                selected.then_some("text-base text-link dark:text-link-dark bg-highlight dark:bg-highlight-dark border-blue-40 hover:bg-highlight hover:text-link dark:hover:bg-highlight-dark dark:hover:text-link-dark"),
                is_pending.then_some("dark:bg-gray-70 bg-gray-3 dark:hover:bg-gray-70 hover:bg-gray-3"),
            )).as_str())}>
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
                    className={cn((
                        Some("pe-1"),
                        is_expanded.unwrap_or(false).then_some("text-link dark:text-link-dark"),
                        (!is_expanded.unwrap_or(false)).then_some("text-tertiary dark:text-tertiary-dark"),
                    ))}>
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
    let (bundlerIsReady, _) = use_state(|| false);
    let (iframeComputedHeight, _) = use_state::<Option<f64>>(|| None);
    let (error, _) = use_state(move || rawError);
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
  expect(jsx).toContain("{error != null && (");
  expect(jsx).not.toContain("const condition");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Preview(true, { message: "boom" })))).toBe('<div><div class="z-50 sticky top-8 "><pre>boom</pre></div><p></p></div>');
  expect(renderToStaticMarkup(createElement(() => lib.Preview(false, undefined)))).toBe("<div><p></p></div>");
});

// A negation of what reads the same, a state's \`!hidden\`, reads the same
// after a later child's statements, a block's, so a prop of it is in place,
// not \`const disabled = !hidden\` before them.
test("a negation of a state's value is read in place before a later child's statements", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub fn Shown() -> JSX::Element {
    let (hidden, _) = use_state(|| false);
    let hidden = *hidden;
    jsx! {
        <div>
            <input disabled={!hidden} />
            <span>{{
                let mut parts = vec!["a"];
                parts.push("b");
                parts.join("-")
            }}</span>
        </div>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("      <input disabled={!hidden} />\n");
  expect(jsx).not.toContain("const disabled");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Shown()))).toBe('<div><input disabled=""/><span>a-b</span></div>');
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
    let (showDropdown, _) = use_state(|| true);
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
    let (isExpanded, setIsExpanded) = use_state(|| false);
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
  // Across lines, as its literal is, and react.dev's (ADR 0835).
  expect(jsx).toContain("const root = {\n    children: [],\n  };");
  expect(jsx).toContain("currentParent.children.push(node);");
  expect(jsx).toContain("root.children.length < 2");
  expect(jsx).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.jsx"));
  const toc = [["#o", "Overview", 2], ["#a", "A", 2], ["#b", "B", 3], ["#c", "C", 2]].map(([url, text, depth]) => ({ url, text, depth }));
  const render = (items: object[]) => renderToStaticMarkup(createElement(lib.TocContext, { value: items }, createElement(lib.InlineToc)));
  expect(render(toc)).toBe('<ul><li><a href="#a">A</a><ul><li><a href="#b">B</a></li></ul></li><li><a href="#c">C</a></li></ul>');
  expect(render(toc.slice(0, 3))).toBe("");
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
fn cn(classes: impl js::Rest<dyn js::IntoUnknown>) -> String {
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
        <div className={cn((Some("callout"), Some(variant.container_classes)))}>
            <h3 className={cn((Some("title"), Some(variant.text_color)))}>
                {variant.icon.map(|icon| jsx! { <i className={cn((Some("icon"), Some(variant.text_color)))}>{icon}</i> })}
                {variant.title}
            </h3>
            <div>{children}</div>
        </div>
    }
}
`, { "cn.js": "export default (...classes) => classes.filter(Boolean).join(\" \");\n" });
  run([...args, "--extern", `js=${join(target, "libjs.rmeta")}`]);
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
    let (&open, set_open) = use_state(|| false);
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
    let (is_expanded, set_is_expanded) = use_state(|| false);
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
