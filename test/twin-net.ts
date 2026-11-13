// The twin net (#1205, ADR 1122): the runtime's hand-written copies of
// std against std's own bodies, compiled by rust-js. The tables'
// generated programs, loud items and panic points, run in the plain
// build, whose recognizers lower to the runtime's helpers, and in the
// build of std's iterator structs alone (`--std-iterators`, ADR 0908),
// which is the oracle: `ITERATOR_ORACLE=std` of
// test/iterator-matrix.test.ts. No native run: each push of the iterators'
// code or the runtime's runs each table's slice (`bun
// scripts/iteratorSlice.ts run twins`, check.yml's `twin-net`), the night
// all of it (`run twinsAll`, nightly.yml's `twins`).
//
// A helper is pitted where a case's plain build imports it and its std
// build doesn't: std's body ran in its place. Each other helper the plain
// build imports, the std build imports too, so no case pits it: `shared`
// says why, by its kind. A run fails on a helper neither pitted nor
// listed, and a whole one on one listed that's pitted now or that nothing
// imports; `bun test/twin-net.ts helpers` asks it of the last run's.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { twinHelpers } from "./iterator-matrix";

/** The runtime's helpers a module of rust-js's imports, by name. */
export function importedHelpers(js: string): string[] {
  return [...js.matchAll(/^import \{([^}]*)\} from "@rust-js\/std";$/gm)].flatMap((m) => m[1].split(",").map((name) => name.trim()).filter(Boolean));
}

/** Why std's build has no body of its own in place of each helper both
 * builds import. */
const why = {
  glue: "rust-js's own: how JS holds a value, a reference, a panic, a drop or a trait's dictionary, no copy of a std body",
  consumers: "an `Iterator` method std provides, whose default body std's build lowers as the plain build does: it compiles std's adapter structs, not the consumers",
  iterators: "a std iterator whose struct std's build doesn't compile, a slice's search",
  "js-own": "a std iterator stepped as JS's own iteration by decision, an array's, a string's `Chars`, any JS iterator's step: its ADR beside it (#1458)",
  memory: "a std method whose body reaches memory, which std's build can't compile (docs/std-coverage.txt): its ADR beside it",
  ranges: "a range's `Iterator`, `Step`'s, which std's build doesn't compile: its literal panics, its specialized traits, its bound's comparisons, each integer's `Step` and a range's every form would need it (#1589)",
  collections: "a method of `Vec`, `VecDeque`, a slice, `String`, `BinaryHeap` or a map or a set, whose body std's build doesn't compile: it compiles std's iterator structs alone",
  sort: "a sort, which rust-js writes as its own in both builds",
  format: "`Debug`, which std's build doesn't compile",
  cells: "a `RefCell`'s borrows and its guards, a map of one too, which std's build doesn't compile",
} as const;

/** Each helper a whole run pits against std's body: a slice may see one
 * only where both builds import it. */
export const pitted: string[] = [
  "$repeatingN",
  "$cycleOf",
  "$closing",
  "$collectDropping",
  "$consume",
  "$enumerate",
  "$exactLen",
  "$filterDropping",
  "$filterEnds",
  "$findBack",
  "$flatMap",
  "$flattenEnds",
  "$foldBack",
  "$holding",
  "$iterator",
  "$knowing",
  "$lazyChain",
  "$lazyMapWhile",
  "$lazyRange",
  "$lazyRangeBack",
  "$lazyScan",
  "$lazySkipWhile",
  "$lazyTakeWhile",
  "$lazyZip",
  "$mapEnds",
  "$nextBack",
  "$range",
  "$rangeLen",
  "$rangeNextBack",
  "$rev",
  "$skipping",
  "$stepBy",
  "$stepByEnds",
  "$taking",
  "$trustedZip",
  "$trustedZipEnds",
  "$untracked",
  "$zipEnds",
];

/** Each helper both builds import, by why no case pits it against std's
 * body. */
