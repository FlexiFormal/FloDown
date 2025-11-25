use parser_utils::{ParseSource, SourcePos, SourceRange, strings::ParseStr};
use url::Url;

#[derive(PartialEq, Eq, Debug, Clone)]
enum NextToken {
    TableStart,
    StrongEmph,
    EndParagraph,
    Math,
    HeadingEnd,
    Highlight,
    BlockCode,
    InlineCode,
    Escape,
    Link(Url),
    LinkEnd,
    InlineCustom,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum TokenizerState {
    Start,
    Top,
    Inline,
    InBlockQuote,
    InDefinition,
    // InTaskList(u8),
    InUnnumberedList,
    InNumberedList,
    InTable,
    Heading,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MarkdownToken<'a, Pos: SourcePos> {
    Text(&'a str, SourceRange<Pos>),
    FrontMatter(&'a str, SourceRange<Pos>),
    BlockQuoteStart(Pos),
    ParagraphStart(Pos),
    ParagraphEnd(Pos),
    BlockMath(&'a str, SourceRange<Pos>),
    BlockCode {
        language: &'a str,
        code: &'a str,
        range: SourceRange<Pos>,
    },
    InlineMath(&'a str, SourceRange<Pos>),
    InlineCode(&'a str, SourceRange<Pos>),
    ThematicBreak(SourceRange<Pos>),
    FootnoteDefinition(&'a str, SourceRange<Pos>),
    Definition(&'a str, SourceRange<Pos>),
    /*TaskList {
        char: u8,
        indent: u8,
        range: SourceRange<Pos>,
    },
    UnnumberedList {
        indent: u8,
        position: Pos,
    },
    NumberedList {
        indent: u8,
        position: Pos,
    },*/
    UnnumberedListItem(u8, Pos),
    NumberedListItem(u8, Pos),
    //ListEnd(Pos),
    Heading(u8, SourceRange<Pos>),
    TableStart(Pos),
    Strong(u8, Pos),
    Emph(u8, Pos),
    Subscript(Pos),
    Superscript(Pos),
    StrikeThrough(Pos),
    Highlight(Pos),
    HeadingEnd(Pos),
    Escaped(char, Pos),
    LinkStart(Url, Pos),
    LinkOrCustomEnd(Pos),
    InlineCustom {
        label: &'a str,
        args: &'a str,
        range: SourceRange<Pos>,
    },
    CustomBlockStart {
        label: &'a str,
        args: &'a str,
        colons: u8,
        range: SourceRange<Pos>,
    },
    CustomBlockEnd(u8, Pos),
}

pub struct MarkdownTokenizer<'a, Pos: SourcePos> {
    source: ParseStr<'a, Pos>,
    state: TokenizerState,
    next: Option<NextToken>,
    in_link: bool,
    inline_only: bool,
}

impl<'a, Pos: SourcePos> Iterator for MarkdownTokenizer<'a, Pos> {
    type Item = MarkdownToken<'a, Pos>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next.take() {
            Some(NextToken::HeadingEnd) => {
                self.state = TokenizerState::Top;
                return Some(MarkdownToken::HeadingEnd(self.source.curr_pos()));
            }
            Some(NextToken::StrongEmph) => return Some(self.strong_emph()),
            Some(NextToken::Highlight) => return Some(self.highlight()),
            Some(NextToken::Escape) => {
                let _ = self.source.pop_head();
                if let Some(next) = self.source.pop_head() {
                    return Some(MarkdownToken::Escaped(next, self.source.curr_pos()));
                }
            }
            Some(NextToken::EndParagraph) => {
                self.state = TokenizerState::Top;
                return Some(MarkdownToken::ParagraphEnd(self.source.curr_pos()));
            }
            Some(NextToken::Math) => {
                let start = self.source.curr_pos();
                let _ = self.source.pop_head();
                let math = self.source.read_until_with_brackets('{', '}', |c| c == '$');
                if self.source.starts_with('$') {
                    let _ = self.source.pop_head();
                }
                return Some(MarkdownToken::InlineMath(
                    math,
                    SourceRange {
                        start,
                        end: self.source.curr_pos(),
                    },
                ));
            }
            Some(NextToken::InlineCode) => {
                let start = self.source.curr_pos();
                let _ = self.source.pop_head();
                let code = self.source.read_until_escaped('`', '\\');
                if self.source.starts_with('`') {
                    let _ = self.source.pop_head();
                }
                return Some(MarkdownToken::InlineCode(
                    code,
                    SourceRange {
                        start,
                        end: self.source.curr_pos(),
                    },
                ));
            }
            Some(NextToken::BlockCode) => {
                self.state = TokenizerState::Top;
                return Some(self.block_code(self.source.curr_pos()));
            }
            Some(NextToken::Link(url)) => {
                self.in_link = true;
                // starts with '['
                let start = self.source.curr_pos();
                let _ = self.source.pop_head();
                return Some(MarkdownToken::LinkStart(url, start));
            }
            Some(NextToken::LinkEnd) => {
                self.in_link = false;
                // ](
                let _ = self.source.pop_head();
                let _ = self.source.pop_head();
                let _ = self.source.read_until_with_brackets('(', ')', |c| c == ')');
                if self.source.starts_with(')') {
                    let _ = self.source.pop_head();
                }
                return Some(MarkdownToken::LinkOrCustomEnd(self.source.curr_pos()));
            }
            Some(NextToken::InlineCustom) => {
                let start = self.source.curr_pos();
                // @[
                let _ = self.source.pop_head();
                let _ = self.source.pop_head();
                let label = self.source.read_until(|c| c == ']').trim();
                // ](
                let _ = self.source.pop_head();
                let _ = self.source.pop_head();
                let args = self.source.read_until_with_brackets('(', ')', |c| c == ')');
                if self.source.starts_with(')') {
                    let _ = self.source.pop_head();
                }
                let range = SourceRange {
                    start,
                    end: self.source.curr_pos(),
                };
                return Some(MarkdownToken::InlineCustom { label, args, range });
            }
            /*
            Some(NextToken::UnnumberedListItem(indent)) => {
                let _ = self.source.pop_head();
                self.skip_space();
                return Some(MarkdownToken::UnnumberedListItem(
                    indent,
                    self.source.curr_pos(),
                ));
            }
            Some(NextToken::NumberedListItem(indent)) => {
                while self.source.pop_head().is_some_and(|c| c.is_ascii_digit()) {}
                self.skip_space();
                return Some(MarkdownToken::NumberedListItem(
                    indent,
                    self.source.curr_pos(),
                ));
            }
            */
            Some(NextToken::TableStart) => (),
            None => (),
        }
        match self.state {
            TokenizerState::Start => self.start(),
            TokenizerState::Top => self.top(),
            //TokenizerState::InUnnumberedList(ind) => Some(self.unnumbered_list_item()),
            //TokenizerState::InNumberedList(ind) => Some(self.numbered_list_item()),
            _ => self.inline(),
        }
    }
}

macro_rules! ret {
    ($slf:expr) => {
        if let Some(r) = $slf {
            return Some(r);
        }
    };
}

impl<'a, Pos: SourcePos> MarkdownTokenizer<'a, Pos> {
    #[must_use]
    pub fn new(source: &'a str) -> Self {
        Self {
            source: ParseStr::new(source),
            state: TokenizerState::Start,
            next: None,
            in_link: false,
            inline_only: false,
        }
    }

    #[must_use]
    pub fn new_inline(source: &'a str) -> Self {
        Self {
            source: ParseStr::new(source),
            state: TokenizerState::Inline,
            next: None,
            in_link: false,
            inline_only: true,
        }
    }

    fn start(&mut self) -> Option<MarkdownToken<'a, Pos>> {
        ret!(self.frontmatter());
        self.state = TokenizerState::Top;
        self.top()
    }

    fn top(&mut self) -> Option<MarkdownToken<'a, Pos>> {
        let indent = self.trim_indent();
        if self.source.rest().is_empty() {
            return None;
        }
        let next = self.source.rest().as_bytes()[0];
        let start = self.source.curr_pos();
        match next {
            // block quote
            b'>' if [Some(b' '), Some(b'\t')]
                .contains(&self.source.rest().as_bytes().get(1).copied()) =>
            {
                Some(self.start_block_quote(start))
            }
            // thematic breaks
            b'_' if self.source.starts_with_str("___") => Some(self.thematic_break(b'_', start)),
            b'-' if self.source.starts_with_str("---") => Some(self.thematic_break(b'-', start)),
            b'*' if self.source.starts_with_str("***") => Some(self.thematic_break(b'*', start)),
            // block math
            b'$' if self.source.starts_with_str("$$") => Some(self.block_math(start)),
            // definition
            /*b'[' if is_footnote(self.source.rest().as_bytes()) => {
                Some(self.start_definition(start))
            }*/
            // list  // or task list
            b'*' | b'-'
                if [Some(b' '), Some(b'\t')]
                    .contains(&self.source.rest().as_bytes().get(1).copied()) =>
            {
                self.state = TokenizerState::InUnnumberedList;
                let _ = self.source.pop_head();
                self.skip_space();
                //self.next = Some(NextToken::UnnumberedListItem(indent));
                Some(MarkdownToken::UnnumberedListItem(indent, start))
            }
            // numbered list
            b if b.is_ascii_digit() && is_numbered_list(self.source.rest().as_bytes()) => {
                self.state = TokenizerState::InNumberedList;
                while self.source.pop_head().is_some_and(|c| c.is_ascii_digit()) {}
                self.skip_space();
                //self.next = Some(NextToken::NumberedListItem);
                Some(MarkdownToken::NumberedListItem(indent, start))
            }
            // heading
            b'#' => Some(self.heading(start)),
            b'|' if is_table(self.source.rest().as_bytes()) => {
                self.next = Some(NextToken::TableStart);
                Some(MarkdownToken::TableStart(start))
            }
            // block code
            b'`' if self.source.starts_with_str("```") => Some(self.block_code(start)),
            // custom block
            b':' if self.source.rest().as_bytes().get(1).copied() == Some(b':') => {
                Some(self.custom_block(start))
            }
            //b'<' /* block html? */ => (),
            _ => {
                self.state = TokenizerState::Inline;
                Some(MarkdownToken::ParagraphStart(start))
            }
        }
        /*
         * - table: | none | left | right | center |
         * - custom block: ::
         */
    }

    /*
     * * ANY *
     * - emph / strong / strikethrough
     * - autolink
     * - character escape / hard break escape
     * - reference
     * - footnote
     * - label / image
     * - custom inline
     * - block math breaks paragraph?
     *
     * * TOP *
     *
     */
    #[allow(clippy::too_many_lines)]
    fn inline(&mut self) -> Option<MarkdownToken<'a, Pos>> {
        let start = self.source.curr_pos();
        let txt = self.source.read_until_is(|src| {
            if src.is_empty() {
                return false;
            }
            let head = src.as_bytes()[0];
            macro_rules! next {
                () => {
                    src.as_bytes().get(1).copied()
                };
            }
            match head {
                // strong/emph/strikethrough
                b'_' | b'*' | b'~' | b'^' => {
                    self.next = Some(NextToken::StrongEmph);
                    true
                }
                b'=' if next!() == Some(b'=') => {
                    self.next = Some(NextToken::Highlight);
                    true
                }
                // block math
                b'$' if next!() == Some(b'$') && !self.inline_only => {
                    self.next = Some(NextToken::EndParagraph);
                    true
                }
                // block code
                b'`' if src.starts_with("```") && !self.inline_only => {
                    self.next = Some(NextToken::BlockCode);
                    true
                }
                // inline code
                b'`' => {
                    self.next = Some(NextToken::InlineCode);
                    true
                }
                // math
                b'$' => {
                    self.next = Some(NextToken::Math);
                    true
                }
                // escaped character
                b'\\' => {
                    self.next = Some(NextToken::Escape);
                    true
                },
                // link or reference or footnote
                b'[' => {
                    if let Some(url) = is_link(src) {
                        self.next = Some(NextToken::Link(url));
                        true
                    } else {
                        // TODO
                        false
                    }
                }
                b']' if self.in_link && next!() == Some(b'(') =>  {
                    self.next = Some(NextToken::LinkEnd);
                    true
                }
                // maybe custom inline
                b'@' if next!() == Some(b'[') && is_custom(src) => {
                    self.next = Some(NextToken::InlineCustom);
                    true
                },
                // maybe image
                b'!' => false,
                // escaped character?
                b'&' => false,
                // maybe table cell break
                b'|' if self.state == TokenizerState::InTable => false,
                // line end - possibly paragraph end
                b'\r' | b'\n' if !self.inline_only => {
                    if self.state == TokenizerState::Heading {
                        self.next = Some(NextToken::HeadingEnd);
                        return true
                    }
                    let mut rest = src.as_bytes();
                    if head == b'\r' && next!() == Some(b'\n') {
                        rest = &rest[2..];
                    } else {
                        rest = &rest[1..];
                    }
                    while !rest.is_empty() {
                        let head = rest[0];
                        rest = &rest[1..];
                        if head == b' ' || head == b'\t' {
                            continue;
                        }
                        // empty line
                        if head == b'\r'
                            || head == b'\n'
                            // block quote
                            || (head == b'>' && (rest.first() == Some(&b' ') || rest.first() == Some(&b'\t')))
                            // thematic breaks
                            || (head==b'_' && rest.starts_with(b"__"))
                            || (head == b'-' && rest.starts_with(b"--"))
                            || (head == b'*' && rest.starts_with(b"**"))
                            // block math
                            || (head == b'$' && rest.first() == Some(&b'$'))
                            // definition
                            //|| (head == b'[' && is_footnote(rest))
                            // list // or task list
                            || ((head == b'*' || head == b'-') && !matches!(self.state,TokenizerState::InUnnumberedList/*|TokenizerState::InTaskList(_)*/) && (rest.first() == Some(&b' ') || rest.first() == Some(&b'\t')))
                            // numbered list
                            || (head.is_ascii_digit() && !matches!(self.state,TokenizerState::InNumberedList) && is_numbered_list(rest))
                            // table
                            || (head == b'|' && is_table(rest))
                            // custom block
                            || (head == b':' && rest.starts_with(b"::"))
                            // heading
                            || head == b'#'
                        {
                            self.next = Some(NextToken::EndParagraph);
                            return true;
                        }
                        if matches!(self.state,TokenizerState::InUnnumberedList/*|TokenizerState::InTaskList(_)*/) && (head == b'*' || head == b'-') && rest.first() == Some(&b' ') {
                            self.state = TokenizerState::Top;//.next = Some(NextToken::MaybeUnnumberedListItem);
                            return true;
                        }
                        if matches!(self.state,TokenizerState::InNumberedList) && head.is_ascii_digit() && is_numbered_list(rest) {
                            self.state = TokenizerState::Top;//.next = Some(NextToken::MaybeNumberedListItem);
                            return true;
                        }
                        break;
                    }
                    false
                }
                _ => false,
            }
        });
        let range = SourceRange {
            start,
            end: self.source.curr_pos(),
        };
        if txt.is_empty() {
            if self.next.is_some() {
                self.next()
            } else {
                None
            }
        } else {
            Some(MarkdownToken::Text(txt, range))
        }
    }

    // ------------------------------------------------------------------------

    fn skip_space(&mut self) {
        while self.source.starts_with(' ') || self.source.starts_with('\t') {
            let _ = self.source.pop_head();
        }
    }

    fn trim_indent(&mut self) -> u8 {
        let mut curr = 0;
        while let Some(h) = self.source.peek_head() {
            match h {
                ' ' => {
                    curr += 1;
                    let _ = self.source.pop_head();
                }
                '\t' => {
                    curr += 4;
                    let _ = self.source.pop_head();
                }
                '\r' | '\n' => {
                    curr = 0;
                    let _ = self.source.pop_head();
                }
                _ => break,
            }
        }
        curr
    }

    // ------------------------------------------------------------------------

    fn heading(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        let num = self.source.read_while(|c| c == '#').len();
        let _ = self.source.read_while(|c| c == ' ' || c == '\t');
        self.state = TokenizerState::Heading;
        #[allow(clippy::cast_possible_truncation)]
        MarkdownToken::Heading(
            num as u8,
            SourceRange {
                start,
                end: self.source.curr_pos(),
            },
        )
    }

    fn start_block_quote(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        self.source.pop_head();
        self.source.pop_head();
        self.state = TokenizerState::InBlockQuote;
        MarkdownToken::BlockQuoteStart(start)
    }

    fn thematic_break(&mut self, char: u8, start: Pos) -> MarkdownToken<'a, Pos> {
        self.source.read_while(|c| c == char as char);
        let end = self.source.curr_pos();
        self.state = TokenizerState::Top;
        MarkdownToken::ThematicBreak(SourceRange { start, end })
    }

    fn start_definition(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        let _ = self.source.pop_head();
        let is_footnote = self.source.starts_with('^') && {
            let _ = self.source.pop_head();
            true
        };
        let lbl = self.source.read_until_str("]:").trim();
        let range = SourceRange {
            start,
            end: self.source.curr_pos(),
        };
        self.state = TokenizerState::InDefinition;
        if is_footnote {
            MarkdownToken::FootnoteDefinition(lbl, range)
        } else {
            MarkdownToken::Definition(lbl, range)
        }
    }

    // precondition: starts with "$$"
    fn block_math(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        self.source.pop_head();
        self.source.pop_head();
        let math = self.source.read_until_str("$$");
        if self.source.starts_with_str("$$") {
            self.source.pop_head();
            self.source.pop_head();
        }
        let end = self.source.curr_pos();
        MarkdownToken::BlockMath(math, SourceRange { start, end })
    }

    fn block_code(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        self.source.pop_head();
        self.source.pop_head();
        self.source.pop_head();
        let (language, _) = self.source.read_until_line_end();
        let code = self.source.read_until_str("```");
        if self.source.starts_with_str("```") {
            self.source.pop_head();
            self.source.pop_head();
            self.source.pop_head();
        }
        let end = self.source.curr_pos();
        MarkdownToken::BlockCode {
            language,
            code,
            range: SourceRange { start, end },
        }
    }

    fn custom_block(&mut self, start: Pos) -> MarkdownToken<'a, Pos> {
        #[allow(clippy::cast_possible_truncation)]
        let indent = self.source.read_until(|c| c != ':').len() as u8;
        self.skip_space();
        let label = self.source.read_until(|c| "\r\n \t".contains(c)).trim();
        let (args, _) = self.source.read_until_line_end();
        if label.is_empty() {
            //self.state = TokenizerState::Top;
            MarkdownToken::CustomBlockEnd(indent, start)
        } else {
            //self.state = TokenizerState::Inline;
            MarkdownToken::CustomBlockStart {
                label,
                args: args.trim(),
                colons: indent,
                range: SourceRange {
                    start,
                    end: self.source.curr_pos(),
                },
            }
        }
    }

