// The builds a net compiles a case in, each by its name and rust-js's
// flags for it: the default, std's iterator structs (ADR 0908), and every
// `RefCell` borrow counted (#2491), and every field read a getter's (oracle
// C). The one catalog the nets, their known
// lists and the reducer (scripts/reduce.ts) read; a module of its own, as
// the reducer and the metamorphic net import each other.
export const buildFlags = {
  plain: [],
  std: ["--std-iterators"],
  cells: ["--count-every-cell"],
  // Every field read a getter's: the export net's oracle C's reference
  // (ADR 1204).
  reads: ["--js-reads"],
} as const satisfies Record<string, readonly string[]>;

export type Build = keyof typeof buildFlags;
