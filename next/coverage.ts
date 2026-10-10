// How much of Next.js's public API the next crate binds: each export of
// each public module, from Next.js's own .d.ts, `+` where the crate binds
// it, a value by its `link_name`, a type by an item of its name in the
// crate; `-` where it doesn't. `bun scripts/next-coverage.ts [out]`
// writes docs/next-coverage.txt, or `out`.

import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { open } from "@rust-js/typescript";

const root = join(import.meta.dir, "..");
const next = join(root, "examples/next/node_modules/next");
const crate = join(root, "next/src");

// The modules an app imports, as Next.js documents them, each its file. The
// rest are build tooling (babel, jest), Next.js internals (client,
// constants, types), or typed only once an app is built (root-params).
const modules: Record<string, string> = {
  next: "index.d.ts",
  "next/app": "app.d.ts",
  "next/cache": "cache.d.ts",
  "next/document": "document.d.ts",
  "next/dynamic": "dynamic.d.ts",
  "next/error": "error.d.ts",
  "next/font/google": "font/google/index.d.ts",
  "next/font/local": "font/local/index.d.ts",
  "next/form": "form.d.ts",
  "next/head": "head.d.ts",
  "next/headers": "headers.d.ts",
  "next/image": "image.d.ts",
  "next/legacy/image": "legacy/image.d.ts",
  "next/link": "link.d.ts",
  "next/navigation": "navigation.d.ts",
  "next/offline": "offline.d.ts",
  "next/og": "og.d.ts",
  "next/router": "router.d.ts",
  "next/script": "script.d.ts",
  "next/server": "server.d.ts",
  "next/web-vitals": "web-vitals.d.ts",
};

type Kind = "value" | "type";

function file(from: string, specifier: string): string | undefined {
  const base = specifier.startsWith("next/") ? join(next, specifier.slice("next/".length)) : resolve(dirname(from), specifier);
  for (const candidate of [`${base}.d.ts`, join(base, "index.d.ts"), base.replace(/\.js$/, ".d.ts")]) {
    if (existsSync(candidate) && statSync(candidate).isFile()) return candidate;
  }
  return undefined;
}

const read = new Map<string, Map<string, Kind>>();

async function exportsOf(path: string, seen: Set<string> = new Set()): Promise<Map<string, Kind>> {
  if (read.has(path)) return read.get(path)!;
  if (seen.has(path)) return new Map();
  seen.add(path);
  const exported = new Map<string, Kind>();
  const local = new Map<string, Kind>();
  const imported = new Map<string, [string, string]>();
  // A session reads the files it's opened with: one each.
  const ts = await open([path]);
  const { declarations } = await ts.read(path);
  await ts.close();
  const kindOf = (d: { kind: string }): Kind => (["interface", "type"].includes(d.kind) ? "type" : "value");
  const reexport = async (specifier: string, names: [string, string][], typeOnly: boolean) => {
    const target = file(path, specifier);
    const there = target ? await exportsOf(target, seen) : new Map<string, Kind>();
    for (const [name, as] of names) exported.set(as, typeOnly ? "type" : (there.get(name) ?? "value"));
  };
  for (const d of declarations) {
    if ("name" in d && typeof d.name === "string" && ["interface", "type", "const", "function", "class", "enum"].includes(d.kind)) {
      local.set(d.name, kindOf(d));
      if (d.exported) exported.set(d.name, kindOf(d));
    }
    if (d.kind === "export-from") await reexport(d.from, d.names, Boolean(d.typeOnly));
    if (d.kind === "export-default") exported.set("default", local.get(d.name) ?? (imported.has(d.name) ? "value" : "value"));
    if (d.kind === "other") {
      // The model leaves a class as text, after its doc comment.
      const text: string = d.text.replace(/^(\s*\/\*[\s\S]*?\*\/)*\s*/, "");
      let m: RegExpMatchArray | null;
      if ((m = text.match(/^export (?:declare )?(?:abstract )?(class|namespace|enum|const|function) (\w+)/))) {
        local.set(m[2], "value");
        exported.set(m[2], "value");
      } else if ((m = text.match(/^(?:declare )?(?:abstract )?(?:class|namespace|enum|const|function) (\w+)/))) {
        local.set(m[1], "value");
      } else if ((m = text.match(/^export \* from ['"](.+)['"]/))) {
        const target = file(path, m[1]);
        if (target) for (const [name, kind] of await exportsOf(target, seen)) if (name !== "default") exported.set(name, kind);
      } else if ((m = text.match(/^export (type )?\{([\s\S]*?)\} from ['"](.+)['"]/))) {
        const names = m[2].split(",").map((s) => s.trim()).filter(Boolean).map((s) => {
          const [name, as] = s.replace(/^type /, "").split(/\s+as\s+/);
          return [name, as ?? name] as [string, string];
        });
        await reexport(m[3], names, Boolean(m[1]));
      } else if ((m = text.match(/^import (\w+) from ['"](.+)['"]/))) {
        imported.set(m[1], [m[2], "default"]);
      } else if ((m = text.match(/^export default (\w+)/))) {
        exported.set("default", local.get(m[1]) ?? "value");
      }
    }
  }
  read.set(path, exported);
  return exported;
}

// What the crate binds: each value's `link_name`, and each item's name, by
// its Rust module's file.
const rust = (dir: string): string[] =>
  readdirSync(dir).flatMap((f) => (statSync(join(dir, f)).isDirectory() ? rust(join(dir, f)) : f.endsWith(".rs") ? [join(dir, f)] : []));
const sources = rust(crate).map((f) => readFileSync(f, "utf8")).join("\n");
const links = new Set([...sources.matchAll(/link_name = "(next[^"#]*)#([^"]+)"/g)].map((m) => `${m[1]}#${m[2]}`));
const items = new Set([...sources.matchAll(/pub (?:struct|enum|type|trait) (\w+)/g)].map((m) => m[1]));

let out = "";
for (const [name, path] of Object.entries(modules)) {
  const exported = await exportsOf(join(next, path));
  const lines = [...exported]
    .filter(([e]) => !e.startsWith("_"))
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([e, kind]) => {
      const bound = kind === "value" ? links.has(`${name}#${e}`) : items.has(e);
      return `${bound ? "+" : "-"} ${kind === "type" ? "type " : ""}${name}#${e}`;
    });
  out += `# ${name} ${lines.filter((l) => l.startsWith("+")).length} of ${lines.length}\n${lines.join("\n")}\n`;
}
writeFileSync(process.argv[2] ?? join(root, "docs/next-coverage.txt"), out);
