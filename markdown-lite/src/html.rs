use smallvec::SmallVec;

use crate::tokenizer::{MarkdownToken, MarkdownTokenizer};

pub trait Options {
    #[cfg(feature = "frontmatter")]
    fn frontmatter(&mut self, frontmatter: serde_value::Value, out: &mut String) {}
    fn inline_custom(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        false
    }
    fn custom_block_start(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        false
    }
    fn custom_block_end(&mut self, label: &str, out: &mut String) -> bool {
        false
    }
    fn inline_math(&mut self, math: &str, out: &mut String) -> bool {
        false
    }
    fn block_math(&mut self, math: &str, out: &mut String) -> bool {
        false
    }
}

#[must_use]
pub fn to_html(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    HTMLConverter {
        tokenizer: MarkdownTokenizer::new(source),
        states: SmallVec::new(),
        out: &mut out,
        options: None,
    }
    .run();
    out
}

pub fn to_html_with_options<'s>(source: &'s str, options: &'s mut dyn Options) -> String {
    let mut out = String::with_capacity(source.len());
    HTMLConverter {
        tokenizer: MarkdownTokenizer::new(source),
        states: SmallVec::new(),
        out: &mut out,
        options: Some(options),
    }
    .run();
    out
}

pub fn inline_html(md: &str, out: &mut String) {
    HTMLConverter {
        tokenizer: MarkdownTokenizer::new_inline(md),
        states: SmallVec::new(),
        out,
        options: None,
    }
    .run();
}

pub fn inline_html_with_options(md: &str, out: &mut String, options: &mut dyn Options) {
    HTMLConverter {
        tokenizer: MarkdownTokenizer::new_inline(md),
        states: SmallVec::new(),
        out,
        options: Some(options),
    }
    .run();
}

struct HTMLConverter<'a> {
    tokenizer: MarkdownTokenizer<'a, ()>,
    states: SmallVec<ConverterState<'a>, 4>,
    out: &'a mut String,
    options: Option<&'a mut dyn Options>,
}

