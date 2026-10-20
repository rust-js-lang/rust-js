# A shipped url preserves parsing and mutation

Status: Draft. A candidate under [the shipped-crate contract](shipped-crates.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)) for parsing,
serialization, errors and mutation; A for internal storage differences
with no observable effect; D outside the qualified API.

## Decision

**Use the platform `URL` only behind an implementation of the qualified
Rust `url` API.** Sharing a WHATWG foundation is a reason to investigate
delegation, not evidence that every method, version or error agrees.

Start by qualifying parsing, joining and read access. Translate platform
exceptions to Rust errors only where the corresponding Rust operation
fails. JS accepting an input is not proof Rust accepts it; JS rejecting
one is not permission to fabricate a Rust error.

The compatibility matrix must cover schemes, relative resolution,
cannot-be-a-base URLs, credentials, ports, IPv4/IPv6, internationalized
hosts, percent encoding, query/fragment presence and serialization.
Preserve distinctions such as an absent query versus an empty query.
Qualify Unicode/IDNA behavior against the selected Rust crate version and
the supported runtime environments.

`Url::clone` makes an independent Rust value, even if the implementation
uses a mutable JS `URL`. Do not accidentally share mutations across
clones or replace stable snapshots with live platform views.

Mutable APIs require their own rows before support. In particular,
`path_segments_mut` and query mutation involve guard lifetimes and
observable state at guard release. Follow the Rust API's ownership,
encoding and commit behavior; setting platform properties or using
`URLSearchParams` is not sufficient evidence of equivalence. Keep
unqualified mutation APIs refused.

## Acceptance

Run the qualified crate's relevant tests and suitable URL-standard cases
against native Rust, plus differential generated inputs. Test
parse/serialize/parse round trips, joins, invalid hosts and ports,
percent escapes, non-ASCII input, empty and absent parts, and opaque URLs.

For mutable APIs, test repeated edits, failed edits, guard release on
normal and unwinding paths, and clone independence. Compare successful
values and Rust error variants, not just whether both sides threw.

Mutations erase empty-versus-absent distinctions, skip an encoding step,
share a clone's mutable object, or convert every platform exception to one
Rust error. Qualification runs in the supported browser and native JS
environments. A new crate or runtime version must re-establish the
contract before it becomes a supported combination.
