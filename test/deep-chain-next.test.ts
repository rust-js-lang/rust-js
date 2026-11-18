// The deep-chain harness's second step: the chain's crates (geometry,
// primitives, kit and forms, test/deep-chain/) under a Next.js app, as a
// Next app over shadcn over Radix over Floating UI is, each a path
// dependency of the app's crate. `rust-js build` compiles them with it,
// `next build` prerenders the page on the server, and in a browser the
// client component's dropdown opens where geometry places it beside its
// trigger, measured from the page's own layout, flips above one near the
// viewport's bottom, closes as an item is chosen, and stops listening to
// the window as it unmounts.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync, realpathSync, existsSync } from "node:fs";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "@playwright/test";
import { buildCompiler, compiler, KeptApp, root, seeded } from "./support";
import { cacheDirOf } from "../tooling/build.js";
import { browserDeadline, clicked, hydrated, seen } from "./browser-waits";
import { nextReady, served } from "./ports";

const example = join(root, "examples/next");
const cli = join(root, "tooling/cli.js");
const react = JSON.stringify(join(root, "react"));

/** Each crate of the chain, and what it depends on, by path. */
const crates: Record<string, string> = {
  geometry: "",
  primitives: `geometry = { path = "../geometry" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  kit: `primitives = { path = "../primitives" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  forms: `primitives = { path = "../primitives" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
};

const page = `// The chain's page: a Server Component of the chain's client component.
mod demo;

use react::{JSX, jsx};

pub fn Home() -> JSX::Element {
    jsx! {
        <main>
            <h1>{"The chain"}</h1>
            <demo::Demo />
        </main>
    }
}

js::export_default!(Home);
`;

const demo = `js::directive!("use client");

use forms::{Countries, Country, Select};
use kit::{DropdownMenu, Menu, Placement};
use primitives::Collection;
use react::{CSSProperties, JSX, jsx, use_state};

fn countries() -> Countries {
    Countries(vec![
        Country { code: "VN", name: "Vietnam" },
        Country { code: "FR", name: "France" },
    ])
}

/// A dropdown of actions at the top, one near the viewport's bottom, whose
/// four items don't fit below it, so it opens above its trigger, and a
/// country select, closed.
pub fn Demo() -> JSX::Element {
    let (open, set_open) = use_state(|| false);
    let (more, set_more) = use_state(|| false);
    let (shown, set_shown) = use_state(|| true);
    let (chosen, set_chosen) = use_state(|| None::<usize>);
    let placement: Placement = "bottom-start".parse().unwrap_or_default();
    let actions = vec!["Edit".to_string(), "Delete".to_string()];
    let names = actions.clone();
    let menu = jsx! {
        <Menu
            label="Actions"
            items={actions}
            onSelect={Some(Box::new(move |index| {
                set_chosen.set(Some(index));
                set_open.set(false);
            }))}
        />
    };
    let shown_choice = chosen.map_or("none".to_string(), |index| names[index].clone());
    let actions_menu = shown.then(|| {
        jsx! {
            <DropdownMenu
                open={*open}
                trigger="Actions"
                placement={Some(placement)}
                onOpenChange={Some(Box::new(move |open| set_open.set(open)))}
                className={Some("dropdown")}
            >
                {menu}
            </DropdownMenu>
        }
    });
    jsx! {
        <section>
            <button className="unmount" onClick={move |_| set_shown.set(false)}>{"Unmount"}</button>
            {actions_menu}
            <p className="chosen">{format!("Chosen: {shown_choice}")}</p>
            <Select label="Country" open={false} options={countries().labels()} placement={Some(placement)} />
            <DropdownMenu
                open={*more}
                trigger="More"
                placement={Some(placement)}
                onOpenChange={Some(Box::new(move |open| set_more.set(open)))}
                className={Some("more")}
                style={Some(CSSProperties::new().position("fixed").top(700).left(8))}
            >
                <Menu label="More" items={["Share", "Copy", "Move", "Archive"].map(String::from).to_vec()} />
            </DropdownMenu>
        </section>
    }
}
`;

/** The example, its page the chain's, the chain's crates beside it, each a
 * path dependency of the app's crate. Kept between runs, its Cargo target
 * in its `node_modules/.cache` (`KeptApp`). */
function chainApp(name: string): string {
  const kept = new KeptApp(name);
  const dir = kept.dir;
  kept.copy(example, "", (path) => !/^(node_modules|\.next|target|Cargo\.lock|\.cargo|app\/page\.(jsx|d\.ts|jsx\.map))(\/|$)/.test(path) && !path.endsWith(".map"));
  for (const entry of readdirSync(join(example, "node_modules"))) {
    if (entry !== ".cache") kept.link(join("node_modules", entry), realpathSync(join(example, "node_modules", entry)));
  }
  for (const [crate, dependencies] of Object.entries(crates)) {
    kept.copy(join(root, "test", "deep-chain", crate), join("chain", crate));
    kept.write(join("chain", crate, "Cargo.toml"), `[package]\nname = "${crate}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${dependencies}\n[lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_js)"] }\n`);
  }
  const chained = Object.keys(crates).map((crate) => `${crate} = { path = "chain/${crate}" }`).join("\n");
  const manifest = readFileSync(join(example, "Cargo.toml"), "utf8").replaceAll('path = "../../', `path = "${root}/`);
  if (!manifest.includes("\n[dependencies]\n")) throw new Error("the example's Cargo.toml has no [dependencies]");
  kept.write("Cargo.toml", manifest.replace("\n[dependencies]\n", `\n[dependencies]\n${chained}\n`));
  kept.write("app/page.rs", page);
  kept.write("app/demo.rs", demo);
  kept.done();
  if (!existsSync(join(cacheDirOf(dir), "target"))) seeded(join(cacheDirOf(dir), "target"));
  return dir;
}

