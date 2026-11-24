# 1180. A known list is a directory, a file per issue

Status: Accepted (2026-11-21, the lead's decision for #2260).
Amends [1090](1090-an-iterator-conformance-matrix.md),
[1103](1103-the-corpus-shuffled-into-crates-finds-what-one-function-tells-another.md)
and [1104](1104-a-case-rewritten-in-meaning-runs-as-it-did.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is how the
tests keep what they know, not anything the JS does.

## Context

Three nets list the failures they know under issue headings, `#<issue>
<title>`, with the rows below:
- the iterator matrix and its tables, `test/iterator-matrix-known.txt`;
- the metamorphic net, `test/metamorphic-known.txt`;
- the neighbour shuffle, `test/neighbours-known.txt`.

Every iterator fix edits one of these files, so two agents' fixes
conflicted there. katniss agent and the Mac mini did on 2026-11-21, and with
five pushers it gets worse. `bun run push` merges a known list's conflict
as a line set, but a line both sides changed, an id list of the matrix,
is still its author's to merge. And the push guard read the removal of a
decided Crater row, `scripts/crater-decided.txt`'s #2253 lines, as
undoing the commit that listed them, so 1977f0c77 had to name it.

## Decision

**Each of the three lists is a directory, `test/known/<list>/`, with one
file per issue.**
- A file is named for its heading, `1022.txt` or `difference-39.txt`. Its
  first line is that heading, and its rows follow.
- `README.txt` holds what the list is, from the single file's header, and
  the iterator matrix's `// generator <table> <version>` lines, which
  belong to the tables, not to any one issue.
- `knownText(list)` in `test/support.ts` is the one reader. It joins the
  README and each file in order, so each net's parser reads the list as
  before.
- `writeKnown(list, text)` is the one writer. It splits a list's text at
  its headings, writes each issue's file and removes the file of an issue
  with no rows left. The matrix's `take-off` and `add` use it. Rewriting
  an unchanged list writes the same bytes.
- A run that finds a listed failure running as Rust does names the
  issue's own file: "take it off test/known/metamorphic/2254.txt".
- `bun run push` merges each issue's file as a line set (`lineSet`), as it
  did the whole list.

**The push guard's known-list rule covers an issue's file and the decided
Crater list.** Taking rows off `test/known/<list>/<n>.txt`, or the whole
file, in a push that `Fixes` or `Refs` #n undoes nothing. The same holds
for a row of `scripts/crater-decided.txt` that names an issue the push
fixes, `#2253`, since a decided line that no longer fails means that issue
is fixed. A decided row of no issue, a `#difference` row, or the list's
comment still counts as undoing.

The other known lists stay single files: the object model's, the memory
net's, Rustlantis', React Compiler's, the Crater's decided list and
rustc's suite. Their rows carry their issue on each line, in formats of
their own, and they rarely conflict. rustc's list is rewritten by
`--bless`.

## Consequences

- Two fixes of two issues touch two files, so they never conflict.
- One issue's file is one place to look for what is known of it.
- The push guard's tests hold both new exemptions, `test/push-guard.test.ts`.
  The push tests hold that every issue's file is a line set.
- Rejected: per-area files, since an issue is what a fix takes off, and
  areas overlap. Also rejected: keeping one file and leaning on the
  line-set merge, which can't merge a line both sides changed.
