use crate::{
    parsesource::ParseSource,
    sourcerefs::{ByteOffset, SourcePos},
};

pub struct ParseStr<'a, P: SourcePos> {
    input: &'a str,
    pub pos: P,
}
impl<'a, P: SourcePos> ParseStr<'a, P> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: P::default(),
        }
    }
    #[inline]
    pub fn starts_with_str(&self, s: &str) -> bool {
        self.input.starts_with(s)
    }
    #[inline]
    pub const fn rest(&self) -> &'a str {
        self.input
    }

    pub fn read_until_is(&mut self, mut pred: impl FnMut(&'a str) -> bool) -> &'a str {
        let mut curr = self.input;
        while !curr.is_empty() {
            if pred(curr) {
                let ret = &self.input[..self.input.len() - curr.len()];
                self.input = curr;
                self.pos.update_str_maybe_newline(ret);
                return ret;
            }
            if let Some(next) = curr.chars().next() {
                curr = &curr[next.len_utf8()..];
            }
        }
        self.pos.update_str_maybe_newline(self.input);
        std::mem::take(&mut self.input)
    }

    /*
    pub fn read_until_is_with_brackets(
        &mut self,
        mut is_open: impl FnMut(char) -> bool,
        mut is_close: impl FnMut(char) -> bool,
        mut pred: impl FnMut(&'a str) -> bool,
    ) -> &'a str {
        let mut depth = 0;
        let mut curr = self.input;

        while !curr.is_empty() {
            // SAFETY: !curr.is_empty()
            let head = unsafe { curr.chars().next().unwrap_unchecked() };
            if is_close(head) && depth > 0 {
                depth -= 1;
            } else if is_open(head) {
                depth += 1;
            } else if depth == 0 && pred(curr) {
                let ret = &self.input[..self.input.len() - curr.len()];
                self.input = curr;
                self.pos.update_str_maybe_newline(ret);
                return ret;
            }
            if let Some(next) = curr.chars().next() {
                curr = &curr[next.len_utf8()..];
            }
        }
        self.pos.update_str_maybe_newline(self.input);
        std::mem::take(&mut self.input)
    }
     */

    pub fn read_until_inclusive(&mut self, pred: impl FnMut(char) -> bool) -> &'a str {
        let i = self.input.find(pred).unwrap_or(self.input.len());
        let (l, r) = self.input.split_at(i + 1);
        self.input = r;
        self.pos.update_str_maybe_newline(l);
        l
    }
    pub fn drop_prefix(&mut self, s: &str) -> bool {
        self.input.starts_with(s) && {
            self.input = &self.input[s.len()..];
            self.pos.update_str_maybe_newline(s);
            true
        }
    }

    pub fn preview_until_with_brackets(
        &self,
        open: char,
        close: char,
        mut pred: impl FnMut(char) -> bool,
    ) -> &'a str {
        let mut depth = 0;
        let i = self
            .input
            .find(|c| {
                if c == open {
                    depth += 1;
                    false
                } else if c == close && depth > 0 {
                    depth -= 1;
                    false
                } else {
                    depth == 0 && pred(c)
                }
            })
            .unwrap_or(self.input.len());
        let (l, _r) = self.input.split_at(i);
        l
    }

    pub fn read_until_escaped(&mut self, find: char, escape: char) -> &'a str {
        let mut chars = self.input.chars();
        let mut i: usize = 0;
        while let Some(c) = chars.next() {
            if c == escape {
                if let Some(c) = chars.next() {
                    i += c.len_utf8();
                }
            } else if c == find {
                let (l, r) = self.input.split_at(i);
                self.input = r;
                self.pos.update_str_maybe_newline(l);
                return l;
            }
            i += c.len_utf8();
        }
        let ret = self.input;
        self.input = "";
        self.pos.update_str_maybe_newline(ret);
        ret
    }
}

impl ParseStr<'_, ByteOffset> {
    #[inline]
    pub const fn offset(&mut self) -> &mut ByteOffset {
        &mut self.pos
    }
}

