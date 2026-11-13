// React Compiler over rust-js's JS (ADR 1142): babel-plugin-react-compiler,
// the one @vitejs/plugin-react's `reactCompilerPreset` resolves, run with a
// logger, and what it says of each function it would compile: every
// component and hook rust-js emits is compiled, none skipped or refused,
// but those test/react-compiler-known.txt lists. Shared by the per-push
// net, test/react-compiler.test.ts, and the nightly's sweep of the
// lowering tests' programs, scripts/react-compiler-sweep.ts.
import { transformSync } from "@babel/core";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const require = createRequire(join(root, "package.json"));

/** The plugin the preset resolves, from @vitejs/plugin-react's own place,
 * as `reactCompilerPreset` does with `import.meta.resolve`. */
export function presetsPlugin(): string {
  const preset = createRequire(require.resolve("@vitejs/plugin-react"));
  return preset.resolve("babel-plugin-react-compiler");
}

/** What React Compiler refused or skipped of a function: where it is, its
 * name as its line declares it, the event and its reason. */
export type Failure = { fn: string; kind: string; reason: string };

const declared = /(?:function\s*\*?\s*([A-Za-z0-9_$]+)|(?:const|let|var)\s+([A-Za-z0-9_$]+)\s*=)/;

/** What React Compiler says of `code`, a module's JS, `filename` its name:
 * each function it skipped, refused or failed on. Every event but a
 * success and a diagnostic, which is a lint, not a bailout. */
export function failures(code: string, filename: string): Failure[] {
  const events: any[] = [];
  try {
    transformSync(code, {
      filename: filename.replace(/\.js$/, ".jsx"),
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ["jsx"] },
      plugins: [[presetsPlugin(), { panicThreshold: "none", logger: { logEvent: (_: string, event: unknown) => events.push(event) } }]],
    });
  } catch (e) {
    return [{ fn: "<module>", kind: "Throw", reason: String(e).split("\n")[0] }];
  }
  const lines = code.split("\n");
  return events
    .filter((e) => e.kind !== "CompileSuccess" && e.kind !== "CompileDiagnostic")
    .map((e) => {
      const detail = e.detail?.options ?? e.detail ?? {};
      const line: number | undefined = e.fnLoc?.start?.line;
      const m = line ? lines[line - 1]?.match(declared) : undefined;
      return { fn: e.fnName ?? m?.[1] ?? m?.[2] ?? `<line ${line}>`, kind: e.kind, reason: String(detail.reason ?? e.reason ?? JSON.stringify(detail)) };
    });
}

/** An entry of the known list: of `source`, its function `fn` fails
 * `reason`, because of an open `react-compiler` issue, `#1702`, or as the
 * hand-written JS `witness` does, a file under test/react-compiler/. */
export type Known = { source: string; fn: string; reason: string; issue?: number; witness?: string; line: number };

/** test/react-compiler-known.txt: `source<TAB>function<TAB>reason<TAB>why`,
 * `why` an issue, `#1702`, or `as hand-written: <witness>.jsx`. */
export function known(): Known[] {
  const text = readFileSync(join(root, "test/react-compiler-known.txt"), "utf8");
  return text.split("\n").flatMap((raw, i) => {
    if (raw.trim() === "" || raw.startsWith("#")) return [];
    const [source, fn, reason, why, ...rest] = raw.split("\t");
    const issue = why?.match(/^#(\d+)$/)?.[1];
    const witness = why?.match(/^as hand-written: (\S+\.jsx)$/)?.[1];
    if (!source || !fn || !reason || rest.length || (!issue && !witness)) {
      throw new Error(`test/react-compiler-known.txt:${i + 1}: an entry is source, function, reason and why, an issue \`#1702\` or \`as hand-written: <file>.jsx\`, tab-separated: ${raw}`);
    }
    return [{ source, fn, reason, issue: issue ? Number(issue) : undefined, witness, line: i + 1 }];
  });
}

/** Whether `entry` names `failure` of `source`. */
export const names = (entry: Known, source: string, failure: Failure) =>
  entry.source === source && entry.fn === failure.fn && entry.reason === failure.reason;
