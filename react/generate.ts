// Generate, from each React release, @types/react and W3C's specs (ADR 0043):
//
//   react/versions.json  each React minor release, and which one first has
//                        each export, event and attribute
//   react/src/elements.rs the DOM's elements, React's and @types/react's
//                        attributes, React's events, and CSS properties,
//                        each event and attribute gated by the release that
//                        first has it
//
// Run it when React has a new release: `bun run generate:react`. It installs
// the latest patch of every minor release since 18.0 into target/react-versions/
// (which needs the network the first time), and reads what each one exports,
// and the events and attributes React DOM registers.
//
// `bun react/generate.ts --elements <file>` writes only elements.rs, to
// `<file>`, from the versions.json there is: what the tests check it against.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { open } from "@rust-js/typescript";
import { EVENT_TYPES, snake } from "./names";

const root = join(import.meta.dir, "..");
const cache = join(root, "target", "react-versions");
const FIRST = [18, 0];

// What each entry point exports, as a program imports it. The server and
// static entries differ by environment: Web streams in one, Node's in the other.
const ENTRIES = [
  "react",
  "react-dom",
  "react-dom/client",
  "react-dom/server.browser",
  "react-dom/server.node",
  "react-dom/static.browser",
  "react-dom/static.node",
];

type Since = { since: string; removed?: string };

// `bun react/generate.ts --read <dir> <NODE_ENV>`: one version's exports, in a
// process of its own, since React picks its build when it's first loaded.
if (process.argv[2] === "--read") {
  const [dir, env] = process.argv.slice(3);
  process.env.NODE_ENV = env;
  const require = createRequire(join(dir, "package.json"));
  const exports: Record<string, string[]> = {};
  for (const entry of ENTRIES) {
    try {
      exports[entry] = Object.keys(require(entry)).filter((name) => name !== "default");
    } catch {
      exports[entry] = [];
    }
  }
  console.log(JSON.stringify(exports));
  process.exit(0);
}

// `bun react/generate.ts --registry <dir>`: the events and attributes React
// DOM knows. They're internal, so this runs its development build with one
// more line that hands them out.
if (process.argv[2] === "--registry") {
  const dir = process.argv[3];
  const { GlobalRegistrator } = await import("@happy-dom/global-registrator");
  GlobalRegistrator.register();
  const require = createRequire(join(dir, "package.json"));
  const cjs = join(dir, "node_modules", "react-dom", "cjs");
  const file = [join(cjs, "react-dom-client.development.js"), join(cjs, "react-dom.development.js")].find(existsSync)!;
  let source = readFileSync(file, "utf8");
  const end = source.lastIndexOf("})();");
  source =
    source.slice(0, end) +
    "exports.__registry = { events: Object.keys(registrationNameDependencies), attributes: Object.values(possibleStandardNames) };\n" +
    source.slice(end);
  const module = { exports: {} as any };
  new Function("exports", "require", "module", "process", source)(module.exports, require, module, {
    env: { NODE_ENV: "development" },
  });
  const { events, attributes } = module.exports.__registry;
  console.log(JSON.stringify({ events: [...new Set(events)].sort(), attributes: [...new Set(attributes)].sort() }));
  process.exit(0);
}

function run(cmd: string[], cwd = root): string {
  const p = spawnSync(cmd[0], cmd.slice(1), { cwd, encoding: "utf8", maxBuffer: 64 << 20 });
  if (p.status !== 0) throw new Error(`${cmd.join(" ")} failed:\n${p.stderr}`);
  return p.stdout;
}

// ── Each minor release since 18.0 ───────────────────────────────────────

const elementsOnly = process.argv[2] === "--elements" ? process.argv[3] : null;
const releases: Record<string, string> = elementsOnly ? (await import("./versions.json")).default.releases : {};
if (!elementsOnly) {
  const registry = await (await fetch("https://registry.npmjs.org/react")).json();
  for (const version of Object.keys(registry.versions).filter((v) => /^\d+\.\d+\.\d+$/.test(v))) {
    const [major, minor] = version.split(".").map(Number);
    if (major < FIRST[0] || (major === FIRST[0] && minor < FIRST[1])) continue;
    const key = `${major}.${minor}`;
    if (!releases[key] || compare(version, releases[key]) > 0) releases[key] = version;
  }
}
const minors = Object.keys(releases).sort(compare);

