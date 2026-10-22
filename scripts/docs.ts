// The guide's examples, compiled (docs/guide/README.md#examples): each
// ```rust fence of each page is compiled by rust-js as it is, and the fences
// right after it are written from what it made, so no page's JavaScript is
// written by hand.
//
//   bun run docs                  write each page's output fences
//   bun run docs --check          fail where a page isn't what the compiler makes
//   bun run docs --page <page.md> of that page alone
//
// A fence after an example, `js` or `jsx`, is its JavaScript; `ts`, its
// declarations; one with no language, what the compiler says of it, which
// fails. `rust excerpt` isn't compiled, and nothing follows it. A line `# ..`
// is compiled but isn't the example's, as rustdoc's: what it needs, a `use`
// or a type an earlier example made. Examples of several files are a run of
// fences each headed `// src/x.rs`, its first the crate's root; each output
// fence after them is headed by the module it's of, `// src/x.js`. A `toml`
// fence of `[dependencies]` right before an example makes it an app that
// `rust-js build` builds, a crate of crates.io's JS headed by its path,
// `// rust-js/crates/either-1.15.0/src/lib.js`.
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { dirname, join, relative } from "node:path";
import { buildCompiler, buildNode, buildReact, buildSerde, cargoApp, compiler, root, run, target } from "../test/support";

const check = process.argv.includes("--check");
const guide = join(root, "docs/guide");
const work = join(target, "docs");

type Fence = { info: string; body: string; start: number; end: number };
/** An example: its files, its output fences, and, of an app `rust-js build`
 * builds, the dependencies its `Cargo.toml` adds. */
type Example = { page: string; sources: { path: string; code: string }[]; outputs: Fence[]; dependencies?: string };

/** Each fence of `text`: its info string, its body, where its body is. */
function fences(text: string): Fence[] {
  const found: Fence[] = [];
  const pattern = /^```([^\n]*)\n([\s\S]*?)^```$/gm;
  for (let m = pattern.exec(text); m; m = pattern.exec(text)) {
    const start = m.index + 4 + m[1].length;
    found.push({ info: m[1].trim(), body: m[2], start, end: start + m[2].length });
  }
  return found;
}

const outputKinds = new Set(["js", "jsx", "ts", ""]);
const header = (body: string) => body.match(/^\/\/ (\S+\.(?:rs|jsx?|d\.ts))\n/)?.[1];

/** The examples of `page`, and what's wrong with how they're written. */
function examples(page: string, text: string, problems: string[]): Example[] {
  const all = fences(text);
  const found: Example[] = [];
  for (let i = 0; i < all.length; i++) {
    const fence = all[i];
    if (fence.info === "rust excerpt") {
      if (all[i + 1] && outputKinds.has(all[i + 1].info) && all[i + 1].start - fence.end < 8) {
        problems.push(`${page}: an excerpt is followed by an output fence, which only an example's is`);
      }
      continue;
    }
    if (fence.info !== "rust") continue;
    // A run of files, `// src/x.rs`, or one.
    const sources = [{ path: header(fence.body) ?? "src/lib.rs", code: fence.body }];
    let j = i + 1;
    if (header(fence.body)) {
      while (all[j]?.info === "rust" && header(all[j].body)) {
        sources.push({ path: header(all[j].body)!, code: all[j].body });
        j++;
      }
    }
    const outputs: Fence[] = [];
    while (all[j] && outputKinds.has(all[j].info) && between(text, all[j - 1], all[j])) {
      outputs.push(all[j]);
      j++;
    }
    // A `toml` fence of `[dependencies]` right before it: an app's, which
    // Cargo resolves, as crates.io's are (ADR 0415).
    const before = all[i - 1];
    const dependencies =
      before?.info === "toml" && before.body.startsWith("[dependencies]\n") && between(text, before, fence)
        ? before.body.slice("[dependencies]\n".length)
        : undefined;
    found.push({ page, sources, outputs, dependencies });
    i = j - 1;
  }
  return found;
}

