// How much of React's typed API the react crate binds: each export of each
// entry point @types/react and @types/react-dom declare, `+` where the crate
// binds it for the latest React, a value by its `link_name`, a type by its
// `types` link or an item of its name; `-` where it doesn't. React's runtime exports are each
// bound or left out on purpose (test/react-versions.test.ts); this is the
// rest, the types its APIs take and give.
// See docs/decisions/0347-react-coverage.md.

import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { open } from "@rust-js/typescript";

import { cfgFlags, latest } from "./cfg.js";

const root = join(import.meta.dir, "..");

// Each entry point, as a program imports it, and its declarations: React's
// are its namespace `React`'s, `React.JSX`'s as `JSX.Element`.
const ENTRIES: Record<string, string> = {
  react: "@types/react/index.d.ts",
  "react-dom": "@types/react-dom/index.d.ts",
  "react-dom/client": "@types/react-dom/client.d.ts",
  "react-dom/server": "@types/react-dom/server.d.ts",
  "react-dom/static": "@types/react-dom/static.d.ts",
};

type Kind = "value" | "type";
export type Module = { name: string; exports: { name: string; kind: Kind; bound: boolean; unbound?: boolean }[] };

// What isn't bound by design, each list with why: `x`, counted apart.
const UNBOUND: Record<string, string> = Object.fromEntries(
  Object.entries({
    // A class component and what types one: Rust's components are
    // functions, the user's choice (2026-10-10). An error boundary is
    // next/error's `catchError`, or a JS component's binding.
    "class components": [
      "Component", "PureComponent", "ComponentClass", "ClassicComponent", "ClassicComponentClass", "ClassicElement", "CElement",
      "ComponentElement", "ComponentLifecycle", "NewLifecycle", "DeprecatedLifecycle", "StaticLifecycle", "ComponentState",
      "GetDerivedStateFromError", "GetDerivedStateFromProps", "ReactInstance", "ClassType", "ClassAttributes", "LegacyRef",
      "JSX.ElementClass", "JSX.ElementAttributesProperty", "JSX.IntrinsicClassAttributes",
    ].map((name) => `react#${name}`),
    // Types of types, which compute props, refs or elements of a component's
    // type, or name what a function is: a Rust function and its props struct
    // are each, and `jsx!` makes elements, `createElement`'s job too.
    "TypeScript's own": [
      "ComponentProps", "ComponentPropsWithoutRef", "ComponentPropsWithRef", "ComponentRef", "ElementRef", "ContextType",
      "CustomComponentPropsWithRef", "PropsWithChildren", "PropsWithoutRef", "PropsWithRef", "JSXElementConstructor",
      "JSX.ElementChildrenAttribute", "JSX.IntrinsicAttributes", "JSX.IntrinsicElements", "JSX.LibraryManagedAttributes",
      "AnyActionArg", "DispatchWithoutAction", "ReducerWithoutAction", "ReducerState", "FC", "FunctionComponent", "ExoticComponent",
      "ProviderExoticComponent", "FunctionComponentElement", "ReactComponentElement", "DOMElement", "DetailedReactHTMLElement",
      "ReactHTMLElement", "ReactSVGElement", "DetailedHTMLProps", "HTMLProps", "SVGProps", "AllHTMLAttributes", "AriaAttributes",
      "DOMAttributes", "Attributes", "RefAttributes", "HTMLElementType", "SVGElementType", "ReactPromise", "FulfilledReactPromise",
      "PendingReactPromise", "RejectedReactPromise", "UntrackedReactPromise", "RendererUsable", "createElement",
      // The base `SyntheticEvent` extends, of any targets.
      "BaseSyntheticEvent",
      // A string, its values suggestions, `(string & {})`; and their parts.
      "AriaRole", "HTMLInputTypeAttribute", "HTMLAttributeAnchorTarget", "HTMLInputAutoCompleteAttribute", "AutoFill",
      "AutoFillAddressKind", "AutoFillBase", "AutoFillContactField", "AutoFillContactKind", "AutoFillCredentialField",
      "AutoFillField", "AutoFillNormalField", "AutoFillSection", "OptionalPostfixToken", "OptionalPrefixToken",
    ].map((name) => `react#${name}`),
    // What React keeps for old code, as test/react-versions.test.ts leaves
    // it out: `useFormState` is `useActionState`'s old name, and React
    // batches every update itself; `MutableRefObject` is `RefObject`'s, as
    // @types/react deprecates it. Each entry's `version` is react-dom's,
    // `dom::VERSION`.
    "old names": [
      "react#MutableRefObject", "react-dom#useFormState", "react-dom#unstable_batchedUpdates", "react-dom/server#version",
      "react-dom/static#version",
    ],
    // Electron's `<webview>`, which no browser has, nor lib.dom nor webapi.
    "Electron's": ["react#WebViewHTMLAttributes"],
  }).flatMap(([why, names]) => names.map((name) => [name, why])),
);