function compare(a: string, b: string): number {
  const [x, y] = [a, b].map((v) => v.split(".").map(Number));
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) - (y[i] ?? 0);
  }
  return 0;
}

type Read = { exports: Record<string, string[]>; events: string[]; attributes: string[] };
const read: Record<string, Read> = {};
for (const minor of elementsOnly ? [] : minors) {
  const version = releases[minor];
  const dir = join(cache, version);
  if (!existsSync(join(dir, "node_modules", "react-dom"))) {
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "package.json"), '{ "private": true }\n');
    run(["bun", "add", `react@${version}`, `react-dom@${version}`], dir);
  }
  // Some exports, like `act`, are only in the development build.
  const dev = JSON.parse(run(["bun", import.meta.path, "--read", dir, "development"]));
  const prod = JSON.parse(run(["bun", import.meta.path, "--read", dir, "production"]));
  const exports = Object.fromEntries(ENTRIES.map((e) => [e, [...new Set([...dev[e], ...prod[e]])].sort()]));
  const { events, attributes } = JSON.parse(run(["bun", import.meta.path, "--registry", dir]));
  read[minor] = { exports, events, attributes };
  console.log(`React ${version}: ${exports.react.length} exports, ${events.length} events, ${attributes.length} attributes`);
}

// The first release with each name, and the first one without it after that.
function since(present: (minor: string) => string[]): Record<string, Since> {
  const names = new Set(minors.flatMap(present));
  const out: Record<string, Since> = {};
  for (const name of [...names].sort()) {
    const has = minors.map((m) => present(m).includes(name));
    const first = has.indexOf(true);
    const gone = has.indexOf(false, first);
    out[name] = gone < 0 ? { since: minors[first] } : { since: minors[first], removed: minors[gone] };
  }
  return out;
}

const latest = minors.at(-1)!;
const versions = elementsOnly
  ? (await import("./versions.json")).default
  : {
      _: "Generated by react/generate.ts from each React release. Do not edit.",
      releases: Object.fromEntries(minors.map((m) => [m, releases[m]])),
      exports: Object.fromEntries(ENTRIES.map((e) => [e, since((m) => read[m].exports[e])])),
      events: since((m) => read[m].events),
      attributes: since((m) => read[m].attributes),
    };
if (!elementsOnly) writeFileSync(join(import.meta.dir, "versions.json"), `${JSON.stringify(versions, null, 1)}\n`);

// ── react/src/elements.rs ────────────────────────────────────────────────────

// Written by hand in lib.rs, where they take their own types.
const HAND_WRITTEN = new Set(["action", "formAction", "style", "dangerouslySetInnerHTML", "children", "ref", "key", "innerHTML", "precedence"]);

function gate(entry: Since): string {
  return entry.since === minors[0] ? "" : `    #[cfg(react = "${entry.since}")]\n`;
}

const lines: string[] = [
  "// Generated by react/generate.ts from React DOM's own tables (React",
  `// ${minors.map((m) => releases[m]).join(", ")}), @types/react, and W3C's specs (@webref/elements, @webref/css). Do not edit.`,
  "//",
  "// An attribute or event React DOM first knows in a later release is gated",
  '// by it, `#[cfg(react = "19.2")]` (ADR 0043).',
  "",
  "use super::*;",
  "",
  "/// Attributes, as React names them: `class_name` is `className`. Any other,",
  '/// like `aria-*` and `data-*`, is [`attr`](Element::attr).',
  "#[doc(hidden)]",
  "impl<T> Element<T> {",
];
// And those @types/react types that React DOM's table, of the names whose
// spelling it warns about, leaves out, as it passes them on as written:
// `translate`, `loading`. Every release has them.
const typesFile = join(root, "node_modules", "@types", "react", "index.d.ts");
const ts = await open([typesFile]);
const { declarations }: { declarations: any[] } = await ts.read(typesFile);
await ts.close();
const typed: string[] = declarations
  .find((d: any) => d.kind === "namespace" && d.name === "React")
  .declarations.filter((d: any) => d.kind === "interface" && /(HTML|SVG)Attributes$/.test(d.name))
  .flatMap((d: any) => d.members)
  .filter((m: any) => m.kind === "property" && /^[a-zA-Z]+$/.test(m.name))
  .map((m: any) => m.name);
