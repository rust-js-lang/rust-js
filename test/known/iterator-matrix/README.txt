// The iterator matrix's cases (ADR 1090), the conformance table's (ADR
// 1092), the points table's (#817), the unwind table's (#854), the
// formatting grid's (#941), the numbers grid's (#940), the async table's
// (#942), the consumer net's, the copy matrix's and the paths table's
// that don't run as native Rust does yet, each under its issue, `#<issue>
// <title>`: a line a build, `plain` or `std` of the matrix's,
// `conformance:plain` or `unwind:std` of a table's, the numbers of its
// cases in `allCases()`, `conformanceCases()`, `pointsCases()`,
// `unwindCases()`, `formatCases()`, `numberCases()`,
// `asyncCases()`, `consumerCases()`, `copyCases()` or `pathsCases()`
// as ranges. Each listed must still fail, so the list only shrinks: `bun
// test/iterator-matrix.ts take-off` takes off what the last runs found
// fixed. Its numbers are of each table's generator version below; a change
// to a table's cases makes its rows again, with each issue's cases found
// again.
// Each issue's lines are a file of its own here, named for its heading,
// `1022.txt` or `difference-39.txt`, the heading its first line, so two
// fixes never edit one file (#2260); `knownText` reads them as one list.
// generator 6bc294cebb0e
// generator conformance c609e2beb696
// generator points 464e7d2488e8
// generator unwind 1a27ee232157
// generator format 3c5c14935214
// generator numbers fd0aacb7b310
// generator async 9fe3888b9e1a
// generator consumers cd7403fc97fc
// generator std 638fd9d4020b
// generator copies 85599ed6142e
// generator paths 40d7e437d151
