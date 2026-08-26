#![cfg_attr(doc,doc = document_features::document_features!())]

use parser_utils::{SourcePos, SourceRange};
use url::Url;

pub mod html;
pub mod latex;
pub mod tokenizer;

/*
pub struct MarkdownParser<'a, Pos: SourcePos, T: FromMarkdown<'a, Pos>> {
    tokenizer: tokenizer::MarkdownTokenizer<'a, Pos>,
    buffer: Vec<T>,
    stack: Vec<Open<'a, Pos, T>>,
}

impl<'a, Pos: SourcePos, T: FromMarkdown<'a, Pos>> Iterator for MarkdownParser<'a, Pos, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        use tokenizer::MarkdownToken as MT;
        macro_rules! ret {
            ($e:expr) => {
                if let Some(a) = $e {
                    return Some(a);
                }
            };
            (I $e:expr) => {
                if let Some(e) = $e
                    && let Some(v) = self.stack.last_mut().and_then(Open::inlines)
                {
                    v.push(e);
                }
            };
            (B? $e:expr) => {
                if let Some(e) = $e {
                    if let Some(v) = self.stack.last_mut().and_then(Open::blocks) {
                        v.push(e);
                    } else {
                        return Some(e);
                    }
                }
            };
            (B $e:expr) => {
                match $e {
                    Ok(e) => {
                        if let Some(v) = self.stack.last_mut().and_then(Open::blocks) {
                            v.push(e);
                        } else {
                            return Some(e);
                        }
                    }
                    Err(ls) if !ls.is_empty() => {
                        if let Some(v) = self.stack.last_mut().and_then(Open::blocks) {
                            v.extend(ls);
                        } else {
                            let mut it = ls.into_iter();
                            let ret = it.next();
                            self.buffer.extend(it.rev());
                            return ret;
                        }
                    }
                    _ => (),
                }
            };
        }
        loop {
            ret!(self.buffer.pop());
            match self.tokenizer.next()? {
                MT::FrontMatter(s, r) => ret!(T::frontmatter(s, r)),
                MT::BlockMath(s, r) => ret!(B? T::block_math(s, r)),
                MT::BlockCode {
                    language,
                    code,
                    range,
                } => ret!(B? T::block_code(code, language,range)),
                MT::BlockQuoteStart(p) => self.stack.push(Open::Quote(Vec::new(), p)),
                MT::ParagraphStart(p) => self.stack.push(Open::Paragraph(Vec::new(), p)),
                MT::ParagraphEnd(p) => match self.stack.pop()? {
                    Open::Paragraph(children, pos) => {
                        ret!(B? T::paragraph(children, SourceRange { start: pos, end: p }))
                    }
                    Open::Quote(children, pos) => {
                        ret!(B T::block_quote(children, SourceRange { start: pos, end: p }))
                    }
                    Open::Numbered(mut lines, line, start, _) => {
                        lines.push((line, SourceRange { start, end: p }));
                        ret!(B? T::numbered_list(lines))
                    }
                    Open::Unnumbered(mut lines, line, start, _) => {
                        lines.push((line, SourceRange { start, end: p }));
                        ret!(B? T::bullet_list(lines))
                    }
                },
                MT::ThematicBreak(p) => ret!(B? T::thematic_break(p)),
                MT::UnnumberedListItem(indent, p) => match self.stack.last() {
                    Open::Paragraph(v, pi) => {

                    }

                }
                // -----------
                MT::Text(s, r) => ret!(I T::Inline::text(s, r)),
                MT::InlineMath(s, r) => ret!(I T::Inline::math(s, r)),
                MT::InlineCode(s, r) => ret!(I T::Inline::code(s, r)),
            }
        }
    }
}

enum Open<'a, Pos: SourcePos, T: FromMarkdown<'a, Pos>> {
    Paragraph(Vec<T::Inline>, Pos),
    Quote(Vec<T>, Pos),
    Numbered(Vec<(Vec<T>, SourceRange<Pos>)>, Vec<T>, Pos, u8),
    Unnumbered(Vec<(Vec<T>, SourceRange<Pos>)>, Vec<T>, Pos, u8),
}
impl<'a, Pos: SourcePos, T: FromMarkdown<'a, Pos>> Open<'a, Pos, T> {
    fn inlines(&mut self) -> Option<&mut Vec<T::Inline>> {
        match self {
            Self::Paragraph(v, _) => Some(v),
            Self::Quote(..) | Self::Numbered(..) | Self::Unnumbered(..) => None,
        }
    }
    fn blocks(&mut self) -> Option<&mut Vec<T>> {
        match self {
            Self::Paragraph(..) => None,
            Self::Quote(v, _) | Self::Numbered(_, v, _, _) | Self::Unnumbered(_, v, _, _) => {
                Some(v)
            }
        }
    }
}

pub trait FromMarkdown<'a, Pos: SourcePos>: Sized {
    type Inline: FromMarkdownInline<'a, Pos>;
    #[inline]
    fn frontmatter(_txt: &'a str, _range: SourceRange<Pos>) -> Option<Self> {
        None
    }
    fn paragraph(children: Vec<Self::Inline>, range: SourceRange<Pos>) -> Option<Self>;
    /// ### Errors
    fn block_quote(children: Vec<Self>, range: SourceRange<Pos>) -> Result<Self, Vec<Self>>;
    fn block_math(txt: &'a str, range: SourceRange<Pos>) -> Option<Self>;
    fn block_code(txt: &'a str, language: &'a str, range: SourceRange<Pos>) -> Option<Self>;
    fn thematic_break(pos: SourceRange<Pos>) -> Option<Self>;
    /// ### Errors
    fn numbered_list(items: Vec<(Vec<Self>, SourceRange<Pos>)>) -> Option<Self>;
    /// ### Errors
    fn bullet_list(items: Vec<(Vec<Self>, SourceRange<Pos>)>) -> Option<Self>;
    fn heading(children: Vec<Self::Inline>, range: SourceRange<Pos>);
    /// ### Errors
    fn custom(
        children: Vec<Self>,
        label: &'a str,
        args: &'a str,
        range: SourceRange<Pos>,
    ) -> Result<Self, Vec<Self>>;
}

pub trait FromMarkdownInline<'a, Pos: SourcePos>: Sized {
    fn text(txt: &'a str, range: SourceRange<Pos>) -> Option<Self>;
    fn math(txt: &'a str, range: SourceRange<Pos>) -> Option<Self>;
    fn code(txt: &'a str, range: SourceRange<Pos>) -> Option<Self>;
    fn strong(children: Vec<Self>, range: SourceRange<Pos>);
    fn emph(children: Vec<Self>, range: SourceRange<Pos>);
    fn subscript(children: Vec<Self>, range: SourceRange<Pos>);
    fn superscript(children: Vec<Self>, range: SourceRange<Pos>);
    fn strikethrough(children: Vec<Self>, range: SourceRange<Pos>);
    fn highlight(children: Vec<Self>, range: SourceRange<Pos>);
    fn escaped(c: char, pos: Pos) -> Option<Self>;
    fn link(children: Vec<Self>, url: Url, range: SourceRange<Pos>) -> Option<Self>;
    fn custom(label: &'a str, args: &'a str, range: SourceRange<Pos>) -> Option<Self>;
}
 */
