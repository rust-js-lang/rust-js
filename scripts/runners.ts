// How many of GitHub's runners a workflow can hold at once: the free plan
// runs 20 jobs at once across the account, and a scheduled workflow, the
// Nightly, holds at most `NIGHTLY_RUNNERS` of them, so the pushes' checks,
// their attribution and the test map's recordings keep the rest (runners.test.ts
// holds it). The bound is the workflow's worst case, not a measurement:
// the heaviest set of jobs no `needs` path orders, its called workflows'
// jobs inlined, each matrix job as many at once as its `max-parallel`
// lets, every `if:` running but one its caller's `inputs` decide false.
import { readFileSync } from "node:fs";
import { join } from "node:path";

/** The runners a scheduled workflow may hold at once, of the free plan's
 * 20 the account runs: 6 kept for the pushes' checks, attribution and test
 * maps. */
export const NIGHTLY_RUNNERS = 14;

const workflows = join(import.meta.dir, "..", ".github", "workflows");

/** Whether `expr`, a job's `if:`, is true, false, or can't be said before
 * the run (`"?"`): of its caller's `inputs` only, each else unknown, a
 * `needs.*` result or output, a status function, `github.*`. Unknown
 * counts as running. */
export function evaluateIf(expr: string, inputs: Record<string, unknown>): true | false | "?" {
  type Val = { known: boolean; value?: unknown };
  const body = expr.trim().replace(/^\$\{\{\s*([\s\S]*?)\s*\}\}$/, "$1");
  const tokens: { kind: string; text?: string }[] = [];
  for (let i = 0; i < body.length; ) {
    const rest = body.slice(i);
    const space = rest.match(/^\s+/);
    if (space) {
      i += space[0].length;
      continue;
    }
    const op = rest.match(/^(&&|\|\||==|!=|[!()])/);
    if (op) {
      tokens.push({ kind: op[0] });
      i += op[0].length;
      continue;
    }
    const literal = rest.match(/^'((?:[^']|'')*)'/);
    if (literal) {
      tokens.push({ kind: "literal", text: literal[1]!.replace(/''/g, "'") });
      i += literal[0].length;
      continue;
    }
    // A name, a property path, or a function's call, whose arguments are
    // read as a whole: `contains(needs.*.result, 'failure')`.
    const name = rest.match(/^[\w.*-]+(\((?:[^()']|'[^']*')*\))?/);
    if (!name) return "?";
    tokens.push({ kind: "name", text: name[0] });
    i += name[0].length;
  }
  let at = 0;
  const truth = (v: Val): true | false | "?" => (!v.known ? "?" : Boolean(v.value) && v.value !== "false");
  const ofTruth = (t: true | false | "?"): Val => (t === "?" ? { known: false } : { known: true, value: t });
  const primary = (): Val => {
    const token = tokens[at++];
    if (token === undefined) return { known: false };
    if (token.kind === "(") {
      const v = or();
      at++;
      return v;
    }
    if (token.kind === "!") {
      const t = truth(primary());
      return ofTruth(t === "?" ? "?" : !t);
    }
    if (token.kind === "literal") return { known: true, value: token.text };
    const text = token.text ?? "";
    if (text === "true" || text === "false") return { known: true, value: text === "true" };
    if (/^\d+$/.test(text)) return { known: true, value: text };
    if (/^inputs\.[\w-]+$/.test(text)) return { known: true, value: inputs[text.slice("inputs.".length)] ?? "" };
    // Of an input and a literal, `contains(inputs.mutation_args, '--corpus')`,
    // GitHub's: whether the text holds the literal, of any case. A job of
    // a `--corpus` dispatch alone is no nightly's (#2404).
    const holds = text.match(/^contains\(\s*inputs\.([\w-]+)\s*,\s*'((?:[^']|'')*)'\s*\)$/);
    if (holds) return { known: true, value: String(inputs[holds[1]!] ?? "").toLowerCase().includes(holds[2]!.replace(/''/g, "'").toLowerCase()) };
    return { known: false };
  };
  const compared = (): Val => {
    const a = primary();
    const op = tokens[at]?.kind;
    if (op !== "==" && op !== "!=") return a;
    at++;
    const b = primary();
    if (!a.known || !b.known) return { known: false };
    const same = String(a.value) === String(b.value);
    return { known: true, value: op === "==" ? same : !same };
  };
  const and = (): Val => {
    let a = compared();
    while (tokens[at]?.kind === "&&") {
      at++;
      const b = compared();
      const [ta, tb] = [truth(a), truth(b)];
      a = ta === false ? a : tb === false ? b : ta === true && tb === true ? b : { known: false };
    }
    return a;
  };
  const or = (): Val => {
    let a = and();
    while (tokens[at]?.kind === "||") {
      at++;
      const b = and();
      const [ta, tb] = [truth(a), truth(b)];
      a = ta === true ? a : ta === false ? b : tb === true ? b : { known: false };
    }
    return a;
  };
  return truth(or());
}

