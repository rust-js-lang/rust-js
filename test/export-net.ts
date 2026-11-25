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
import { calls, jsConforms, jsShow, printable, rustShow, type Giving, type NoneForm, type Shaped, type Tree } from "./generated-calls";
import { compiler, fixture, nativeCrates, programFlags, run } from "./support";
import { es2022, typeChecked } from "./type-checked";
import { underLoad } from "../scripts/timing";
import { buildFlags } from "./builds";

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
  // Oracle B's (ADR 1204): a struct's fields and where it's defined, a
  // transparent one's field, a function's result's `None`.
  fields?: { rust?: string; js: string; ty: Tree; none?: NoneForm; property?: Tree }[];
  def?: { crate: string; rust: string[] };
  transparent?: Tree;
  variants?: Shaped["variants"];
  positional?: Tree[];
  unit?: Shaped["unit"];
  sealed?: boolean;
  class?: boolean;
  result_none?: NoneForm;
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
function called(path: Offered, small: boolean, giving: Giving): string[][] | undefined {
  if (path.kind === "function") {
    // Nor one whose JS takes its caller's location after its arguments.
    if (path.generic || path.located || !path.ty || !printable(path.ty)) return undefined;
    return calls(path.params ?? [], small, giving)?.map((given) => given.map((v) => `${v.js}\u0000${v.rust}`));
  }
  if ((path.kind === "const" || path.kind === "static") && path.ty && printable(path.ty)) return [[]];
  return undefined;
}

/** A function JS calls for its result's shape alone, oracle B's (ADR
 * 1204), or a constant or static it reads: one `called` leaves out as
 * native's printer can't print what it gives, a struct's, an enum's, a
 * `Box`'s, given the values `calls` makes of its parameters. */
