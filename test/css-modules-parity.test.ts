// The parity fixture of CSS Modules (#1512, ADR 1133): what each bundler
// exports of each construct, Next's Turbopack, Next's webpack (css-loader)
// and Vite, each as it's configured out of the box, its keys as they are
// (`asIs`), against the keys rust-js checks a binding by. rust-js's are
// what all three export, so a getter it accepts reads a class whichever
// builds the app; a key one exports and another doesn't is refused.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

/** Each construct, a file of its own. */
const cases: Record<string, string> = {
  plain: ".plain { color: red }",
  hyphen: ".card-title { color: red }",
  camel: ".camelCase { color: red }",
  global_only: ":global(.globalOnly) { color: red } .kept { color: red }",
  global_mixed: ":global(.g) .mixed { color: red }",
  local_fn: ":local(.localFn) { color: red }",
  nested: ".outer { color: red; .nested { color: blue } }",
  nested_amp: ".outer { color: red; &.amped { color: blue } }",
  media: "@media (min-width: 1px) { .inMedia { color: red } }",
  supports: "@supports (display: grid) { .inSupports { color: red } }",
  layer: "@layer base { .inLayer { color: red } }",
  id: "#anId { color: red }",
  keyframes: "@keyframes spin { from { color: red } } .anim { animation: spin 1s }",
  keyframes_unused: "@keyframes lonely { from { color: red } } .k { color: red }",
  grid_areas: '.grid { grid-template-areas: "head"; } .item { grid-area: head }',
  grid_lines: ".grid { grid-template-columns: [left] 1fr [right]; }",
  container: ".box { container-name: card } @container card (min-width: 1px) { .inContainer { color: red } }",
  dashed: ".dashed { --accent: red; color: var(--accent) }",
  escaped: ".a\\:b { color: red } .\\31 0 { color: red }",
  composes_local: ".base { color: red } .composer { composes: base; color: blue }",
  composes_global: ".composer { composes: gl from global; color: blue }",
  composes_other: '.composer { composes: other from "./other.module.css"; color: blue }',
  compound: ".first.second { color: red }",
  view_transition: ".vt { view-transition-name: vtName }",
  counter_style: "@counter-style thumbs { system: cyclic; symbols: x; suffix: ' '; } .list { list-style: thumbs }",
  counter_style_nested: "@media (min-width: 1px) { .list { list-style-type: stars !important } }",
  counter_style_class: ".list { list-style-type: stars } .stars { color: red }",
  animation_undefined: ".a { animation: fade 1s }",
  attribute: "[data-x] .attr { color: red }",
  pseudo: ".btn:hover .icon::before { color: red }",
  non_ascii: ".日本 { color: red }",
  keyword: ".type { color: red } .self { color: red }",
};
/** What Next's webpack refuses, its CSS Modules pure: a rule of no local
 * class, and `:local(..)` alone. Their keys are the others' where it builds. */
const webpackRefuses = ["global_only", "local_fn"];

type Keys = Record<string, string[]>;

/** A directory of the cases' files, `styles/`, and a module importing
 * those of `names`, whose `keys` is each one's keys. */
function styled(dir: string, names: string[]): string {
  mkdirSync(join(dir, "styles"), { recursive: true });
  for (const [name, css] of Object.entries(cases)) writeFileSync(join(dir, "styles", `${name}.module.css`), `${css}\n`);
  writeFileSync(join(dir, "styles", "other.module.css"), ".other { color: red }\n");
  const imports = names.map((name) => `import ${name} from "./styles/${name}.module.css";`).join("\n");
  return `${imports}\n\nexport const keys = { ${names.map((name) => `${name}: Object.keys(${name}).sort()`).join(", ")} };\n`;
}

/** Each case's keys as a Next.js app built by `bundler` exports them: its
 * page printing them, statically exported. */
