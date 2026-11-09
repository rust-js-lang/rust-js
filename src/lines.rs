//! Where each line of a text starts, found once: a line asked of an offset
//! is a search, not a count of each newline before it, which each comment
//! of a crate's modules did of the crate's whole text (#882).

/// The offset each line of a text starts at, the first `0`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LineStarts(Vec<u32>);

impl LineStarts {
    pub(crate) fn of(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            (text.bytes().enumerate())
                .filter(|&(_, byte)| byte == b'\n')
                .map(|(i, _)| i as u32 + 1),
        );
        LineStarts(starts)
    }

    /// How many lines the text has: its newlines and one.
    pub(crate) fn count(&self) -> usize {
        self.0.len()
    }

    /// Where line `line`, from 0, starts, if the text has it.
    pub(crate) fn start(&self, line: usize) -> Option<u32> {
        self.0.get(line).copied()
    }

    /// The line `offset` is on, from 0: how many newlines are before it,
    /// `text[..offset].matches('\n').count()`.
    pub(crate) fn line(&self, offset: usize) -> u32 {
        (self.0.partition_point(|&start| start as usize <= offset) - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::LineStarts;

    #[test]
    fn a_line_is_how_many_newlines_are_before_it() {
        for text in ["", "a", "\n", "ab\ncd\n\nef", "\n\nx\n"] {
            let lines = LineStarts::of(text);
            for offset in 0..=text.len() {
                assert_eq!(
                    lines.line(offset) as usize,
                    text[..offset].matches('\n').count(),
                    "{text:?} at {offset}"
                );
            }
        }
    }
}