const kinds: Record<string, Kind> = { interface: "type", type: "type", function: "value", const: "value" };

async function declared(file: string): Promise<Map<string, Kind>> {
  const ts = await open([file]);
  const { declarations } = await ts.read(file);
  await ts.close();
  const react: any = declarations.find((d: any) => d.kind === "namespace" && d.name === "React");
  const out = new Map<string, Kind>();
  // A namespace's declarations are each exported; a module's, those it says.
  const walk = (list: any[], prefix: string, all: boolean) => {
    for (const d of list) {
      if (d.kind === "namespace" && all) walk(d.declarations, `${prefix}${d.name}.`, all);
      else if (kinds[d.kind] && (all || d.exported)) out.set(prefix + d.name, kinds[d.kind]);
      else if (d.kind === "other") {
        // The model leaves a class as text, after its doc comment.
        const text: string = d.text.replace(/^(\s*\/\*[\s\S]*?\*\/|\s*\/\/[^\n]*)*\s*/, "");
        const m = text.match(all ? /^(?:export )?(?:declare )?(?:abstract )?class (\w+)/ : /^export (?:declare )?(?:abstract )?class (\w+)/);
        if (m) out.set(prefix + m[1], "value");
      }
    }
  };
  walk(react ? react.declarations : declarations, "", Boolean(react));
  return out;
}

