import { expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, symlinkSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "@playwright/test";
import { buildCompiler, compiler, fixture, root } from "./support";

const example = join(root, "examples/next");
const bin = join(root, "next-plugin/bin.js");

const about = `#![allow(non_snake_case)]

use next::legacy::image::Image;
use next::link::Link;
use next::script::Script;
use react::attributes::AnchorHTMLAttributes;
use react::{CSSProperties, JSX, jsx};

pub fn About() -> JSX::Element {
    jsx! {
        <main>
            <h1>{"About, in Rust"}</h1>
            <div className="logo" style={CSSProperties::new().position("relative").width(100).height(20)}>
                <Image src="/next.svg" layout={Some("fill")} objectFit={Some("cover")} alt={Some("Next.js logo")} {..Default::default()} />
            </div>
            <HomeLink className="home" {..Default::default()} />
            <Script id={Some("inline")}>{"window.inlined = true;"}</Script>
        </main>
    }
}

#[derive(Default)]
pub struct HomeLinkProps<'a> {
    pub class_name: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: AnchorHTMLAttributes<'a>,
}

pub fn HomeLink(HomeLinkProps { class_name, anchor }: HomeLinkProps) -> JSX::Element {
    let classes = [class_name, "link"].join(" ");
    jsx! {
        <Link href="/" anchor={anchor} className={Some(classes.as_str())} aria-label={Some("Home page")} {..Default::default()}>
            {"Home"}
        </Link>
    }
}

js::export_default!(About);
`;

// The example, beside it: its own Rust and output, the example's packages,
// and the crates by path from this checkout, as the example has them. A
// Server Component's route, app/page.rs, renders a client component of its
// crate's, app/counter.rs, and a route below it, app/about/page.rs, links
// back (ADR 0192), by a link of its own that gives next/link a class it
// computes, an aria-label, and the anchor's other props (ADR 0200, 0208).
function app(name: string): string {
  const dir = fixture(name);
  cpSync(example, dir, {
    recursive: true,
    filter: (path) => !/^(node_modules|\.next|target|Cargo\.lock|\.cargo)(\/|$)/.test(relative(example, path)) && !path.endsWith(".map"),
  });
  // Its packages are the example's, but its node_modules its own: what
  // Cargo writes is there, its .cache's (ADR 0192).
  mkdirSync(join(dir, "node_modules"));
  for (const entry of readdirSync(join(example, "node_modules"))) {
    if (entry !== ".cache") symlinkSync(realpathSync(join(example, "node_modules", entry)), join(dir, "node_modules", entry));
  }
  const cargo = join(dir, "Cargo.toml");
  // With declarations, as react.dev has them (ADR 0196).
  writeFileSync(cargo, readFileSync(cargo, "utf8").replaceAll('path = "../../', `path = "${root}/`) + "\n[package.metadata.rust-js]\ndeclarations = true\n");
  const page = join(dir, "app/page.rs");
  writeFileSync(page, readFileSync(page, "utf8")
    .replace("use next::image::Image;", "mod about {\n    pub mod page;\n}\nmod request {\n    pub mod page;\n}\nmod actions;\nmod api {\n    pub mod hello {\n        pub mod route;\n    }\n}\nmod og {\n    pub mod route;\n}\n#[path = \"../proxy.rs\"]\nmod proxy;\nmod counter;\nmod linked;\nmod route_path;\n#[path = \"../pages/codes/[code].rs\"]\nmod code;\n#[path = \"../pages/_app.rs\"]\nmod app;\n#[path = \"../pages/_document.rs\"]\nmod document;\n\nuse next::image::Image;")
    .replace('{" file."}\n                    </h1>', '{" file."}\n                    </h1>\n                    <counter::Counter />'));
  writeFileSync(join(dir, "app/counter.rs"), counter("Count "));
  // The Pages Router's route, as react.dev's pages read it: compiled, not
  // rendered, as this app's routes are the App Router's.
  // A link of a ref and passHref, as react.dev's SidebarLink has it.
  writeFileSync(join(dir, "app/linked.rs"), linked);
  writeFileSync(join(dir, "app/route_path.rs"), "pub fn route_path() -> String {\n    next::router::use_router().as_path().to_string()\n}\n\n" + routeEvents);
  // A route of the request, its headers and cookies, which make it
  // dynamic, and a Server Action that sets them.
  mkdirSync(join(dir, "app/request"));
  writeFileSync(join(dir, "app/request/page.rs"), request);
  writeFileSync(join(dir, "app/actions.rs"), actions);
  // A Route Handler, and a proxy that rewrites a path to another route.
  mkdirSync(join(dir, "app/api/hello"), { recursive: true });
  writeFileSync(join(dir, "app/api/hello/route.rs"), route);
  writeFileSync(join(dir, "proxy.rs"), proxy);
  // An image drawn from JSX, next/og's.
  mkdirSync(join(dir, "app/og"));
  writeFileSync(join(dir, "app/og/route.rs"), og);
  mkdirSync(join(dir, "app/about"));
  writeFileSync(join(dir, "app/about/page.rs"), about);
  // A Pages Router page built for the paths it gives, each with its props,
  // and not found for one, as react.dev's errors page is.
  mkdirSync(join(dir, "pages/codes"), { recursive: true });
  writeFileSync(join(dir, "pages/codes/[code].rs"), code);
  // The Pages Router's app, which renders each page in it, as react.dev's
  // _app does.
  writeFileSync(join(dir, "pages/_app.rs"), `#![allow(non_snake_case)]

use next::app::AppProps;
use react::{JSX, jsx};

pub fn MyApp(AppProps { component: Component, page_props: pageProps, .. }: AppProps) -> JSX::Element {
    jsx! { <main className="app"><Component {...pageProps} /></main> }
}

js::export_default!(MyApp);
`);
  // The Pages Router's document, its `<html>` and `<body>`, as react.dev's
  // _document has them.
  writeFileSync(join(dir, "pages/_document.rs"), `#![allow(non_snake_case)]

use next::document::{Head, Html, Main, NextScript};
use react::attributes::{HTMLAttributes, HtmlHTMLAttributes};
use react::{JSX, jsx};

pub fn MyDocument() -> JSX::Element {
    jsx! {
        <Html
            props={HtmlHTMLAttributes {
                html: HTMLAttributes {
                    lang: Some("eo"),
                    dir: Some("ltr"),
                    ..Default::default()
                },
                ..Default::default()
            }}
            {..Default::default()}>
            <Head />
            <body className="document">
                <Main />
                <NextScript />
            </body>
        </Html>
    }
}

js::export_default!(MyDocument);
`);
  return dir;
}

const request = `#![allow(non_snake_case)]

use next::headers::{cookies, draft_mode, headers};
use react::{JSX, jsx};

pub async fn Request() -> JSX::Element {
    let agent = headers().await.get("user-agent").unwrap_or_default();
    let jar = cookies().await;
    let theme = match jar.get("theme") {
        Some(cookie) => cookie.value,
        None => "light",
    };
    let draft = draft_mode().await.is_enabled();
    let mode = if draft { "draft" } else { "live" };
    jsx! { <p>{theme}{" "}{jar.size()}{" "}{mode}{" "}{agent.len()}</p> }
}

js::export_default!(Request);
`;

const route = `#![allow(non_snake_case)]

use next::server::{NextRequest, after, connection, next_response, user_agent};
use react::webapi::Response;

pub struct Greeting {
    pub hello: String,
    pub bot: bool,
}

pub async fn GET(request: &'static NextRequest) -> &'static Response {
    connection().await;
    let hello = request.next_url().search_params().get("name").unwrap_or_default();
    let bot = user_agent(request).is_bot;
    after(|| {
        let _ = 1;
    });
    let response = next_response::json(Greeting { hello, bot });
    response.cookies().set("seen", "1");
    response
}
`;

const og = `#![allow(non_snake_case)]

use next::og::{ImageResponseOptions, image_response};
use react::webapi::Response;
use react::{CSSProperties, jsx};

pub async fn GET() -> &'static Response {
    let image = jsx! { <div style={CSSProperties::new().display("flex").font_size(64)}>{"Rust"}</div> };
    image_response::new_with_options(image, ImageResponseOptions { width: Some(600), height: Some(315), ..Default::default() })
}
`;

const proxy = `use next::server::{Matcher, MiddlewareConfig, NextMiddlewareResult, NextRequest, next_response};
use react::webapi::{Response, url};

// /old is /about, the browser's URL kept.
pub fn proxy(request: &'static NextRequest) -> NextMiddlewareResult {
    let about = url::new_with_base("/about", &request.url());
    let response: &Response = next_response::rewrite(about);
    Some(response)
}

#[allow(non_upper_case_globals)]
pub static config: MiddlewareConfig<'static> = MiddlewareConfig { matcher: Some(Matcher::Path("/old")), regions: None, unstable_allow_dynamic: None };
`;

const actions = `js::directive!("use server");

use next::headers::{CookieOptions, DeletedCookie, SameSite, cookies};

pub async fn leave(forbidden: bool) {
    if forbidden {
        next::navigation::forbidden();
    }
    next::navigation::redirect_with_type("/", next::navigation::RedirectType::Replace);
}

pub async fn remember(theme: String) {
    let jar = cookies().await;
    jar.set_with_options("theme", &theme, CookieOptions { max_age: Some(3600.0), same_site: Some(SameSite::Str("lax")), ..Default::default() });
    jar.delete_cookie(DeletedCookie { name: "old", domain: None, path: Some("/"), secure: None, same_site: None, partitioned: None, http_only: None, max_age: None, priority: None });
}
`;

const code = `#![allow(non_snake_case)]

use next::{
    GetStaticPathsContext, GetStaticPathsResult, GetStaticPropsContext, GetStaticPropsResult, StaticNotFound, StaticPath,
    StaticPathParams, StaticProps,
};
use react::{JSX, jsx};

pub struct Params {
    pub code: String,
}

pub struct CodeProps {
    pub code: String,
}

pub fn Code(CodeProps { code }: CodeProps) -> JSX::Element {
    jsx! { <p>{"Code "}{code}</p> }
}

js::export_default!(Code);

pub async fn getStaticProps(GetStaticPropsContext { params, .. }: GetStaticPropsContext<Params>) -> GetStaticPropsResult<CodeProps> {
    match params {
        Some(Params { code }) if code != "0" => GetStaticPropsResult::Props(StaticProps { props: CodeProps { code } }),
        _ => GetStaticPropsResult::NotFound(StaticNotFound { not_found: true }),
    }
}

pub async fn getStaticPaths(_: GetStaticPathsContext) -> GetStaticPathsResult<Params> {
    let params = |code: &str| StaticPath::Params(StaticPathParams { params: Params { code: code.to_string() } });
    GetStaticPathsResult { paths: vec![params("0"), params("1"), StaticPath::Path("/codes/2".to_string())], fallback: false }
}
`;

const linked = `#![allow(non_snake_case)]
js::directive!("use client");

use next::link::Link;
use react::webapi::HTMLAnchorElement;
use react::{JSX, jsx, use_ref};

fn classes(home: bool) -> &'static str {
    if home { "home" } else { "away" }
}

fn label(home: bool) -> &'static str {
    if home { "Home" } else { "Away" }
}

// The page's head, by next/head, as react.dev's Search preconnects there.
pub fn Preconnect() -> JSX::Element {
    jsx! { <next::head::Head><link rel="preconnect" href="https://example.net" /></next::head::Head> }
}

// Its classes made before its children, as JSX makes them.
pub fn Linked() -> JSX::Element {
    let anchor = use_ref::<Option<&'static HTMLAnchorElement>>(None);
    jsx! { <Link href="/" ref={Some(anchor)} title={Some("Home")} className={Some(classes(true))} passHref={Some(true)}>{label(true)}</Link> }
}

// An image titled, as react.dev's TopNav titles its logo.
pub fn Titled() -> JSX::Element {
    jsx! { <next::image::Image src="/next.svg" alt="Next.js logo" title={Some("Next.js")} width={Some(90)} height={Some(18)} /> }
}

pub struct LocatedProps<'a> {
    pub prefix: &'a str,
}

// A component next/router's withRouter gives the router, as react.dev's Seo
// is, and rendered as a tag.
thread_local! {
    pub static Located: react::ComponentValue<LocatedProps<'static>> = next::router::with_router(
        |next::router::WithRouterProps { props: LocatedProps { prefix }, router }| {
            jsx! { <p>{prefix}{router.as_path()}</p> }
        },
    );
}

// A third party script, loaded once the page is idle, which a client
// component handles the load of.
pub fn Idle() -> JSX::Element {
    jsx! { <next::script::Script src={Some("https://example.net/idle.js")} strategy={Some("lazyOnload")} onLoad={Some(Box::new(|_| {}))} /> }
}

// Scripts loaded where they are called, as a Script of the same props.
pub fn load(src: &str) {
    let props = |src| next::script::ScriptProps { script: react::attributes::ScriptHTMLAttributes { src: Some(src), ..Default::default() }, ..Default::default() };
    next::script::handle_client_script_load::<JSX::Element>(props(src));
    next::script::init_script_loader::<JSX::Element>(vec![props(src)]);
}

// The page's Web Vitals, whether it's offline, and a search form, as
// next/web-vitals, next/offline and next/form give them.
pub fn Search() -> JSX::Element {
    next::web_vitals::use_report_web_vitals(|metric| {
        let _ = (metric.name(), metric.value());
    });
    let offline = next::offline::use_offline();
    jsx! {
        <next::form::Form action="/search" className={Some("search")}>
            <input name="query" disabled={offline} />
        </next::form::Form>
    }
}

pub struct ShownProps<'a> {
    pub text: &'a str,
    pub children: JSX::Element,
}

// A boundary of its children's errors, as next/error's catchError makes one,
// and Next.js's error page.
thread_local! {
    pub static Shown: react::ComponentValue<ShownProps<'static>> = next::error::catch_error(
        |ShownProps { text, .. }, info: &'static next::error::ErrorInfo| {
            jsx! { <button onClick={move |_| info.reset()}>{text}</button> }
        },
    );
}

pub fn Missing() -> JSX::Element {
    jsx! { <Shown text="again"><next::error::Error statusCode={404} title={Some("Gone")} /></Shown> }
}

pub struct Slug {
    pub slug: String,
}

// The route, its query, segment and parameters, and going on with options,
// as next/navigation gives them.
pub fn Navigator() -> JSX::Element {
    let router = next::navigation::use_router();
    let query = next::navigation::use_search_params().get("q").unwrap_or_default();
    let segment = next::navigation::use_selected_layout_segment().unwrap_or_default();
    let Slug { slug } = next::navigation::use_params();
    jsx! {
        <button key={router.bfcache_id()} onClick={move |_| router.push_with_options("/about", next::navigation::NavigateOptions { scroll: Some(false), ..Default::default() })}>
            {query}{segment}{slug}
        </button>
    }
}

pub fn Location() -> JSX::Element {
    jsx! { <Located prefix="at " /> }
}
`;

// A handler of the router's events, given on and taken off by the same
// function, as react.dev's usePendingRoute has them.
const routeEvents = `// The singleton router, Next.js's default export of next/router.
pub fn go(url: &str) {
    next::router::Router.push(url);
}

pub fn use_route_events() {
    let events = next::router::use_router().events();
    react::use_effect(
        move || {
            let started: &'static dyn Fn(&str) = Box::leak(Box::new(|url: &str| {
                let _ = url.len();
            }));
            events.on(next::router::RouterEvent::RouteChangeStart, started);
            move || events.off(next::router::RouterEvent::RouteChangeStart, started)
        },
        (events,),
    );
}
`;

function counter(label: string): string {
  return `#![allow(non_snake_case)]
js::directive!("use client");

use react::{JSX, jsx, use_state};

pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! { <button onClick={move |_| set_count.update(|n| n + 1)}>{"${label}"}{count}</button> }
}
`;
}


function command(dir: string, args: string[]) {
  return spawn(process.execPath, [bin, ...args], { cwd: dir, env: { ...process.env, RUST_JS_COMPILER: compiler, NEXT_TELEMETRY_DISABLED: "1" } });
}

function finished(child: ReturnType<typeof spawn>): Promise<{ code: number | null; output: string }> {
  let output = "";
  child.stdout?.on("data", (data) => (output += data));
  child.stderr?.on("data", (data) => (output += data));
  return new Promise((done) => child.on("exit", (code) => done({ code, output })));
}

test("rust-js-next build builds a Next.js app whose routes and components are Rust", async () => {
  buildCompiler();
  const dir = app("next-build");
  const { code, output } = await finished(command(dir, ["build"]));
  expect([code, output.includes("○ /about")]).toEqual([0, true]);
  // What Cargo writes isn't the app's, which Turbopack watches: node_modules'.
  expect([existsSync(join(dir, "target")), existsSync(join(dir, "node_modules/.cache/rust-js/target"))]).toEqual([false, true]);
  const statements = (file: string) => readFileSync(join(dir, file), "utf8").split("\n").filter((line) => line && !line.startsWith("//"));
  // A client component's module is one, and a route's default is its page.
  expect(statements("app/counter.jsx")[0]).toBe('"use client";');
  expect(statements("app/page.jsx")[0]).not.toBe('"use client";');
  expect(readFileSync(join(dir, "app/page.jsx"), "utf8")).toContain("export default Home;");
  expect(readFileSync(join(dir, "app/about/page.jsx"), "utf8")).toContain('import Link from "next/link";');
  expect(readFileSync(join(dir, "app/linked.jsx"), "utf8")).toContain('<Link href="/" ref={anchor} title="Home" className={classes(true)} passHref>\n      {label(true)}');
  const routePath = readFileSync(join(dir, "app/route_path.js"), "utf8");
  expect(routePath.includes("return useRouter().asPath;")).toBe(true);
  expect([routePath.includes('import Router, { useRouter } from "next/router";'), routePath.includes("Router.push(url);")]).toEqual([true, true]);
  expect(readFileSync(join(dir, "app/linked.jsx"), "utf8")).toContain('import Head from "next/head";');
  expect(readFileSync(join(dir, "app/linked.jsx"), "utf8")).toContain('<Image src="/next.svg" alt="Next.js logo" title="Next.js" width={90} height={18} />');
  const linkedJsx = readFileSync(join(dir, "app/linked.jsx"), "utf8");
  expect([linkedJsx.includes("export const Located = withRouter(({ prefix, router }) => ("), linkedJsx.includes('return <Located prefix="at " />;')]).toEqual([true, true]);
  expect([routePath.includes("const events = useRouter().events;"), routePath.includes('events.on("routeChangeStart", started);'), routePath.includes('events.off("routeChangeStart", started)')]).toEqual([true, true, true]);
  // The Server Component's page is rendered at build time, the counter in it.
  expect(readFileSync(join(dir, ".next/server/app/index.html"), "utf8")).toContain("Count <!-- -->0");
  const aboutHtml = readFileSync(join(dir, ".next/server/app/about.html"), "utf8");
  expect([aboutHtml.includes('class="home link"'), aboutHtml.includes('aria-label="Home page"')]).toEqual([true, true]);
  // next/legacy/image, as react.dev's TeamMember has it: its layout and
  // fit, an `<img>` that fills its parent and covers it.
  const aboutJsx = readFileSync(join(dir, "app/about/page.jsx"), "utf8");
  expect([aboutJsx.includes('import Image from "next/legacy/image";'), aboutJsx.includes('<Image src="/next.svg" layout="fill" objectFit="cover" alt="Next.js logo" />')]).toEqual([true, true]);
  expect([aboutHtml.includes('alt="Next.js logo"'), aboutHtml.includes("object-fit:cover")]).toEqual([true, true]);
  // next/script: its props as written, and the inline one in the page.
  expect(readFileSync(join(dir, "app/linked.jsx"), "utf8")).toContain('<Script src="https://example.net/idle.js" strategy="lazyOnload" onLoad={() => {}} />');
  expect(aboutJsx).toContain('<Script id="inline">window.inlined = true;</Script>');
  expect(aboutHtml).toContain("window.inlined = true;");
  expect(readFileSync(join(dir, "app/linked.jsx"), "utf8")).toContain("handleClientScriptLoad(props(src));\n  initScriptLoader([props(src)]);");
  // next/web-vitals, next/offline, next/form and next/error.
  expect(linkedJsx).toContain("useReportWebVitals((metric) => {");
  expect(linkedJsx).toContain('<Form action="/search" className="search">\n      <input name="query" disabled={offline} />\n    </Form>');
  // Next.js's error page, named apart from JS's `Error` (ADR 0038).
  expect(linkedJsx).toContain('import Error$, { catchError } from "next/error";');
  expect(linkedJsx).toContain("export const Shown = catchError(({ text }, info) => (\n  <button onClick={() => info.reset()}>{text}</button>\n));");
  expect(linkedJsx).toContain('<Shown text="again">\n      <Error$ statusCode={404} title="Gone" />\n    </Shown>');
  // next/server: a Route Handler, and a proxy of its config.
  expect(output).toContain("ƒ /api/hello");
  expect(output).toContain("ƒ Proxy");
  const routeJs = readFileSync(join(dir, "app/api/hello/route.js"), "utf8");
  expect(routeJs).toContain('import { NextResponse, after, connection, userAgent } from "next/server";');
  expect(routeJs).toContain("const response = NextResponse.json({ hello, bot });");
  expect(readFileSync(join(dir, "app/og/route.jsx"), "utf8")).toContain("return new ImageResponse(image, { width: 600, height: 315 });");
  const proxyJs = readFileSync(join(dir, "proxy.js"), "utf8");
  expect(proxyJs).toContain('export const config = { matcher: "/old" };');
  expect(proxyJs).toContain('const about = new URL("/about", request.url);\n  const response = NextResponse.rewrite(about);\n  return response;');
  // next/navigation.
  expect(linkedJsx).toContain('const query = useSearchParams().get("q") ?? "";');
  expect(linkedJsx).toContain("const { slug } = useParams();");
  expect(linkedJsx).toContain('router.push("/about", { scroll: false })');
  expect(linkedJsx).toContain("<button key={router.bfcacheId} onClick={() => router.push(");
  expect(readFileSync(join(dir, "app/actions.js"), "utf8")).toContain('  if (forbidden$1) {\n    forbidden();\n  }\n  redirect("/", "replace");');
  // next/headers: the request's route is dynamic, rendered as it's asked.
  expect(output).toContain("ƒ /request");
  const requestJsx = readFileSync(join(dir, "app/request/page.jsx"), "utf8");
  expect(requestJsx).toContain("const jar = await cookies();");
  expect(requestJsx).toContain("(await draftMode()).isEnabled");
  const actionsJs = readFileSync(join(dir, "app/actions.js"), "utf8");
  expect(actionsJs).toContain('jar.set("theme", theme, { sameSite: "lax", maxAge: 3600 });');
  expect(actionsJs).toContain('jar.delete({ name: "old", path: "/" });');
  // A page's module has no declarations beside it, which Turbopack would
  // take as a page of its own; another module has (ADR 0276).
  expect([existsSync(join(dir, "pages/codes/[code].d.ts")), existsSync(join(dir, "app/linked.d.ts"))]).toEqual([false, true]);
  // The Pages Router's page, built for each path but the one not found.
  const built = (path: string) => existsSync(join(dir, `.next/server/pages/codes/${path}.html`));
  expect([built("0"), built("1"), built("2")]).toEqual([false, true, true]);
  expect(readFileSync(join(dir, ".next/server/pages/codes/1.html"), "utf8")).toContain('<main class="app"><p>Code <!-- -->1</p></main>');
  expect(readFileSync(join(dir, "pages/_app.jsx"), "utf8")).toContain("export function MyApp({ Component, pageProps }) {");
  // Each page in the document's `<html>` and `<body>`.
  expect(readFileSync(join(dir, "pages/_document.jsx"), "utf8")).toContain('<Html lang="eo" dir="ltr">');
  const documented = readFileSync(join(dir, ".next/server/pages/codes/1.html"), "utf8");
  expect([documented.includes('<html lang="eo" dir="ltr">'), documented.includes('<body class="document">')]).toEqual([true, true]);
  // Its props as written, an anchor's first, which the props it names
  // replace (ADR 0203, 0208).
  expect(readFileSync(join(dir, "app/about/page.jsx"), "utf8")).toContain('<Link href="/" {...anchor} className={classes} aria-label="Home page">');

  // A Rust error is the build's, rustc's message, and no Next.js build.
  writeFileSync(join(dir, "app/counter.rs"), counter("Count ") + "pub fn broken(");
  const failed = await finished(command(dir, ["build"]));
  expect([failed.code, failed.output.includes("error"), failed.output.includes("Next.js 16")]).toEqual([1, true, false]);
}, 300_000);

// `rust-js-next compile` writes the app's JS and runs no Next.js, for what an
// app's build reads of its Rust first, as react.dev's static files read its
// llmsTxt.
test("rust-js-next compile writes the app's JS alone", async () => {
  buildCompiler();
  const dir = app("next-compile");
  const { code } = await finished(command(dir, ["compile"]));
  expect([code, existsSync(join(dir, "app/page.jsx")), existsSync(join(dir, ".next"))]).toEqual([0, true, false]);
});

test("rust-js-next dev serves Rust routes, refreshes a save in place, and recovers from an error", async () => {
  buildCompiler();
  const dir = app("next-dev");
  const port = 3200 + Math.floor(Math.random() * 500);
  const server = command(dir, ["dev", "--port", String(port)]);
  let output = "";
  server.stdout?.on("data", (data) => (output += data));
  server.stderr?.on("data", (data) => (output += data));
  let browser;
  try {
    while (!output.includes("Ready")) {
      if (server.exitCode !== null) throw new Error(output);
      await Bun.sleep(100);
    }
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(`http://localhost:${port}/`);
    // The counter, whose name ends in its count: Next.js's dev server
    // shows a Dev Tools button too, sometimes before the first click.
    const button = page.getByRole("button", { name: /\d$/ });
    await button.filter({ hasText: "Count 0" }).waitFor();
    await button.click();
    await button.filter({ hasText: "Count 1" }).waitFor();
    // Next.js's Fast Refresh is live a while after the page loads, longer
    // under the suite's load, and an update before then is lost: a save
    // until one is shown, each another, as the same JS is no update.
    for (let tries = 0; ; tries++) {
      writeFileSync(join(dir, "app/counter.rs"), counter(`Warm${tries} `));
      try {
        await button.filter({ hasText: `Warm${tries} 1` }).waitFor({ timeout: 5_000 });
        break;
      } catch (error) {
        if (tries === 8) throw error;
      }
    }

    // A save is a Fast Refresh: the client component keeps its state.
    writeFileSync(join(dir, "app/counter.rs"), counter("Clicks "));
    await button.filter({ hasText: "Clicks 1" }).waitFor({ timeout: 30_000 });

    // An error is rustc's, in the terminal, and Next.js goes on; fixed, the
    // page is the Rust's again.
    const before = output.length;
    writeFileSync(join(dir, "app/counter.rs"), counter("Clicks ") + "pub fn broken(");
    while (!output.slice(before).includes("error")) await Bun.sleep(100);
    expect(server.exitCode).toBe(null);
    writeFileSync(join(dir, "app/counter.rs"), counter("Taps "));
    await button.filter({ hasText: "Taps 1" }).waitFor({ timeout: 30_000 });

    // A route below it, and next/link back, without loading the page again.
    await page.goto(`http://localhost:${port}/about`);
    await page.getByRole("heading", { name: "About, in Rust" }).waitFor();
    await page.evaluate(() => ((window as any).loaded = true));
    await page.getByRole("link", { name: "Home" }).click();
    await button.filter({ hasText: "Taps 0" }).waitFor();
    expect(await page.evaluate(() => (window as any).loaded)).toBe(true);
    expect(existsSync(join(dir, "app/about/page.jsx"))).toBe(true);

    // next/headers: the request's cookie, as the route reads it.
    await page.context().addCookies([{ name: "theme", value: "dark", url: `http://localhost:${port}` }]);
    await page.goto(`http://localhost:${port}/request`);
    await page.getByText(/^dark \d+ live \d+$/).waitFor();

    // next/server: the Route Handler's JSON and cookie, and the proxy's
    // rewrite of /old to /about, its URL kept.
    const hello = await page.request.get(`http://localhost:${port}/api/hello?name=Ada`);
    expect([await hello.json(), hello.headers()["set-cookie"]?.startsWith("seen=1")]).toEqual([{ hello: "Ada", bot: false }, true]);
    const image = await page.request.get(`http://localhost:${port}/og`);
    expect([image.status(), image.headers()["content-type"]]).toEqual([200, "image/png"]);
    await page.goto(`http://localhost:${port}/old`);
    await page.getByRole("heading", { name: "About, in Rust" }).waitFor();
    expect(new URL(page.url()).pathname).toBe("/old");
  } finally {
    await browser?.close();
    server.kill("SIGTERM");
    await new Promise((done) => (server.exitCode !== null ? done(undefined) : server.on("exit", done)));
  }
}, 300_000);
