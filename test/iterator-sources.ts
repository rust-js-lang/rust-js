// The iterator tables' sources (ADR 1090): what each case's iterator is
// made of, the crate's own iterators by the steps each writes, and the
// wrappers generic and boxed code holds one in; split from
// test/iterator-matrix.ts at its cap, by concept.

/** What an iterator yields: an item that prints as it's dropped, `D`, a
 * number, a reference to either, or a pair of them. */
/** An item: the crate's `D`, which prints as it drops; a number; a
 * `String` of a number's digits, `S`, whose freeing JS never sees, a
 * collection's items no `Drop` of the crate's runs on (#1681's step (a));
 * a reference to one; or a pair an `enumerate` or a `zip` makes. */
export type Item = "D" | "N" | "S" | "&D" | "&N" | "&S" | "pair" | Nested;
/** An item whose drop is its type's drop glue, not a `Drop` of its own: an
 * `Option<D>`, a `Vec<D>`, a `(D, D)` or a struct holding a `D`, `Held`
 * (#1826). */
export type Nested = "OD" | "VD" | "DD" | "HD";
export const nested = (item: Item): item is Nested => item === "OD" || item === "VD" || item === "DD" || item === "HD";

/** An iterator so far: what it yields, whether a `D` it owns is in it
 * (whose `Drop` may panic), and the traits it has. */
export type Shape = { item: Item; owns: boolean; back: boolean; exact: boolean };

/** Where a case panics: nowhere, in the source's `next`, in the chain's
 * first closure, in the consumer's closure, or in an item's `Drop`; or, of
 * the points table, at each of its points in turn (`run_points`), which no
 * other table's `sites` makes. */
export type Site = "none" | "next" | "stage" | "consumer" | "drop" | "drop+drop" | "points";
export const sites: Site[] = ["none", "next", "stage", "consumer", "drop"];

export type Source = {
  name: string;
  shape: Shape;
  /** Its expression, given where it panics. */
  expr: (site: Site) => string;
  /** A statement before it, a `let` it borrows from. */
  before?: string;
  /** Whether its own `next` can panic: a crate's iterator. */
  own: boolean;
  /** Passed to a generic `fn` as `I`, so the case's chain is of `I`. */
  generic?: boolean;
  /** The wrapper it's in: a sized `Box` of it, whose chain is of the box,
   * or, given to a generic `fn`, its parameter `Box<I>` or `&mut Box<I>`,
   * the box lent and dropped after the case's consumer (#2219, #2221,
   * #2223). */
  wrapper?: "Box" | "&mut Box";
};

/** The crate's own iterators, by which of `Iterator`'s steps each has of
 * its own: each prints as it runs one, so which step std's adapters call
 * is seen, and drops as `drop src`. */
export type Own = { name: string; steps: ("fold" | "nth" | "last" | "count" | "size_hint" | "rfold" | "nth_back")[]; exact: boolean; back: boolean };
export const owns: Own[] = [
  { name: "Plain", steps: [], exact: false, back: false },
  { name: "Folds", steps: ["fold"], exact: false, back: false },
  { name: "Nths", steps: ["nth"], exact: false, back: false },
  { name: "Lasts", steps: ["last"], exact: false, back: false },
  { name: "Counts", steps: ["count"], exact: false, back: false },
  { name: "Hints", steps: ["size_hint"], exact: false, back: false },
  { name: "Exact", steps: ["size_hint"], exact: true, back: false },
  { name: "Backs", steps: [], exact: false, back: true },
  { name: "Every", steps: ["fold", "nth", "last", "count", "size_hint", "rfold", "nth_back"], exact: true, back: true },
];

export const ownSource = (own: Own, generic = false, wrapper?: Source["wrapper"]): Source => ({
  name: `${generic ? "generic " : ""}${wrapper === "&mut Box" ? "&mut " : ""}${wrapper ? "boxed " : ""}own ${own.name}`,
  shape: { item: "D", owns: true, back: own.back, exact: own.exact },
  expr: (site) => {
    const made = `${own.name}::new(${site === "next" ? 3 : 0})`;
    return wrapper ? `Box::new(${made})` : made;
  },
  own: true,
  generic,
  wrapper,
});

/** Of a source's name, the crate's iterator it's made of, `own Every` of
 * `generic &mut boxed own Every`. */
export const ownName = (source: string) => source.replace(/^(generic )?(&mut )?(boxed )?/, "");

export const sources: Source[] = [
  { name: "vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter()", own: false },
  { name: "rev vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter().rev()", own: false },
  { name: "range", shape: { item: "N", owns: false, back: true, exact: true }, expr: () => "(1..6u32)", own: false },
  { name: "vec iter", shape: { item: "&D", owns: false, back: true, exact: true }, before: "let items = ds();", expr: () => "items.iter()", own: false },
  { name: "array iter", shape: { item: "&N", owns: false, back: true, exact: true }, before: "let nums = [1u32, 2, 3, 4, 5];", expr: () => "nums.iter()", own: false },
  ...owns.map((own) => ownSource(own)),
  { name: "generic vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter()", own: false, generic: true },
  ownSource(owns.find((own) => own.name === "Every")!, true),
  // A sized `Box` of the crate's iterator, the chain over the box, and
  // generic code's `Box<I>` and `&mut Box<I>` of it (#2219, #2221, #2223).
  ownSource(owns.find((own) => own.name === "Every")!, false, "Box"),
  ownSource(owns.find((own) => own.name === "Every")!, true, "Box"),
  ownSource(owns.find((own) => own.name === "Every")!, true, "&mut Box"),
  // A `Box<dyn Iterator>` a function gives, and one a binding gives, JS's
  // own iterator of an array, as webapi's `keys()` is (ADR 1136).
  { name: "boxed dyn", shape: { item: "D", owns: true, back: false, exact: false }, expr: () => "boxed(ds())", own: false },
  { name: "js values", shape: { item: "N", owns: false, back: false, exact: false }, before: "let js_nums: Vec<u32> = vec![1, 2, 3, 4, 5];", expr: () => "js_values(&js_nums)", own: false },
  // A `Vec<String>`'s, by value and borrowed: items with no `Drop` of the
  // crate's, whose freeing JS never sees (#1681's step (a)).
  { name: "strings", shape: { item: "S", owns: false, back: true, exact: true }, expr: () => "texts().into_iter()", own: false },
  { name: "strings iter", shape: { item: "&S", owns: false, back: true, exact: true }, before: "let words = texts();", expr: () => "words.iter()", own: false },
  // Items whose drop is their type's drop glue, not a `Drop` of their own,
  // each dropping the `D`s it holds (#1826).
  { name: "options", shape: { item: "OD", owns: true, back: true, exact: true }, expr: () => "options().into_iter()", own: false },
  { name: "vecs", shape: { item: "VD", owns: true, back: true, exact: true }, expr: () => "vecs().into_iter()", own: false },
  { name: "d pairs", shape: { item: "DD", owns: true, back: true, exact: true }, expr: () => "pairs_of_d().into_iter()", own: false },
  { name: "helds", shape: { item: "HD", owns: true, back: true, exact: true }, expr: () => "helds().into_iter()", own: false },
];
