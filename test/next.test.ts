import { expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, symlinkSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "@playwright/test";
import { buildCompiler, compiler, fixture, root } from "./support";

const example = join(root, "examples/next");
const bin = join(root, "next-plugin/bin.js");

const about = `#![allow(non_snake_case)]

use next::link::Link;
use react::attributes::AnchorHtmlAttributes;
use react::{Element, jsx};

pub fn About() -> Element {
    jsx! {
        <main>
            <h1>{"About, in Rust"}</h1>
            <HomeLink className="home" {..Default::default()} />
        </main>
    }
}

#[derive(Default)]
pub struct HomeLinkProps<'a> {
    pub class_name: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: AnchorHtmlAttributes<'a>,
}

pub fn HomeLink(HomeLinkProps { class_name, anchor }: HomeLinkProps) -> Element {
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
  writeFileSync(cargo, readFileSync(cargo, "utf8").replaceAll('path = "../../', `path = "${root}/`));
  const page = join(dir, "app/page.rs");
  writeFileSync(page, readFileSync(page, "utf8")
    .replace("use next::image::Image;", "mod about {\n    pub mod page;\n}\nmod counter;\nmod route_path;\n\nuse next::image::Image;")
    .replace('{" file."}\n                    </h1>', '{" file."}\n                    </h1>\n                    <counter::Counter />'));
  writeFileSync(join(dir, "app/counter.rs"), counter("Count "));
  // The Pages Router's route, as react.dev's pages read it: compiled, not
  // rendered, as this app's routes are the App Router's.
  writeFileSync(join(dir, "app/route_path.rs"), "pub fn route_path() -> String {\n    next::router::use_router().as_path().to_string()\n}\n");
  mkdirSync(join(dir, "app/about"));
  writeFileSync(join(dir, "app/about/page.rs"), about);
  return dir;
}

function counter(label: string): string {
  return `#![allow(non_snake_case)]
js::directive!("use client");

use react::{Element, jsx, use_state};

pub fn Counter() -> Element {
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
  const routePath = readFileSync(join(dir, "app/route_path.js"), "utf8");
  expect([routePath.includes('import { useRouter } from "next/router";'), routePath.includes("return useRouter().asPath;")]).toEqual([true, true]);
  // The Server Component's page is rendered at build time, the counter in it.
  expect(readFileSync(join(dir, ".next/server/app/index.html"), "utf8")).toContain("Count <!-- -->0");
  const aboutHtml = readFileSync(join(dir, ".next/server/app/about.html"), "utf8");
  expect([aboutHtml.includes('class="home link"'), aboutHtml.includes('aria-label="Home page"')]).toEqual([true, true]);
  // Its props as written, an anchor's first, which the props it names
  // replace (ADR 0203, 0208).
  expect(readFileSync(join(dir, "app/about/page.jsx"), "utf8")).toContain('<Link href="/" {...anchor} className={classes} aria-label="Home page">');

  // A Rust error is the build's, rustc's message, and no Next.js build.
  writeFileSync(join(dir, "app/counter.rs"), counter("Count ") + "pub fn broken(");
  const failed = await finished(command(dir, ["build"]));
  expect([failed.code, failed.output.includes("error"), failed.output.includes("Next.js 16")]).toEqual([1, true, false]);
}, 300_000);

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
  } finally {
    await browser?.close();
    server.kill("SIGTERM");
    await new Promise((done) => (server.exitCode !== null ? done(undefined) : server.on("exit", done)));
  }
}, 300_000);
