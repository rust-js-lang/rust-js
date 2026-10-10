// ADR 0043: the react crate against every React release since 18.0.
//
// react/versions.json records, for each release, what it exports, read from
// the release itself by react/generate.ts. rustdoc says what the crate has
// for each release, built with its `--cfg react="…"` flags. They must agree:
// a binding a release lacks would crash in the browser, and one gated later
// than it needs to be is missing for nothing.

import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { build } from "vite";
import react from "@vitejs/plugin-react";
import rustJs from "../vite-plugin/index.js";
import { latest, releases } from "../react/cfg.js";
import { bindings } from "../react/coverage";
import versions from "../react/versions.json" with { type: "json" };
import { buildCompiler, compiler, fixture, root, run, target } from "./support";

type Exports = Record<string, Record<string, { since: string; removed?: string }>>;
const exports = versions.exports as Exports;

// Which entries of the package a binding's module is checked against. The
// server and static entries are one module each, whose APIs are for Web
// streams (the browser build) or Node's (the Node build).
function entries(module: string, name: string): string[] {
  if (module === "react-dom/server" || module === "react-dom/static") {
    const node = /Pipeable|NodeStream/.test(name);
    const web = ["renderToReadableStream", "resume", "prerender", "resumeAndPrerender"].includes(name);
    return [node || !web ? `${module}.node` : null, web || !node ? `${module}.browser` : null].filter(Boolean) as string[];
  }
  return [module];
}

function atMost(a: string, b: string): boolean {
  const [x, y] = [a, b].map((v) => v.split(".").map(Number));
  return x[0] < y[0] || (x[0] === y[0] && x[1] <= y[1]);
}

function exported(entry: string, name: string, release: string): boolean {
  const e = exports[entry]?.[name];
  return !!e && atMost(e.since, release) && !(e.removed && atMost(e.removed, release));
}

// What the crate imports from React's packages, built for each release.
const bound = Object.fromEntries(releases.map((r) => [r, bindings(r).links]));

test("every binding is in each React release the crate has it for", () => {
  const missing: string[] = [];
  for (const release of releases) {
    for (const binding of bound[release]) {
      const [module, name] = binding.split("#");
      for (const entry of entries(module, name)) {
        if (!exported(entry, name, release)) missing.push(`${binding} (${entry}) in React ${release}`);
      }
    }
  }
  expect(missing).toEqual([]);
}, 120_000);

test("no binding is gated later than the first release that has it", () => {
  const late: string[] = [];
  for (const release of releases) {
    for (const binding of bound[latest]) {
      const [module, name] = binding.split("#");
      if (entries(module, name).every((entry) => exported(entry, name, release)) && !bound[release].has(binding)) {
        late.push(`${binding}: React ${release} has it`);
      }
    }
  }
  expect(late).toEqual([]);
});

// What the crate leaves out of the latest React, and why. Anything else it
// exports must be bound.
const LEFT_OUT: Record<string, string[]> = {
  // Legacy APIs (react.dev/reference/react/legacy): class components, which
  // rust-js can't write, and what JSX and hooks replaced. `Children`,
  // `cloneElement`, `isValidElement` and `createRef` are bound, as MDX
  // components use them.
  react: [
    "Component", "PureComponent", "createElement",
    // Internal, and unstable.
    "__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE", "__COMPILER_RUNTIME", "unstable_useCacheRefresh",
  ],
  // `useFormState` is `useActionState`'s old name.
  "react-dom": ["__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE", "unstable_batchedUpdates", "useFormState"],
  // Each entry's `version` is react-dom's, which the crate has.
  "react-dom/client": ["version"],
  "react-dom/server.browser": ["version"],
  "react-dom/server.node": ["version"],
  "react-dom/static.browser": ["version"],
  "react-dom/static.node": ["version"],
};