impl<'a, P: SourcePos + 'a> ParseSource<'a> for ParseStr<'a, P> {
    type Pos = P;
    type Str = &'a str;
    type Source = &'a str;
    fn source(&self) -> &Self::Source {
        &self.input
    }
    #[inline]
    fn curr_pos(&self) -> P {
        self.pos
    }
    fn pop_head(&mut self) -> Option<char> {
        if let Some(c) = self.input.chars().next() {
            if c == '\n' {
                self.pos.update_newline(false);
                self.input = &self.input[1..];
                Some('\n')
            } else if c == '\r' {
                if self.input.chars().nth(1) == Some('\n') {
                    self.input = &self.input[2..];
                    self.pos.update_newline(true);
                } else {
                    self.input = &self.input[1..];
                    self.pos.update_newline(false);
                }
                Some('\n')
            } else {
                self.pos.update(c);
                self.input = &self.input[c.len_utf8()..];
                Some(c)
            }
        } else {
            None
        }
    }
    fn read_until_line_end(&mut self) -> (&'a str, P) {
        if let Some(i) = self.input.find(['\r', '\n']) {
            if self.input.as_bytes()[i] == b'\r' && self.input.as_bytes().get(i + 1) == Some(&b'\n')
            {
                let (l, r) = self.input.split_at(i);
                self.input = &r[2..];
                self.pos.update_str_no_newline(l);
                let pos = self.pos;
                self.pos.update_newline(true);
                return (l, pos);
            }
            let (l, r) = self.input.split_at(i);
            self.input = &r[1..];
            self.pos.update_str_no_newline(l);
            let pos = self.pos;
            self.pos.update_newline(false);
            (l, pos)
        } else {
            let ret = self.input;
            self.pos.update_str_no_newline(ret);
            self.input = "";
            (ret, self.pos)
        }
    }
    fn trim_start(&mut self) {
        while let Some(c) = self.input.chars().next() {
            if c == '\n' {
                self.input = &self.input[1..];
                self.pos.update_newline(false);
            } else if c == '\r' {
                self.input = &self.input[1..];
                if self.input.starts_with('\n') {
                    self.input = &self.input[1..];
                    self.pos.update_newline(true);
                } else {
                    self.pos.update_newline(false);
                }
            } else if c.is_whitespace() {
                self.input = &self.input[c.len_utf8()..];
                self.pos.update(c);
            } else {
                break;
            }
        }
    }
    fn starts_with(&mut self, c: char) -> bool {
        self.input.starts_with(c)
    }
    fn read_while(&mut self, mut pred: impl FnMut(char) -> bool) -> Self::Str {
        let i = self.input.find(|c| !pred(c)).unwrap_or(self.input.len());
        let (l, r) = self.input.split_at(i);
        self.input = r;
        self.pos.update_str_maybe_newline(l);
        l
    }
    fn read_until_with_brackets(
        &mut self,
        open: char,
        close: char,
        mut pred: impl FnMut(char) -> bool,
    ) -> Self::Str {
        let mut depth = 0;
        let i = self
            .input
            .find(|c| {
                if c == close && depth > 0 {
                    depth -= 1;
                    false
                } else if c == open {
                    depth += 1;
                    false
                } else {
                    depth == 0 && pred(c)
                }
            })
            .unwrap_or(self.input.len());
        let (l, r) = self.input.split_at(i);
        self.input = r;
        self.pos.update_str_maybe_newline(l);
        l
    }
    fn peek_head(&mut self) -> Option<char> {
        self.input.chars().next()
    }
    fn read_n(&mut self, i: usize) -> Self::Str {
        let (l, mut r) = self.input.split_at(i);
        if l.ends_with('\r') && r.starts_with('\n') {
            r = &r[1..];
        }
        self.input = r;
        self.pos.update_str_maybe_newline(l);
        l
    }
    fn read_until_str(&mut self, s: &str) -> Self::Str {
        if let Some(i) = self.input.find(s) {
            let (l, r) = self.input.split_at(i);
            self.input = r;
            self.pos.update_str_maybe_newline(l);
            l
        } else {
            let ret = self.input;
            self.input = "";
            self.pos.update_str_maybe_newline(ret);
            ret
        }
    }
    fn skip(&mut self, i: usize) {
        let (a, b) = self.input.split_at(i);
        self.input = b;
        self.pos.update_str_maybe_newline(a);
    }
}
