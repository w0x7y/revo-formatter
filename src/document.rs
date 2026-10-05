//! Borrowed document algebra and display-column-aware group rendering.
use unicode_width::UnicodeWidthStr;

pub(crate) enum Doc<'a> {
    Text(&'a str),
    Soft(&'static str),
    Hard,
    Indent(Box<Doc<'a>>),
    Concat(Vec<Doc<'a>>),
    Group(Box<Doc<'a>>),
}

impl<'a> Doc<'a> {
    pub(crate) fn concat(parts: Vec<Self>) -> Self {
        Self::Concat(parts)
    }
    pub(crate) fn indent(self) -> Self {
        Self::Indent(Box::new(self))
    }
    pub(crate) fn group(self) -> Self {
        Self::Group(Box::new(self))
    }

    fn flat_width(&self) -> Option<usize> {
        match self {
            Self::Text(s) if s.contains(['\n', '\r']) => None,
            Self::Text(s) => Some(s.width()),
            Self::Soft(s) => Some(s.width()),
            Self::Hard => None,
            Self::Indent(d) | Self::Group(d) => d.flat_width(),
            Self::Concat(parts) => parts
                .iter()
                .try_fold(0usize, |n, d| n.checked_add(d.flat_width()?)),
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
                Doc::Group(inner) => {
                    let column = if self.pending_indent {
                        depth * self.indent_width
                    } else {
                        self.column
                    };
                    let fits = inner
                        .flat_width()
                        .is_some_and(|n| n <= self.width.saturating_sub(column));
                    self.write(inner, depth, flat || fits);
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
