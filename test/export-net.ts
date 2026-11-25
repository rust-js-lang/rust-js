// The export net (#2481): each path a crate offers JS, as its manifest's
// `offered_paths` says (found of rustc's `module_children`, apart from what
// writes the JS and its `.d.ts`), is in its JS, of its kind, and in its
// `.d.ts`; and each function of plain types JS calls, and each constant
// and static of one it reads, gives what native Rust's does, given the same
// values (test/generated-calls.ts). No JS exception but a Rust panic
// escapes. A re-export the writer drops is a path JS lacks (#2544), not one
// both agree is gone. Of each crate of a case: a crate alone, or a library
// and the app using it, each compiled as a library with its declarations.

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { calls, jsShow, printable, rustShow, type Tree } from "./generated-calls";
import { compiler, fixture, nativeCrates, programFlags, run } from "./support";
import { es2022, typeChecked } from "./type-checked";
import { underLoad } from "../scripts/timing";

/** A path a crate offers JS, as its manifest says. */
export type Offered = {
  rust: string[];
  js: string[];
  kind: "function" | "const" | "static" | "namespace" | "type";
  params?: Tree[];
  ty?: Tree;
  generic?: boolean;
  type_params?: number;
  located?: boolean;
  of_no_value?: boolean;
};

type Crate = { name: string; file: string; uses: string[]; edition?: string };

/** The crates of `files` in `cases/name`, as test/crates.ts' `chain` reads
 * them: a file's crate its name or its `//@ crate-name:`, its edition its
 * `//@ edition:`, and each crate before it its dependency, or those its
 * `//@ uses:` names. */
function crates(cases: string, name: string, files: string[]): Crate[] {
  const read = (file: string) => readFileSync(join(cases, name, `${file}.rs`), "utf8");
  const names = files.map((file) => read(file).match(/^\/\/@ crate-name: (\w+)\n/)?.[1] ?? file);
  return files.map((file, i) => {
    const text = read(file);
    const declared = text.match(/^\/\/@ uses:(.*)\n/m)?.[1];
    const uses = names.slice(0, i).filter((crate) => declared === undefined || declared.split(",").map((s) => s.trim()).includes(crate));
    return { name: names[i]!, file: join(cases, name, `${file}.rs`), uses: uses.filter((used) => used !== "std" || text.includes("#![no_std]")), edition: text.match(/^\/\/@ edition: (\d+)\n/m)?.[1] };
  });
}

/** What's offered at `path` is a value JS reads: of a namespace, one that
 * offers a value below it, as one of types alone has no JS. */
function valued(offered: Offered[], path: Offered): boolean {
  if (path.kind === "type") return false;
  if (path.kind !== "namespace") return true;
  const below = (p: Offered) => p.js.length > path.js.length && path.js.every((word, i) => p.js[i] === word);
  return offered.some((p) => below(p) && p.kind !== "namespace" && p.kind !== "type");
}

/** A function JS calls and native Rust calls alike, given the same values,
 * or a constant or static it reads: of plain types alone, not generic. */
function called(path: Offered, small: boolean): string[][] | undefined {
  if (path.kind === "function") {
    // Nor one whose JS takes its caller's location after its arguments.
    if (path.generic || path.located || !path.ty || !printable(path.ty)) return undefined;
    return calls(path.params ?? [], small)?.map((given) => given.map((v) => `${v.js}\u0000${v.rust}`));
  }
  if ((path.kind === "const" || path.kind === "static") && path.ty && printable(path.ty)) return [[]];
  return undefined;
}

/** What the net checked of a case, and each problem it found: none where
 * each crate's JS and `.d.ts` offer what Rust does. `paths`, the values
 * found in the JS; `calls`, the results compared with native Rust's;
 * `declared`, the paths read through the `.d.ts`: what a case of the
 * net's own holds above none, so it never passes having checked nothing;
 * `offered`, each crate's paths, `kind rust -> js`. */
/** How long a run of a crate's calls of the values generated is given
 * before it's run of small ones alone: each case's takes under a second,
 * so 15 s, as the machine's load makes it, is room, not a wait. */
const bigRunMs = underLoad(15_000);

export type Netted = { problems: string[]; paths: number; calls: number; declared: number; offered: Record<string, string[]> };

