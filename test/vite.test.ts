import { expect, test } from "bun:test";
import { mkdirSync, writeFileSync, readFileSync, unlinkSync, existsSync } from "node:fs";
import { join } from "node:path";
import { createServer, build } from "vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";
import { chromium } from "@playwright/test";
import { buildCompiler, cargoApp, compiler, fixture, root } from "./support";

// A Vite app is Vite's alone, its plugins React's and Tailwind's, and its
// Rust is compiled by `rust-js build`, its own process (ADR 0412): Vite
// sees the JS it writes beside the Rust as any edit.

// Each server's optimized dependencies are its own, in its root: a fixture
// has no package.json, so Vite's default is this checkout's node_modules/.vite,
// which servers side by side rewrite under each other, and a page never loads.
const cacheDir = ".vite";
const cli = join(root, "tooling/cli.js");

/** A Vite app in `dir` whose Rust is a Cargo package, `app`, of
 * `src/App.rs` (ADR 0101): its JS is `src/App.jsx`, beside it, which
 * `main.jsx` imports, as `./App`, whichever file it is. */
function viteApp(dir: string, rust: string, toml = "") {
  cargoApp(dir, rust, { lib: "src/App.rs", react: true, toml });
  writeFileSync(join(dir, "index.html"), '<div id="root"></div><script type="module" src="/src/main.jsx"></script>');
  writeFileSync(join(dir, "src/main.jsx"), 'import {createRoot} from "react-dom/client"; import {App} from "./App"; createRoot(document.getElementById("root")).render(<App/>);');
}

/** `rust-js build`, in `dir`, as the developer runs it: what it said, and
 * how it ended. */
function rustJs(dir: string, args: string[] = [], env: Record<string, string> = {}) {
  const run = Bun.spawnSync([process.execPath, cli, "build", ...args], {
    cwd: dir,
    env: { ...process.env, RUST_JS_COMPILER: compiler, ...env },
    stdout: "pipe",
    stderr: "pipe",
  });
  return { code: run.exitCode, output: run.stdout.toString() + run.stderr.toString() };
}

/** `file` written, then `rust-js build`, which must succeed. */
function saved(dir: string, file: string, text: string) {
  writeFileSync(file, text);
  const built = rustJs(dir);
  expect(built.output).toBe("");
  expect(built.code).toBe(0);
}

