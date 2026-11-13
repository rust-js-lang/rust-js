//! Where comments go in the JS's lines, which oxc's printer writes none
//! of (ADRs 0444, 0630, 0658, 0708): a line split where a branch's comments
//! go before its value, each comment after code placed in its line, and
//! where each of the map's places is moved to. Split from to_oxc.rs by
//! concept, as a file is at most 1500 lines.

use super::*;

/// Each conditional's branch's comments, before its value on lines of
/// their own, `? // In modern browsers ..`, as oxc writes none before the
/// first (ADR 0658): its line split where its value's JS starts.
pub(super) fn arm_splits<'a>(
    arms: &'a ArmComments,
    map: &SourceMap,
    generated_lines: &[&str],
    offset: &dyn Fn(u32, u32) -> Option<u32>,
    splits: &mut IndexMap<u32, Vec<(u32, &'a [String])>>,
) {
    for (lo, hi, operator, texts) in arms {
        // The branch's first JS the map places, and back from it, on its
        // line, to the operator it's after, `?`, `:`, `+` or `&&`: an
        // arrow's `(` isn't placed.
        let first = (map.get_tokens())
            .filter(|t| offset(t.get_src_line(), t.get_src_col()).is_some_and(|at| *lo <= at && at < *hi))
            .map(|t| (t.get_dst_line(), t.get_dst_col()))
            .min();
        let Some((line, column)) = first else { continue };
        // On its line, or one before it, an arrow's body's first JS
        // being the first placed.
        let found = (0..=line).rev().take(3).find_map(|at| {
            let written = generated_lines.get(at as usize)?;
            // An `if`'s test's: its `if (` too, where the `if` is placed
            // with its test.
            let opening = match *operator {
                "if ("
                    if written
                        .get(column as usize..)
                        .is_some_and(|rest| rest.starts_with(*operator)) =>
                {
                    operator.len()
                }
                _ => 0,
            };
            let before = if at == line {
                written.get(..column as usize + opening)?
            } else {
                written
            };
            before
                .rfind(operator)
                .map(|found| (at, written, found + operator.len()))
        });
        if let Some((line, written, after)) = found {
            let start = after + written[after..].len() - written[after..].trim_start().len();
            splits.entry(line).or_default().push((start as u32, texts.as_slice()));
        }
    }
}

/// The JS's lines and what's placed in them: where each line is in `code`,
/// which the lines are written to, after the header's `header_lines`; where
/// each split line's pieces are, each from its column, on its line; and
/// where each line has a comment inside it, the piece it's in, where, and
/// how wide, what the JS after it there is moved by (ADR 0630, amended).
pub(super) struct Laid {
    lines: Vec<u32>,
    pieces: IndexMap<u32, Vec<(u32, u32)>>,
    widened: IndexMap<u32, Vec<(u32, u32, u32)>>,
}