export function exportNet(cases: string, name: string, files: string[]): Netted {
  const dir = fixture(`export-net-${name}`);
  const out = join(dir, "js");
  const all = crates(cases, name, files);
  const problems: string[] = [];
  const checked = { paths: 0, calls: 0, declared: 0 };
  const listed: Record<string, string[]> = {};
  for (const [i, crate] of all.entries()) {
    const manifest = join(out, crate.name, "lib.manifest.json");
    // Each but the last a library, the last the crate the others are of, as
    // test/crates.ts' `chain` builds a pair's app.
    const library = i < all.length - 1 || all.length === 1;
    run([compiler, "--declarations", crate.file, "-o", join(out, crate.name, "lib.js"), ...(library ? ["--library"] : []), "--manifest", manifest,
      ...crate.uses.flatMap((u) => ["--dependency", join(out, u, "lib.manifest.json")]),
      "--", "--crate-name", crate.name, ...(library ? [`--emit=metadata=${join(out, crate.name, `lib${crate.name}.rmeta`)}`] : []), "-Awarnings",
      ...(crate.edition ? [`--edition=${crate.edition}`] : ["--edition=2024"]),
      ...crate.uses.flatMap((u) => ["--extern", `${u}=${join(out, u, `lib${u}.rmeta`)}`]),
      ...crate.uses.flatMap((u) => ["-L", `dependency=${join(out, u)}`])], 600_000);
    const offered: Offered[] = JSON.parse(readFileSync(manifest, "utf8")).offered_paths ?? [];
    const of = (p: Offered) => `${crate.name}::${p.rust.join("::")}`;
    listed[crate.name] = offered.map((p) => `${p.kind} ${p.rust.join("::")} -> ${p.js.join(".")}`);
    // Its JS: each value it offers, of its kind, and each call's result,
    // and native Rust's of the same calls, of the crate built natively: of
    // the values generated, or, where a run of them is past its time, a
    // function given `u32::MAX` that makes as many, of small ones alone.
    const values = offered.filter((p) => valued(offered, p));
    // Its lines alone, marked, not what the functions it calls print.
    const marked = (text: string) => text.split("\n").filter((l) => l.startsWith("\u0001")).map((l) => l.slice(1));
    // A run of the values generated is given `bigRunMs`: one of them past
    // it, `squares(u32::MAX)`'s, is run again of small ones alone, never
    // waited on to the small run's `120_000`, which once was the net's
    // tail, 121 s of one pair's 153 s part.
    const limit = (small: boolean) => (small ? 120_000 : bigRunMs);
    const netted = (small: boolean) => {
      const tested = offered.flatMap((p) => (called(p, small) ?? []).map((given, k) => ({ p, given, label: `${of(p)}#${k}` })));
      const harness = join(out, `net-${crate.name}.mjs`);
      writeFileSync(harness, `import * as M from "./${crate.name}/lib.js";
import { $isPanic } from "@rust-js/std";
${jsShow}
const at = (path) => path.reduce((o, k) => (o == null ? undefined : o[k]), M);
const lines = [];
// A call of a path the JS lacks, which its kind's line says, is said missing.
const said = (label, path, f, t) => {
  if (at(path) === undefined) return void lines.push("\\u0001" + label + " missing");
  try {
    lines.push("\\u0001" + label + " " + show(f(), t));
  } catch (e) {
    lines.push("\\u0001" + label + " " + ($isPanic(e) ? "panic: " + e.message : "JS-ERROR " + (e?.name ?? typeof e) + ": " + (e?.message ?? String(e))));
  }
};
${values.map((p) => `lines.push("\\u0001" + ${JSON.stringify(`kind ${of(p)}`)} + " " + typeof at(${JSON.stringify(p.js)}));`).join("\n")}
${tested.map(({ p, given, label }) => p.kind === "function"
    ? `said(${JSON.stringify(label)}, ${JSON.stringify(p.js)}, () => at(${JSON.stringify(p.js)})(${given.map((g) => g.split("\u0000")[0]).join(", ")}), ${JSON.stringify(p.ty)});`
    : `said(${JSON.stringify(label)}, ${JSON.stringify(p.js)}, () => at(${JSON.stringify(p.js)}), ${JSON.stringify(p.ty)});`).join("\n")}
// On a line of its own, past what a call printed without one.
console.log("\\n" + lines.join("\\n"));
`);
      const js = marked(run(["bun", harness], limit(small)));
      if (tested.length === 0) return { js, native: [] as string[] };
      const rustPath = (p: Offered) => `${crate.name}::${p.rust.join("::")}`;
      const main = `${rustShow}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
${tested.map(({ p, given, label }) => {
    const expr = p.kind === "function" ? `${rustPath(p)}(${given.map((g) => g.split("\u0000")[1]).join(", ")})` : p.kind === "static" ? `&${rustPath(p)}` : rustPath(p);
    return `    let said = shown(|| ${expr});\n    println!("\\n\\u{1}{} {}", ${JSON.stringify(label)}, said);`;
  }).join("\n")}
}
`;
      const built = nativeCrates(all.slice(0, i + 1), main, programFlags("2024"));
      if ("error" in built) return { js, native: [] as string[], unbuilt: built.error };
      return { js, native: marked(run([built.binary], limit(small))) };
    };
    let ran: ReturnType<typeof netted>;
    try {
      ran = netted(false);
    } catch (error) {
      if (!/didn.t finish in|timed out/i.test(String(error))) throw error;
      ran = netted(true);
    }
    const { js, native } = ran;
    const wanted = (p: Offered) => p.kind === "function" ? "function" : p.kind === "namespace" ? "object" : undefined;
    for (const p of values) {
      const line = js.find((l) => l.startsWith(`kind ${of(p)} `));
      const kind = line?.slice(`kind ${of(p)} `.length);
      const expected = wanted(p);
      if (kind !== undefined && kind !== "undefined") checked.paths++;
      if (kind === "undefined" || (expected !== undefined && kind !== expected)) problems.push(`${of(p)}: JS ${p.js.join(".")} is ${kind}, not a ${p.kind}`);
    }
    for (const line of js.filter((l) => / JS-ERROR /.test(l))) problems.push(`${line}: a JS exception, not a Rust panic`);
    if ("unbuilt" in ran && ran.unbuilt) problems.push(`${crate.name}: native Rust's calls didn't build: ${ran.unbuilt.split("\n").slice(0, 6).join(" | ")}`);
    for (const line of native) {
      const label = line.slice(0, line.indexOf(" "));
      const ours = js.find((l) => l.startsWith(`${label} `));
      checked.calls++;
      if (ours !== line && !/ (JS-ERROR |missing$)/.test(ours ?? "")) problems.push(`${label}: JS gives ${ours?.slice(label.length + 1) ?? "nothing"}, native Rust ${line.slice(label.length + 1)}`);
    }
    // Its `.d.ts`: each value and type it offers, by its path, which tsc
    // refuses where the declarations lack one.
    // But a type of no value, JS's to make, and a function its `.d.ts` says
    // it leaves out, as its JS takes what a JS caller can't give (ADR 1077):
    // said there, never dropped unsaid.
    const notes = readdirSync(join(out, crate.name), { recursive: true }).map(String)
      .filter((file) => file.endsWith(".d.ts")).map((file) => readFileSync(join(out, crate.name, file), "utf8")).join("\n");
    const leftOut = (p: Offered) => p.kind === "function" && notes.includes(`// \`${p.js.at(-1)}\` isn't declared: `);
    const declared = offered.filter((p) => (p.kind === "type" ? !p.of_no_value : valued(offered, p)) && !leftOut(p));
    if (declared.length > 0) {
      const reads = join(out, `net-${crate.name}.ts`);
      mkdirSync(out, { recursive: true });
      writeFileSync(reads, `import * as M from "./${crate.name}/lib.js";\n\n${declared.map((p, k) => p.kind === "type"
        ? `export type T${k} = M.${p.js.join(".")}${p.type_params ? `<${Array(p.type_params).fill("unknown").join(", ")}>` : ""};`
        : `export const v${k} = M.${p.js.join(".")};`).join("\n")}\n`);
      checked.declared += declared.length;
      for (const error of typeChecked(out, `net-${crate.name}.ts`, es2022)) problems.push(`${crate.name}'s .d.ts: ${error}`);
    }
  }
  return { problems, ...checked, offered: listed };
}
