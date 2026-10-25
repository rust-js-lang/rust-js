// The reducer of generated programs (ADR 0092): a program that fails, made
// as small as it still fails the same way, one change at a time.

import { isInt, print, type Expr, type Program, type Stmt } from "./generate";

// Smaller programs, each one change from `program`: a statement taken away,
// an `if` or a loop made what's inside it, an expression made one of its
// parts or a literal. Some are no longer valid Rust, which the caller finds.
function* smaller(program: Program): Generator<Program> {
  function* inBlock(stmts: Stmt[], rebuild: (next: Stmt[]) => Program): Generator<Program> {
    for (let i = 0; i < stmts.length; i++) {
      const s = stmts[i];
      const put = (replacement: Stmt[]) => rebuild([...stmts.slice(0, i), ...replacement, ...stmts.slice(i + 1)]);
      yield put([]);
      if (s.kind === "if" || s.kind === "if-let" || s.kind === "if-let-b" || s.kind === "slice-pat") {
        yield put(s.then);
        yield put(s.else);
        yield* inBlock(s.then, (next) => put([{ ...s, then: next }]));
        yield* inBlock(s.else, (next) => put([{ ...s, else: next }]));
      } else if (s.kind === "for" || s.kind === "for-each" || s.kind === "for-map" || s.kind === "while" || s.kind === "labeled") {
        yield put(s.body);
        yield* inBlock(s.body, (next) => put([{ ...s, body: next }]));
      }
      if (s.kind === "set" && s.write) yield put([{ ...s, write: undefined }]);
      if (s.kind === "set" && s.note !== undefined) yield put([{ ...s, note: undefined }]);
      if (s.kind === "set" && s.op !== "=") yield put([{ ...s, op: "=" }]);
      if ("value" in s && s.value) for (const value of simpler(s.value)) yield put([{ ...s, value } as Stmt]);
      if (s.kind === "closure") for (const body of simpler(s.body)) yield put([{ ...s, body }]);
      if (s.kind === "dchain") {
        for (let j = 0; j < s.steps.length; j++) yield put([{ ...s, steps: [...s.steps.slice(0, j), ...s.steps.slice(j + 1)] }]);
        if (s.rev) yield put([{ ...s, rev: false }]);
      }
      if (s.kind === "call-mut") {
        if (s.twice) yield put([{ ...s, twice: false }]);
        for (const body of simpler(s.body)) yield put([{ ...s, body }]);
        for (const result of simpler(s.result)) yield put([{ ...s, result }]);
      }
    }
  }
  yield* inBlock(program, (next) => next);
}

// Simpler expressions of the same type: its parts that have it, and a
// literal; then each part made simpler.
function* simpler(e: Expr): Generator<Expr> {
  if (e.kind === "lit" || e.kind === "var" || e.kind === "zero") return;
  const parts = Object.entries(e).filter(([, v]) => v && typeof v === "object" && "kind" in v) as [string, Expr][];
  for (const [, p] of parts) if (p.ty === e.ty) yield p;
  yield e.ty === "bool" ? { kind: "lit", ty: e.ty, value: false } : isInt(e.ty) ? { kind: "lit", ty: e.ty, value: 0n } : { kind: "zero", ty: e.ty };
  if (e.kind === "vec") {
    for (let i = 0; i < e.items.length; i++) yield { ...e, items: [...e.items.slice(0, i), ...e.items.slice(i + 1)] };
  }
  // A chain of one step fewer, or not reversed: which adapter it needs.
  if (e.kind === "chain") {
    for (let i = 0; i < e.steps.length; i++) yield { ...e, steps: [...e.steps.slice(0, i), ...e.steps.slice(i + 1)] };
    if (e.rev) yield { ...e, rev: false };
  }
  for (const [key, p] of parts) for (const q of simpler(p)) yield { ...e, [key]: q } as Expr;
}

/** `program` made as small as it still `fails`: each change that keeps it
 * failing is kept, until none does, or until `until`, a time, has passed,
 * when it's the smallest so far and not `complete`. A change is kept only
 * if it makes the program shorter, so there's an end to them, and each one
 * kept is given to `kept`, so what's found so far is never lost. */
export async function reduce(
  program: Program,
  fails: (p: Program) => Promise<boolean>,
  { until = Infinity, kept = () => {} }: { until?: number; kept?: (p: Program) => void } = {},
): Promise<{ program: Program; complete: boolean }> {
  let current = program;
  let changed = true;
  while (changed) {
    changed = false;
    const length = print(current).length;
    for (const candidate of smaller(current)) {
      if (Date.now() > until) return { program: current, complete: false };
      if (print(candidate).length < length && (await fails(candidate))) {
        current = candidate;
        kept(current);
        changed = true;
        break;
      }
    }
  }
  return { program: current, complete: true };
}

/** How many statements a program has, at every depth. */
export function size(program: Program): number {
  return program.reduce((n, s) => {
    const inner =
      s.kind === "if" || s.kind === "if-let" || s.kind === "if-let-b" || s.kind === "slice-pat"
        ? size(s.then) + size(s.else)
        : s.kind === "for" || s.kind === "for-each" || s.kind === "for-map" || s.kind === "while" || s.kind === "labeled"
          ? size(s.body)
          : 0;
    return n + 1 + inner;
  }, 0);
}