/** Whether only blank lines are between two fences: one's output follows it. */
function between(text: string, a: Fence, b: Fence): boolean {
  return text.slice(a.end + 3, b.start - 4 - b.info.length).trim() === "";
}

/** The example's code as it's compiled: its `# ` lines' too, as rustdoc's (not an `#[attribute]`). */
const compiled = (code: string) => code.replace(/^#(?: (.*))?$/gm, "$1");

/** The crates an example may use, as rustc's flags. */
function crates(code: string): string[] {
  // Node's crate is built with its own js and webapi crates, which it's
  // given beside it, and not React's.
  if (/\bnode::/.test(code)) {
    const node = buildNode();
    return ["--extern", `js=${join(node, "libjs.rmeta")}`, "--extern", `webapi=${join(node, "libwebapi.rmeta")}`,
      "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node];
  }
  buildReact();
  const flags = ["--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`,
    "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  if (/\bserde/.test(code)) flags.push(...buildSerde());
  return flags;
}

/** A generated file's text without its header and source map line. */
function generated(path: string): string {
  const lines = readFileSync(path, "utf8").split("\n");
  const body = lines.filter((line, k) => !(k === 0 && line.startsWith("// Generated by rust-js")) && !line.startsWith("//# sourceMappingURL="));
  return body.join("\n").trim() + "\n";
}

/** What the example's output fences say, compiled: each fence's new body. */
async function compile(example: Example, n: number): Promise<{ bodies: string[]; problem?: string }> {
  const dir = join(work, relative(guide, example.page).replace(/\//g, "_"), String(n));
  rmSync(dir, { recursive: true, force: true });
  for (const { path, code } of example.sources) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    // Its header names the file, and isn't a comment of its code: a blank line, so lines keep their numbers.
    writeFileSync(join(dir, path), compiled(code.replace(/^\/\/ \S+\.rs\n/, "\n")));
  }
  // Its root: `src/lib.rs` where it's one of them, else its first.
  const rootFile = (example.sources.find((s) => /^(src\/)?(lib|main)\.rs$/.test(s.path)) ?? example.sources[0]).path;
  const declarations = example.outputs.some((o) => o.info === "ts");
  const out = join(dir, rootFile.replace(/\.rs$/, ".js"));
  // Named as Cargo names a package's crate, not by its root's file, which
  // may be no crate's name: a Next.js route's, `pages/posts/[slug].rs`.
  const code = example.sources.map((s) => s.code).join("\n");
  // An app `rust-js build` builds, of what Cargo resolves: crates.io's, and
  // Next.js's crate, which the react and node crates' are built with.
  const crate = (name: string, dir: string) =>
    new RegExp(`\\b${name}::`).test(code) ? `${name} = { package = "rust-js-${dir}", path = ${JSON.stringify(join(root, dir))} }\n` : "";
  const next = crate("next", "next");
  const app = example.dependencies !== undefined || next !== "";
  if (app) {
    const root = example.sources.find((s) => s.path === rootFile)!;
    cargoApp(dir, compiled(root.code.replace(/^\/\/ \S+\.rs\n/, "\n")), {
      lib: rootFile,
      react: /\breact::/.test(code) || next !== "",
      toml: next + crate("js", "builtins") + crate("webapi", "webapi") + crate("node", "node") + (example.dependencies ?? ""),
    });
  }
  const proc = Bun.spawn(
    app
      ? [process.execPath, join(root, "tooling", "cli.js"), "build"]
      : [compiler, join(dir, rootFile), "-o", out, ...(declarations ? ["--declarations"] : []), "--",
          "--crate-name=example", ...crates(code)],
    { cwd: dir, stdout: "pipe", stderr: "pipe", env: { ...process.env, RUST_JS_COMPILER: compiler } },
  );
  const [stderr, exit] = [await new Response(proc.stderr).text(), await proc.exited];
  // What the compiler said, but Cargo's progress, which is a run's own.
  const progress = /^ +(Blocking|Compiling|Checking|Downloading|Downloaded|Updating|Locking|Adding|Finished|Fresh)\b/;
  const said =
    stderr
      .replaceAll(`${dir}/`, "")
      .replaceAll(dir, ".")
      .replace(/\u001b\[[0-9;]*m/g, "")
      .split("\n")
      .filter((line) => !progress.test(line))
      .join("\n")
      .trim() + "\n";
  const where = `${relative(root, example.page)}: the example of ${rootFile} at fence ${n}`;
  if (exit !== 0 && !example.outputs.some((o) => o.info === "")) {
    return { bodies: [], problem: `${where} doesn't compile:\n${said}` };
  }
  if (exit === 0 && example.outputs.some((o) => o.info === "")) {
    return { bodies: [], problem: `${where} compiles, where its fence says what the compiler refuses` };
  }
  const files = existsSync(dir) ? readdirSync(dir, { recursive: true }).map(String) : [];
  const bodies: string[] = [];
  for (const output of example.outputs) {
    if (output.info === "") {
      bodies.push(said);
      continue;
    }
    // Its module, by its header, or the root's.
    const named = header(output.body);
    const stem = (named ?? rootFile).replace(/\.(rs|jsx?|d\.ts)$/, "");
    const ext = output.info === "ts" ? [".d.ts"] : [".js", ".jsx"];
    const file = ext.map((e) => `${stem}${e}`).find((f) => files.includes(f));
    if (!file) return { bodies: [], problem: `${where} writes no ${stem}${ext.join(" or ")}` };
    bodies.push((named ? `// ${named}\n` : "") + generated(join(dir, file)));
  }
  return { bodies };
}

async function main() {
  buildCompiler();
  // `--page docs/guide/x.md`, each that one alone.
  const only = process.argv.flatMap((arg, k) => (process.argv[k - 1] === "--page" ? [join(root, arg)] : []));
  const pages = readdirSync(guide, { recursive: true })
    .map(String)
    .filter((p) => p.endsWith(".md"))
    .map((p) => join(guide, p))
    .filter((p) => only.length === 0 || only.includes(p))
    .sort();
  const problems: string[] = [];
  const jobs: { page: string; example: Example; n: number }[] = [];
  const texts = new Map<string, string>();
  for (const page of pages) {
    const text = readFileSync(page, "utf8");
    texts.set(page, text);
    examples(page, text, problems).forEach((example, n) => jobs.push({ page, example, n }));
  }
  buildReact();
  const results: { page: string; example: Example; bodies: string[] }[] = [];
  let next = 0;
  await Promise.all(
    Array.from({ length: availableParallelism() }, async () => {
      while (next < jobs.length) {
        const job = jobs[next++];
        const { bodies, problem } = await compile(job.example, job.n);
        if (problem) problems.push(problem);
        else results.push({ page: job.page, example: job.example, bodies });
      }
    }),
  );
  const stale: string[] = [];
  for (const page of pages) {
    let text = texts.get(page)!;
    const edits = results
      .filter((r) => r.page === page)
      .flatMap((r) => r.example.outputs.map((fence, k) => ({ fence, body: r.bodies[k] })))
      .filter(({ fence, body }) => fence.body !== body)
      .sort((a, b) => b.fence.start - a.fence.start);
    if (edits.length === 0) continue;
    stale.push(`${relative(root, page)}: ${edits.length} output fence${edits.length === 1 ? "" : "s"}`);
    for (const { fence, body } of edits) text = text.slice(0, fence.start) + body + text.slice(fence.end);
    if (!check) writeFileSync(page, text);
  }
  for (const problem of problems) console.error(problem);
  if (check && stale.length > 0) {
    console.error(`not what the compiler makes, \`bun run docs\` writes them:\n${stale.join("\n")}`);
  } else if (!check) {
    for (const line of stale) console.log(`wrote ${line}`);
  }
  console.log(`${jobs.length} examples`);
  if (problems.length > 0 || (check && stale.length > 0)) process.exit(1);
}

await main();
