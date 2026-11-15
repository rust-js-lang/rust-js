// A chain of crates as a real app's is (the deep-chain harness, step 1):
// an app over a shadcn-like kit and a forms crate, each over headless
// primitives (a diamond), over Floating-UI-like geometry, over the react,
// webapi and js crates. Built by Cargo with rust-js as its wrapper; what
// the chain computes is what native Rust prints, its page renders on the
// server, and its declarations type-check under strict tsc.

import { beforeAll, expect, test } from "bun:test";
import { cpSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { checkCargo } from "../tooling/cargo.js";
import { channel } from "../scripts/toolchain.ts";
import { buildCompiler, buildReact, compiler, fixture, root, run, seeded } from "./support";
import { cargo } from "./crates";

beforeAll(() => {
  buildCompiler();
  buildReact();
}, 600_000);

const react = JSON.stringify(join(root, "react"));
/** Each crate of the chain, and what it depends on. */
const crates: Record<string, string> = {
  geometry: "",
  primitives: `geometry = { path = "../geometry" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  kit: `primitives = { path = "../primitives" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  forms: `primitives = { path = "../primitives" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  app: `geometry = { path = "../geometry" }\nprimitives = { path = "../primitives" }\nkit = { path = "../kit" }\nforms = { path = "../forms" }\nreact = { package = "rust-js-react", path = ${react} }\n`,
  check: 'app = { path = "../app" }\n',
};

/** TypeScript's errors of `file` in `dir`, strict, with React's types. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, `tsconfig-${file}.json`);
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", lib: ["es2022", "dom"], jsx: "react-jsx", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

/** The chain's workspace in `dir`, its crates' sources test/deep-chain's. */
function workspace(dir: string): string {
  writeFileSync(join(dir, "Cargo.toml"), `[workspace]\nmembers = ${JSON.stringify(Object.keys(crates))}\nresolver = "2"\n`);
  for (const [name, dependencies] of Object.entries(crates)) {
    cpSync(join(root, "test", "deep-chain", name), join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${dependencies}\n[lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_js)"] }\n`);
  }
  return join(dir, "Cargo.toml");
}

test("an app four crates deep over the react crate computes what native Rust does and renders on the server", async () => {
  const dir = fixture("deep-chain");
  seeded(join(dir, "target"));
  const manifest = workspace(dir);
  const { js, crates: built } = await checkCargo({ manifestPath: manifest, toolchain: channel, compiler, offline: true, packageNames: ["app"] });
  // Each crate of the chain is rust-js's, each its own JS.
  expect(built.map((c) => c.crate).filter((name) => name in crates).sort()).toEqual(["app", "forms", "geometry", "kit", "primitives"]);
  // What no crate changes in place is shared on its way through calls,
  // geometry's `Rect` and `Placement` and a menu's `Vec` of names; its
  // `Point`, which geometry's middleware moves, still copied (#1871).
  const primitives = readFileSync(built.find((c) => c.crate === "primitives")!.js!, "utf8");
  expect(primitives).toContain("computePosition(rect, value, boundary, placement, value$1)");
  expect(primitives).not.toContain("items.slice()");
  expect(primitives).toContain("point: { ...state.point }");
  const app = await import(js!);
  const native = run(cargo(manifest, "run", "-p", "check"));
  expect(app.report()).toBe(native);
  // Placed below, it overflows the boundary, so it flips above.
  expect(native).toContain("bottom-start: top-start at (100, 580)");
  // Its page, on the server, as Next.js renders a server component.
  const page = renderToStaticMarkup(createElement(app.Page));
  expect(page).toContain('<span class="dropdown" data-placement="bottom-start"><button aria-expanded="true">Actions</button>');
  expect(page).toContain('<ul role="menu" aria-label="Actions" data-open="true"><li role="menuitem">Edit</li><li role="menuitem">Delete</li></ul>');
  expect(page).toContain('<li role="menuitem">Vietnam (VN)</li>');
  expect(page).toContain("<p>Chosen: none</p>");
  expect(page).toContain('<button class="inline-flex items-center rounded-md border bg-background h-8 px-3 save" type="submit">Save</button>');
  // Its declarations, through each crate's, as a TypeScript caller of the
  // kit and the forms crate reads them: types carried four crates deep.
  const of = (name: string) => `./${relative(dir, built.find((c) => c.crate === name)!.js!)}`;
  writeFileSync(join(dir, "caller.tsx"), `import { Page, report } from "${of("app")}";
import { Button, buttonClasses, DropdownMenu, Menu, type DropdownProps, type Placement } from "${of("kit")}";
import { Select, type Country } from "${of("forms")}";

export const text: string = report();
export const page = <Page />;
export const classes: string = buttonClasses("Ghost", "Small", "w-full");
// Through kit's re-export, primitives' props, of geometry's placement.
const placement: Placement = { side: "Top", align: "Start" };
const props: DropdownProps = { open: true, trigger: "More", placement, onOpenChange: (open: boolean) => console.log(open), className: "x" };
export const dropdown = <DropdownMenu {...props}><Menu label="More" items={["Edit"]} /></DropdownMenu>;
export const button = <Button variant="Outline" size="Small" type="submit">Save</Button>;
export const select = <Select label="Country" open={false} options={["Vietnam (VN)"]} onChange={(index: number) => console.log(index)} />;
export const country: Country = { code: "VN", name: "Vietnam" };
// @ts-expect-error No such variant.
buttonClasses("Huge", "Small");
// @ts-expect-error A placement's side is one of four.
export const wrong: Placement = { side: "Middle", align: "Start" };
`);
  expect(typeChecked(dir, "caller.tsx")).toEqual([]);
}, 600_000);

