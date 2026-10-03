// The pilot (ROADMAP M3.3, `examples/pilot`): a contacts app whose models and
// their rules are one crate, used by a native server and by a React client
// rust-js compiles, as Cargo checks the workspace (ADR 0101). Real Vite, a
// real browser, the server running: every flow the roadmap names.

import { afterAll, beforeAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, readdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { dirname, join, relative } from "node:path";
import { chromium, type Browser, type Locator, type Page } from "@playwright/test";
import type { Subprocess } from "bun";
import { build, createServer, type ViteDevServer } from "vite";
import { checkCargo } from "../tooling/cargo.js";
import { runSync } from "./child";
import { buildCompiler, buildSerde, compiler, fixture, root, run } from "./support";

const checkout = join(root, "examples", "pilot");
const pin = readFileSync(join(root, "rust-toolchain.toml"), "utf8").match(/channel = "([^"]+)"/)![1];

// The pilot, copied under `target/`, where its build writes its JS beside
// its Rust (ADR 0041): never into the checkout, whatever compiler a test is
// given.
const pilot = fixture("pilot");
cpSync(checkout, pilot, { recursive: true, filter: (from) => !/(^|[/\\])(target|node_modules|dist|\.cargo)([/\\]|$)/.test(relative(checkout, from)) });

/** The pilot is an app of its own, which names its packages by version
 * (ROADMAP M3.3): installed as a package manager installs them, this
 * checkout's, and Cargo told where its crates are by its `postinstall`. */
function install() {
  const modules = join(pilot, "node_modules");
  const packs = fixture("pilot-crates");
  run([process.execPath, "scripts/package-npm-crates.ts", packs]);
  for (const name of ["builtins", "webapi", "react"]) {
    const at = join(modules, "@rust-js", name);
    mkdirSync(at, { recursive: true });
    run(["tar", "-xzf", join(packs, `${name}.tgz`), "-C", at, "--strip-components", "1"]);
  }
  // rust-js's JS packages, and the libraries, as this checkout has them.
  const linked: [string, string][] = [
    ["@rust-js/runtime", join(root, "runtime")],
    ["@rust-js/vite-plugin", join(root, "vite-plugin")],
    ["@rust-js/build", join(root, "tooling")],
    ...["react", "react-dom", "sonner", "vite", "@vitejs/plugin-react"].map((name): [string, string] => [name, join(root, "node_modules", name)]),
  ];
  for (const [name, target] of linked) {
    mkdirSync(dirname(join(modules, name)), { recursive: true });
    symlinkSync(target, join(modules, name));
  }
  const patched = runSync([process.execPath, join(root, "tooling", "patch.js"), pilot], pilot, 60_000);
  if (patched.code !== 0) throw new Error(`rust-js-patch failed:\n${patched.stderr}`);
}

/** The JS beside a pilot's Rust, by path. */
function writtenJs(dir: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const crate of ["frontend", "models"]) {
    const src = join(dir, crate, "src");
    for (const name of readdirSync(src).filter((n) => /\.jsx?$/.test(n))) found.set(join(crate, "src", name), readFileSync(join(src, name), "utf8"));
  }
  return found;
}

let api: Subprocess | undefined;
let address = "";
let vite: ViteDevServer | undefined;
let browser: Browser | undefined;

/** The native server, on a port of its own, given `env`: it, and where it
 * listens. */
async function startServer(env: Record<string, string> = {}): Promise<[Subprocess, string]> {
  const server = Bun.spawn([join(pilot, "target", "debug", "server")], { env: { ...process.env, PORT: "0", ...env }, stdout: "pipe", stderr: "inherit" });
  const reader = (server.stdout as ReadableStream<Uint8Array>).getReader();
  let text = "";
  while (!text.includes("\n")) {
    const { value, done } = await reader.read();
    if (done) throw new Error("the server ended before it listened");
    text += new TextDecoder().decode(value);
  }
  reader.releaseLock();
  return [server, text.match(/listening on (\S+)/)![1]];
}

