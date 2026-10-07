// Generate react/src/attributes.rs from @types/react (ADR 0208): each of
// React's element attribute interfaces, `AnchorHTMLAttributes` and the
// rest, as a Rust struct a component's props flatten (ADR 0204, 0205),
// read by TypeScript's own parser through @rust-js/typescript (ADR 0206).
//
//   bun react/attributes.ts [out]     writes react/src/attributes.rs, or `out`
//
// Run it when @types/react moves: `bun run generate:attributes`.

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { open } from "@rust-js/typescript";
import { EVENT_TYPES, snake } from "./names";

type Model = any;

// The workspace's, pinned in its package.json.
const typesFile = join(import.meta.dir, "..", "node_modules", "@types", "react", "index.d.ts");
const typesVersion = JSON.parse(readFileSync(join(typesFile, "..", "package.json"), "utf8")).version;

const ts = await open([typesFile]);
const { declarations }: { declarations: Model[] } = await ts.read(typesFile);
await ts.close();

// What @types/react declares: its namespace `React`'s, the file's own
// (`Booleanish`), and `React.JSX`'s `IntrinsicElements`.
const react = declarations.find((d: Model) => d.kind === "namespace" && d.name === "React").declarations;
const jsx = react.find((d: Model) => d.kind === "namespace" && d.name === "JSX").declarations;
const intrinsic = jsx.find((d: Model) => d.kind === "interface" && d.name === "IntrinsicElements");
const interfaces = new Map<string, Model>(react.filter((d: Model) => d.kind === "interface").map((d: Model) => [d.name, d]));
const aliases = new Map<string, Model>(
  [...declarations, ...react].filter((d: Model) => d.kind === "type").map((d: Model) => [d.name, d.type]),
);

// The element each interface types, `HTMLAnchorElement` of `<a>`'s: one
// interface of several elements' is their base's.
const elementOf = new Map<string, Set<string>>();
for (const member of intrinsic.members) {
  const [attributes, element] = member.type.args ?? [];
  const props = attributes?.kind === "reference" && attributes.args?.[0]?.kind === "reference" ? attributes : null;
  if (!props || element?.kind !== "reference") continue;
  const name = props.name.replace(/^React\./, "");
  if (!elementOf.has(name)) elementOf.set(name, new Set());
  elementOf.get(name)!.add(element.name);
}
const element = (name: string, base: string) => {
  const elements = [...(elementOf.get(name) ?? [])];
  return elements.length === 1 ? elements[0] : base;
};

// Each Rust struct: React's interface, and what it flattens. HTML's and
// SVG's own are flat, `AriaAttributes` and `DOMAttributes` copied in, as
// a struct flattens one other only.
type Struct = { rust: string; ts: string; element: string; own: Model[]; flatten?: { field: string; struct: string } };
const structs: Struct[] = [];
const flat = (name: string) => [interfaces.get("AriaAttributes"), interfaces.get("DOMAttributes"), interfaces.get(name)].flatMap((i) => i.members);
structs.push({ rust: "HTMLAttributes", ts: "HTMLAttributes", element: "HTMLElement", own: flat("HTMLAttributes") });
structs.push({ rust: "SVGAttributes", ts: "SVGAttributes", element: "SVGElement", own: flat("SVGAttributes") });
// As @types/react names it: `AnchorHTMLAttributes`.
const rustName = (ts: string) => ts;
for (const [name, declaration] of interfaces) {
  if (!/^[A-Z][a-z]+HTMLAttributes$/.test(name) || name === "AllHTMLAttributes") continue;
  const [base] = declaration.extends;
  if (declaration.extends.length !== 1 || !/HTMLAttributes$/.test(base.name)) {
    throw new Error(`${name} extends ${declaration.extends.map((e: Model) => e.name).join(", ")}`);
  }
  structs.push({
    rust: rustName(name),
    ts: name,
    element: element(name, "HTMLElement"),
    own: declaration.members,
    flatten: { field: base.name === "HTMLAttributes" ? "html" : snake(base.name.replace("HTMLAttributes", "")), struct: rustName(base.name) },
  });
}

// What React's props are in Rust: what's hand-written in lib.rs, or has a
// type of its own there, isn't one of these.
const HAND_WRITTEN = new Set(["children", "dangerouslySetInnerHTML", "ref", "key", "action", "formAction"]);
const skipped = new Map<string, number>();

