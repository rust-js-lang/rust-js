//! What the runtime's helpers declare, read once of their sources: each `$`
//! name a helper declares, which helper declares it, and, of a function,
//! its parameters in order, as its own JS writes them (#2334). The one
//! reader of the helpers' sources' declarations: which helper exports a
//! name (`helper_of`, `exports`), where a panic's location goes
//! (`location_at`), what each argument of a call is
//! (`js::HelperCall`), each answered of the helper's own parameter list,
//! and whether a call of it gives a JS iterator (`gives_iterator`), as its
//! own source says (#2229).

use super::Helper;
use crate::collections::IndexMap;

/// A parameter a helper's function declares: its name, or of a pattern,
/// `[aLow, aHigh]`, its text; and what it is of its list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub kind: ParamKind,
}

/// What a parameter is of its list: one a call gives, one with a default,
/// `kind = "lent"`, or the rest, `...items`, which JS gives no default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    Given,
    Defaulted,
    Rest,
}

/// What a helper declares of one name: the helper, and of a function, its
/// parameters, of a class, a `const` or a `let`, none; and whether a call
/// of it gives a JS iterator (`GIVES_AN_ITERATOR`).
pub(super) struct Declared {
    pub(super) helper: Helper,
    pub(super) params: Option<Vec<Param>>,
    pub(super) gives_iterator: bool,
}

/// The line a helper's function is preceded by where a call of it gives a
/// JS iterator that its header doesn't say: one a generator's, `function*`,
/// says itself. The runtime's helpers carry no typed return, no JSDoc
/// `@returns` nor a `.d.ts`, to read it of (#2229).
pub const GIVES_AN_ITERATOR: &str = "// gives: an iterator";

/// Whether a call of the helper `name` gives a JS iterator, as its source
/// says: a generator's, or a function marked `GIVES_AN_ITERATOR`. The one
/// answer of which helpers' calls are JS iterators (#2229); false of a name
/// no helper declares.
pub fn gives_iterator(name: &str) -> bool {
    declarations().get(name).is_some_and(|declared| declared.gives_iterator)
}

/// Every `$` name the helpers declare, in `Helper`'s order, each with what
/// declares it; of a name two declare, the last's, as `package_module`
/// refuses two.
pub(super) fn declarations() -> &'static IndexMap<&'static str, Declared> {
    static DECLARED: std::sync::LazyLock<IndexMap<&'static str, Declared>> = std::sync::LazyLock::new(|| {
        (Helper::ALL.iter())
            .flat_map(|&helper| {
                declared_in(helper.source()).map(move |(name, params, gives_iterator)| {
                    (
                        name,
                        Declared {
                            helper,
                            params,
                            gives_iterator,
                        },
                    )
                })
            })
            .collect()
    });
    &DECLARED
}

/// The parameters the helper `name`'s function declares, in order; `None`
/// of a name no helper declares as a function.
pub fn params(name: &str) -> Option<&'static [Param]> {
    declarations().get(name)?.params.as_deref()
}

/// Each `$` name `source` declares at a line's start; of a function, its
/// parameters, its list read to its closing `)` across lines; and whether
/// it gives a JS iterator, a generator or one the line before it marks
/// (`GIVES_AN_ITERATOR`).
pub(super) fn declared_in(source: &'static str) -> impl Iterator<Item = (&'static str, Option<Vec<Param>>, bool)> {
    let mut at = 0;
    let mut marked = false;
    source.split_inclusive('\n').filter_map(move |line| {
        let start = at;
        at += line.len();
        let after_mark = std::mem::replace(&mut marked, line.trim_end() == GIVES_AN_ITERATOR);
        let keyword = ["async function ", "function* ", "function ", "class ", "const ", "let "]
            .iter()
            .find_map(|keyword| line.strip_prefix(keyword).map(|rest| (*keyword, rest)));
        let name = keyword.map(|(_, rest)| {
            &rest[..rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .unwrap_or(rest.len())]
        });
        let Some(((keyword, _), name)) = keyword.zip(name).filter(|(_, name)| name.starts_with('$')) else {
            assert!(
                !after_mark,
                "`{GIVES_AN_ITERATOR}` is followed by no helper's declaration: {line}"
            );
            return None;
        };
        let params = keyword.contains("function").then(|| {
            let open = start + keyword.len() + name.len();
            parameters(&source[open..])
        });
        assert!(
            !after_mark || keyword.contains("function") && keyword != "function* ",
            "`{GIVES_AN_ITERATOR}` marks a function its header doesn't say gives one, not `{name}`"
        );
        Some((name, params, keyword == "function* " || after_mark))
    })
}