// Real Vite, real compiler, real React Fast Refresh. Isolated sources and
// outputs: the example application and its development server are untouched.
test("Vite builds and refreshes an app's Rust as rust-js build writes it, through an error and module changes", async () => {
  buildCompiler();
  const dir = fixture("vite");
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
mod text;
pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! { <button onClick={move |_| set_count.update(|n| n + 1)}>{text::label()}{count}</button> }
}
`;
  viteApp(dir, source);
  const app = join(dir, "src/App.rs"), text = join(dir, "src/text.rs"), output = join(dir, "src/App.jsx");
  saved(dir, text, 'pub fn label() -> &\'static str { "Count " }');
  await build({ root: dir, configFile: false, plugins: [react()], logLevel: "silent" });
  expect(readFileSync(join(dir, "dist/index.html"), "utf8")).toContain("/assets/");
  const server = await createServer({ root: dir, cacheDir, configFile: false, plugins: [react()], logLevel: "silent", server: { port: 0 } });
  let browser;
  try {
    await server.listen();
    if (!server.resolvedUrls) throw new Error("Vite did not expose a listening URL");
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(server.resolvedUrls.local[0]);
    const button = page.getByRole("button");
    await button.filter({ hasText: "Count 0" }).waitFor();
    await button.click();
    await button.filter({ hasText: "Count 1" }).waitFor();
    saved(dir, text, 'pub fn label() -> &\'static str { "Changed " }');
    await button.filter({ hasText: "Changed 1" }).waitFor();

    saved(dir, app, source.replace("<button onClick", '<button className="updated" onClick'));
    await page.locator("button.updated").waitFor();
    expect(await button.textContent()).toBe("Changed 1");

    // An error is rustc's, rust-js build's, and nothing is written: the page
    // keeps what last compiled, its state too.
    const good = readFileSync(output, "utf8");
    writeFileSync(app, source + "pub fn broken(");
    const broken = rustJs(dir);
    expect([broken.code, broken.output.includes("unclosed delimiter")]).toEqual([1, true]);
    expect(readFileSync(output, "utf8")).toBe(good);
    saved(dir, app, source);
    await button.filter({ hasText: "Changed 1" }).waitFor();

    // A module gone is an error too, and back, it's what the page shows.
    unlinkSync(text);
    expect(rustJs(dir).code).toBe(1);
    saved(dir, text, 'pub fn label() -> &\'static str { "Restored " }');
    await button.filter({ hasText: "Restored 1" }).waitFor();

    // A declared module, added and edited before a build: the build is of
    // the latest source.
    writeFileSync(app, source.replace("mod text;", "mod text; mod extra;").replace("text::label()", "extra::label()"));
    writeFileSync(join(dir, "src/extra.rs"), 'pub fn label() -> &\'static str { "First " }');
    saved(dir, join(dir, "src/extra.rs"), 'pub fn label() -> &\'static str { "Latest " }');
    await button.filter({ hasText: /Latest [01]/ }).waitFor();
  } finally {
    await browser?.close();
    await server.close();
  }
}, 180_000);

// Tailwind reads class names in the JSX rust-js writes, and React Compiler
// memoizes the component (ADR 0041). The `.rs` files aren't Tailwind's
// sources: an edit of one, before it's built, would be a file it scans
// changing, which reloads the page. A build is still a Fast Refresh.
test("Tailwind and React Compiler keep Fast Refresh's state", async () => {
  buildCompiler();
  const dir = fixture("vite-tailwind");
  const app = (classes: string) => `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! {
        <button className="${classes}" onClick={move |_| set_count.update(|n| n + 1)}>
            {"Count "}
            {count}
        </button>
    }
}
`;
  viteApp(dir, app("font-bold"));
  writeFileSync(join(dir, "src/index.css"), '@import "tailwindcss" source(none);\n@source "./App.jsx";\n');
  writeFileSync(join(dir, "src/main.jsx"), 'import "./index.css"; import {createRoot} from "react-dom/client"; import {App} from "./App"; createRoot(document.getElementById("root")).render(<App/>);');
  expect(rustJs(dir).code).toBe(0);
  const plugins = [react(), babel({ presets: [reactCompilerPreset()] }), tailwindcss()];
  const server = await createServer({ root: dir, cacheDir, configFile: false, plugins, logLevel: "silent", server: { port: 0 } });
  let browser;
  try {
    await server.listen();
    if (!server.resolvedUrls) throw new Error("Vite did not expose a listening URL");
    const url = server.resolvedUrls.local[0];
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(url);
    const button = page.getByRole("button");
    await button.filter({ hasText: "Count 0" }).waitFor();
    expect(await button.evaluate(b => getComputedStyle(b).fontWeight)).toBe("700");
    // React Compiler's memo cache, in the component rust-js wrote.
    expect(await (await fetch(new URL("src/App.jsx", url))).text()).toMatch(/const \$ = _c\(\d+\);/);
    await button.click();
    await button.filter({ hasText: "Count 1" }).waitFor();
    await page.evaluate(() => { (window as any).sameDocument = true; });

    // A class that's new to Tailwind: its CSS arrives, and the state stays.
    saved(dir, join(dir, "src/App.rs"), app("font-bold underline"));
    await page.locator("button.underline").waitFor();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("button")!).textDecorationLine === "underline");
    expect(await button.textContent()).toBe("Count 1");
    expect(await page.evaluate(() => (window as any).sameDocument)).toBe(true);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 120_000);

// The generated JSX is committed, as ReScript recommends for its JS (ADR
// 0041), and the source map isn't. So a checkout without rust-js still
// builds, Vite's alone; `rust-js build` says there's none to compile it.
test("Without rust-js, Vite builds the committed JSX, and rust-js build says so", async () => {
  buildCompiler();
  const dir = fixture("vite-committed");
  viteApp(dir, `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn App() -> JSX::Element {
    jsx! {
        <p>{"Committed"}</p>
    }
}
`);
  expect(rustJs(dir).code).toBe(0);
  unlinkSync(join(dir, "src/App.jsx.map"));
  const missing = join(dir, "no-rust-js");
  const none = rustJs(dir, [], { RUST_JS_COMPILER: missing });
  expect([none.code, none.output.trim()]).toEqual([1, `rust-js: no rust-js at ${missing}: install rust-js or build it with cargo build to compile the Rust`]);
  await build({ root: dir, configFile: false, plugins: [react()], logLevel: "silent" });
  expect(readFileSync(join(dir, "dist/index.html"), "utf8")).toContain("/assets/");
}, 120_000);

// `rust-js build` writes the patch that tells the app's Cargo, and its
// editor, where the crates its npm packages have are; a crate installed
// twice stops it, with who asked for each.
test("rust-js build writes the app's patch, and stops at a crate installed twice", async () => {
  buildCompiler();
  const dir = fixture("vite-patch");
  viteApp(dir, `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn App() -> JSX::Element {
    jsx! {
        <p>{"Patched"}</p>
    }
}
`);
  writeFileSync(join(dir, "package.json"), JSON.stringify({ private: true, dependencies: { "@x/a": "~0.0.1" } }));
  const crate = (at: string, version: string) => {
    mkdirSync(join(at, "src"), { recursive: true });
    writeFileSync(join(at, "package.json"), JSON.stringify({ name: "@x/a", version, "rust-js": { crate: "zz-a" } }));
    writeFileSync(join(at, "Cargo.toml"), `[package]\nname = "zz-a"\nversion = "${version}"\nedition = "2024"\n`);
    writeFileSync(join(at, "src", "lib.rs"), "");
  };
  crate(join(dir, "node_modules", "@x", "a"), "0.0.1");
  expect(rustJs(dir).code).toBe(0);
  expect(readFileSync(join(dir, ".cargo", "config.toml"), "utf8")).toContain('zz-a = { path = "node_modules/@x/a" }');

  // Another package that asks for another version of it.
  const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  writeFileSync(join(dir, "package.json"), JSON.stringify({ ...pkg, dependencies: { ...pkg.dependencies, "@x/b": "~0.0.1" } }));
  mkdirSync(join(dir, "node_modules", "@x", "b"), { recursive: true });
  writeFileSync(join(dir, "node_modules", "@x", "b", "package.json"), JSON.stringify({ name: "@x/b", version: "0.0.1", "rust-js": { crate: "zz-b" }, peerDependencies: { "@x/a": "~0.0.2" } }));
  writeFileSync(join(dir, "node_modules", "@x", "b", "Cargo.toml"), `[package]\nname = "zz-b"\nversion = "0.0.1"\nedition = "2024"\n`);
  crate(join(dir, "node_modules", "@x", "b", "node_modules", "@x", "a"), "0.0.2");
  const twice = rustJs(dir);
  expect([twice.code, twice.output.includes("zz-a is installed twice")]).toEqual([1, true]);
}, 120_000);

// A crate's checks (ADR 0117), of its Cargo build: `rust-js build` runs
// each; what one says, of the Rust, is its own output.
test("rust-js build runs a crate's checks", () => {
  buildCompiler();
  const dir = fixture("vite-checks");
  viteApp(dir, `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn App() -> JSX::Element {
    jsx! {
        <p>{"Checked"}</p>
    }
}
`, `[package.metadata.rust-js.hooks]
check = [
  { run = ["sh", "-c", "touch built"] },
  { run = ["sh", "-c", "echo \\"$1:4:3: a note\\"", "sh", "{files}"], fail = "never" },
]
`);
  const checked = rustJs(dir);
  expect(checked.code).toBe(0);
  expect(existsSync(join(dir, "built"))).toBe(true);
  expect(checked.output).toMatch(/src\/App\.rs:\d+:\d+: \[sh\] a note/);
}, 120_000);

// Fast Refresh keeps state under a context and in a memoized component
// (ADR 0041). Building a module runs it again, so a context made in it is a
// new one, and React remounts what's under its provider: hand-written React
// does the same. A context in a module of its own, as React advises, is
// untouched when a component's module changes: rust-js leaves unchanged
// files alone.
test("Fast Refresh keeps state under a context from its own module, and in memo", async () => {
  buildCompiler();
  const dir = fixture("vite-context");
  const app = (label: string) => `#![allow(non_snake_case)]