    // precondition: source starts with '*', '_', '^' or '~'
    fn strong_emph(&mut self) -> MarkdownToken<'a, Pos> {
        let start = self.source.curr_pos();
        let head = self.source.rest().as_bytes()[0];
        self.source.skip(1);
        let double = self.source.rest().as_bytes().first() == Some(&head) && {
            self.source.skip(1);
            true
        };
        match head {
            b'*' | b'_' if double => MarkdownToken::Strong(head, start),
            b'~' if double => MarkdownToken::StrikeThrough(start),
            b'~' => MarkdownToken::Subscript(start),
            b'^' => MarkdownToken::Superscript(start),
            _ => MarkdownToken::Emph(head, start),
        }
    }

    // precondition: source starts with "=="
    fn highlight(&mut self) -> MarkdownToken<'a, Pos> {
        let start = self.source.curr_pos();
        self.source.skip(2);
        MarkdownToken::Highlight(start)
    }

    pub fn frontmatter(&mut self) -> Option<MarkdownToken<'a, Pos>> {
        let (fence, fence_char) = if self.source.rest().trim_start().starts_with("+++") {
            ("+++", '+')
        } else if self.source.rest().trim_start().starts_with("---") {
            ("---", '-')
        } else {
            return None;
        };
        self.source.trim_start();
        let start = self.source.curr_pos();
        let _ = self.source.read_while(|c| c == fence_char);
        self.source.trim_start();
        let mut opens = None;
        let fm = self.source.read_until_is(|s| {
            if s.is_empty() {
                return false;
            }
            let head = s.as_bytes()[0];
            if b"\"'#".contains(&head) {
                match opens {
                    None => {
                        opens = Some(if s.starts_with("\"\"\"") {
                            Quotes::TripleDouble
                        } else if s.starts_with("'''") {
                            Quotes::TripleSingle
                        } else if s.starts_with('\"') {
                            Quotes::Double
                        } else if s.starts_with('\'') {
                            Quotes::Single
                        } else {
                            Quotes::Comment
                        });
                    }
                    Some(Quotes::Single) if head == b'\'' => opens = None,
                    Some(Quotes::Double) if head == b'"' => opens = None,
                    Some(Quotes::TripleDouble) if s.starts_with("\"\"\"") => opens = None,
                    Some(Quotes::TripleSingle) if s.starts_with("'''") => opens = None,
                    _ => (),
                }
            } else if opens == Some(Quotes::Comment) && b"\r\n".contains(&head) {
                opens = None;
            }

            s.starts_with(fence)
        });
        let _ = self.source.read_while(|c| c == fence_char);
        let range = SourceRange {
            start,
            end: self.source.curr_pos(),
        };
        Some(MarkdownToken::FrontMatter(fm, range))
    }
}

