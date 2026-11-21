// The metamorphic net's rewrites (#939, test/metamorphic.ts) that don't run
// as native Rust does yet, each under its issue, `#<issue> <title>`: a line a
// rewrite of a case, its builds, `plain`, `std` or both, the rewrite, and the
// case. The nightly runs each, so the list only shrinks: one that runs as
// Rust does is taken off. A rewrite that goes wrong for another reason is a
// line under another issue. One whose difference the user decided, listed
// in ADR 0262 and the guide's differences, is under that difference's
// number in the guide's list, `#difference <n>`: no wrong answer, and taken
// off where it runs as Rust does too.
// Each issue's lines are a file of its own here, named for its heading,
// `1022.txt` or `difference-39.txt`, the heading its first line, so two
// fixes never edit one file (#2260); `knownText` reads them as one list.