impl HTMLConverter<'_> {
    #[allow(clippy::too_many_lines)]
    fn run(mut self) {
        use std::fmt::Write;
        macro_rules! close {
            (inline) => {
                while let Some(s) = self.states.last().copied() {
                    match s {
                        ConverterState::Emph(_)
                        | ConverterState::Strong(_)
                        | ConverterState::Strikethrough
                        | ConverterState::Highlight
                        | ConverterState::Subscript
                        | ConverterState::Superscript
                        | ConverterState::Heading(_)
                        | ConverterState::InLink
                        /*| ConverterState::ListItem*/ => {
                            self.states.pop();
                            Self::close(&mut self.out, s,&mut self.options);
                        }
                        _ => break,
                    }
                }
            };
            () => {
                while let Some(s) = self.states.pop() {
                    Self::close(&mut self.out, s,&mut self.options);
                    if s == ConverterState::Paragraph {
                        break
                    }
                }
            };
            (force) => {
                while let Some(s) = self.states.pop() {
                    Self::close(&mut self.out, s,&mut self.options);
                }
            };
        }
        for t in self.tokenizer {
            match t {
                MarkdownToken::FrontMatter(s, _) => {
                    #[cfg(feature = "frontmatter")]
                    {
                        if let Some(fm) = &mut self.options {
                            match serde_yml::from_str::<'_, serde_value::Value>(s) {
                                Ok(v) => fm.frontmatter(v, self.out),
                                Err(e) => {
                                    let _ = write!(
                                        self.out,
                                        "<pre style=\"border:1px dotted gray;background-color:lightgray\">{e:#?}</pre>"
                                    );
                                }
                            }
                        }
                    }
                    #[cfg(not(feature = "frontmatter"))]
                    let _ = write!(
                        self.out,
                        "<pre style=\"border:1px dotted gray;background-color:lightgray\">{s}</pre>"
                    );
                }
                MarkdownToken::InlineCustom { label, args, .. } => {
                    if let Some(opt) = &mut self.options
                        && opt.inline_custom(label, args, self.out)
                    {
                    } else {
                        let _ = write!(
                            self.out,
                            "@[{}]({})",
                            html_escape::encode_safe(label),
                            html_escape::encode_safe(args)
                        );
                    }
                }
                MarkdownToken::CustomBlockStart {
                    label,
                    args,
                    colons,
                    range,
                } => {
                    close!();
                    self.states.push(ConverterState::CustomBlock(colons, label));
                    if let Some(opt) = &mut self.options
                        && opt.custom_block_start(label, args, self.out)
                    {
                    } else {
                        for _ in 0..colons as usize {
                            self.out.push(':');
                        }
                        self.out.push(' ');
                        self.out.push_str(label);
                        self.out.push(' ');
                        html_escape::encode_safe_to_string(args, self.out);
                        self.out.push('\n');
                    }
                }
                MarkdownToken::CustomBlockEnd(_, ()) => {
                    while let Some(s) = self.states.pop() {
                        Self::close(self.out, s, &mut self.options);
                        if matches!(s, ConverterState::CustomBlock(_, _)) {
                            break;
                        }
                    }
                }
                MarkdownToken::ParagraphStart(()) => {
                    self.states.push(ConverterState::Paragraph);
                    self.out.push_str("<p>");
                }
                MarkdownToken::ParagraphEnd(()) => {
                    close!();
                    self.out.push_str("</p>");
                }
                MarkdownToken::Text(t, _) => {
                    // TODO: HTML escaping
                    let _ = self.out.write_str(&html_escape::encode_safe(t));
                }
                MarkdownToken::Emph(u, ())
                    if self.states.last() == Some(&ConverterState::Emph(u)) =>
                {
                    self.states.pop();
                    self.out.push_str("</em>");
                }
                MarkdownToken::Emph(u, ()) => {
                    self.states.push(ConverterState::Emph(u));
                    self.out.push_str("<em>");
                }
                MarkdownToken::Strong(u, ())
                    if self.states.last() == Some(&ConverterState::Strong(u)) =>
                {
                    self.states.pop();
                    self.out.push_str("</strong>");
                }
                MarkdownToken::Strong(u, ()) => {
                    self.states.push(ConverterState::Strong(u));
                    self.out.push_str("<strong>");
                }
                MarkdownToken::StrikeThrough(())
                    if self.states.last() == Some(&ConverterState::Strikethrough) =>
                {
                    self.states.pop();
                    self.out.push_str("</s>");
                }
                MarkdownToken::StrikeThrough(()) => {
                    self.states.push(ConverterState::Strikethrough);
                    self.out.push_str("<s>");
                }

                MarkdownToken::Highlight(())
                    if self.states.last() == Some(&ConverterState::Highlight) =>
                {
                    self.states.pop();
                    self.out.push_str("</mark>");
                }
                MarkdownToken::Highlight(()) => {
                    self.states.push(ConverterState::Highlight);
                    self.out.push_str("<mark>");
                }
                MarkdownToken::Superscript(())
                    if self.states.last() == Some(&ConverterState::Superscript) =>
                {
                    self.states.pop();
                    self.out.push_str("</sup>");
                }
                MarkdownToken::Superscript(()) => {
                    self.states.push(ConverterState::Superscript);
                    self.out.push_str("<sup>");
                }
                MarkdownToken::Subscript(())
                    if self.states.last() == Some(&ConverterState::Subscript) =>
                {
                    self.states.pop();
                    self.out.push_str("</sub>");
                }
                MarkdownToken::Subscript(()) => {
                    self.states.push(ConverterState::Subscript);
                    self.out.push_str("<sub>");
                }
                MarkdownToken::InlineMath(s, _) => {
                    close!(inline);
                    if let Some(opts) = self.options.as_mut()
                        && opts.inline_math(s, self.out)
                    {
                    } else {
                        let _ = write!(
                            self.out,
                            "<span style=\"font-family:monospace;background-color:crimson\">{s}</span>"
                        );
                    }
                }
                MarkdownToken::BlockMath(s, _) => {
                    close!();
                    if let Some(opts) = self.options.as_mut()
                        && opts.block_math(s, self.out)
                    {
                    } else {
                        let _ = write!(
                            self.out,
                            "<div style=\"width:100%\"><div style=\"font-family:monospace;margin-left:auto;margin-right:auto;background-color:crimson;width:fit-content;\">{s}</div>"
                        );
                    }
                }
                MarkdownToken::InlineCode(s, _) => {
                    close!(inline);
                    let _ = write!(
                        self.out,
                        "<span style=\"font-family:monospace;background-color:lightgray\">{}</span>",
                        html_escape::encode_safe(s)
                    );
                }
                MarkdownToken::Heading(lvl, _) => {
                    close!();
                    self.states.push(ConverterState::Heading(lvl));
                    let _ = write!(self.out, "<{}>", heading(lvl));
                }
                MarkdownToken::HeadingEnd(()) => {
                    close!();
                }
                MarkdownToken::ThematicBreak(_) => {
                    close!();
                    let _ = self.out.write_str("<hr/>");
                }
                MarkdownToken::BlockCode { code, .. } => {
                    close!();
                    let _ = write!(
                        self.out,
                        "<pre style=\"background-color:lightgray\">{}</pre>",
                        html_escape::encode_safe(code)
                    );
                }
                MarkdownToken::LinkStart(url, ()) => {
                    let _ = write!(self.out, "<a href=\"{url}\">");
                    self.states.push(ConverterState::InLink);
                }
                MarkdownToken::Escaped(c, ()) => {
                    let _ = self
                        .out
                        .write_str(&html_escape::encode_safe(&c.to_string()));
                }
                MarkdownToken::LinkOrCustomEnd(()) => {
                    while let Some(s) = self.states.pop() {
                        Self::close(self.out, s, &mut self.options);
                        if s == ConverterState::InLink {
                            break;
                        }
                    }
                }
                MarkdownToken::UnnumberedListItem(indent, ()) => {
                    close!(inline);
                    match self.states.last() {
                        Some(ConverterState::UnorderedList(i) | ConverterState::OrderedList(i))
                            if *i < indent + 2 && indent < *i + 2 =>
                        {
                            let _ = self.out.write_str("</li><li>");
                        }
                        Some(ConverterState::UnorderedList(i) | ConverterState::OrderedList(i))
                            if *i < indent =>
                        {
                            self.states.push(ConverterState::UnorderedList(indent));
                            let _ = self.out.write_str("<ul><li>");
                        }
                        Some(ConverterState::UnorderedList(_))
                            if matches!(
                                self.states.get(self.states.len() - 2),
                                Some(ConverterState::UnorderedList(_))
                            ) =>
                        {
                            let _ = self.states.pop();
                            let _ = self.out.write_str("</ul><li>");
                        }
                        Some(ConverterState::OrderedList(_))
                            if matches!(
                                self.states.get(self.states.len() - 2),
                                Some(ConverterState::UnorderedList(_))
                            ) =>
                        {
                            let _ = self.states.pop();
                            let _ = self.out.write_str("</ol><li>");
                        }
                        _ => {
                            self.states.push(ConverterState::UnorderedList(indent));
                            let _ = self.out.write_str("<ul><li>");
                        }
                    }
                }
                MarkdownToken::NumberedListItem(indent, ()) => {
                    close!(inline);
                    match self.states.last() {
                        Some(ConverterState::UnorderedList(i) | ConverterState::OrderedList(i))
                            if *i < indent + 2 && indent < *i + 2 =>
                        {
                            let _ = self.out.write_str("</li><li>");
                        }
                        Some(ConverterState::UnorderedList(i) | ConverterState::OrderedList(i))
                            if *i < indent =>
                        {
                            self.states.push(ConverterState::OrderedList(indent));
                            let _ = self.out.write_str("<ol><li>");
                        }
                        Some(ConverterState::UnorderedList(_))
                            if matches!(
                                self.states.get(self.states.len() - 2),
                                Some(ConverterState::OrderedList(_))
                            ) =>
                        {
                            let _ = self.states.pop();
                            let _ = self.out.write_str("</ul><li>");
                        }
                        Some(ConverterState::OrderedList(_))
                            if matches!(
                                self.states.get(self.states.len() - 2),
                                Some(ConverterState::OrderedList(_))
                            ) =>
                        {
                            let _ = self.states.pop();
                            let _ = self.out.write_str("</ol><li>");
                        }
                        _ => {
                            self.states.push(ConverterState::OrderedList(indent));
                            let _ = self.out.write_str("<ol><li>");
                        }
                    }
                }
                o => {
                    let _ = write!(self.out, "<pre>TODO: {o:#?}</pre>");
                }
            }
        }
        close!(force);
    }

    fn close(out: &mut String, state: ConverterState, options: &mut Option<&mut dyn Options>) {
        use std::fmt::Write;
        match state {
            ConverterState::Emph(_) => out.push_str("</em>"),
            ConverterState::Strong(_) => out.push_str("</strong>"),
            ConverterState::Paragraph => out.push_str("</p>"),
            ConverterState::Strikethrough => out.push_str("</s>"),
            ConverterState::Highlight => out.push_str("</mark>"),
            ConverterState::Superscript => out.push_str("</sup>"),
            ConverterState::Subscript => out.push_str("</sub>"),
            ConverterState::Heading(lvl) => {
                let _ = write!(out, "</{}>", heading(lvl));
            }
            ConverterState::UnorderedList(_) => out.push_str("</li></ul>"),
            ConverterState::OrderedList(_) => out.push_str("</li></ol>"),
            ConverterState::InLink => out.push_str("</a>"),
            ConverterState::CustomBlock(colons, label) => {
                if let Some(opt) = options
                    && opt.custom_block_end(label, out)
                {
                } else {
                    for _ in 0..colons as usize {
                        out.push(':');
                    }
                    out.push('\n');
                }
            } //ConverterState::ListItem => out.push_str("</li>"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ConverterState<'a> {
    Emph(u8),
    Strong(u8),
    Strikethrough,
    Highlight,
    Superscript,
    Subscript,
    Paragraph,
    Heading(u8),
    UnorderedList(u8),
    OrderedList(u8),
    InLink,
    CustomBlock(u8, &'a str),
}

const fn heading(u: u8) -> &'static str {
    match u {
        0 => "h1",
        1 => "h2",
        2 => "h3",
        3 => "h4",
        4 => "h5",
        _ => "h6",
    }
}
