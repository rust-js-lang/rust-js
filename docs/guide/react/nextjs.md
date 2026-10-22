# Next.js

Write a Next.js app's routes and components in Rust. rust-js compiles each
`.rs` file to the JS file Next.js expects beside it, `app/page.rs` to
`app/page.jsx`, and Next.js serves it as it would one you wrote
([ADR 0192](../../decisions/0192-next.md)). The `next` crate binds Next.js's modules as Next.js types
them: [`next/coverage.txt`](../../../next/coverage.txt) lists every export.

## Start an app

```bash
bun create @rust-js my-site --template next
cd my-site
bun run dev          # next dev, as Next.js runs it
bunx rust-js build   # in another terminal, after each edit
```

Two processes, each its own ([ADR 0412](../../decisions/0412-rust-js-build-is-its-own-process.md)): `next dev`, and `rust-js build`,
which compiles the app's Rust once each time you run it, to `.jsx` and
`.d.ts` files beside the `.rs` ones. Next.js sees the new JS as any edit,
and Fast Refresh keeps the page's state. A compile error is rustc's, in
`rust-js build`'s terminal, and the page keeps the JS that last compiled.

- **The app is a Cargo package**, its `Cargo.toml`'s `[lib] path =
  "app/page.rs"`, which `rust-js build` builds with Cargo. Its crates,
  `react`, `next` and `js`, are npm packages: `rust-js build` tells Cargo
  they're in `node_modules`. See [building](../project/building.md).
- **The generated JS is committed**, so a diff shows what a change did to
  it. `bun run build` runs `rust-js build --check && next build`: it stops
  if what's committed isn't what the Rust compiles to.
- **Other files stay JS**: `next.config.mjs`, CSS, and any route you
  haven't written in Rust. Next.js's config, `NextConfig`, is left to JS.

## Routes are modules

The crate root is `app/page.rs`. Every other route, layout and component
is a module of it, declared where Next.js finds its file:

```rust
// app/page.rs
mod about {
    pub mod page;
}
mod actions;
mod api {
    pub mod hello {
        pub mod route;
    }
}
mod counter;
mod layout;
mod nav;

use next::image::Image;
use next::link::Link;
use react::{JSX, jsx};

use counter::Counter;
use nav::Nav;

pub fn Home() -> JSX::Element {
    jsx! {
        <main>
            <Nav />
            <h1>{"Hello, Next.js"}</h1>
            <Image src="/next.svg" alt="Next.js logo" width={Some(100)} height={Some(20)} priority={Some(true)} />
            <Counter />
            <Link href="/about">{"About"}</Link>
            <form action={actions::save_name}>
                <input name="name" />
                <button type="submit">{"Save"}</button>
            </form>
        </main>
    }
}

js::export_default!(Home);
```

```jsx
// app/page.jsx
import Image from "next/image";
import Link from "next/link";
import { saveName } from "./actions.js";
import { Counter } from "./counter.jsx";
import { Nav } from "./nav.jsx";

export function Home() {
  return (
    <main>
      <Nav />
      <h1>Hello, Next.js</h1>
      <Image src="/next.svg" alt="Next.js logo" width={100} height={20} priority />
      <Counter />
      <Link href="/about">About</Link>
      <form action={saveName}>
        <input name="name" />
        <button type="submit">Save</button>
      </form>
    </main>
  );
}

export default Home;
```

- **A route's page is its default export**: `js::export_default!(Home);`
  is `export default Home;`, the function keeping its name.
- **`app/about/page.rs`** is `mod about { pub mod page; }`, and becomes
  `app/about/page.jsx`. A file whose name isn't a Rust module's,
  `pages/posts/[slug].rs`, is a module by `#[path = "../pages/posts/[slug].rs"]`.
- **A file with JSX is `.jsx`**, one without is `.js`: `actions.js`,
  `api/hello/route.js`.

## Server and client components

A module is a Server Component unless it says `"use client"`, as one with
state, effects or handlers must. Say it first in the file:

```rust
// app/counter.rs
js::directive!("use client");

use react::{JSX, jsx, use_state};

pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! { <button onClick={move |_| set_count.update(|n| n + 1)}>{"Count "}{count}</button> }
}
```