/** A value a caller's `with:` gives, of `inputs`:
 * a literal, or `${{ inputs.x || <literal> }}`; otherwise unknown. */
function valueOf(value: unknown, inputs: Record<string, unknown>): unknown {
  if (typeof value !== "string") return value;
  const expr = value.match(/^\$\{\{\s*([\s\S]*?)\s*\}\}$/)?.[1];
  if (expr === undefined) return value;
  const either = expr.match(/^inputs\.([\w-]+)\s*\|\|\s*'?([\w-]+)'?$/);
  if (either) return inputs[either[1]!] || either[2];
  const input = expr.match(/^inputs\.([\w-]+)$/);
  if (input) return inputs[input[1]!];
  return undefined;
}

/** A matrix job's `max-parallel`, of `inputs`: a number, or an input read
 * through `fromJSON`, `${{ fromJSON(inputs.x || '<n>') }}`; NaN where only
 * the run can say. An input is a string, and a `max-parallel` that's a
 * string makes GitHub create none of the matrix's jobs, with no error:
 * nightly 44216138054's suite, rustc tests, test map and mutations, whose
 * caps were `${{ inputs.max_parallel || <n> }}` given '2' and '9'. */
function capOf(value: unknown, inputs: Record<string, unknown>): number {
  if (typeof value !== "string") return Number(value);
  const expr = value.match(/^\$\{\{\s*([\s\S]*?)\s*\}\}$/)?.[1];
  if (expr === undefined) return Number(value);
  const parsed = expr.match(/^fromJSON\(([\s\S]*)\)$/)?.[1];
  if (parsed !== undefined) return Number(valueOf(`\${{ ${parsed} }}`, inputs));
  if (/\binputs\./.test(expr)) throw new Error(`\`max-parallel: ${value}\` reads an input, a string, and GitHub creates no job of a matrix whose \`max-parallel\` is a string: write \`\${{ fromJSON(inputs.<name> || '<n>') }}\``);
  return Number.NaN;
}

type Job = { if?: unknown; uses?: string; needs?: string | string[]; with?: Record<string, unknown>; strategy?: { matrix?: unknown; "max-parallel"?: unknown } };

/** A job of a workflow as its run can hold runners: its name, its called
 * workflows' prefixed, and how many at once, 0 where its `if:` is false. */
export type Held = { id: string; at: number; why?: string };

/** The jobs of `file`, run with `inputs`, its called workflows inlined,
 * and their order: `[a, b]` where `b` needs `a`. A job its `if:` rules out
 * holds none, but keeps its place: what needs it waits for it. */
export function heldJobs(file: string, inputs: Record<string, unknown> = {}, prefix = "", read = (name: string) => readFileSync(join(workflows, name), "utf8")): { jobs: Held[]; order: [string, string][] } {
  const jobs = (Bun.YAML.parse(read(file)) as { jobs: Record<string, Job> }).jobs;
  const held: Held[] = [];
  const order: [string, string][] = [];
  const members = new Map<string, string[]>();
  // A job its `if:` rules out, or whose `if:` asks no status, so runs only
  // where each it needs succeeded (GitHub's implied `success()`), and one
  // of which was ruled out. In the YAML's order, which is each job's after
  // those it needs, as GitHub requires none.
  const out = new Set<string>();
  const ruledOutOf = (id: string, job: Job): boolean => {
    if (job.if !== undefined && evaluateIf(String(job.if), inputs) === false) return true;
    const asksStatus = job.if !== undefined && /\b(always|cancelled|failure|success)\(\)/.test(String(job.if));
    return !asksStatus && [job.needs ?? []].flat().some((need) => out.has(need));
  };
  for (const [id, job] of Object.entries(jobs)) {
    const name = `${prefix}${id}`;
    const ruledOut = ruledOutOf(id, job);
    if (ruledOut) out.add(id);
    if (job.uses?.startsWith("./.github/workflows/") && !ruledOut) {
      const given = Object.fromEntries(Object.entries(job.with ?? {}).map(([key, value]) => [key, valueOf(value, inputs)]));
      const called = heldJobs(job.uses.slice("./.github/workflows/".length), given, `${name}/`, read);
      held.push(...called.jobs);
      order.push(...called.order);
      members.set(id, called.jobs.map((j) => j.id));
      continue;
    }
    members.set(id, [name]);
    if (ruledOut) {
      held.push({ id: name, at: 0 });
      continue;
    }
    const size = matrixSize(job.strategy?.matrix);
    const cap = job.strategy?.["max-parallel"] === undefined ? undefined : capOf(job.strategy["max-parallel"], inputs);
    const capped = cap === undefined || Number.isNaN(cap) ? undefined : cap;
    const at = size === undefined ? capped : capped === undefined ? size : Math.min(size, capped);
    held.push(at === undefined ? { id: name, at: Number.POSITIVE_INFINITY, why: "a matrix of a size known only as it runs, and no `max-parallel` its caller's inputs say" } : { id: name, at });
  }
  for (const [id, job] of Object.entries(jobs)) {
    for (const need of [job.needs ?? []].flat()) for (const a of members.get(need) ?? []) for (const b of members.get(id) ?? []) order.push([a, b]);
  }
  return { jobs: held, order };
}