export const shared: Record<string, keyof typeof why> = {
  "$add": "collections",
  "$advanceSlice": "js-own", // ADRs 0422, 0465: an array's iterator an adapter holds, advanced by index
  "$andModify": "collections",
  "$append": "collections",
  "$asChunksMut": "collections",
  "$asciiCase": "collections",
  "$assign": "glue",
  "$at": "glue",
  "$binarySearchBy": "collections",
  "$borrow": "cells",
  "$borrowMut": "cells",
  "$BTreeMap": "collections",
  "$BTreeSet": "collections",
  "$btreeSet": "collections",
  "$byteLen": "collections",
  "$capacity": "collections",
  "$catchable": "glue",
  "$catchUnwind": "glue",
  "$chars": "js-own", // ADR 1117: a string's `Chars`, the runtime's lazy one
  "$chunkBy": "collections",
  "$chunkByMut": "collections",
  "$chunksExactMut": "collections",
  "$chunksMut": "collections",
  "$clampBy": "collections",
  "$cleanup": "glue",
  "$cloneFromSlice": "collections",
  "$cloneIntoVec": "collections",
  "$cmp": "glue",
  "$cmpItems": "consumers",
  "$collectOptions": "consumers",
  "$collectResults": "consumers",
  "$collectsInPlace": "consumers",
  "$collectCounted": "consumers",
  "$collectVec": "consumers",
  "$copyFromSlice": "collections",
  "$copyRange": "collections",
  "$copyWithin": "collections",
  "$count": "consumers",
  "$clone": "glue",
  "$debugDuration": "format",
  "$debugF64": "format",
  "$debugList": "format",
  "$debugMap": "format",
  "$debugSet": "format",
  "$debugStr": "format",
  "$decodeUtf16": "iterators", // ADR 0327, #1584: `char::decode_utf16`'s, whose error is its code, where std's body makes `DecodeUtf16Error { code }`
  "$dedupBy": "collections",
  "$disposeIterator": "js-own", // ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator, a binding's, closed as JS's own
  "$droppingIterator": "js-own", // ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator stepped dropping what it gives, as JS's own
  "$dedupByCells": "collections",
  "$dequeInsert": "collections",
  "$dequeSwapRemove": "collections",
  "$dispose": "glue",
  "$drain": "collections",
  "$drainAll": "collections",
  "$dropEach": "glue",
  "$dropItems": "glue",
  "$dropping": "glue",
  "$endEntry": "collections",
  "$entriesShown": "format",
  "$entry": "collections",
  "$entryGet": "collections",
  "$entryInsert": "collections",
  "$entryOrInsert": "collections",
  "$entryOrInsertWith": "collections",
  "$entryRemove": "collections",
  "$eq": "glue",
  "$exchange": "glue",
  "$extend": "collections",
  "$extendMap": "collections",
  "$extendText": "collections",
  "$extractIf": "collections",
  "$fill": "collections",
  "$fillWith": "collections",
  "$findDropping": "consumers",
  "$foldDropping": "consumers",
  "$folded": "consumers",
  "$foldOf": "consumers",
  "$foldToArray": "consumers",
  "$getDisjointMut": "collections",
  "$given": "glue",
  "$givenDrop": "glue",
  "$grew": "collections",
  "$grownBy": "collections",
  "$grownVec": "collections",
  "$guardFilterMap": "cells",
  "$guardMap": "cells",
  "$guardMapSplit": "cells",
  "$hashedMap": "collections",
  "$hashedSet": "collections",
  "$hashSlice": "glue",
  "$heapAppend": "collections",
  "$heapExtend": "collections",
  "$heapFrom": "collections",
  "$heapPop": "collections",
  "$heapPush": "collections",
  "$heapRetain": "collections",
  "$heapSorted": "collections",
  "$held": "glue",
  "$index": "glue",
  "$insert": "collections",
  "$insertAt": "collections",
  "$insertMut": "collections",
  "$insertStr": "collections",
  "$intoIter": "js-own", // ADRs 0061, 0475: a generic `IntoIterator` of an array, its `$iter`
  "$intsOrdered": "glue",
  "$isSortedBy": "consumers",
  "$isSortedByKey": "consumers",
  "$iter": "js-own", // ADR 0071: an array's iterator, which knows where it is
  "$iterCmp": "consumers",
  "$iterEq": "consumers",
  "$iterEqGiven": "consumers",
  "$keepCapacity": "collections",
  "$KeyMap": "collections",
  "$KeySet": "collections",
  "$last": "consumers",
  "$lastDropping": "consumers",
  "$lastSome": "consumers",
  "$lent": "glue",
  "$live": "glue",
  "$madeEach": "glue",
  "$madeInTurn": "glue",
  "$makeMut": "glue",
  "$mapExtractIf": "collections",
  "$mapFrom": "collections",
  "$mapGetDisjointMut": "collections",
  "$max": "consumers",
  "$maxBy": "consumers",
  "$maxByKey": "consumers",
  "$maxOf": "consumers",
  "$min": "consumers",
  "$minBy": "consumers",
  "$minByKey": "consumers",
  "$minOf": "consumers",
  "$mutAt": "glue",
  "$mutEntries": "collections",
  "$mutGet": "collections",
  "$mutValues": "collections",
  "$newCyclic": "glue",
  "$next": "js-own", // ADR 0140: the step of any JS iterator, its item or `undefined`
  "$nextSome": "js-own", // ADRs 0140, 0051: the step of any JS iterator, its item boxed where it may look like `None`
  "$nth": "consumers",
  "$nthBack": "consumers",
  "$nthDropping": "consumers",
  "$nthSome": "consumers",
  "$ord": "glue",
  "$ordTreeMap": "collections",
  "$ordTreeSet": "collections",
  "$orInsert": "collections",
  "$orInsertWith": "collections",
  "$orInsertWithKey": "collections",
  "$panicMessage": "glue",
  "$partialEq": "glue",
  "$partiallyOrdered": "glue",
  "$partialOrd": "glue",
  "$partition": "consumers",
  "$partitionPoint": "collections",
  "$pathJoin": "collections",
  "$peek": "memory", // ADR 0561: `Peekable::peek`, whose body reaches memory
  "$peekMut": "collections",
  "$peekMutDrop": "collections",
  "$peekMutTop": "collections",
  "$pop": "collections",
  "$popFrontIf": "collections",
  "$popIf": "collections",
  "$position": "consumers",
  "$pushFrontMut": "collections",
  "$pushMut": "collections",
  "$rangeCountOut": "ranges",
  "$rangeFirst": "ranges",
  "$rangeLast": "ranges",
  "$rangeLastOut": "ranges",
  "$rangeNext": "ranges",
  "$rangeNth": "ranges",
  "$rangeNthBack": "ranges",
  "$rcClone": "glue",
  "$rcDrop": "glue",
  "$rchunksMut": "collections",
  "$reduce": "consumers",
  "$remove": "collections",
  "$removeAt": "collections",
  "$removeEntry": "collections",
  "$removeItem": "collections",
  "$removeOpt": "collections",
  "$repeat": "collections", // `vec![item; count]`, `Vec::from_elem`
  "$repeating": "memory", // ADR 0932, #1569: `Repeat`'s `last` and `count`, which only panic, inline `fmt::Arguments::from_str`'s pointer transmute
  "$replaceRange": "collections",
  "$replaceWith": "collections",
  "$reserveHint": "collections",
  "$resize": "collections",
  "$resizeWith": "collections",
  "$retain": "collections",
  "$retainBack": "collections",
  "$retainDeque": "collections",
  "$retainMap": "collections",
  "$retainMut": "collections",
  "$retainSet": "collections",
  "$retainText": "collections",
  "$roomForCounted": "collections",
  "$roomForHint": "collections",
  "$rotateLeft": "collections",
  "$rotateRight": "collections",
  "$selectNthUnstable": "sort",
  "$setReplace": "collections",
  "$setTake": "collections",
  "$shift": "collections",
  "$formatting": "format",
  "$out": "format",
  "$printing": "format",
  "$showBorrowed": "format",
  "$sink": "format",
  "$sizeHint": "glue",
  "$slice": "collections",
  "$sliceSplitAt": "collections",
  "$sliceSplitBy": "collections",
  "$sliceSplitByMut": "collections",
  "$lazySliceSplitBy": "iterators",
  "$lazySliceSplitByMut": "iterators",
  "$lazyChunkBy": "iterators",
  "$lazyChunkByMut": "iterators",
  "$slicing": "glue",
  "$some": "glue",
  "$someGet": "glue",
  "$someMut": "glue",
  "$someValue": "glue",
  "$sortByCachedKey": "sort",
  "$sortedEntries": "collections",
  "$sortedKeys": "collections",
  "$sortStable": "sort",
  "$sortUnstable": "sort",
  "$splice": "collections",
  "$splicing": "collections",
  "$splitAtMut": "collections",
  "$splitChunkMut": "collections",
  "$splitEndMut": "collections",
  "$splitOff": "collections",
  "$strDrain": "collections",
  "$strExtendWithin": "collections",
  "$strPop": "collections",
  "$strRemove": "collections",
  "$strSplitOff": "collections",
  "$strTruncate": "collections",
  "$swap": "collections",
  "$swapRemove": "collections",
  "$swapWithSlice": "collections",
  "$take": "collections",
  "$took": "glue",
  "$traitImpl": "glue",
  "$treeAppend": "collections",
  "$treeEnd": "collections",
  "$treeFrom": "collections",
  "$treeFromSorted": "collections",
  "$treeRange": "collections",
  "$treeSplitOff": "collections",
  "$truncate": "collections",
  "$tryFoldOk": "consumers",
  "$tryOf": "glue",
  "$unborrow": "cells",
  "$unwindsOn": "glue",
  "$unwound": "glue",
  "$unwrap": "glue",
  "$unwrapOk": "glue",
  "$unzip": "consumers",
  "$upgrade": "glue",
  "$view": "glue",
  "$viewGet": "glue",
  "$withCapacity": "collections",
};