```jsx
// app/counter.jsx
"use client";

import { useState } from "react";

export function Counter() {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount((n) => (n + 1) | 0)}>Count {count}</button>;
}
```

A Server Component may be `async`, as Next.js's are:
`pub async fn About() -> JSX::Element` is `export async function About()`.
It can `await` `next::headers`' `headers()`, `cookies()` and
`draft_mode()`, and a page's `params`, a `js::Promise` of its struct.

## Server Actions and caching

A module of Server Actions says `js::directive!("use server");`. Each
`pub async fn` is an action a form's `action` takes:

```rust
// app/actions.rs
js::directive!("use server");

use next::cache::revalidate_path;
use react::webapi::{FormData, FormDataEntryValue};

pub async fn save_name(data: &'static FormData) {
    if let Some(FormDataEntryValue::Str(name)) = data.get("name") {
        println!("saving {name}");
    }
    revalidate_path("/", None);
}
```

```js
// app/actions.js
"use server";

import { revalidatePath } from "next/cache";

export async function saveName(data) {
  const name = data.get("name");
  if (typeof name === "string") {
    console.log(`saving ${name}`);
  }
  revalidatePath("/");
}
```

- An inline action is an `async` closure whose first statement is
  `js::directive!("use server");`.
- A module of `js::directive!("use cache");` is cached as Next.js caches
  one; `next::cache` has `cache_life`, `cache_tag`, `revalidate_path`,
  `revalidate_tag`, `update_tag`, `refresh` and `unstable_cache`.
- `next::navigation::redirect`, `not_found`, `forbidden` and
  `unauthorized` stop rendering, as Next.js's do.

## Next.js's components

Each is a component of the `next` crate, used by its JSX name. Its
optional props are `Option`s: give one as `Some(..)`, or leave it out, and
Next.js takes its default ([ADR 0438](../../decisions/0438-an-optional-prop-is-given-some.md)). Its other props are required.

| Rust | Next.js |
|---|---|
| `next::link::Link` | `next/link`: `href`, `prefetch`, `replace`, `scroll`, `onNavigate`, `ref`, and an `<a>`'s attributes |
| `next::image::Image` | `next/image`: `src` (a path, or a `StaticImageData` a module imports), `alt`, `width`, `height`, `priority`, `fill`, `placeholder`.. |
| `next::script::Script` | `next/script`: `src`, `strategy`, `onLoad`, and inline children |
| `next::form::Form` | `next/form` |
| `next::head::Head` | `next/head`, in the Pages Router |
| `next::error::Error` | Next.js's error page |
| `next::document::{Html, Head, Main, NextScript}` | `pages/_document`'s |

```rust
// app/nav.rs
js::directive!("use client");

use next::link::Link;
use next::navigation::use_pathname;
use react::{JSX, jsx};

pub fn Nav() -> JSX::Element {
    let pathname = use_pathname();
    let class_of = |href: &str| if pathname == href { "active" } else { "" };
    jsx! {
        <nav>
            <Link href="/" className={Some(class_of("/"))}>{"Home"}</Link>
            <Link href="/about" className={Some(class_of("/about"))}>{"About"}</Link>
        </nav>
    }
}
```

```jsx
// app/nav.jsx
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

export function Nav() {
  const pathname = usePathname();
  const classOf = (href) => (pathname === href ? "active" : "");
  return (
    <nav>
      <Link href="/" className={classOf("/")}>
        Home
      </Link>
      <Link href="/about" className={classOf("/about")}>
        About
      </Link>
    </nav>
  );
}
```

`next::navigation`'s hooks are a client component's: `use_router`,
`use_pathname`, `use_search_params`, `use_params`,
`use_selected_layout_segment` and the rest. The Pages Router's are
`next::router`'s, `use_router` and `with_router`.

### Loaded later: `next/dynamic`

`dynamic` in a `thread_local!`, as Next.js's compiler needs it at the
module's top:

```rust
js::directive!("use client");

use next::dynamic::{DynamicOptions, dynamic};
use next::script::Script;
use react::{ComponentValue, JSX, import_module, jsx};

thread_local! {
    static MAP_VIEW: ComponentValue<()> = dynamic(
        || import_module("./map.jsx"),
        Some(DynamicOptions {
            ssr: Some(false),
            loading: Some(Box::new(|_| jsx! { <p>{"Loading map"}</p> })),
            ..Default::default()
        }),
    );
}

pub fn Contact() -> JSX::Element {
    jsx! {
        <>
            <MAP_VIEW />
            <Script src={Some("https://example.com/widget.js")} strategy={Some("lazyOnload")} />
        </>
    }
}
```

```jsx
"use client";

import dynamic from "next/dynamic";
import Script from "next/script";

const MapView = dynamic(() => import("./map.jsx"), {
  loading: () => <p>Loading map</p>,
  ssr: false,
});

export function Contact() {
  return (
    <>
      <MapView />
      <Script src="https://example.com/widget.js" strategy="lazyOnload" />
    </>
  );
}
```

## Layouts, fonts and metadata

```rust
// app/layout.rs
js::import!("./globals.css");

use next::Metadata;
use next::font::NextFont;
use next::font::google::{Geist, Subset, VariableFontWithoutAxes};
use next::metadata::Title;
use react::{JSX, ReactNode, jsx};

thread_local! {
    static GEIST: NextFont = Geist(VariableFontWithoutAxes {
        variable: Some("--font-geist-sans"),
        subsets: Some(&[Subset::Latin]),
        ..Default::default()
    });
}

pub fn generate_metadata() -> Metadata<'static> {
    Metadata {
        title: Some(Title::Str("My site")),
        ..Default::default()
    }
}

pub struct RootLayoutProps<'a> {
    pub children: &'a dyn ReactNode,
}

pub fn RootLayout(RootLayoutProps { children }: RootLayoutProps) -> JSX::Element {
    let font = GEIST.with(|font| font.variable.clone().unwrap_or_default());
    jsx! {
        <html lang="en" className={font}>
            <body>{children}</body>
        </html>
    }
}

js::export_default!(RootLayout);
```

```jsx
// app/layout.jsx
import { Geist } from "next/font/google";
import "./globals.css";

const geist = Geist({ variable: "--font-geist-sans", subsets: ["latin"] });

export function generateMetadata() {
  return { title: "My site" };
}

export function RootLayout({ children }) {
  const font = geist.variable ?? "";
  return (
    <html lang="en" className={font}>
      <body>{children}</body>
    </html>
  );
}

export default RootLayout;
```

- **A font** is called in a `thread_local!`, at the module's top, as
  Next.js's compiler reads it: each of Google's fonts is a function of
  `next::font::google`, and a file's is `next::font::local::localFont`.
  Its `NextFont` has `class_name`, `style` and `variable`.
- **Metadata** is `generate_metadata`, which Next.js finds by its JS name,
  `generateMetadata`; `generate_viewport` likewise. Every `Metadata` field
  is an `Option`, so `..Default::default()` fills the rest.
- **`robots.rs` and `sitemap.rs`** give `next::MetadataRoute`'s `Robots`
  and `Sitemap` from their default export.
- **CSS** is imported by `js::import!("./globals.css");`.

## Route handlers and the proxy

A route handler is an `async fn` of the HTTP method's name. Next.js reads
`GET`, which isn't a Rust function's name, so name it for JS:

```rust
// app/api/hello/route.rs
use next::server::{NextRequest, next_response};
use react::webapi::Response;

pub struct Greeting {
    pub hello: String,
}

#[cfg_attr(rust_js, rust_js::name = "GET")]
pub async fn get(request: &'static NextRequest) -> &'static Response {
    let name = request
        .next_url()
        .search_params()
        .get("name")
        .unwrap_or_default();
    next_response::json(Greeting { hello: name }, None)
}
```

```js
// app/api/hello/route.js
import { NextResponse } from "next/server";

export async function GET(request) {
  const name = request.nextUrl.searchParams.get("name") ?? "";
  return NextResponse.json({ hello: name });
}
```

