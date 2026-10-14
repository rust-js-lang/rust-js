// A JS name as Rust names it, the one way every binding generator does:
// React's, webapi's, the builtins' (ADR 0376). `getElementById` is
// `get_element_by_id`, `innerHTML` `inner_html`, `HTMLIFrameElement`
// `html_iframe_element`; and one that's a Rust keyword is the raw
// identifier of it, `type` → `r#type`, which is the name `type` to rustc,
// rustdoc, serde and rust-js alike: a field's JS name is its own. The four
// keywords Rust has no raw identifier of, `self`, `Self`, `super` and
// `crate`, take a `_` after them, `self_`, as nothing else can.

/** Rust's keywords, strict and reserved, of the 2024 edition: what a name
 * can't be as it is. */
export const KEYWORDS = new Set(
  ("as async await break const continue crate dyn else enum extern false fn for gen if impl in let loop match " +
    "mod move mut pub ref return self Self static struct super trait true try type unsafe use where while " +
    "abstract become box do final macro override priv typeof unsized virtual yield").split(" "),
);

/** The keywords a raw identifier can't be. */
const UNRAW = new Set(["self", "Self", "super", "crate"]);

/** `HTMLInputElement` → `["HTML", "Input", "Element"]`, `innerHTML` →
 * `["inner", "HTML"]`. "HTML" then a one-letter word, `HTMLIFrameElement`,
 * would read as "HTMLI": it's split off first, `html_iframe_element`. A `-`
 * or a `:`, `aria-label`, splits words too. */
export function words(name: string): string[] {
  return name.replace(/^HTML(?=[A-Z][A-Z])/, "Html").match(/[A-Z]+(?![a-z])|[A-Z]?[a-z0-9]+/g) ?? [name];
}

/** `getElementById` → `get_element_by_id`: its words, lowercase, joined,
 * as a part of a name, `set_` and it. */
export function snakeWords(name: string): string {
  return words(name)
    .map((word) => word.toLowerCase())
    .join("_");
}

/** `name` as it is in Rust, a keyword escaped: `r#type`, `self_`. */
export function identifier(name: string): string {
  if (!KEYWORDS.has(name)) return name;
  return UNRAW.has(name) ? `${name}_` : `r#${name}`;
}

/** What `identifier` escaped, as it was: `type` of `r#type`, `self` of
 * `self_`, for a name made of it and more, `continue_with_key`. */
export function plain(identifier: string): string {
  if (identifier.startsWith("r#")) return identifier.slice(2);
  const name = identifier.slice(0, -1);
  return identifier.endsWith("_") && UNRAW.has(name) ? name : identifier;
}

/** A JS name as a Rust function's, a field's, a module's or a parameter's:
 * `getElementById` → `get_element_by_id`, `type` → `r#type`. */
export function snake(name: string): string {
  return identifier(snakeWords(name));
}