/// `generated`'s lines written to `code`, each comment of `trailing` and
/// each split of `splits` placed in them.
pub(super) fn laid(
    generated: &oxc_codegen::CodegenReturn,
    trailing: &[Trailing],
    generated_lines: &[&str],
    splits: &IndexMap<u32, Vec<(u32, &[String])>>,
    header_lines: u32,
    code: &mut String,
) -> Laid {
    let mut lines: Vec<u32> = Vec::new();
    // Where each comment after code on its line goes in the JS's lines.
    let trailing::Placed { after, inside, ends } = match &generated.map {
        Some(map) => trailing::placed(trailing, map, generated_lines),
        None => trailing::Placed::default(),
    };
    // Where each split line's pieces are: each from its column, on its line.
    let mut pieces: IndexMap<u32, Vec<(u32, u32)>> = IndexMap::default();
    // Where each line has a comment inside it: the piece it's in, where, and
    // how wide, what the JS after it there is moved by (ADR 0630, amended).
    let mut widened: IndexMap<u32, Vec<(u32, u32, u32)>> = IndexMap::default();
    let mut added = 0;
    for (i, line) in generated.code.lines().enumerate() {
        let i = i as u32;
        let out_line = header_lines + i + added;
        let mut at = 0;
        let mut placed = vec![(0, out_line)];
        let inside_of = |from: u32, to: u32| {
            let comments: Vec<(u32, Inside, &str)> = (inside.get(&i).into_iter().flatten())
                .filter(|&&(column, ..)| column >= from && column < to)
                .map(|&(column, inside, text)| (column - from, inside, text))
                .collect();
            comments
        };
        // Where it's split: before a branch's comments, on lines of their
        // own, or after a line comment, which ends its line.
        let mut cuts: Vec<(u32, &[String], Option<&str>)> = (splits.get(&i).into_iter().flatten())
            .map(|&(column, texts)| (column, texts, None))
            .chain((ends.get(&i).into_iter().flatten()).map(|&(column, text)| (column, &[][..], Some(text))))
            .collect();
        cuts.sort_by_key(|&(column, ..)| column);
        for (column, texts, end) in cuts {
            let Some(head) = line.get(at as usize..column as usize).filter(|_| column > at) else {
                continue;
            };
            // What's first on its line, an object's `}`: its comments the
            // lines before it, no line of nothing (ADR 0708, amended).
            if head.trim().is_empty() && end.is_none() {
                added += closing::lines_of(texts, code);
                at = column;
                placed.push((column, header_lines + i + added));
                continue;
            }
            let (head, moved) = with_inside_comments(head.trim_end(), &inside_of(at, column));
            code.push_str(&head);
            widened
                .entry(i)
                .or_default()
                .extend(moved.into_iter().map(|(end, width)| (at, at + end, width)));
            if let Some(end) = end {
                code.push(' ');
                code.push_str(end);
            }
            for text in texts {
                code.push('\n');
                code.push_str(text);
            }
            // At its line's end, an array's item's `,` on the line before
            // it: the comments its last lines, no line of nothing after.
            let ends_line = line[column as usize..].trim().is_empty();
            if !ends_line {
                code.push('\n');
            }
            added += u32::from(!ends_line) + texts.len() as u32;
            at = column;
            placed.push((column, header_lines + i + added));
        }
        let (rest, moved) = with_inside_comments(&line[at as usize..], &inside_of(at, u32::MAX));
        code.push_str(&rest);
        widened
            .entry(i)
            .or_default()
            .extend(moved.into_iter().map(|(end, width)| (at, at + end, width)));
        for comment in after.get(&i).into_iter().flatten() {
            code.push(' ');
            code.push_str(comment);
        }
        code.push('\n');
        lines.push(out_line);
        if placed.len() > 1 {
            pieces.insert(i, placed);
        }
    }
    Laid { lines, pieces, widened }
}

impl Laid {
    /// Where the map's `column` of the JS's `line` is in the module's code.
    pub(super) fn to(&self, line: u32, column: u32) -> (u32, u32) {
        let Laid { lines, pieces, widened } = self;
        let last_shift = lines.last().map_or(0, |&last| last + 1 - lines.len() as u32);
        // Moved by the comments inside its piece of its line before it.
        let wider = |line: u32, from: u32, column: u32| -> u32 {
            (widened.get(&line).into_iter().flatten())
                .filter(|&&(piece, at, _)| piece == from && at <= column)
                .map(|&(.., width)| width)
                .sum()
        };
        let to = |line: u32, column: u32| match pieces.get(&line) {
            Some(placed) => {
                let &(from, to) = placed
                    .iter()
                    .rev()
                    .find(|&&(from, _)| column >= from)
                    .expect("the first is at 0");
                (to, column - from + wider(line, from, column))
            }
            None => (
                lines.get(line as usize).copied().unwrap_or(line + last_shift),
                column + wider(line, 0, column),
            ),
        };
        to(line, column)
    }
}