function shapedOnly(path: Offered, small: boolean, giving: Giving): string[][] | undefined {
  if (called(path, small, giving) || !path.ty) return undefined;
  if (path.kind === "function") {
    if (path.generic || path.located) return undefined;
    return calls(path.params ?? [], small, giving)?.map((given) => given.map((v) => `${v.js}\u0000${v.rust}`));
  }
  return path.kind === "const" || path.kind === "static" ? [[]] : undefined;
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

export type Netted = { problems: string[]; paths: number; calls: number; declared: number; reads: number; volatiles: number; shapes: { checked: number; unchecked: number; why: Record<string, number> }; notGiven: Record<string, number>; unread?: string; offered: Record<string, string[]> };

export function exportNet(cases: string, name: string, files: string[]): Netted {
  const dir = fixture(`export-net-${name}`);
  const out = join(dir, "js");
  const reads = join(dir, "js-reads");
  // Why Rust's reads weren't compared, where the reference build refuses
  // a crate (oracle C): said, never passed.
  let unread: string | undefined;
  const all = crates(cases, name, files);
  const problems: string[] = [];
  const checked = { paths: 0, calls: 0, declared: 0, reads: 0, volatiles: 0 };
  const shapes = { checked: 0, unchecked: 0, why: {} as Record<string, number> };
  // Each type of the crates' no value was made of, by why: said, never
  // dropped silently (oracle C).
  const notGiven: Record<string, number> = {};
  const listed: Record<string, string[]> = {};
  // Each struct's shape, of every crate's records so far, by where it's
  // defined: a crate's types are its dependencies' too.
  const types: Record<string, Shaped> = {};
  for (const [i, crate] of all.entries()) {
    const manifest = join(out, crate.name, "lib.manifest.json");
    // Each but the last a library, the last the crate the others are of, as
    // test/crates.ts' `chain` builds a pair's app.
    const library = i < all.length - 1 || all.length === 1;
    // Built twice: as it is, and with every field read a getter's, each
    // kept where Rust reads it (\`--js-reads\`, ADR 1022), oracle C's
    // reference of Rust's reads.
    for (const [into, flags] of [[out, []], [reads, [...buildFlags.reads]]] as const) {
      // A crate the reference build refuses, or one after it, which reads
      // its manifest: its reads not compared, and why said (\`unread\`).
      if (into === reads && unread !== undefined) continue;
      const compile = () => run([compiler, "--declarations", crate.file, "-o", join(into, crate.name, "lib.js"), ...(library ? ["--library"] : []), "--manifest", join(into, crate.name, "lib.manifest.json"), ...flags,
        ...crate.uses.flatMap((u) => ["--dependency", join(into, u, "lib.manifest.json")]),
        "--", "--crate-name", crate.name, ...(library ? [`--emit=metadata=${join(into, crate.name, `lib${crate.name}.rmeta`)}`] : []), "-Awarnings",
        ...(crate.edition ? [`--edition=${crate.edition}`] : ["--edition=2024"]),
        ...crate.uses.flatMap((u) => ["--extern", `${u}=${join(into, u, `lib${u}.rmeta`)}`]),
        ...crate.uses.flatMap((u) => ["-L", `dependency=${join(into, u)}`])], 600_000);
      if (into === out) compile();
      else {
        try {
          compile();
        } catch (error) {
          unread = `${crate.name}: the --js-reads build refuses it: ${String(error).split("\n").find((line) => /^error: rust-js|panicked at/.test(line)) ?? String(error).split("\n")[1]}`;
        }
      }
    }
    const offered: Offered[] = JSON.parse(readFileSync(manifest, "utf8")).offered_paths ?? [];
    const of = (p: Offered) => `${crate.name}::${p.rust.join("::")}`;
    listed[crate.name] = offered.map((p) => `${p.kind} ${p.rust.join("::")} -> ${p.js.join(".")}`);
    // Each type's shape, and the path a caller names it by, the first crate's
    // to offer it, its own before an app's re-export of it.
    for (const p of offered) {
      if (p.kind !== "type" || !p.def) continue;
      const key = `${p.def.crate}::${p.def.rust.join("::")}`;
      types[key] = { path: types[key]?.path ?? `${crate.name}::${p.rust.join("::")}`, sealed: p.sealed, class: p.class, fields: p.fields, transparent: p.transparent, variants: p.variants, positional: p.positional, unit: p.unit };
    }
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
      // Values of the crate's types too, from their records (oracle C), each
      // it gives none of counted by why.
      const giving: Giving = { types, missing: {} };
      const tested = offered.flatMap((p) => [
        ...(called(p, small, giving) ?? []).map((given, k) => ({ p, given, label: `${of(p)}#${k}`, native: true })),
        ...(shapedOnly(p, small, giving) ?? []).map((given, k) => ({ p, given, label: `${of(p)}#${k}`, native: false })),
      ]);
      const compared = tested.filter((t) => t.native);
      const harness = join(out, `net-${crate.name}.mjs`);
      writeFileSync(harness, `import * as M from "./${crate.name}/lib.js";
import { $isPanic } from "@rust-js/std";
// Run as \`proxies\`, each argument given as a counting proxy of what JS
// made (test/counting-proxies.ts, oracle C), which must give what the
// plain run gives, its log of reads printed; as \`volatile <file>\`, each
// field the file's paths name, read once by Rust, a getter answering
// another value after its first, which a second read would see.
const mode = process.argv[2] ?? "plain";
const once = mode === "volatile" ? new Set(JSON.parse((await import("node:fs")).readFileSync(process.argv[3], "utf8"))) : new Set();
const { counted, note, log } = mode === "plain" ? { counted: (v) => v, note: () => {}, log: [] } : (await import(${JSON.stringify(join(import.meta.dir, "counting-proxies.ts"))})).countingProxies();
// Another value of a primitive's kind, which its getter answers after the first.
const other = (v) => (typeof v === "number" ? v + 1 : typeof v === "bigint" ? v + 1n : typeof v === "string" ? v + "!" : typeof v === "boolean" ? !v : v);
// \`value\` with each field \`once\` names a getter of two answers.
const volatile = (value, path) => {
  if (value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map((item, i) => volatile(item, path + "[" + i + "]"));
  const made = {};
  for (const [key, v] of Object.entries(value)) {
    const at = path + "." + key;
    made[key] = once.has(at) && (v === null || typeof v !== "object") ? { $reads: [v, other(v)] } : volatile(v, at);
  }
  return made;
};
const given = (value, path) => counted(mode === "volatile" ? volatile(value, path) : value, path);
${jsShow}
${jsConforms}
const types = ${JSON.stringify(types)};
const count = { checked: 0, unchecked: 0, why: {} };
const at = (path) => path.reduce((o, k) => (o == null ? undefined : o[k]), M);
const lines = [];
// A call of a path the JS lacks, which its kind's line says, is said missing.
// Its result's shape too, oracle B's: a line of its own, which no native
// line matches.
const said = (label, path, f, t, none) => {
  note(label);
  if (at(path) === undefined) return void lines.push("\\u0001" + label + " missing");
  try {
    const v = f();
    lines.push("\\u0001" + label + " " + show(v, t));
    const problem = conforms(v, t, none, types, count);
    if (problem !== null) lines.push("\\u0001shape " + label + " " + problem);
  } catch (e) {
    lines.push("\\u0001" + label + " " + ($isPanic(e) ? "panic: " + e.message : "JS-ERROR " + (e?.name ?? typeof e) + ": " + (e?.message ?? String(e))));
  }
};
${values.map((p) => `lines.push("\\u0001" + ${JSON.stringify(`kind ${of(p)}`)} + " " + typeof at(${JSON.stringify(p.js)}));`).join("\n")}
${tested.map(({ p, given, label }) => p.kind === "function"
    ? `said(${JSON.stringify(label)}, ${JSON.stringify(p.js)}, () => at(${JSON.stringify(p.js)})(${given.map((g, k) => `given(${g.split("\u0000")[0]}, ${JSON.stringify(`${label}.arg${k}`)})`).join(", ")}), ${JSON.stringify(p.ty)}, ${JSON.stringify(p.result_none ?? "plain")});`
    : `said(${JSON.stringify(label)}, ${JSON.stringify(p.js)}, () => at(${JSON.stringify(p.js)}), ${JSON.stringify(p.ty)}, "plain");`).join("\n")}
lines.push("\\u0001counted " + count.checked + " " + count.unchecked);
lines.push("\\u0001why " + JSON.stringify(count.why));
if (mode === "proxies") lines.push("\\u0001reads " + JSON.stringify(log));
// On a line of its own, past what a call printed without one.
console.log("\\n" + lines.join("\\n"));
`);
      const js = marked(run(["bun", harness], limit(small)));
      // Its lines but the proxies' log.
      const said = (lines: string[]) => lines.filter((l) => !l.startsWith("reads "));
      const logOf = (lines: string[]) => JSON.parse(lines.find((l) => l.startsWith("reads "))?.slice("reads ".length) ?? "[]") as string[];
      const differ = (a: string[], b: string[], what: (k: number) => string) =>
        Array.from({ length: Math.max(a.length, b.length) }, (_, k) => k).filter((k) => a[k] !== b[k]).slice(0, 3).map(what);
      // And again, each argument a counting proxy: the same lines.
      const proxied = marked(run(["bun", harness, "proxies"], limit(small)));
      const unlike = differ(js, said(proxied), (k) => `given proxies, ${said(proxied)[k] ?? "nothing"}, where given plain values, ${js[k] ?? "nothing"}`);
      if (unread === undefined) {
        // Rust's reads, the \`--js-reads\` build's log: the readable build's
        // must be it, read for read (oracle C).
        const referenceHarness = join(reads, `net-${crate.name}.mjs`);
        writeFileSync(referenceHarness, readFileSync(harness, "utf8"));
        const reference = logOf(marked(run(["bun", referenceHarness, "proxies"], limit(small))));
        const readable = logOf(proxied);
        unlike.push(...differ(readable, reference, (k) => `reads: the readable build's ${readable[k] ?? "nothing"} where Rust's reads (--js-reads) are ${reference[k] ?? "nothing"}, at read ${k}`));
        // And each field Rust reads once a getter of two answers: the same
        // lines, as a second read would see the second.
        const counts = new Map<string, number>();
        for (const line of reference) if (line.startsWith("get ")) counts.set(line.slice(4), (counts.get(line.slice(4)) ?? 0) + 1);
        const onceFile = join(out, `once-${crate.name}.json`);
        writeFileSync(onceFile, JSON.stringify([...counts].filter(([, n]) => n === 1).map(([path]) => path)));
        const volatileRun = said(marked(run(["bun", harness, "volatile", onceFile], limit(small))));
        checked.reads += reference.length;
        checked.volatiles += [...counts.values()].filter((n) => n === 1).length;
        unlike.push(...differ(js, volatileRun, (k) => `given getters of a second answer where Rust reads once, ${volatileRun[k] ?? "nothing"}, where given plain values, ${js[k] ?? "nothing"}`));
      }
      for (const [why, n] of Object.entries(giving.missing)) notGiven[why] = (notGiven[why] ?? 0) + n;
      if (compared.length === 0) return { js, native: [] as string[], unlike };
      const rustPath = (p: Offered) => `${crate.name}::${p.rust.join("::")}`;
      const main = `${rustShow}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
${compared.map(({ p, given, label }) => {
    const expr = p.kind === "function" ? `${rustPath(p)}(${given.map((g) => g.split("\u0000")[1]).join(", ")})` : p.kind === "static" ? `&${rustPath(p)}` : rustPath(p);
    return `    let said = shown(|| ${expr});\n    println!("\\n\\u{1}{} {}", ${JSON.stringify(label)}, said);`;
  }).join("\n")}
}
`;
      const built = nativeCrates(all.slice(0, i + 1), main, programFlags("2024"));
      if ("error" in built) return { js, native: [] as string[], unbuilt: built.error, unlike };
      return { js, native: marked(run([built.binary], limit(small))), unlike };
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
    // Oracle C: a call given counting proxies gives what it gives plain.
    for (const line of ran.unlike) problems.push(line);
    // Oracle B: each value's JS shape its record's type says.
    for (const line of js.filter((l) => l.startsWith("shape "))) problems.push(`${line.slice("shape ".length)}: of the wrong JS shape`);
    for (const [why, n] of Object.entries(JSON.parse(js.find((l) => l.startsWith("why "))?.slice(4) ?? "{}") as Record<string, number>)) shapes.why[why] = (shapes.why[why] ?? 0) + n;
    const counted = js.find((l) => l.startsWith("counted "))?.split(" ");
    if (counted) {
      shapes.checked += Number(counted[1]);
      shapes.unchecked += Number(counted[2]);
    }
    if ("unbuilt" in ran && ran.unbuilt) problems.push(`${crate.name}: native Rust's calls didn't build: ${ran.unbuilt.split("\n").slice(0, 6).join(" | ")}`);
    for (const line of native) {
      const label = line.slice(0, line.indexOf(" "));
      const ours = js.find((l) => l.startsWith(`${label} `) && !l.startsWith("shape "));
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
  return { problems, ...checked, shapes, notGiven, unread, offered: listed };
}