beforeAll(async () => {
  buildCompiler();
  install();
  // serde and serde_json, fetched as serde/build.sh locks them, which the
  // pilot's lockfile has too: its build is offline.
  buildSerde();
  // In the pilot, whose `.cargo/config.toml` is the patch.
  const built = runSync(["cargo", `+${pin}`, "build", "--offline", "--quiet", "-p", "server"], pilot, 600_000);
  if (built.code !== 0) throw new Error(`the server's build failed:\n${built.stderr}`);
  [api, address] = await startServer();
  process.env.PILOT_API = address;
  vite = await createServer({ root: join(pilot, "web"), configFile: join(pilot, "web", "vite.config.js"), logLevel: "silent", server: { port: 0 } });
  await vite.listen();
  browser = await chromium.launch({ headless: true });
}, 600_000);

afterAll(async () => {
  await browser?.close();
  await vite?.close();
  api?.kill();
  delete process.env.PILOT_API;
});

/** What `names` shows, once it's `expected`, or what it last was. */
async function settled(names: Locator, expected: string[]): Promise<string[]> {
  let last: string[] = [];
  for (let i = 0; i < 100; i++) {
    last = await names.allTextContents();
    if (JSON.stringify(last) === JSON.stringify(expected)) break;
    await Bun.sleep(50);
  }
  return last;
}

async function open(hash = "#/"): Promise<Page> {
  const page = await browser!.newPage();
  await page.goto(vite!.resolvedUrls!.local[0] + hash);
  return page;
}