/** How many of a matrix's jobs there are, where its YAML says. */
function matrixSize(matrix: unknown): number | undefined {
  if (matrix === undefined) return 1;
  if (typeof matrix !== "object" || matrix === null) return undefined;
  let size = 1;
  const keys = Object.keys(matrix);
  for (const [key, value] of Object.entries(matrix)) {
    if (key === "exclude") return undefined;
    if (key === "include") {
      if (keys.length === 1 && Array.isArray(value)) size = value.length;
      else return undefined;
      continue;
    }
    if (!Array.isArray(value)) return undefined;
    size *= value.length;
  }
  return size;
}

/** The most runners `jobs` hold at once, ordered by `order`: the heaviest
 * set no two of which a path orders (Dilworth's theorem, weighted: the
 * fewest chains that cover each job as many times as its runners, a
 * minimum flow, found as the total less the most chains merge). */
export function mostAtOnce(jobs: Held[], order: [string, string][]): number {
  const n = jobs.length;
  const index = new Map(jobs.map((job, i) => [job.id, i]));
  const next: number[][] = Array.from({ length: n }, () => []);
  for (const [a, b] of order) next[index.get(a)!]!.push(index.get(b)!);
  const reaches: boolean[][] = Array.from({ length: n }, () => new Array<boolean>(n).fill(false));
  for (let i = 0; i < n; i++) {
    const stack = [...next[i]!];
    while (stack.length > 0) {
      const j = stack.pop()!;
      if (reaches[i]![j]) continue;
      reaches[i]![j] = true;
      stack.push(...next[j]!);
    }
  }
  // Each job an entry `i` and an exit `n + i`; a chain may go from a job's
  // exit to the entry of any it reaches. Flow from the sink back to the
  // source merges two chains into one.
  const source = 2 * n;
  const sink = 2 * n + 1;
  const size = 2 * n + 2;
  const unbounded = Number.MAX_SAFE_INTEGER;
  const capacity = new Map<number, number>();
  const links: number[][] = Array.from({ length: size }, () => []);
  const key = (a: number, b: number) => a * size + b;
  const link = (a: number, b: number, c: number) => {
    if (!capacity.has(key(a, b)) && !capacity.has(key(b, a))) {
      links[a]!.push(b);
      links[b]!.push(a);
    }
    capacity.set(key(a, b), (capacity.get(key(a, b)) ?? 0) + c);
    if (!capacity.has(key(b, a))) capacity.set(key(b, a), 0);
  };
  let total = 0;
  for (let i = 0; i < n; i++) {
    const at = jobs[i]!.at;
    if (!Number.isFinite(at)) return Number.POSITIVE_INFINITY;
    total += at;
    link(sink, n + i, at);
    link(i, source, at);
    link(i, n + i, unbounded);
    for (let j = 0; j < n; j++) if (reaches[j]![i]) link(n + j, i, unbounded);
  }
  let merged = 0;
  for (;;) {
    const from = new Array<number>(size).fill(-1);
    from[sink] = sink;
    const queue = [sink];
    for (let head = 0; head < queue.length && from[source] === -1; head++) {
      const a = queue[head]!;
      for (const b of links[a]!) {
        if (from[b] === -1 && capacity.get(key(a, b))! > 0) {
          from[b] = a;
          queue.push(b);
        }
      }
    }
    if (from[source] === -1) break;
    let flow = unbounded;
    for (let b = source; b !== sink; b = from[b]!) flow = Math.min(flow, capacity.get(key(from[b]!, b))!);
    for (let b = source; b !== sink; b = from[b]!) {
      capacity.set(key(from[b]!, b), capacity.get(key(from[b]!, b))! - flow);
      capacity.set(key(b, from[b]!), capacity.get(key(b, from[b]!))! + flow);
    }
    merged += flow;
  }
  return total - merged;
}

/** The scheduled workflows: what `NIGHTLY_RUNNERS` bounds. */
export function scheduled(read = (name: string) => readFileSync(join(workflows, name), "utf8"), names: string[]): string[] {
  return names.filter((name) => {
    const on = (Bun.YAML.parse(read(name)) as { on?: Record<string, unknown> }).on;
    return on !== undefined && typeof on === "object" && "schedule" in on;
  });
}

if (import.meta.main) {
  const file = process.argv[2] ?? "nightly.yml";
  const { jobs, order } = heldJobs(file);
  console.log(`${file}: at most ${mostAtOnce(jobs, order)} runners at once, of ${jobs.filter((job) => job.at > 0).length} jobs that may run`);
  for (const job of jobs.filter((j) => j.at > 0)) console.log(`  ${job.id}\t${job.at}${job.why ? `: ${job.why}` : ""}`);
}