// invariant: starts with '['
fn is_link<'s>(rest: &'s str) -> Option<Url> {
    let mut ps = ParseStr::<'s, ()>::new(&rest[1..]);
    let txt = ps.read_until_with_brackets('[', ']', |c| c == ']');
    if txt.contains(['\r', '\n']) {
        return None;
    }
    if ps.starts_with(']') {
        let _ = ps.pop_head();
    }
    if !ps.starts_with('(') {
        return None;
    }
    let _ = ps.pop_head();
    let url = ps.read_until_with_brackets('(', ')', |c| c == ')');
    url.parse().ok()
}

// invariant: starts with "@["
fn is_custom(rest: &str) -> bool {
    let mut ps = ParseStr::<'_, ()>::new(&rest[2..]);
    let label = ps.read_until(|c| c == ']');
    ps.starts_with_str("](") && !label.trim().contains(['\r', ' ', '\n', '\t'])
}

fn is_footnote(source: &[u8]) -> bool {
    let Some(i) = source.iter().position(|b| *b == b']') else {
        return false;
    };
    source.get(i + 1) == Some(&b':') && {
        let Some(sec) = source.get(i + 2) else {
            return false;
        };
        *sec == b' ' || *sec == b'\t'
    }
}

fn is_table(source: &[u8]) -> bool {
    let rest = source;
    let Some(line_end) = rest.iter().position(|b| *b == b'\r' || *b == b'\n') else {
        return false;
    };
    let mut rest = &rest[line_end..];
    if rest.starts_with(b"\r\n") {
        rest = &rest[2..];
    } else {
        rest = &rest[1..];
    }
    let Some(line_end) = rest.iter().position(|b| *b == b'\r' || *b == b'\n') else {
        return false;
    };
    let line = &rest[..line_end - 1];
    if line.first() != Some(&b'|') {
        return false;
    }
    let line = &line[1..];
    let mut start_end = Some(true); // true=start, false=end, None=middle
    line.iter().all(|b| {
        let b = *b;
        (start_end == Some(true)
            && (b == b' '
                || b == b'\t'
                || ((b == b'-' || b == b':') && {
                    start_end = None;
                    true
                })))
            || (start_end.is_none()
                && (b == b'-'
                    || (b == b':' && {
                        start_end = Some(false);
                        true
                    })
                    || (b == b'|' && {
                        start_end = Some(true);
                        true
                    })))
    })
}

fn is_numbered_list(source: &[u8]) -> bool {
    let Some(i) = source.iter().position(|c| !c.is_ascii_digit()) else {
        return false;
    };
    let then = source.get(i).copied();
    then == Some(b'.') || then == Some(b')')
}

#[derive(PartialEq, Eq)]
enum Quotes {
    Single,
    Double,
    TripleSingle,
    TripleDouble,
    Comment,
}