use react::{JSX, MemoExoticComponent, jsx, memo, use_context, use_state};
mod theme;
use theme::THEME;
thread_local! {
    static FAST_LABEL: MemoExoticComponent<LabelProps> = memo(Label);
}
pub struct LabelProps {
    pub text: &'static str,
}
pub fn Label(LabelProps { text }: LabelProps) -> JSX::Element {
    let theme = use_context(&THEME);
    let (n, set_n) = use_state(0);
    jsx! {
        <button className={*theme} onClick={move |_| set_n.update(|n| n + 1)}>
            {text}
            {" "}
            {n}
        </button>
    }
}
pub fn App() -> JSX::Element {
    jsx! {
        <THEME value="dark">
            <FAST_LABEL text="${label}" />
        </THEME>
    }
}
`;
  viteApp(dir, app("Count"));
  saved(dir, join(dir, "src/theme.rs"), `use react::{Context, create_context};
thread_local! {
    pub static THEME: Context<&'static str> = create_context("light");
}
`);
  const server = await createServer({ root: dir, cacheDir, configFile: false, plugins: [react()], logLevel: "silent", server: { port: 0 } });
  let browser;
  try {
    await server.listen();
    if (!server.resolvedUrls) throw new Error("Vite did not expose a listening URL");
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(server.resolvedUrls.local[0]);
    const theme = readFileSync(join(dir, "src/theme.js"), "utf8");
    expect(theme).toContain('export const THEME = createContext("light");');
    expect(readFileSync(join(dir, "src/App.jsx"), "utf8")).toContain('<THEME value="dark">');
    const button = page.getByRole("button");
    await button.filter({ hasText: "Count 0" }).waitFor();
    await button.click();
    await button.filter({ hasText: "Count 1" }).waitFor();
    await page.evaluate(() => { (window as any).sameDocument = true; });

    saved(dir, join(dir, "src/App.rs"), app("Clicks"));
    await button.filter({ hasText: /^Clicks/ }).waitFor();
    // The state, the context's value, and the page are all still there.
    expect(await button.textContent()).toBe("Clicks 1");
    expect(await button.getAttribute("class")).toBe("dark");
    expect(await page.evaluate(() => (window as any).sameDocument)).toBe(true);
    expect(readFileSync(join(dir, "src/theme.js"), "utf8")).toBe(theme);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 120_000);

// A Cargo workspace (ADR 0101), a full-stack app's: Cargo builds each crate,
// rust-js as its workspace wrapper, of the package `--package` names, and
// writes each crate's JS beside its Rust. The app is a directory of the
// workspace, Vite's root, which imports the package's JS where it is, and a
// package the Rust imports is the app's, in the workspace's node_modules.
// A build of a crate the component uses is a Fast Refresh.
test("rust-js build builds a Cargo workspace's package, which Vite refreshes when a crate it uses changes", async () => {
  buildCompiler();
  const dir = fixture("vite-cargo");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["ui", "models"]\nresolver = "2"\n');
  writeFileSync(join(dir, "package.json"), '{ "private": true, "type": "module" }\n');
  for (const name of ["ui", "models"]) mkdirSync(join(dir, name, "src"), { recursive: true });
  writeFileSync(join(dir, "models/Cargo.toml"), '[package]\nname = "models"\nversion = "0.1.0"\nedition = "2024"\n');
  const models = join(dir, "models/src/lib.rs");
  writeFileSync(models, 'pub fn label() -> &\'static str {\n    "Count "\n}\n');
  writeFileSync(join(dir, "ui/Cargo.toml"), `[package]\nname = "ui"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nmodels = { path = "../models" }\nreact = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }\n`);
  const ui = join(dir, "ui/src/lib.rs");
  const source = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};

#[rust_js::link_name = "widget#greeting"]
fn greeting() -> &'static str {
    unreachable!()
}

pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(0);
    jsx! { <button title={greeting()} onClick={move |_| set_count.update(|n| n + 1)}>{models::label()}{count}</button> }
}
`;
  writeFileSync(ui, source);
  mkdirSync(join(dir, "node_modules/widget"), { recursive: true });
  writeFileSync(join(dir, "node_modules/widget/package.json"), '{ "name": "widget", "type": "module", "main": "index.js" }\n');
  writeFileSync(join(dir, "node_modules/widget/index.js"), 'export function greeting() { return "hello from widget"; }\n');
  const web = join(dir, "web");
  mkdirSync(join(web, "src"), { recursive: true });
  writeFileSync(join(web, "index.html"), '<div id="root"></div><script type="module" src="/src/main.jsx"></script>');
  writeFileSync(join(web, "src/main.jsx"), 'import {createRoot} from "react-dom/client"; import {App} from "../../ui/src/lib"; createRoot(document.getElementById("root")).render(<App/>);');
  // A workspace has no package of its own: `--package` names the app's.
  const unnamed = rustJs(dir);
  expect([unnamed.code, unnamed.output.includes("`rust-js build --package <name>`")]).toEqual([1, true]);
  const built = rustJs(dir, ["--package", "ui"]);
  expect([built.code, built.output]).toEqual([0, ""]);
  // Each crate's JS imports the others' beside their Rust (ADR 0041).
  expect(readFileSync(join(dir, "ui/src/lib.jsx"), "utf8")).toContain('from "../../models/src/lib.js"');
  expect(existsSync(join(dir, "models/src/lib.js"))).toBe(true);
  await build({ root: web, configFile: false, plugins: [react()], logLevel: "silent" });
  expect(readFileSync(join(web, "dist/index.html"), "utf8")).toContain("/assets/");
  const server = await createServer({ root: web, cacheDir, configFile: false, plugins: [react()], logLevel: "silent", server: { port: 0 } });
  let browser;
  try {
    await server.listen();
    if (!server.resolvedUrls) throw new Error("Vite did not expose a listening URL");
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(server.resolvedUrls.local[0]);
    const button = page.getByRole("button");
    await button.filter({ hasText: "Count 0" }).waitFor();
    expect(await button.getAttribute("title")).toBe("hello from widget");
    await button.click();
    await button.filter({ hasText: "Count 1" }).waitFor();
    await page.evaluate(() => { (window as any).sameDocument = true; });

    writeFileSync(models, 'pub fn label() -> &\'static str {\n    "Changed "\n}\n');
    expect(rustJs(dir, ["--package", "ui"]).code).toBe(0);
    await button.filter({ hasText: "Changed 1" }).waitFor();
    expect(await page.evaluate(() => (window as any).sameDocument)).toBe(true);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 180_000);