`next::server` has the rest: `NextRequest`, `next_response`'s `json`,
`redirect`, `rewrite` and `next`, `after`, `connection` and `user_agent`.
A proxy, `proxy.rs`, exports `proxy` and its `config`, a `static` named
for JS the same way:

```rust
use next::server::{Matcher, MiddlewareConfig, NextMiddlewareResult, NextRequest, next_response};
use react::webapi::url;

pub fn proxy(request: &'static NextRequest) -> NextMiddlewareResult {
    let about = url::new("/about", Some(&request.url()));
    Some(next_response::rewrite(about, None))
}

#[cfg_attr(rust_js, rust_js::name = "config")]
pub static CONFIG: MiddlewareConfig<'static> = MiddlewareConfig {
    matcher: Some(Matcher::Path("/old")),
    regions: None,
    unstable_allow_dynamic: None,
};
```

```js
import { NextResponse } from "next/server";

export function proxy(request) {
  const about = new URL("/about", request.url);
  return NextResponse.rewrite(about);
}

export const config = { matcher: "/old" };
```

A `static` keeps its Rust name, as a constant does ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)), so one
Next.js reads by another name says it. `next::og`'s `image_response`
draws an image from JSX.

## The Pages Router

A page in `pages/` exports its component by default, and its data
functions by their JS names:

```rust
// pages/posts/[slug].rs
use next::{
    GetStaticPathsContext, GetStaticPathsFallback, GetStaticPathsResult, GetStaticPropsContext,
    GetStaticPropsResult, StaticNotFound, StaticPath, StaticPathParams, StaticProps,
};
use react::{JSX, jsx};

pub struct Params {
    pub slug: String,
}

pub struct PostProps {
    pub title: String,
}

pub fn Post(PostProps { title }: PostProps) -> JSX::Element {
    jsx! { <h1>{title}</h1> }
}

js::export_default!(Post);

pub async fn get_static_props(
    GetStaticPropsContext { params, .. }: GetStaticPropsContext<Params>,
) -> GetStaticPropsResult<PostProps> {
    match params {
        Some(Params { slug }) => GetStaticPropsResult::Props(StaticProps {
            props: PostProps { title: slug },
            revalidate: None,
        }),
        None => GetStaticPropsResult::NotFound(StaticNotFound {
            not_found: true,
            revalidate: None,
        }),
    }
}

pub async fn get_static_paths(_: GetStaticPathsContext) -> GetStaticPathsResult<Params> {
    let path = |slug: &str| {
        StaticPath::Params(StaticPathParams {
            params: Params {
                slug: slug.to_string(),
            },
            locale: None,
        })
    };
    GetStaticPathsResult {
        paths: vec![path("hello"), path("world")],
        fallback: GetStaticPathsFallback::Blocking,
    }
}
```

```jsx
// pages/posts/[slug].jsx
export function Post({ title }) {
  return <h1>{title}</h1>;
}

export async function getStaticProps({ params }) {
  return params ? { props: { title: params.slug } } : { notFound: true };
}

export async function getStaticPaths(_) {
  const path = (slug) => ({ params: { slug } });
  return { paths: [path("hello"), path("world")], fallback: "blocking" };
}

export default Post;
```

- `get_server_side_props` is `getServerSideProps`, of a
  `GetServerSidePropsContext`.
- An API route, `pages/api/greet.rs`, default-exports a function of a
  `&NextApiRequest` and a `&NextApiResponse<T>`.
- `pages/_app.rs` takes `next::app::AppProps` and renders its page,
  `AppProps { component: Component, page_props, .. }` and
  `<Component {...page_props} />`; `pages/_document.rs` renders
  `next::document`'s `Html`, `Head`, `Main` and `NextScript`.

## Also bound

`next::headers`, `next::cache`, `next::navigation`, `next::router`,
`next::server`, `next::og`, `next::web_vitals`' `use_report_web_vitals`,
`next::offline`'s `use_offline`, `next::error`'s `catch_error`, a boundary
of its children's errors, instrumentation's `register` and
`on_request_error`, and a custom server, `next::next`. Each is described in
the crate's documentation, by the Next.js page it binds.
