// The tables test/iterator-matrix.test.ts runs (ADR 1090), each its cases,
// its programs and its slice: the iterator matrix's own, the conformance
// and points tables, test/iterator-matrix.ts's, and each other table's of
// its own module, split from test/iterator-matrix.ts at its cap.

import { asyncCases, asyncProgram, asyncStrata, type AsyncCase } from "./async-points";
import { consumerCases, consumerProgram, consumerStrata, type ConsumerCase } from "./consumer-net";
import { copyCases, copyProgram, copyStrata, type CopyCase } from "./copy-matrix";
import { formatCases, formatProgram, formatStrata, type FormatCase } from "./format-grid";
import {
  allCases,
  batches,
  type Case,
  conformanceCases,
  pairs,
  pointsCases,
  program,
  type Row,
  slice,
  strata,
  aborts,
  type Table,
  pathsLibrary,
} from "./iterator-matrix";
import { isLibraryPath, pathsCases, pathStrata } from "./iterator-paths";
import { numberCases, numberProgram, numberStrata, type NumberCase } from "./number-grid";
import { refused, stdNetCases, stdNetProgram, stdNetStrata, type NetCase } from "./std-net";
import { buildReact, target } from "./support";
import { unwindCases, unwindProgram, unwindStrata, type UnwindCase } from "./unwind-points";
import { pickedPerBatch } from "../scripts/iteratorRows";
import { join } from "node:path";

/** A table as test/iterator-matrix.test.ts runs it: its cases; a program
 * of some, each case's function `case_<i>`, `i` its index among them, run
 * in turn by `main`, each printing `case <id>` first; the cases in
 * programs; the programs a run without `ITERATOR_MATRIX` runs; and, of a
 * table of adapters and consumers, so many of each pairing (#818); and,
 * of one whose programs use a crate of rust-js's bindings, what builds it,
 * which gives what rustc is told of it as JS's are compiled (#942). */
export type TableOf<C extends Row> = {
  cases(): C[];
  /** Of a table that pins its refusals, each case refused, by its id: what
   * the refusal says, and its issue. */
  refused?: Record<string, [said: string, issue: number]>;
  program(cases: C[]): string;
  /** Of a table whose programs use a library of their own, the paths
   * table's `library` paths', its text; `undefined` of a batch with none. */
  library?(cases: C[]): string | undefined;
  /** Its cases in programs: of a pick, the rows a change names, as many
   * a program as the rows' owner says (`pickedPerBatch`). */
  batches(cases: C[], picked?: boolean): C[][];
  slice(all: C[][]): number[];
  pairs?(cases: C[], perPair: number): C[];
  prepare?(): string[];
};

/** `of`, an iterator table, as the harness runs it. */
const iteratorTable = (of: Table, cases: () => Case[]): TableOf<Case> => ({
  cases,
  program: (batch) => program(batch, of),
  batches: (all, picked) => batches(all, picked ? pickedPerBatch : 60, aborts),
  slice: (all) => slice(all, strata),
  pairs,
});

/** The unwind table (#854), test/unwind-points.ts's cases. */
const unwindTable: TableOf<UnwindCase> = {
  cases: unwindCases,
  program: unwindProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, unwindStrata),
};

/** The formatting grid (#941), test/format-grid.ts's cases. */
const formatTable: TableOf<FormatCase> = {
  cases: formatCases,
  program: formatProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, formatStrata),
};

/** The numbers grid (#940), test/number-grid.ts's cases. */
const numbersTable: TableOf<NumberCase> = {
  cases: numberCases,
  program: numberProgram,
  // Of 200, as each case is one line: some 400 programs, not 1,400.
  batches: (all) => batches(all, 200),
  slice: (all) => slice(all, numberStrata),
};

/** The async table (#942), test/async-points.ts's cases: its programs
 * await by the `js` crate's `settle`, which React's build makes. */
const asyncTable: TableOf<AsyncCase> = {
  cases: asyncCases,
  program: asyncProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, asyncStrata),
  prepare: () => {
    buildReact();
    return ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  },
};

/** The consumer net, test/consumer-net.ts's cases: each std method that
 * takes an iterator by value, of each source, in each variant. Its slice
 * is all of it, which its strata, each template in each variant, pick
 * anyway: some 40 programs, 10 s on the Mac in 8 parts. */
const consumersTable: TableOf<ConsumerCase> = {
  cases: consumerCases,
  program: consumerProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, consumerStrata),
};

/** The std net (#1171), test/std-net.ts's cases: each std method that
 * changes a collection in place or calls what it's given, as it is and with
 * what it calls panicking. Its slice is all of it. */
const stdTable: TableOf<NetCase> = {
  cases: stdNetCases,
  refused,
  program: stdNetProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, stdNetStrata),
};

/** The copy matrix, test/copy-matrix.ts's cases: each compiled iterator
 * form copied each way and stepped by each ability, interleaved with its
 * original; its slice, each adapter and source with each way and ability. */
const copiesTable: TableOf<CopyCase> = {
  cases: copyCases,
  program: copyProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, copyStrata),
};

/** The paths table (#2112), test/iterator-paths.ts's cases: each adapter
 * through each step std's writes, by each path generic and boxed code
 * reaches it; its slice, each path with each adapter, consumer and site. */
const pathsTable: TableOf<Case> = {
  cases: pathsCases,
  program: (batch) => program(batch, "paths"),
  library: pathsLibrary,
  // A `library` path's cases in programs of their own, each with its library,
  // the others' programs one crate each (step 2b′).
  batches: (all) => {
    const library = (c: Case) => c.variant.kind === "path" && isLibraryPath(c.variant.path);
    return [...batches(all.filter((c) => !library(c))), ...batches(all.filter(library))];
  },
  slice: (all) => slice(all, pathStrata),
};

/** The table `of`, as test/iterator-matrix.test.ts runs it. */
export function tableOf(of: Table): TableOf<Row> {
  if (of === "copies") return copiesTable;
  if (of === "paths") return pathsTable;
  if (of === "std") return stdTable;
  if (of === "consumers") return consumersTable;
  if (of === "unwind") return unwindTable;
  if (of === "format") return formatTable;
  if (of === "numbers") return numbersTable;
  if (of === "async") return asyncTable;
  return iteratorTable(of, of === "matrix" ? allCases : of === "conformance" ? conformanceCases : pointsCases);
}

/** The cases of `table`. */
export const casesOf = (of: Table): Row[] => tableOf(of).cases();