/** geometry's `clamp`, as its Rust writes it: `min` where the value is
 * below it, else `max` where above it, else the value. */
const shifted = (value: number, min: number, max: number) => (value < min ? min : value > max ? max : value);

function finished(child: ReturnType<typeof spawn>): Promise<{ code: number | null; output: string }> {
  let output = "";
  child.stdout?.on("data", (data) => (output += data));
  child.stderr?.on("data", (data) => (output += data));
  // On `close`, once its output is all read: `exit` may come first, its
  // last lines not yet read (#1951).
  return new Promise((done) => child.on("close", (code) => done({ code, output })));
}

const next = (dir: string, args: string[], env: Record<string, string> = {}) =>
  spawn(process.execPath, [join(dir, "node_modules/next/dist/bin/next"), ...args], { cwd: dir, env: { ...process.env, RUST_JS_COMPILER: compiler, NEXT_TELEMETRY_DISABLED: "1", ...env } });

test("a Next app over the chain's crates builds, renders on the server, and its dropdown opens, flips, closes and unmounts in a browser", async () => {
  buildCompiler();
  const dir = chainApp("deep-chain-next");
  const compiled = await finished(spawn(process.execPath, [cli, "build"], { cwd: dir, env: { ...process.env, RUST_JS_COMPILER: compiler } }));
  expect([compiled.code, compiled.output]).toEqual([0, ""]);
  const built = await finished(next(dir, ["build"]));
  expect([built.code, built.output.includes("○ /")]).toEqual([0, true]);
  // The client component's module is one, the page a Server Component's.
  expect(readFileSync(join(dir, "app/demo.jsx"), "utf8").split("\n").find((line) => line && !line.startsWith("//"))).toBe('"use client";');
  // Prerendered on the server: each dropdown closed, at the placement asked.
  const html = readFileSync(join(dir, ".next/server/app/index.html"), "utf8");
  expect(html).toContain('<span class="dropdown" data-placement="bottom-start"><button aria-expanded="false">Actions</button></span>');
  expect(html).toContain('<p class="chosen">Chosen: none</p>');
  expect(html).toContain('<button aria-expanded="false">Country</button>');

  const { server, port } = await served((port) => next(dir, ["start", "--port", String(port)]), nextReady);
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
    const tab = await browser.newPage({ viewport: { width: 1024, height: 768 } });
    tab.setDefaultTimeout(browserDeadline());
    const errors: string[] = [];
    tab.on("pageerror", (error) => errors.push(String(error)));
    tab.on("console", (message) => message.type() === "error" && errors.push(message.text()));
    // The window's resize listeners, counted as they're added and removed.
    await tab.addInitScript(() => {
      const listening = new Set<unknown>();
      const add = window.addEventListener.bind(window);
      const remove = window.removeEventListener.bind(window);
      window.addEventListener = ((type: string, listener: unknown, options?: unknown) => {
        if (type === "resize") listening.add(listener);
        return add(type, listener as EventListener, options as AddEventListenerOptions);
      }) as typeof window.addEventListener;
      window.removeEventListener = ((type: string, listener: unknown, options?: unknown) => {
        if (type === "resize") listening.delete(listener);
        return remove(type, listener as EventListener, options as EventListenerOptions);
      }) as typeof window.removeEventListener;
      (window as unknown as { resizeListeners: () => number }).resizeListeners = () => listening.size;
    });
    await tab.goto(`http://localhost:${port}/`);
    const trigger = tab.getByRole("button", { name: "Actions" });
    await hydrated(trigger);
    // Each mounted popover listens to the window once.
    const listeners = () => tab.evaluate(() => (window as unknown as { resizeListeners: () => number }).resizeListeners());
    expect(await listeners()).toBe(3);
    // Opened, its panel where geometry places it of the page's own layout:
    // below the trigger, by the offset, aligned to its start.
    const panel = tab.locator(".dropdown [role=dialog]");
    await clicked(trigger, panel);
    await seen(tab.locator('.dropdown[data-placement="bottom-start"]'));
    const placed = await tab.evaluate(() => {
      const button = document.querySelector(".dropdown button")!.getBoundingClientRect();
      const dialog = document.querySelector(".dropdown [role=dialog]") as HTMLElement;
      return { x: Number(dialog.dataset.x), y: Number(dialog.dataset.y), left: button.left, bottom: button.bottom, width: dialog.getBoundingClientRect().width };
    });
    // Shift keeps it inside 1024 px less 8 of padding, as geometry's
    // `clamp` does, `max` where it's past both: a block panel is the
    // window's width, so it's at -8.
    expect(Math.abs(placed.x - shifted(placed.left, 8, 1024 - placed.width - 8))).toBeLessThan(0.01);
    expect(Math.abs(placed.y - (placed.bottom + 4))).toBeLessThan(0.01);
    // An item chosen closes it.
    await tab.getByRole("menuitem", { name: "Delete" }).click();
    await seen(tab.getByText("Chosen: Delete"));
    expect(await panel.count()).toBe(0);
    // Near the viewport's bottom, it flips above its trigger: placed again
    // there, its offset not added again, as geometry's `Flip` places it.
    const more = tab.getByRole("button", { name: "More" });
    await clicked(more, tab.locator(".more [role=dialog]"));
    await seen(tab.locator('.more[data-placement="top-start"]'));
    const flipped = await tab.evaluate(() => {
      const button = document.querySelector(".more button")!.getBoundingClientRect();
      const dialog = document.querySelector(".more [role=dialog]") as HTMLElement;
      return { y: Number(dialog.dataset.y), top: button.top, height: dialog.getBoundingClientRect().height };
    });
    expect(Math.abs(flipped.y - (flipped.top - flipped.height))).toBeLessThan(0.01);
    // Unmounted, the dropdown stops listening to the window.
    await tab.locator("button.unmount").click();
    await tab.locator(".dropdown").waitFor({ state: "detached" });
    expect(await listeners()).toBe(2);
    expect(errors).toEqual([]);
  } finally {
    await browser?.close();
    server.kill();
  }
}, 900_000);
