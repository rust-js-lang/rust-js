// The neighbour shuffle's pairs (#853, test/neighbours.ts) that don't run as
// native Rust does yet, each under its issue, `#<issue> <title>`: a line a
// pair, its builds, `plain`, `std` or both, the case that goes wrong, `-` of
// a crash that points at none, `beside`, and the fewest neighbours that make
// it go wrong, as the shuffle narrows it; `slow` of one that runs as Rust
// does but too long to finish, its range stepped where alone it's moved.
// Each still goes wrong: `bun test test/neighbours.test.ts` compiles each
// pair alone, so the list only shrinks. A group of a later seed whose case
// goes wrong beside another neighbour, for the same reason, is a line more
// under the same issue.
// Each issue's lines are a file of its own here, named for its heading,
// `1022.txt` or `difference-39.txt`, the heading its first line, so two
// fixes never edit one file (#2260); `knownText` reads them as one list.