// A TypeScript type as a field's: `None` its `undefined`.
function rustType(name: string, type: Model): string | null {
  const types = (type.kind === "union" ? type.types : [type]).filter(
    (t: Model) => !(t.kind === "keyword" && t.keyword === "undefined"),
  );
  if (name.startsWith("on") && types.length === 1 && types[0].kind === "reference" && /EventHandler$/.test(types[0].name)) {
    const event = EVENT_TYPES[name.replace(/Capture$/, "")] ?? "SyntheticEvent";
    return `event::${event === "SyntheticEvent" ? "ReactEvent" : event}Handler`;
  }
  if (types.length === 1 && types[0].kind === "reference" && types[0].name === "CSSProperties") return "CSSProperties";
  const kinds = new Set(types.flatMap((t: Model) => kinds_of(t)));
  // `width`'s `number | string`, and `draggable`'s `Booleanish`, a
  // `boolean | "true" | "false"`: either, each its value in JS (ADR 0228).
  if (kinds.has("string") && kinds.has("number") && !kinds.has("boolean")) return "NumberOrString<'a>";
  if (kinds.has("string") && kinds.has("boolean") && !kinds.has("number")) return "Booleanish<'a>";
  // A string, where one's a type of it: `string | Blob`'s, which Rust gives.
  if (kinds.has("string")) return "&'a str";
  if (kinds.has("other")) return null;
  if (kinds.size === 1 && kinds.has("boolean")) return "bool";
  if (kinds.size === 1 && kinds.has("number")) return "f64";
  return null;
}

// What JS values a type takes: strings, numbers, booleans, or another.
function kinds_of(t: Model): string[] {
  switch (t.kind) {
    case "keyword":
      if (t.keyword === "string" || t.keyword === "any") return ["string"];
      if (t.keyword === "number") return ["number"];
      if (t.keyword === "boolean") return ["boolean"];
      // A field's `None`.
      if (t.keyword === "undefined") return [];
      return ["other"];
    case "literal":
      return [typeof t.value === "boolean" ? "boolean" : typeof t.value];
    case "union":
      return t.types.flatMap(kinds_of);
    // `"on" | "off" | (string & {})`: any string, as written.
    case "intersection":
      return t.types.some((p: Model) => p.kind === "keyword" && p.keyword === "string") ? ["string"] : ["other"];
    case "array":
      return t.element.kind === "keyword" && t.element.keyword === "string" ? ["string"] : ["other"];
    case "reference": {
      const alias = aliases.get(t.name.replace(/^React\./, ""));
      return alias ? kinds_of(alias) : ["other"];
    }
    default:
      return ["other"];
  }
}

const lines: string[] = [
  `// Generated by react/attributes.ts from @types/react ${typesVersion}'s element attributes. Do not edit.`,
  "//",
  "// Each is React's interface as a struct a component's props flatten",
  "// (ADR 0208): `#[rust_js::flatten] anchor: AnchorHTMLAttributes<'a>`.",
  "",
  "use super::*;",
  "",
];
for (const struct of structs) {
  lines.push(
    `/// \`${struct.ts}<${struct.element}>\`'s props, as @types/react types them.`,
    `#[cfg_attr(rust_js, rust_js::types = "react#${struct.ts}<${struct.element}>")]`,
    "#[derive(Default)]",
    `pub struct ${struct.rust}<'a> {`,
  );
  const fields = new Set<string>();
  for (const member of struct.own) {
    if (member.kind !== "property" || HAND_WRITTEN.has(member.name)) continue;
    const type = rustType(member.name, member.type);
    if (!type) {
      skipped.set(`${struct.ts}.${member.name}`, 1);
      continue;
    }
    const field = snake(member.name);
    if (fields.has(field)) throw new Error(`${struct.rust} has two \`${field}\``);
    fields.add(field);
    const bare = field.replace(/^r#/, "");
    lines.push(`    /// \`${member.name}\``);
    if (bare !== member.name) lines.push(`    #[cfg_attr(rust_js, rust_js::name = "${member.name}")]`);
    lines.push(`    pub ${field}: Option<${type}>,`);
  }
  if (struct.flatten) {
    lines.push(
      `    /// What \`${struct.ts}\` extends.`,
      "    #[cfg_attr(rust_js, rust_js::flatten)]",
      `    pub ${struct.flatten.field}: ${struct.flatten.struct}<'a>,`,
    );
  }
  lines.push("}", "");
}
writeFileSync(process.argv[2] ?? join(import.meta.dir, "src", "attributes.rs"), `${lines.join("\n").trimEnd()}\n`);
console.log(`react/src/attributes.rs: ${structs.length} structs from @types/react ${typesVersion}; left out, with no Rust type: ${[...skipped.keys()].join(", ") || "none"}`);
