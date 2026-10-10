// How much of Next.js's public API the next crate binds: each export of
// each public module, from Next.js's own .d.ts, `+` where the crate binds
// it, a value by its `link_name`, a type by an item of its name in the
// crate; `-` where it doesn't.
// See docs/decisions/0346-next-coverage.md.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { open } from "@rust-js/typescript";

const next = join(import.meta.dir, "../examples/next/node_modules/next");
const crate = join(import.meta.dir, "src");

// The modules an app imports, as Next.js documents them, each its file. The
// rest are build tooling (babel, jest), Next.js internals (client,
// constants, types), or typed only once an app is built (root-params).
const MODULES: Record<string, string> = {
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
export type Module = { name: string; exports: { name: string; kind: Kind; bound: boolean }[] };

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
      if ("exported" in d && d.exported) exported.set(d.name, kindOf(d));
    }
    if (d.kind === "export-from") await reexport(d.from, d.names, false);
    if (d.kind === "export-default") exported.set("default", local.get(d.name) ?? "value");
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
      } else if ((m = text.match(/^export (type )?\{([\s\S]*?)\}\s*;?\s*$/))) {
        // A name of the module's own, or one it imports, exported as it is.
        for (const s of m[2].split(",").map((s) => s.trim()).filter(Boolean)) {
          const [name, as] = s.replace(/^type /, "").split(/\s+as\s+/);
          exported.set(as ?? name, m[1] || s.startsWith("type ") ? "type" : (local.get(name) ?? "value"));
        }
      } else if ((m = text.match(/^export default (\w+)/))) {
        exported.set("default", local.get(m[1]) ?? "value");
      }
    }
  }
  // `export default function dynamic` is the default, not a `dynamic`: the
  // model has it as any exported function.
  for (const m of readFileSync(path, "utf8").matchAll(/^export default (?:declare )?(?:async )?(?:function|class) (\w+)/gm)) {
    exported.set("default", exported.get(m[1]) ?? "value");
    exported.delete(m[1]);
  }
  read.set(path, exported);
  return exported;
}

// What the crate binds: each value's `link_name`, a constructor's, `new
// next/server#NextRequest`, and a static's, `next/server#NextResponse.json`,
// its class's; and each item's name: a
// type's, or a value's an item stands for, a class's struct, or the enum of
// a const object's strings, `RedirectType`.
function bindings(): { links: Set<string>; items: Set<string> } {
  const rust = (dir: string): string[] =>
    readdirSync(dir).flatMap((f) => (statSync(join(dir, f)).isDirectory() ? rust(join(dir, f)) : f.endsWith(".rs") ? [join(dir, f)] : []));
  const sources = rust(crate).map((f) => readFileSync(f, "utf8")).join("\n");
  return {
    links: new Set([...sources.matchAll(/link_name = "(?:new )?(next[^"#]*)#(\w+)/g)].map((m) => `${m[1]}#${m[2]}`)),
    items: new Set([...sources.matchAll(/pub (?:struct|enum|type|trait) (\w+)|pub use [\w:]+ as (\w+);/g)].map((m) => m[1] ?? m[2])),
  };
}

export async function measure(): Promise<Module[]> {
  const { links, items } = bindings();
  const modules: Module[] = [];
  for (const [name, path] of Object.entries(MODULES)) {
    const exports = [...(await exportsOf(join(next, path)))]
      .filter(([e]) => !e.startsWith("_"))
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([e, kind]) => ({ name: e, kind, bound: links.has(`${name}#${e}`) || items.has(e) }));
    modules.push({ name, exports });
  }
  return modules;
}

/** The baseline's text: each module's count, then each export, `+` bound. */
export function render(modules: Module[]): string {
  const all = modules.flatMap((m) => m.exports);
  const bound = all.filter((e) => e.bound).length;
  return [
    `# The next crate against Next.js's public modules: bun test test/next-coverage.test.ts`,
    `# exports: ${bound} of ${all.length} (${((100 * bound) / all.length).toFixed(1)}%)`,
    ...modules.flatMap((m) => [
      `# ${m.name} ${m.exports.filter((e) => e.bound).length} of ${m.exports.length}`,
      ...m.exports.map((e) => `${e.bound ? "+" : "-"} ${e.kind === "type" ? "type " : ""}${m.name}#${e.name}`),
    ]),
    "",
  ].join("\n");
}