/// The parameters of a list `text` starts with, `(a, b = { c: 1 }, ...d)`:
/// split at its commas outside brackets and strings, to its closing `)`.
fn parameters(text: &'static str) -> Vec<Param> {
    debug_assert!(text.starts_with('('), "a function's name is followed by its parameters");
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut from = 1;
    let mut params = Vec::new();
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q && !text[..i].ends_with('\\') => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, '(' | '[' | '{') => depth += 1,
            (None, ')') if depth == 1 => {
                params.extend(param(&text[from..i]));
                break;
            }
            (None, ')' | ']' | '}') => depth -= 1,
            (None, ',') if depth == 1 => {
                params.extend(param(&text[from..i]));
                from = i + 1;
            }
            _ => {}
        }
    }
    params
}

/// One parameter's text, `kind = "lent"`, `...items`, `[aLow, aHigh]`:
/// none of what's blank, a list's trailing comma's.
fn param(text: &'static str) -> Option<Param> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (rest, text) = match text.strip_prefix("...") {
        Some(text) => (Some(ParamKind::Rest), text),
        None => (None, text),
    };
    // Its default, after the `=` outside its pattern: a name's is its first.
    let pattern = text.starts_with(['[', '{']);
    let named = match pattern {
        true => &text[..=text.rfind([']', '}']).unwrap_or(text.len() - 1)],
        false => text.split('=').next().unwrap_or(text).trim(),
    };
    let defaulted = text[named.len()..].trim_start().starts_with('=');
    let kind = rest.unwrap_or(if defaulted {
        ParamKind::Defaulted
    } else {
        ParamKind::Given
    });
    Some(Param { name: named, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The helpers whose calls give a JS iterator, as their sources say:
    /// the generators, `$keysIn`, `$recvItems` and `$tryRecvItems`, and
    /// each marked `GIVES_AN_ITERATOR`, `$lent` and `$held` among them,
    /// which `is_js_iterator` writes no `Iterator.from` around (#2229).
    #[test]
    fn the_helpers_that_give_an_iterator() {
        let mut giving: Vec<&str> = (declarations().iter())
            .filter(|(_, declared)| declared.gives_iterator)
            .map(|(&name, _)| name)
            .collect();
        giving.sort_unstable();
        assert_eq!(
            giving,
            [
                "$charIndices",
                "$chars",
                "$cloned",
                "$closing",
                "$copied",
                "$copiedGiven",
                "$cycle",
                "$cycleOf",
                "$decodeUtf16",
                "$enumerate",
                "$filterDropping",
                "$filterEnds",
                "$flatMap",
                "$flattenEnds",
                "$given",
                "$hashSetUnion",
                "$held",
                "$holding",
                "$intoIter",
                "$iterator",
                "$keysIn",
                "$lazyChain",
                "$lazyChunkBy",
                "$lazyChunkByMut",
                "$lazyMapWhile",
                "$lazyMatchIndicesBy",
                "$lazyMatchesBy",
                "$lazyRange",
                "$lazyRangeBack",
                "$lazyRsplitBy",
                "$lazyScan",
                "$lazySkipWhile",
                "$lazySliceSplitBy",
                "$lazySliceSplitByMut",
                "$lazySplitBy",
                "$lazySplitInclusiveBy",
                "$lazySplitTerminatorBy",
                "$lazyTakeWhile",
                "$lazyZip",
                "$lent",
                "$mapEnds",
                "$mapExtractIf",
                "$memchrIter",
                "$onceWith",
                "$recvItems",
                "$repeating",
                "$repeatingN",
                "$repeatingWith",
                "$rev",
                "$skipping",
                "$someItems",
                "$somes",
                "$splice",
                "$splicing",
                "$stdIterator",
                "$stepBy",
                "$stepsOf",
                "$successors",
                "$taking",
                "$treeSetAlgebra",
                "$trustedZip",
                "$trustedZipEnds",
                "$tryRecvItems",
                "$zipEnds",
            ]
        );
        assert!(gives_iterator("$lent") && !gives_iterator("$unwrap"));
    }

    #[test]
    fn a_mark_followed_by_no_declaration_is_refused() {
        let refused = std::panic::catch_unwind(|| declared_in("// gives: an iterator\nconst x = 1;\n").count());
        assert!(refused.is_err());
    }

    #[test]
    fn a_list_is_read_across_lines_brackets_and_strings() {
        use ParamKind::*;
        let names = |text: &'static str| parameters(text).iter().map(|p| (p.name, p.kind)).collect::<Vec<_>>();
        assert_eq!(
            names("(a, b = { c: 1, d: [2, 3] }, s = \"),\", ...rest) {"),
            [("a", Given), ("b", Defaulted), ("s", Defaulted), ("rest", Rest)]
        );
        assert_eq!(
            names("(\n  items,\n  index,\n) {"),
            [("items", Given), ("index", Given)]
        );
        assert_eq!(
            names("([aLow, aHigh], { type, value }) {"),
            [("[aLow, aHigh]", Given), ("{ type, value }", Given)]
        );
        assert_eq!(names("() {"), []);
    }
}
