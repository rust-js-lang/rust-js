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
    splits: &mut IndexMap<u32, Vec<(usize, &'a [String])>>,
) {
    for (lo, hi, operator, texts) in arms {
        // The branch's first JS the map places, and back from it, on its
        // line, to the operator it's after, `?`, `:`, `+` or `&&`: an
        // arrow's `(` isn't placed.
        let first = (map.get_tokens())
            .filter(|t| offset(t.get_src_line(), t.get_src_col()).is_some_and(|at| *lo <= at && at < *hi))
            .map(|t| (t.get_dst_line(), MapCol(t.get_dst_col())))
            .min();
        let Some((line, column)) = first else { continue };
        // On its line, or one before it, an arrow's body's first JS
        // being the first placed.
        let found = (0..=line).rev().take(3).find_map(|at| {
            let written = generated_lines.get(at as usize)?;
            let column = column.byte_in(written);
            // An `if`'s test's: its `if (` too, where the `if` is placed
            // with its test.
            let opening = match *operator {
                "if (" if written.get(column..).is_some_and(|rest| rest.starts_with(*operator)) => operator.len(),
                _ => 0,
            };
            let before = if at == line {
                written.get(..column + opening)?
            } else {
                written
            };
            before
                .rfind(operator)
                .map(|found| (at, written, found + operator.len()))
        });
        if let Some((line, written, after)) = found {
            let start = after + written[after..].len() - written[after..].trim_start().len();
            splits.entry(line).or_default().push((start, texts.as_slice()));
        }
    }
}

/// The JS's lines and what's placed in them: where each line is in `code`,
/// which the lines are written to, after the header's `header_lines`; where
/// each split line's pieces are, each from its byte, on its line; and where
/// each line has a comment inside it, the piece it's in, where, and how
/// wide, what the JS after it there is moved by (ADR 0630, amended). Each
/// place in a line is its byte, each width the map's units: the map's
/// columns are read and written only by `Laid::to`, through the line's
/// text (#1668).
pub(super) struct Laid<'a> {
    generated_lines: &'a [&'a str],
    lines: Vec<u32>,
    pieces: IndexMap<u32, Vec<(usize, u32)>>,
    widened: IndexMap<u32, Vec<(usize, usize, MapCol)>>,
}

/// `generated`'s lines written to `code`, each comment of `trailing` and
/// each split of `splits` placed in them.
pub(super) fn laid<'a>(
    generated: &oxc_codegen::CodegenReturn,
    trailing: &[Trailing],
    generated_lines: &'a [&'a str],
    splits: &IndexMap<u32, Vec<(usize, &[String])>>,
    header_lines: u32,
    code: &mut String,
) -> Laid<'a> {
    let mut lines: Vec<u32> = Vec::new();
    // Where each comment after code on its line goes in the JS's lines.
    let trailing::Placed { after, inside, ends } = match &generated.map {
        Some(map) => trailing::placed(trailing, map, generated_lines),
        None => trailing::Placed::default(),
    };
    // Where each split line's pieces are: each from its column, on its line.
    let mut pieces: IndexMap<u32, Vec<(usize, u32)>> = IndexMap::default();
    // Where each line has a comment inside it: the piece it's in, where, and
    // how wide, what the JS after it there is moved by (ADR 0630, amended).
    let mut widened: IndexMap<u32, Vec<(usize, usize, MapCol)>> = IndexMap::default();
    let mut added = 0;
    for (i, line) in generated.code.lines().enumerate() {
        let i = i as u32;
        let out_line = header_lines + i + added;
        let mut at = 0;
        let mut placed = vec![(0, out_line)];
        let inside_of = |from: usize, to: usize| {
            let comments: Vec<(usize, Inside, &str)> = (inside.get(&i).into_iter().flatten())
                .filter(|&&(column, ..)| column >= from && column < to)
                .map(|&(column, inside, text)| (column - from, inside, text))
                .collect();
            comments
        };
        // Where it's split: before a branch's comments, on lines of their
        // own, or after a line comment, which ends its line.
        let mut cuts: Vec<(usize, &[String], Option<&str>)> = (splits.get(&i).into_iter().flatten())
            .map(|&(column, texts)| (column, texts, None))
            .chain((ends.get(&i).into_iter().flatten()).map(|&(column, text)| (column, &[][..], Some(text))))
            .collect();
        cuts.sort_by_key(|&(column, ..)| column);
        for (column, texts, end) in cuts {
            let Some(head) = line.get(at..column).filter(|_| column > at) else {
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
            let ends_line = line[column..].trim().is_empty();
            if !ends_line {
                code.push('\n');
            }
            added += u32::from(!ends_line) + texts.len() as u32;
            at = column;
            placed.push((column, header_lines + i + added));
        }
        let (rest, moved) = with_inside_comments(&line[at..], &inside_of(at, usize::MAX));
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
    Laid {
        generated_lines,
        lines,
        pieces,
        widened,
    }
}

impl Laid<'_> {
    /// Where the map's `column` of the JS's `line` is in the module's code:
    /// its byte in the line, the piece of it that's there, and the map's
    /// units before it in that piece, the comments inside it before it
    /// added.
    pub(super) fn to(&self, line: u32, column: MapCol) -> (u32, MapCol) {
        let Laid {
            generated_lines,
            lines,
            pieces,
            widened,
        } = self;
        let text = generated_lines.get(line as usize).copied().unwrap_or_default();
        let at = column.byte_in(text);
        let (from, to) = match pieces.get(&line) {
            Some(placed) => *placed
                .iter()
                .rev()
                .find(|&&(from, _)| at >= from)
                .expect("the first is at 0"),
            None => {
                let last_shift = lines.last().map_or(0, |&last| last + 1 - lines.len() as u32);
                (0, lines.get(line as usize).copied().unwrap_or(line + last_shift))
            }
        };
        // Moved by the comments inside its piece of its line before it.
        let wider: u32 = (widened.get(&line).into_iter().flatten())
            .filter(|&&(piece, inside, _)| piece == from && inside <= at)
            .map(|&(.., width)| width.0)
            .sum();
        (to, MapCol(column.0 - MapCol::of_byte(text, from).0 + wider))
    }
}