// An app of its own (ROADMAP M3.3), as one outside this repository is: its
// crates and rust-js's packages by version, not a member of its workspace.
// It names a release, which this checkout's crates, the ones the test
// installs, are an upgrade of within its requirements: the pilot is the app
// a release's upgrade is checked with (M4.3).
test("the pilot names rust-js's crates and packages by version, not by a path into this checkout", () => {
  const version = (dir: string) => readFileSync(join(root, dir, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  const manifest = JSON.parse(readFileSync(join(checkout, "package.json"), "utf8"));
  const frontend = readFileSync(join(checkout, "frontend", "Cargo.toml"), "utf8");
  // `~0.0.2` takes 0.0.2 up to 0.1.0, as Cargo and npm both read it.
  const minor = (v: string) => v.split(".").slice(0, 2).join(".");
  for (const [dir, crate] of [["builtins", "js"], ["webapi", "webapi"], ["react", "react"]]) {
    const named = manifest.dependencies[`@rust-js/${dir}`];
    expect(frontend).toContain(`${crate} = { package = "rust-js-${dir}", version = "~${named}" }`);
    expect(Bun.semver.order(version(dir), named)).toBeGreaterThanOrEqual(0);
    expect(minor(version(dir))).toBe(minor(named));
  }
  // One release of the compiler's packages, the react crate's too.
  const release = manifest.devDependencies["@rust-js/native"];
  expect([manifest.dependencies["@rust-js/runtime"], manifest.dependencies["@rust-js/react"], manifest.devDependencies["@rust-js/build"], manifest.devDependencies["@rust-js/vite-plugin"]]).toEqual([release, release, release, release]);
  expect(manifest.scripts.postinstall).toBe("rust-js-patch");
  for (const crate of ["frontend", "models", "server"]) {
    expect(readFileSync(join(checkout, crate, "Cargo.toml"), "utf8")).not.toContain('path = "../../');
  }
  const workspaces: string[] = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).workspaces;
  expect(workspaces.filter((dir) => dir.startsWith("examples/pilot"))).toEqual([]);
});

// What's committed is what rust-js writes now: a change to the Rust is
// committed with its JS, as ReScript's projects do.
test("the pilot's committed JS is what rust-js writes from its Rust", async () => {
  const committed = writtenJs(checkout);
  expect(committed.size).toBeGreaterThan(0);
  // Built here, as Vite's plugin builds it, so a build that fails fails this.
  const react = JSON.parse(readFileSync(join(pilot, "node_modules", "react", "package.json"), "utf8")).version;
  await checkCargo({ manifestPath: join(pilot, "Cargo.toml"), toolchain: pin, compiler, packageName: "frontend", offline: true, react, inSource: true });
  expect(writtenJs(pilot)).toEqual(committed);
}, 600_000);

test("the list loads from the server, and a search shows only what matches", async () => {
  const page = await open();
  const names = page.locator(".contacts li a");
  await names.first().waitFor();
  expect(await names.allTextContents()).toEqual(["Ada Lovelace", "Alan Turing", "Grace Hopper"]);
  await page.getByLabel("Search").fill("gr");
  await page.locator(".contacts li a", { hasText: "Grace Hopper" }).waitFor();
  expect(await settled(names, ["Grace Hopper"])).toEqual(["Grace Hopper"]);
  await page.getByLabel("Search").fill("nobody");
  await page.getByText("No contacts match.").waitFor();
  await page.close();
}, 60_000);

// A search's answer that comes after a newer one's must never show: the
// older request is aborted.
test("a slow answer to an older search doesn't replace a newer one's", async () => {
  const page = await open();
  await page.locator(".contacts li a").first().waitFor();
  let slow = 0;
  await page.route(/\/api\/contacts\?q=a$/, async (route) => {
    slow++;
    await Bun.sleep(1_000);
    await route.continue().catch(() => {});
  });
  await page.getByLabel("Search").fill("a");
  await page.getByLabel("Search").fill("alan");
  const names = page.locator(".contacts li a");
  await page.locator(".contacts li a", { hasText: "Alan Turing" }).waitFor();
  expect(await settled(names, ["Alan Turing"])).toEqual(["Alan Turing"]);
  await Bun.sleep(1_500);
  expect(slow).toBe(1);
  expect(await names.allTextContents()).toEqual(["Alan Turing"]);
  await page.close();
}, 60_000);

test("a contact is its own page, and one that doesn't exist says so", async () => {
  const page = await open();
  await page.getByRole("link", { name: "Grace Hopper" }).click();
  await page.getByRole("heading", { name: "Grace Hopper" }).waitFor();
  expect(page.url()).toEndWith("#/contacts/3");
  expect(await page.locator("dd").allTextContents()).toEqual(["grace@example.com", "85"]);
  await page.goto(vite!.resolvedUrls!.local[0] + "#/contacts/99");
  await page.getByText("There's no contact 99.").waitFor();
  await page.goto(vite!.resolvedUrls!.local[0] + "#/nowhere");
  await page.getByText("There's no such page.").waitFor();
  await page.close();
}, 60_000);

// The form checks what the server would, before it sends; what only the
// server knows, a taken email, comes back by field; and a new contact is
// its page, announced by the npm package's toast.
test("the form checks as the server does, shows the server's errors, and adds a contact", async () => {
  const page = await open("#/new");
  const posts: string[] = [];
  page.on("request", (request) => { if (request.method() === "POST") posts.push(request.postData() ?? ""); });
  const submit = page.getByRole("button", { name: "Add contact" });
  await page.getByLabel("Email").fill("not an email");
  await page.getByLabel("Age").fill("old");
  await submit.click();
  const errors = page.locator(".field-error");
  await errors.first().waitFor();
  expect(await errors.allTextContents()).toEqual(['a name is required', '"not an email" isn\'t an email address', '"old" isn\'t a whole number']);
  expect(await page.getByLabel("Email").getAttribute("aria-invalid")).toBe("true");
  expect(posts).toEqual([]);

  await page.getByLabel("Name").fill("Ada Again");
  await page.getByLabel("Email").fill("ADA@example.com");
  await page.getByLabel("Age").fill("30");
  await submit.click();
  await page.getByText("ADA@example.com is taken").waitFor();
  expect(posts.map((body) => JSON.parse(body))).toEqual([{ name: "Ada Again", email: "ADA@example.com", age: 30 }]);

  await page.getByLabel("Name").fill("  Katherine Johnson ");
  await page.getByLabel("Email").fill("katherine@example.com");
  await page.getByLabel("Age").fill("101");
  await submit.click();
  await page.getByRole("heading", { name: "Katherine Johnson" }).waitFor();
  await page.getByText("Added Katherine Johnson").waitFor();
  expect(page.url()).toMatch(/#\/contacts\/\d+$/);
  await page.getByRole("link", { name: "Contacts", exact: true }).click();
  await page.getByRole("link", { name: "Katherine Johnson" }).waitFor();
  await page.close();
}, 60_000);

// The server holds a request that isn't the form's to the same rules, and
// JSON that isn't a contact is refused, each with the server's reason.
test("the server refuses invalid JSON and a contact breaking the rules", async () => {
  const post = (body: string) => fetch(`${address}/api/contacts`, { method: "POST", headers: { "content-type": "application/json" }, body });
  const broken = await post("{ not json");
  expect(broken.status).toBe(400);
  expect((await broken.json()).message).toStartWith("invalid JSON");
  const wrongType = await post('{"name":"Bo","email":"bo@example.com","age":"ten"}');
  expect(wrongType.status).toBe(400);
  const invalid = await post('{"name":" ","email":"bo","age":5}');
  expect(invalid.status).toBe(422);
  expect((await invalid.json()).errors.map((e: { field: string }) => e.field)).toEqual(["name", "email", "age"]);
}, 60_000);

// The Rust edited as the pilot runs: `list.rs`, its heading in it.
const list = join(pilot, "frontend", "src", "list.rs");
const listSource = readFileSync(list, "utf8");
const heading = (text: string) => {
  const edited = listSource.replace('<h1>{"Contacts"}</h1>', `<h1>{"${text}"}</h1>`);
  if (edited === listSource) throw new Error("list.rs has no heading to edit");
  return edited;
};

// A save is a Fast Refresh (M3.4): the page shows the new Rust and keeps its
// state, the search typed, and itself: it isn't reloaded.
test("a save of the Rust is a Fast Refresh, which keeps what's typed", async () => {
  const page = await open();
  await page.getByLabel("Search").fill("gr");
  await page.locator(".contacts li a", { hasText: "Grace Hopper" }).waitFor();
  await page.evaluate(() => Object.assign(window, { kept: true }));
  try {
    writeFileSync(list, heading("People"));
    await page.getByRole("heading", { name: "People" }).waitFor();
    expect(await page.getByLabel("Search").inputValue()).toBe("gr");
    expect(await page.evaluate(() => "kept" in window)).toBe(true);
  } finally {
    writeFileSync(list, listSource);
  }
  await page.getByRole("heading", { name: "Contacts" }).waitFor();
  await page.close();
}, 60_000);

// A compile error is Vite's overlay, and the app runs on as it last compiled;
// fixed, the overlay goes, and the page is the fix (M3.4).
test("a compile error shows in Vite's overlay, and the app runs on until it's fixed", async () => {
  const page = await open();
  await page.locator(".contacts li a").first().waitFor();
  try {
    writeFileSync(list, listSource + "\npub fn broken(");
    await page.locator("vite-error-overlay").waitFor();
    await page.getByLabel("Search").fill("ada");
    expect(await settled(page.locator(".contacts li a"), ["Ada Lovelace"])).toEqual(["Ada Lovelace"]);
    writeFileSync(list, heading("Fixed"));
    await page.locator("vite-error-overlay").waitFor({ state: "detached" });
    await page.getByRole("heading", { name: "Fixed" }).waitFor();
  } finally {
    writeFileSync(list, listSource);
  }
  await page.getByRole("heading", { name: "Contacts" }).waitFor();
  await page.close();
}, 60_000);

// What the browser runs maps back to the Rust (M3.4): its devtools show
// `list.rs`, as it was saved, and set its breakpoints there.
test("the client the browser runs maps back to its Rust", async () => {
  const page = await open();
  await page.locator(".contacts li a").first().waitFor();
  const loaded = await page.evaluate(() => performance.getEntriesByType("resource").map((entry) => entry.name));
  const url = loaded.find((name) => /\/list\.jsx(\?|$)/.test(name));
  expect(url).toBeDefined();
  const module = await (await fetch(url!)).text();
  const inline = module.match(/\/\/# sourceMappingURL=data:application\/json;(?:charset=utf-8;)?base64,(\S+)/);
  expect(inline).not.toBeNull();
  const map = JSON.parse(Buffer.from(inline![1], "base64").toString("utf8"));
  // Each source as the browser finds it, from the module's URL.
  const sources = map.sources.map((source: string) => decodeURIComponent(new URL(source, new URL(map.sourceRoot ?? "", url)).pathname));
  const at = sources.indexOf(`/@fs${list}`);
  expect(at).toBeGreaterThanOrEqual(0);
  expect(map.sourcesContent[at]).toBe(listSource);
  await page.close();
}, 60_000);

/** What the server answers a request for `path` as it's written, which
 * `fetch` would have made a URL of first: its status. */
async function rawStatus(at: string, path: string): Promise<number> {
  const { hostname, port } = new URL(at);
  const answer = await new Promise<string>((resolve, reject) => {
    let text = "";
    const socket = connect(Number(port), hostname, () => socket.write(`GET ${path} HTTP/1.1\r\nhost: ${hostname}\r\n\r\n`));
    socket.on("data", (chunk) => (text += chunk));
    socket.on("end", () => resolve(text));
    socket.on("error", reject);
  });
  return Number(answer.split(" ")[1]);
}

// A deployment (M3.4): Vite's production build, served by the native server
// beside its API, one process, as the app is deployed.
test("the production build, served by the native server with its API, works", async () => {
  await build({ root: join(pilot, "web"), configFile: join(pilot, "web", "vite.config.js"), logLevel: "silent" });
  const dist = join(pilot, "web", "dist");
  const [server, at] = await startServer({ DIST: dist });
  try {
    const page = await browser!.newPage();
    await page.goto(`${at}/#/`);
    const names = page.locator(".contacts li a");
    await names.first().waitFor();
    expect(await names.allTextContents()).toEqual(["Ada Lovelace", "Alan Turing", "Grace Hopper"]);
    await page.getByLabel("Search").fill("gr");
    expect(await settled(names, ["Grace Hopper"])).toEqual(["Grace Hopper"]);
    await names.first().click();
    await page.getByRole("heading", { name: "Grace Hopper" }).waitFor();
    expect(await page.locator("dd").allTextContents()).toEqual(["grace@example.com", "85"]);
    await page.close();
    // Its files, and nothing outside them.
    const index = await fetch(`${at}/`);
    expect(index.headers.get("content-type")).toStartWith("text/html");
    expect(await index.text()).toBe(readFileSync(join(dist, "index.html"), "utf8"));
    expect(await rawStatus(at, "/../../Cargo.toml")).toBe(404);
    expect(await rawStatus(at, "/assets/../../../Cargo.toml")).toBe(404);
    expect((await fetch(`${at}/nothing.js`)).status).toBe(404);
  } finally {
    server.kill();
  }
}, 600_000);

// Last: it stops the server. The list says why it has nothing, and loads
// once the server is back.
test("the list says when the server can't answer, and tries again", async () => {
  api!.kill();
  await api!.exited;
  const page = await open();
  await page.getByRole("alert").waitFor();
  expect(await page.getByRole("alert").textContent()).toContain("the server answered");
  const port = new URL(address).port;
  api = Bun.spawn([join(pilot, "target", "debug", "server")], { env: { ...process.env, PORT: port }, stdout: "ignore", stderr: "inherit" });
  await Bun.sleep(500);
  await page.getByRole("button", { name: "Try again" }).click();
  await page.getByRole("link", { name: "Ada Lovelace" }).waitFor();
  await page.close();
}, 60_000);