/** Of a run's `target/twin-net/*.json`, what each build of each batch
 * imported: the helpers some case pitted, and those the plain build
 * imported and none pitted. */
export function helpersOf(dir = twinHelpers): { pitted: Set<string>; unpitted: Set<string>; judged: number; of: number } {
  const pitted = new Set<string>();
  const used = new Set<string>();
  let judged = 0;
  let of = 0;
  // In its batches' order, not the directory's, which Linux's ext4 gives
  // by its hash, so the helpers come in one order everywhere.
  for (const file of existsSync(dir) ? readdirSync(dir).sort() : []) {
    const batch: { judged: number; of: number; plain: string[]; std: string[] } = JSON.parse(readFileSync(join(dir, file), "utf8"));
    judged += batch.judged;
    of += batch.of;
    const std = new Set(batch.std);
    for (const helper of batch.plain.filter((name) => name.startsWith("$"))) {
      used.add(helper);
      if (!std.has(helper)) pitted.add(helper);
    }
  }
  return { pitted, unpitted: new Set([...used].filter((helper) => !pitted.has(helper))), judged, of };
}

/** What's wrong with `pitted` and `shared`, of a run's helpers: a helper
 * both builds import that neither lists; and of a whole run, `whole`, one
 * `shared` lists that's pitted now or that nothing imports, and one
 * `pitted` lists that no case pits now, or one pitted that it doesn't,
 * which a slice can't say. */
