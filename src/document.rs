//! Borrowed document algebra and display-column-aware group rendering.
use unicode_width::UnicodeWidthStr;

pub(crate) enum Doc<'a> {
    Text(&'a str),
    Soft(&'static str),
    Hard,
    Indent(Box<Doc<'a>>),
    Concat(Vec<Doc<'a>>),
    Group(Group<'a>),
    Fill {
        first: Box<Doc<'a>>,
        rest: Vec<Doc<'a>>,
    },
}

// Constructed only by Doc::group: the document and cached measurement always
// travel together, and neither can be independently changed by a caller.
pub(crate) struct Group<'a> {
    inner: Box<Doc<'a>>,
    flat_width: Option<usize>,
    following_width: usize,
}

impl<'a> Doc<'a> {
    pub(crate) fn concat(parts: Vec<Self>) -> Self {
        Self::Concat(parts)
    }
    pub(crate) fn indent(self) -> Self {
        Self::Indent(Box::new(self))
    }
    pub(crate) fn group(self) -> Self {
        let flat_width = self.flat_width();
        Self::Group(Group {
            inner: Box::new(self),
            flat_width,
            following_width: 0,
        })
    }
    /// Attach a suffix while reserving its columns in every nested fit decision.
    /// Accepts any document; callers need not know its grouping representation.
    pub(crate) fn followed_by(self, suffix: Self) -> Self {
        let mut head = match self {
            Self::Group(_) => self,
            other => other.group(),
        };
        if let Self::Group(group) = &mut head {
            group.following_width = suffix.flat_width().unwrap_or(0);
        }
        Self::concat(vec![head, suffix]).group()
    }

    /// Contents own list breaks and fit after the opening has been rendered.
    /// A compound opening (such as generics plus `(`) can break independently;
    /// item groups still fit within the contents' group.
    pub(crate) fn enclosed(
        opening: Self,
        leading: Self,
        content: Self,
        trailing: Self,
        close: &'a str,
    ) -> Self {
        let content = match content {
            Self::Group(group) => *group.inner,
            other => other,
        };
        Self::concat(vec![
            opening,
            Self::concat(vec![
                Self::concat(vec![leading, content]).indent(),
                trailing,
                Self::Text(close),
            ])
            .group(),
        ])
    }
    pub(crate) fn fill(first: Self, rest: Vec<Self>) -> Self {
        Self::Fill {
            first: Box::new(first),
            rest,
        }
    }

    fn flat_width(&self) -> Option<usize> {
        match self {
            Self::Text(s) if s.contains(['\n', '\r']) => None,
            Self::Text(s) => Some(s.width()),
            Self::Soft(s) => Some(s.width()),
            Self::Hard => None,
            Self::Indent(d) => d.flat_width(),
            Self::Group(group) => group.flat_width,
            Self::Concat(parts) => parts
                .iter()
                .try_fold(0usize, |n, d| n.checked_add(d.flat_width()?)),
            Self::Fill { first, rest } => rest
                .iter()
                .try_fold(first.flat_width()?, |n, d| n.checked_add(d.flat_width()?)),
        }
    }
}

pub(crate) fn render(doc: &Doc<'_>, indent_width: usize, width: usize, ending: &str) -> String {
    struct Renderer<'a> {
        output: String,
        column: usize,
        pending_indent: bool,
        indent_width: usize,
        width: usize,
        ending: &'a str,
    }
    impl Renderer<'_> {
        fn write(&mut self, doc: &Doc<'_>, depth: usize, flat: bool) {
            match doc {
                Doc::Text(text) => {
                    if text.is_empty() {
                        return;
                    }
                    if self.pending_indent {
                        self.column = depth * self.indent_width;
                        self.output.extend(std::iter::repeat_n(' ', self.column));
                        self.pending_indent = false;
                    }
                    self.output.push_str(text);
                    if let Some((_, last)) = text.rsplit_once('\n') {
                        self.column = last.width();
                    } else {
                        self.column += text.width();
                    }
                }
                Doc::Soft(text) if flat => self.write(&Doc::Text(text), depth, flat),
                Doc::Soft(_) | Doc::Hard => {
                    // A CRLF line comment owns its CR in the raw token tape.
                    self.output.push_str(if self.output.ends_with('\r') {
                        "\n"
                    } else {
                        self.ending
                    });
                    self.column = 0;
                    self.pending_indent = true;
                }
                Doc::Indent(inner) => self.write(inner, depth + 1, flat),
                Doc::Concat(parts) => {
                    for part in parts {
                        self.write(part, depth, flat);
                    }
                }
                Doc::Group(group) => {
                    let column = if self.pending_indent {
                        depth * self.indent_width
                    } else {
                        self.column
                    };
                    let fits = group
                        .flat_width
                        .and_then(|n| n.checked_add(group.following_width))
                        .is_some_and(|n| n <= self.width.saturating_sub(column));
                    // Nested fill groups must also leave room for an attached
                    // suffix, such as a control-flow header's ` do`.
                    let width = self.width;
                    self.width = width.saturating_sub(group.following_width);
                    self.write(&group.inner, depth, flat || fits);
                    self.width = width;
                }
                Doc::Fill { first, rest } => {
                    self.write(first, depth, flat);
                    for chunk in rest {
                        let column = if self.pending_indent {
                            (depth + 1) * self.indent_width
                        } else {
                            self.column
                        };
                        let fits = chunk
                            .flat_width()
                            .is_some_and(|n| n <= self.width.saturating_sub(column));
                        self.write(chunk, depth + 1, flat || fits);
                    }
                }
            }
        }
    }
    let mut renderer = Renderer {
        output: String::new(),
        column: 0,
        pending_indent: true,
        indent_width,
        width,
        ending,
    };
    renderer.write(doc, 0, false);
    renderer.output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_suffix_reserves_columns_for_an_ungrouped_fill() {
        let head = Doc::fill(
            Doc::Text("if first_argument +"),
            vec![Doc::concat(vec![Doc::Soft(" "), Doc::Text("22")]).group()],
        );
        let suffix = Doc::Text(" do");
        let doc = head.followed_by(suffix);
        assert_eq!(render(&doc, 2, 24, "\n"), "if first_argument +\n  22 do");
        assert_eq!(render(&doc, 2, 80, "\n"), "if first_argument + 22 do");
    }
}
