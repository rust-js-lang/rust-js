// The tables test/iterator-matrix.test.ts runs (ADR 1090), each its cases,
// its programs and its slice: the iterator matrix's own, the conformance
// and points tables, test/iterator-matrix.ts's, and each other table's of
// its own module, split from test/iterator-matrix.ts at its cap.

import { asyncCases, asyncProgram, asyncStrata, type AsyncCase } from "./async-points";
import { consumerCases, consumerProgram, consumerStrata, type ConsumerCase } from "./consumer-net";
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
} from "./iterator-matrix";
import { numberCases, numberProgram, numberStrata, type NumberCase } from "./number-grid";
import { stdNetCases, stdNetProgram, stdNetStrata, type NetCase } from "./std-net";
import { buildReact, target } from "./support";
import { unwindCases, unwindProgram, unwindStrata, type UnwindCase } from "./unwind-points";
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
  program(cases: C[]): string;
  batches(cases: C[]): C[][];
  slice(all: C[][]): number[];
  pairs?(cases: C[], perPair: number): C[];
  prepare?(): string[];
};

/** `of`, an iterator table, as the harness runs it. */
const iteratorTable = (of: Table, cases: () => Case[]): TableOf<Case> => ({
  cases,
  program: (batch) => program(batch, of),
  batches: (all) => batches(all, 60, aborts),
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
  program: stdNetProgram,
  batches: (all) => batches(all),
  slice: (all) => slice(all, stdNetStrata),
};

/** The table `of`, as test/iterator-matrix.test.ts runs it. */
export function tableOf(of: Table): TableOf<Row> {
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