export function sharedProblems(run: { pitted: Set<string>; unpitted: Set<string> }, whole = true): string[] {
  const listed = new Set(pitted);
  const problems = [...run.unpitted]
    .filter((helper) => shared[helper] === undefined && !listed.has(helper))
    .map((helper) => `${helper}: both builds import it and \`shared\` doesn't say why std's build has no body of its own in its place`);
  if (whole) {
    const off = (helpers: string[], list: string, said: string) => helpers.map((helper) => `${helper}: ${said}: take it off \`${list}\``);
    problems.push(
      ...off(Object.keys(shared).filter((helper) => run.pitted.has(helper)), "shared", "a case pits it against std's body now"),
      ...off(Object.keys(shared).filter((helper) => !run.pitted.has(helper) && !run.unpitted.has(helper)), "shared", "no case imports it now"),
      ...off(pitted.filter((helper) => !run.pitted.has(helper)), "pitted", "no case pits it now"),
      ...[...run.pitted].filter((helper) => !listed.has(helper)).map((helper) => `${helper}: a case pits it against std's body: add it to \`pitted\``),
    );
  }
  return problems.sort();
}

if (import.meta.main) {
  if (process.argv[2] !== "helpers") throw new Error("usage: bun test/twin-net.ts helpers");
  const helpers = helpersOf();
  if (helpers.of === 0) throw new Error(`no run's helpers in ${twinHelpers}: run \`bun scripts/iteratorSlice.ts run twins\``);
  console.log(`${helpers.judged} of ${helpers.of} cases judged against std's bodies; ${helpers.pitted.size} helpers pitted: ${[...helpers.pitted].sort().join(" ")}`);
  const problems = sharedProblems(helpers);
  for (const problem of problems) console.log(problem);
  process.exit(problems.length === 0 ? 0 : 1);
}
