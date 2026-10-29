//! Line-level edits, applied bottom-up so earlier line numbers stay valid.
//!
//! Inserted and replaced blocks are kept one blank line away from their neighbours, and
//! deletions collapse the blank lines left behind, so generated content never merges with
//! the user's paragraphs or leaves gaps.

#[derive(Debug)]
struct Op {
    start: usize,
    end: usize,
    lines: Vec<String>,
    pad: bool,
    /// For deletions: keep a blank line between the neighbours if both are text.
    separate: bool,
}

fn blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// An empty HTML comment: invisible, and it ends any block, including indented code.
pub(crate) const SEPARATOR: &str = "<!-- -->";

/// A batch of edits against one version of a document.
#[derive(Debug)]
pub(crate) struct Editor<'a> {
    lines: &'a [String],
    ops: Vec<Op>,
}

impl<'a> Editor<'a> {
    pub(crate) fn new(lines: &'a [String]) -> Self {
        Self {
            lines,
            ops: Vec::new(),
        }
    }

    fn push(&mut self, start: usize, end: usize, lines: Vec<String>, pad: bool, separate: bool) {
        self.ops.push(Op {
            start,
            end,
            lines,
            pad,
            separate,
        });
    }

    /// Replace lines `start..end`, keeping blank lines around the new block.
    pub(crate) fn replace(&mut self, start: usize, end: usize, lines: Vec<String>) {
        self.push(start, end, lines, true, false);
    }

    /// Replace lines `start..end` exactly, without padding.
    pub(crate) fn replace_exact(&mut self, start: usize, end: usize, lines: Vec<String>) {
        self.push(start, end, lines, false, false);
    }

    /// Insert a block before line `at`. Blocks inserted at the same line keep their order.
    pub(crate) fn insert(&mut self, at: usize, lines: Vec<String>) {
        if let Some(op) = self.ops.iter_mut().find(|op| op.start == at && op.end == at) {
            op.lines.push(String::new());
            op.lines.extend(lines);
            return;
        }
        self.push(at, at, lines, true, false);
    }

    /// Delete lines `start..end` that sit between blocks: collapse the blank lines left at the
    /// seam, and keep the neighbouring blocks apart.
    pub(crate) fn delete(&mut self, start: usize, end: usize) {
        self.push(start, end, Vec::new(), true, true);
    }

    /// Delete lines `start..end` from inside a block (raw HTML that swallowed them), leaving
    /// the block whole.
    pub(crate) fn delete_within(&mut self, start: usize, end: usize) {
        self.push(start, end, Vec::new(), true, false);
    }

    /// Apply every edit. Returns `None` when there were no edits.
    ///
    /// # Panics
    ///
    /// When two edits overlap, which is a bug in the transform that made them.
    pub(crate) fn apply(mut self) -> Option<Vec<String>> {
        if self.ops.is_empty() {
            return None;
        }
        self.ops.sort_by_key(|op| std::cmp::Reverse((op.start, op.end)));
        let mut out = self.lines.to_vec();
        let mut floor: Option<usize> = None;
        for op in self.ops {
            if let Some(floor) = floor {
                assert!(
                    op.end <= floor,
                    "overlapping edits at lines {}..{}",
                    op.start,
                    op.end
                );
            }
            let deleting = op.lines.is_empty();
            let mut new = op.lines;
            if op.pad && !new.is_empty() {
                if op.start > 0 && !blank(&out[op.start - 1]) {
                    new.insert(0, String::new());
                }
                if op.end < out.len() && !blank(&out[op.end]) {
                    new.push(String::new());
                }
            }
            out.splice(op.start..op.end, new);
            if op.pad && deleting {
                let s = op.start;
                if s < out.len() && blank(&out[s]) && (s == 0 || blank(&out[s - 1])) {
                    out.remove(s);
                } else if s == out.len() && s > 0 && blank(&out[s - 1]) {
                    out.remove(s - 1);
                } else if op.separate && s > 0 && s < out.len() && !blank(&out[s - 1]) && !blank(&out[s]) {
                    // The deleted lines kept two blocks apart; a blank line still must.
                    out.insert(s, String::new());
                }
            }
            floor = Some(op.start);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::Editor;

    fn owned(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| (*s).to_owned()).collect()
    }

    fn run(lines: &[&str], edit: impl FnOnce(&mut Editor<'_>)) -> Vec<String> {
        let lines = owned(lines);
        let mut editor = Editor::new(&lines);
        edit(&mut editor);
        editor.apply().unwrap_or(lines)
    }

    #[test]
    fn insert_pads_with_single_blank_lines() {
        assert_eq!(
            run(&["# T", "text"], |e| e.insert(1, owned(&["b"]))),
            ["# T", "", "b", "", "text"]
        );
        assert_eq!(
            run(&["# T", "", "text"], |e| e.insert(2, owned(&["b"]))),
            ["# T", "", "b", "", "text"]
        );
    }

    #[test]
    fn delete_collapses_the_seam() {
        assert_eq!(run(&["a", "", "x", "", "b"], |e| e.delete(2, 3)), ["a", "", "b"]);
        assert_eq!(
            run(&["x", "", "a", "", "y"], |e| {
                e.delete(0, 1);
                e.delete(4, 5);
            }),
            ["a"]
        );
    }

    #[test]
    fn delete_between_two_blocks_keeps_them_apart() {
        assert_eq!(run(&["a", "x", "b"], |e| e.delete(1, 2)), ["a", "", "b"]);
    }

    #[test]
    fn inserts_at_one_line_keep_their_order() {
        let out = run(&["text"], |e| {
            e.insert(0, owned(&["first"]));
            e.insert(0, owned(&["second"]));
        });
        assert_eq!(out, ["first", "", "second", "", "text"]);
    }

    #[test]
    fn replace_and_insert_at_the_same_line() {
        let out = run(&["a", "old", "b"], |e| {
            e.replace(1, 2, owned(&["new"]));
            e.insert(1, owned(&["before"]));
        });
        assert_eq!(out, ["a", "", "before", "", "new", "", "b"]);
    }

    #[test]
    #[should_panic(expected = "overlapping edits")]
    fn overlapping_edits_panic() {
        run(&["a", "b", "c"], |e| {
            e.replace(0, 2, owned(&["x"]));
            e.replace(1, 3, owned(&["y"]));
        });
    }
}