// What the crate binds for `release`, from rustdoc's JSON of it, built with
// its `--cfg react="…"` flags: each value's `link_name` or `test`, an import
// as a call, a constructor or a component, `react#useState`, `new x#Y`,
// `<react#Suspense>`, as `module#name`, and a type's `test`,
// `isValidElement` of `ReactElement` (ADR 0214); each type's `types` link,
// `react#MouseEvent` of `"react#MouseEvent<T>"`; and each type's name, and
// a discriminated union's variant's as TypeScript names a union's members,
// `FormStatusPending` of `FormStatus::Pending` (ADR 0284).
export function bindings(release: string = latest): { links: Set<string>; types: Set<string>; items: Set<string> } {
  const out = join(root, "target", "react-docs", release);
  mkdirSync(out, { recursive: true });
  const run = (cmd: string[]) => {
    // rustdoc's JSON is a nightly's, which a stable rustdoc gives so.
    const p = Bun.spawnSync(cmd, { cwd: root, stderr: "pipe", env: { ...process.env, RUSTC_BOOTSTRAP: "1" } });
    if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")} failed:\n${p.stderr}`);
  };
  run(["webapi/build.sh", "-o", join(out, "libwebapi.rmeta")]);
  run([
    "rustdoc", "-Zunstable-options", "--document-hidden-items", "--output-format=json", "--edition=2024", "--crate-name=react",
    // rustdoc's JSON is a nightly's, as the tests have it: so is registering the
    // tool that rust-js knows itself, for the crate's attributes (ADR 0112).
    "-Zcrate-attr=feature(register_tool)", "-Zcrate-attr=register_tool(rust_js)",
    // Its attributes are `cfg_attr(rust_js, ..)`, as rust-js reads them (ADR 0113).
    "--cfg=rust_js", "--check-cfg=cfg(rust_js)",
    // As rust-js checks it, for its target (ADR 0090).
    "--target=wasm32-unknown-unknown",
    "react/src/lib.rs", "--extern", `webapi=${join(out, "libwebapi.rmeta")}`, "--extern", `js=${join(out, "libjs.rmeta")}`, "-L", out, ...cfgFlags(release).flags, "-o", out,
  ]);
  const docs = JSON.parse(readFileSync(join(out, "react.json"), "utf8"));
  const links = new Set<string>();
  const types = new Set<string>();
  const items = new Set<string>();
  type Item = { name?: string; inner?: { enum?: { variants: string[] } }; attrs?: { other?: string }[] };
  for (const item of Object.values(docs.index) as Item[]) {
    if (item.name && ["struct", "enum", "type_alias", "trait"].some((k) => k in (item.inner ?? {}))) items.add(item.name);
    if (item.inner?.enum && item.attrs?.some((a) => a.other?.includes("rust_js::tag ="))) {
      for (const id of item.inner.enum.variants) items.add(`${item.name}${(docs.index[id] as Item).name}`);
    }
    for (const attr of item.attrs ?? []) {
      const name = attr.other?.match(/rust_js::(?:link_name|test) = "(.*)"\]$/)?.[1] ?? attr.other?.match(/LinkName \{name: "(.*)"\}/)?.[1];
      const path = name?.replace(/^new /, "").replace(/^<(.*)>$/, "$1");
      if (path?.includes("#")) {
        const [module, rest] = [path.slice(0, path.lastIndexOf("#")), path.slice(path.lastIndexOf("#") + 1)];
        links.add(`${module}#${rest.split(".")[0]}`);
      }
      const type = attr.other?.match(/rust_js::types = "(react[^"#]*#[\w.]+)/)?.[1];
      if (type) types.add(type);
    }
  }
  return { links, types, items };
}

export async function measure(): Promise<Module[]> {
  const { links, types, items } = bindings();
  const modules: Module[] = [];
  for (const [name, file] of Object.entries(ENTRIES)) {
    const exports = [...(await declared(join(root, "node_modules", file)))]
      .filter(([e]) => !e.startsWith("_") && !e.includes("DO_NOT_USE"))
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([e, kind]) => ({
        name: e,
        kind,
        bound: kind === "value" ? links.has(`${name}#${e}`) : types.has(`${name}#${e}`) || items.has(e.split(".").pop()!),
        unbound: `${name}#${e}` in UNBOUND,
      }));
    modules.push({ name, exports });
  }
  return modules;
}

/** The baseline's text: each module's count, then each export, `+` bound. */
export function render(modules: Module[]): string {
  // What isn't bound by design is counted apart.
  const counted = (m: Module) => m.exports.filter((e) => !e.unbound);
  const all = modules.flatMap(counted);
  const bound = all.filter((e) => e.bound).length;
  const unbound = modules.flatMap((m) => m.exports.filter((e) => e.unbound)).length;
  return [
    `# The react crate against @types/react and @types/react-dom: bun test test/react-coverage.test.ts`,
    `# exports: ${bound} of ${all.length} (${((100 * bound) / all.length).toFixed(1)}%)`,
    `# not bound by design (x): ${unbound}, class components, TypeScript's own, old names or Electron's`,
    ...modules.flatMap((m) => [
      `# ${m.name} ${counted(m).filter((e) => e.bound).length} of ${counted(m).length}`,
      ...m.exports.map((e) => `${e.unbound ? "x" : e.bound ? "+" : "-"} ${e.kind === "type" ? "type " : ""}${m.name}#${e.name}`),
    ]),
    "",
  ].join("\n");
}