function next(bundler: "turbopack" | "webpack"): Keys {
  const dir = fixture(`css-parity-${bundler}`);
  const names = Object.keys(cases).filter((name) => bundler === "turbopack" || !webpackRefuses.includes(name));
  mkdirSync(join(dir, "app"), { recursive: true });
  const keysModule = styled(join(dir, "app"), names);
  writeFileSync(join(dir, "app", "keys.js"), keysModule);
  writeFileSync(join(dir, "app", "layout.js"), "export default function Layout({ children }) { return <html><body>{children}</body></html>; }\n");
  writeFileSync(join(dir, "app", "page.js"), 'import { keys } from "./keys.js";\n\nexport default function Page() { return <pre id="keys">{JSON.stringify(keys)}</pre>; }\n');
  writeFileSync(join(dir, "next.config.mjs"), 'export default { output: "export" };\n');
  writeFileSync(join(dir, "package.json"), '{ "private": true, "type": "module" }\n');
  mkdirSync(join(dir, "node_modules"));
  for (const pkg of ["next", "react", "react-dom"]) symlinkSync(join(root, "examples/next/node_modules", pkg), join(dir, "node_modules", pkg));
  const args = ["build", ...(bundler === "webpack" ? ["--webpack"] : [])];
  const built = Bun.spawnSync([process.execPath, join(dir, "node_modules/next/dist/bin/next"), ...args], { cwd: dir, env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1" }, stderr: "pipe" });
  expect([bundler, built.exitCode, built.stderr.toString()]).toEqual([bundler, 0, ""]);
  const page = readFileSync(join(dir, "out", "index.html"), "utf8");
  const printed = page.match(/<pre id="keys">(.*?)<\/pre>/s)![1];
  const keys: Keys = JSON.parse(printed.replaceAll("&quot;", '"').replaceAll("&amp;", "&").replaceAll("&lt;", "<").replaceAll("&gt;", ">"));
  // Next's webpack adds `__checksum` to a module a server component
  // imports, its own, for reloading it: no class of the file's.
  return Object.fromEntries(Object.entries(keys).map(([name, list]) => [name, list.filter((key) => key !== "__checksum")]));
}

/** Each case's keys as Vite exports them, built for Node, out of the box. */
function vite(): Keys {
  const dir = fixture("css-parity-vite");
  writeFileSync(join(dir, "entry.js"), styled(dir, Object.keys(cases)));
  writeFileSync(join(dir, "package.json"), '{ "private": true, "type": "module" }\n');
  const built = Bun.spawnSync([process.execPath, join(root, "node_modules/vite/bin/vite.js"), "build", "--ssr", "entry.js", "--outDir", "dist", "--logLevel", "error"], { cwd: dir, stderr: "pipe" });
  expect([built.exitCode, built.stderr.toString()]).toEqual([0, ""]);
  const read = Bun.spawnSync([process.execPath, "--input-type=module", "-e", `import { keys } from ${JSON.stringify(join(dir, "dist", "entry.js"))}; console.log(JSON.stringify(keys));`], { stderr: "pipe" });
  return JSON.parse(read.stdout.toString());
}

/** Each case's keys as rust-js checks a binding by: what `rust-js
 * bindings` prints of it, a getter each, and its note of those it names
 * none of. */
function rustJs(): Keys {
  const dir = fixture("css-parity-rust-js");
  styled(dir, []);
  const keys: Keys = {};
  for (const name of Object.keys(cases)) {
    const printed = Bun.spawnSync([compiler, "--css-module-binding", `./styles/${name}.module.css`], { cwd: dir, stderr: "pipe" });
    expect([name, printed.stderr.toString()]).toEqual([name, ""]);
    const text = printed.stdout.toString();
    const getters = [...text.matchAll(/^    #\[cfg_attr\(rust_js, rust_js::link_name = "get ([^"]+)"\)\]$/gm)].map((m) => m[1]);
    const noted = text.match(/No getter is written of (.*?): name each yourself/s)?.[1] ?? "";
    keys[name] = [...getters, ...[...noted.matchAll(/`([^`]+)`/g)].map((m) => m[1])].sort();
  }
  return keys;
}

test("the keys rust-js checks a CSS Module's binding by are those every bundler exports", () => {
  const exported = { turbopack: next("turbopack"), webpack: next("webpack"), vite: vite() };
  const ours = rustJs();
  const sorted = (keys: string[]) => [...keys].sort();
  for (const name of Object.keys(cases)) {
    const each = Object.entries(exported).filter(([, keys]) => keys[name]).map(([, keys]) => keys[name]);
    const everywhere = sorted(each[0].filter((key) => each.every((keys) => keys.includes(key))));
    expect([name, ours[name]]).toEqual([name, everywhere]);
  }
  // Where they disagree, and so what's refused: the names of a view
  // transition and a counter style, Turbopack's alone.
  const disagree = Object.keys(cases).filter((name) => {
    const each = Object.values(exported).filter((keys) => keys[name]).map((keys) => JSON.stringify(sorted(keys[name])));
    return new Set(each).size > 1;
  });
  expect(disagree).toEqual(["view_transition", "counter_style", "counter_style_nested"]);
  expect([exported.turbopack.view_transition, exported.turbopack.counter_style]).toEqual([["vt", "vtName"], ["list", "thumbs"]]);
  expect(exported.turbopack.counter_style_nested).toEqual(["list", "stars"]);
  // A getter of one is refused, naming who exports it; of one none does, a
  // grid's area, as any class the file doesn't export.
  for (const [name, key, said] of [
    ["view_transition", "vtName", "`./styles/view_transition.module.css`'s `vtName` is exported by Turbopack, but not by webpack's css-loader or Vite"],
    ["counter_style", "thumbs", "`./styles/counter_style.module.css`'s `thumbs` is exported by Turbopack, but not by webpack's css-loader or Vite"],
    ["grid_areas", "head", "`./styles/grid_areas.module.css` exports no class `head`; its classes are `grid` and `item`"],
  ]) {
    expect([name, refusal(name, key)]).toEqual([name, expect.stringContaining(said)]);
  }
}, 300_000);

/** What rust-js says of a binding of `key` of the case `name`'s file. */
function refusal(name: string, key: string): string {
  const dir = fixture(`css-parity-refused-${name}`);
  styled(dir, []);
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

use js::JsObject;

#[cfg_attr(rust_js, rust_js::css_module)]
pub struct Styles(PhantomData<JsObject>);

impl Styles {
    #[cfg_attr(rust_js, rust_js::link_name = "get ${key}")]
    pub fn class(&self) -> String {
        unreachable!()
    }
}

unsafe extern "Rust" {
    #[link_name = "./styles/${name}.module.css#default"]
    pub safe static STYLES: &'static Styles;
}
`);
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  return Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: root, stderr: "pipe" }).stderr.toString();
}