// What each attribute takes, as @types/react types it in every interface
// that has it (ADR 0228): `tabIndex` a number, `width` a number or text,
// `draggable` a `Booleanish`, `disabled` a `bool`. One it doesn't type,
// React DOM's table's alone, or of another type, takes any `Value`.
const reactNamespace = declarations.find((d: any) => d.kind === "namespace" && d.name === "React").declarations;
const typeAliases = new Map<string, any>(
  [...declarations, ...reactNamespace].filter((d: any) => d.kind === "type").map((d: any) => [d.name, d.type]),
);
const kindsOf = (t: any): string[] => {
  switch (t.kind) {
    case "keyword":
      if (t.keyword === "undefined") return [];
      return ["string", "number", "boolean"].includes(t.keyword) ? [t.keyword] : ["other"];
    case "literal":
      return [typeof t.value === "string" ? "string" : typeof t.value === "boolean" ? "boolean" : "other"];
    case "union":
      return t.types.flatMap(kindsOf);
    // `"on" | "off" | (string & {})`: any string, as written.
    case "intersection":
      return t.types.some((p: any) => p.kind === "keyword" && p.keyword === "string") ? ["string"] : ["other"];
    case "reference": {
      const alias = typeAliases.get(t.name.replace(/^React\./, ""));
      return alias ? kindsOf(alias) : ["other"];
    }
    default:
      return ["other"];
  }
};
const valueKinds = new Map<string, Set<string>>();
for (const d of reactNamespace) {
  if (d.kind !== "interface" || !/(HTML|SVG)Attributes$/.test(d.name)) continue;
  for (const m of d.members) {
    if (m.kind !== "property" || !/^[a-zA-Z][a-zA-Z0-9]*$/.test(m.name)) continue;
    if (!valueKinds.has(m.name)) valueKinds.set(m.name, new Set());
    for (const kind of kindsOf(m.type)) valueKinds.get(m.name)!.add(kind);
  }
}
const valueType = (name: string): string => {
  const kinds = [...(valueKinds.get(name) ?? ["other"])].sort().join(" ");
  const types: Record<string, string> = {
    boolean: "bool",
    number: "impl value::Number",
    string: "impl value::Text",
    "number string": "impl value::NumberOrString",
    "boolean string": "impl value::Booleanish",
    // `string | Blob`, a template literal`s: text, as Rust gives none of the other.
    "other string": "impl value::Text",
    "number other string": "impl value::NumberOrString",
  };
  return types[kinds] ?? "impl Value";
};