test("every export of the latest React is bound, or left out on purpose", () => {
  const names = (module: string) => new Set([...bound[latest]].filter((b) => b.startsWith(`${module}#`)).map((b) => b.split("#")[1]));
  const unbound: Record<string, string[]> = {};
  for (const entry of Object.keys(exports)) {
    const module = entry.replace(/\.(browser|node)$/, "");
    const have = names(module);
    unbound[entry] = Object.keys(exports[entry]).filter((n) => exported(entry, n, latest) && !have.has(n)).sort();
  }
  expect(unbound).toEqual(Object.fromEntries(Object.entries(LEFT_OUT).map(([e, n]) => [e, [...n].sort()])));
});

test("elements.rs is what react/generate.ts makes of versions.json", () => {
  const out = join(fixture("react-elements"), "elements.rs");
  run(["bun", "react/generate.ts", "--elements", out]);
  expect(readFileSync(out, "utf8")).toBe(readFileSync(join(root, "react/src/elements.rs"), "utf8"));
}, 60_000);

// Each element's attributes, as @types/react types them (ADR 0208).
test("attributes.rs is what react/attributes.ts makes of @types/react", () => {
  const out = join(fixture("react-attributes"), "attributes.rs");
  run(["bun", "react/attributes.ts", out]);
  expect(readFileSync(out, "utf8")).toBe(readFileSync(join(root, "react/src/attributes.rs"), "utf8"));
}, 60_000);

// The strings an attribute of a few takes, which JSX checks a literal against (ADR 0228).
test("literals.rs is what react/literals.ts makes of @types/react", () => {
  const out = join(fixture("react-literals"), "literals.rs");
  run(["bun", "react/literals.ts", out]);
  expect(readFileSync(out, "utf8")).toBe(readFileSync(join(root, "src/jsx_syntax/literals.rs"), "utf8"));
}, 60_000);

// A program for React 18.2 can't use what React 19.2 added: it's a compile
// error, which names the release it needs, not a crash in the browser.
const usesUseEffectEvent = `#![allow(non_snake_case)]
use react::{JSX, jsx, use_effect_event, use_state};
pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(0);
    let log = use_effect_event(move || set_count.set(*count));
    jsx! {
        <button onClick={move |_| log()}>{count}</button>
    }
}
`;

test("React 18.2's crate rejects React 19.2's API at compile time", () => {
  buildCompiler();
  const dir = fixture("react-18");
  writeFileSync(join(dir, "app.rs"), usesUseEffectEvent);
  const compile = (version: string) => {
    const meta = join(target, "react", version);
    run(["react/build.sh", "-o", join(meta, "libreact.rmeta"), "--react", version]);
    return Bun.spawnSync([compiler, join(dir, "app.rs"), "-o", join(dir, `app-${version}.js`), "--", "--extern", `react=${join(meta, "libreact.rmeta")}`, "-L", meta], { cwd: root, stderr: "pipe" });
  };
  const old = compile("18.2.0");
  expect(old.exitCode).not.toBe(0);
  const message = old.stderr.toString();
  expect(message).toContain("unresolved import `react::use_effect_event`");
  expect(message).toContain('#[cfg(react = "19.2")]');
  expect(compile("19.3.0").exitCode).toBe(0);
}, 120_000);

test("The Vite plugin builds for the React a project has installed", async () => {
  buildCompiler();
  const dir = fixture("vite-react-18");
  mkdirSync(join(dir, "src"));
  mkdirSync(join(dir, "node_modules", "react"), { recursive: true });
  writeFileSync(join(dir, "node_modules", "react", "package.json"), '{ "name": "react", "version": "18.2.0" }\n');
  writeFileSync(join(dir, "index.html"), '<div id="root"></div><script type="module" src="/src/App.jsx"></script>');
  writeFileSync(join(dir, "src/App.rs"), usesUseEffectEvent);
  const attempt = build({ root: dir, configFile: false, plugins: [rustJs({ rustJs: compiler }), react()], logLevel: "silent" });
  await expect(attempt).rejects.toThrow("this project has React 18.2.0");
}, 120_000);