// Each HTML tag's element, as the webapi crate's `Tag` gives it (ADR 0224):
// `<button>` is an `HTMLButtonElement`; and each SVG tag's, as its `SVGTag`
// gives it, `<circle>` an `SVGCircleElement`, but a name HTML has too, `<a>`,
// which is HTML's, as @types/react's `JSX.IntrinsicElements` has it.
const webapiSource = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
const tagTypes = new Map([...webapiSource.matchAll(/rust_js::name = "([^"]+)"\)\]\n    pub struct (\w+);/g)].map(([, name, type]) => [type, name]));
const tagElements = new Map<string, string>();
for (const [, type, element] of webapiSource.matchAll(/impl Tag for tags::(\w+) \{ type Element = (\w+); \}/g)) {
  tagElements.set(tagTypes.get(type)!, element);
}
const htmlElements = new Set(["HTMLElement", ...tagElements.values()]);
const svgElements = new Set<string>(["SVGElement"]);
for (const [, type, element] of webapiSource.matchAll(/impl SVGTag for svg_tags::(\w+) \{ type Element = (\w+); \}/g)) {
  svgElements.add(element);
  if (!tagElements.has(tagTypes.get(type)!)) tagElements.set(tagTypes.get(type)!, element);
}

// What each tag takes, as @types/react's `JSX.IntrinsicElements` says:
// `button` a `ButtonHTMLAttributes`, with what it extends, `circle` an
// `SVGProps`, all of `SVGAttributes`, as each SVG tag. An attribute both
// `HTMLAttributes` and `SVGAttributes` have is every element's; one of
// `HTMLAttributes` only, HTML's elements' (`has::Html`); one of
// `SVGAttributes`, SVG's (`has::Svg`); another, only its tags' elements'.
// Any `Element` takes each, a tag value's say (ADR 0228).
const reactTypes = declarations.find((d: any) => d.kind === "namespace" && d.name === "React").declarations;
const interfaces = new Map<string, any>(reactTypes.filter((d: any) => d.kind === "interface").map((d: any) => [d.name, d]));
const propsOf = (name: string): Set<string> => {
  const i = interfaces.get(name.replace(/^React\./, ""));
  if (!i) return new Set();
  const own = i.members.filter((m: any) => m.kind === "property" && /^[a-zA-Z][a-zA-Z0-9]*$/.test(m.name)).map((m: any) => m.name);
  return new Set([...own, ...i.extends.flatMap((e: any) => [...propsOf(e.name)])]);
};
const everyElements = propsOf("HTMLAttributes");
const svgAttributes = propsOf("SVGAttributes");
const intrinsic = reactTypes
  .find((d: any) => d.kind === "namespace" && d.name === "JSX")
  .declarations.find((d: any) => d.kind === "interface" && d.name === "IntrinsicElements");
const accepts = new Map<string, { elements: Set<string>; tags: Set<string> }>();
for (const member of intrinsic.members) {
  const [props] = member.type.name === "React.DetailedHTMLProps" ? member.type.args : [];
  const element = tagElements.get(member.name);
  if (!props || !element) continue;
  for (const prop of propsOf(props.name)) {
    if (everyElements.has(prop)) continue;
    if (!accepts.has(prop)) accepts.set(prop, { elements: new Set(), tags: new Set() });
    accepts.get(prop)!.elements.add(element);
    accepts.get(prop)!.tags.add(member.name);
  }
}
const pascal = (name: string) => name[0].toUpperCase() + name.slice(1);

const attributes: Record<string, Since> = {
  ...Object.fromEntries(typed.map((name) => [name, { since: minors[0] }])),
  ...versions.attributes,
};

const methods = new Set(["children", "key", "r#ref", "attr"]);
for (const [name, entry] of Object.entries(attributes).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
  if (entry.removed || HAND_WRITTEN.has(name) || name.startsWith("on")) continue;
  const method = snake(name);
  if (methods.has(method)) throw new Error(`two attributes are \`${method}\``);
  methods.add(method);
  const ty = valueType(name);
  const bounded = accepts.has(name) || everyElements.has(name) !== svgAttributes.has(name);
  const bound = bounded ? `\n    where\n        T: has::${pascal(name)},` : "";
  lines.push(
    `    /// \`${name}\``,
    `${gate(entry)}    #[cfg_attr(rust_js, rust_js::link_name = "prop ${name}")]`,
    `    pub fn ${method}(self, value: ${ty}) -> Element<T>${bound}${bound ? "\n    {" : " {"}`,
    "        unreachable!()",
    "    }",
    "",
  );
}
lines.push(
  "}",
  "",
  "/// What takes each attribute that isn't every element's, as @types/react's",
  "/// `JSX.IntrinsicElements` says (ADR 0228): `has::Href` an `<a>`'s",
  "/// `HTMLAnchorElement`, `has::Cx` each SVG element, and any `Element`, a",
  "/// tag value's say.",
  "pub mod has {",
  "    use super::webapi;",
  "",
  "    /// HTML's elements, which take what `HTMLAttributes` has, and any `Element`.",
  "    pub trait Html {}",
  ...[...htmlElements].sort().map((e) => `    impl Html for webapi::${e} {}`),
  "    impl Html for webapi::Element {}",
  "",
  "    /// SVG's elements, which take what `SVGAttributes` has, and any `Element`.",
  "    pub trait Svg {}",
  ...[...svgElements].sort().map((e) => `    impl Svg for webapi::${e} {}`),
  "    impl Svg for webapi::Element {}",
);
const bounded = new Set([...accepts.keys(), ...[...everyElements, ...svgAttributes].filter((n) => everyElements.has(n) !== svgAttributes.has(n))]);
for (const name of [...bounded].sort()) {
  if (!(name in attributes) || HAND_WRITTEN.has(name) || name.startsWith("on")) continue;
  const html = accepts.get(name);
  const svg = svgAttributes.has(name);
  const list = [
    ...(everyElements.has(name) ? ["HTML's elements"] : html ? [...html.tags].sort().map((t) => `<${t}>`) : []),
    ...(svg ? ["SVG's elements"] : []),
  ].join(", ");
  lines.push(
    "",
    `    /// An element that takes \`${name}\`: ${list}.`,
    `    #[diagnostic::on_unimplemented(message = "\`{Self}\` takes no \`${name}\`", label = "not an attribute of this tag", note = "@types/react gives \`${name}\` to ${list}")]`,
    `    pub trait ${pascal(name)} {}`,
    // HTML's tags' elements one by one, and the rest of a family by its
    // marker, which any `Element` has.
    ...(html ? [...html.elements].sort().map((e) => `    impl ${pascal(name)} for webapi::${e} {}`) : []),
    ...(everyElements.has(name) ? [`    impl<T: Html> ${pascal(name)} for T {}`] : []),
    ...(svg ? [`    impl<T: Svg> ${pascal(name)} for T {}`] : []),
    ...(!svg && !everyElements.has(name) ? [`    impl ${pascal(name)} for webapi::Element {}`] : []),
  );
}
lines.push("}", "", "/// Event handlers: `on_click` is `onClick`. A handler must not borrow", "/// anything, since it runs later: write it `move |e| ..`. Its event is of", "/// the element's, `event::Mouse<T>` (ADR 0224).", "#[doc(hidden)]", "impl<T> Element<T> {");
for (const [name, entry] of Object.entries<Since>(versions.events)) {
  if (entry.removed) continue;
  const base = name.replace(/Capture$/, "");
  const type = EVENT_TYPES[base] ?? "SyntheticEvent";
  const method = snake(name);
  if (methods.has(method)) throw new Error(`two methods are \`${method}\``);
  methods.add(method);
  lines.push(
    `    /// \`${name}\``,
    `${gate(entry)}    #[cfg_attr(rust_js, rust_js::link_name = "prop ${name}")]`,
    `    pub fn ${method}(self, handler: impl Fn(&event::${type}<T>) + 'static) -> Element<T> {`,
    "        unreachable!()",
    "    }",
    "",
  );
}
lines.push("}", "");

// Elements: HTML's, and SVG's with its animations, filters and masks.
const webrefElements = await import("@webref/elements");
const specs = await (webrefElements.default ?? webrefElements).listAll();
const tags = new Set<string>();
for (const spec of ["html", "SVG2", "svg-animations", "filter-effects-1", "css-masking-1"]) {
  for (const element of specs[spec]?.elements ?? []) {
    if (!element.obsolete && /^[a-zA-Z][a-zA-Z0-9]*$/.test(element.name)) tags.add(element.name);
  }
}
lines.push("/// The DOM's elements: `div()` is `<div>`, `linear_gradient()` `<linearGradient>`,", "/// each of its DOM element, `button()` an `HTMLButtonElement`'s.", "pub mod html {", "    use super::{Element, webapi};", "", "    unsafe extern \"Rust\" {");
for (const tag of [...tags].sort()) {
  const element = tagElements.get(tag);
  lines.push(`        /// \`<${tag}>\``, `        #[link_name = "<${tag}>"]`, `        pub safe fn ${snake(tag)}() -> Element${element ? `<webapi::${element}>` : ""};`);
}
lines.push("    }", "}", "");

// CSSProperties: CSS's properties, as React names them, `backgroundColor`.
const webrefCss = await import("@webref/css");
const css = await (webrefCss.default ?? webrefCss).listAll();
const properties = [...new Set(css.properties.map((p: { name: string }) => p.name))]
  .filter((name) => !name.startsWith("-"))
  .sort() as string[];
lines.push("/// CSS properties: `background_color` is `backgroundColor`. A number is in", "/// pixels where CSS needs a unit, as React makes it.", "impl CSSProperties {");
const styleMethods = new Set(["new", "set"]);
for (const property of properties) {
  const camel = property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
  const method = snake(camel);
  if (styleMethods.has(method)) continue;
  styleMethods.add(method);
  lines.push(
    `    /// \`${property}\``,
    `    #[cfg_attr(rust_js, rust_js::link_name = "prop ${camel}")]`,
    `    pub fn ${method}(self, value: impl Value) -> CSSProperties {`,
    "        unreachable!()",
    "    }",
    "",
  );
}
lines.push("}");
writeFileSync(elementsOnly ?? join(import.meta.dir, "src", "elements.rs"), `${lines.join("\n")}\n`);
console.log(
  `react/versions.json: ${minors.length} releases, ${latest} latest; react/src/elements.rs: ${methods.size} element methods, ${tags.size} elements, ${styleMethods.size} style properties`,
);
