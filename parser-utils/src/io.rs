use crate::{parsesource::ParseSource, sourcerefs::SourcePos};
use std::io::Read;

pub struct ParseReader<R: Read, P: SourcePos> {
    inner: R,
    buf: Vec<char>,
    pos: P,
}
impl<R: Read, P: SourcePos> ParseReader<R, P> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            pos: P::default(),
        }
    }
}

impl<'a, R: Read + 'a, P: SourcePos + 'a> ParseSource<'a> for ParseReader<R, P> {
    type Pos = P;
    type Str = String;
    type Source = R;
    #[inline]
    fn source(&self) -> &Self::Source {
        &self.inner
    }
    #[inline]
    fn curr_pos(&self) -> P {
        self.pos
    }
    #[inline]
    fn skip(&mut self, i: usize) {
        for _ in 0..i {
            self.pop_head();
        }
    }
    fn pop_head(&mut self) -> Option<char> {
        match self.get_char() {
            Some('\n') => {
                self.pos.update_newline(false);
                Some('\n')
            }
            Some('\r') => {
                match self.get_char() {
                    Some('\n') => {
                        self.pos.update_newline(true);
                    }
                    Some(c) => {
                        self.pos.update_newline(false);
                        self.push_char(c);
                    }
                    None => {
                        self.pos.update_newline(false);
                    }
                }
                Some('\n')
            }
            Some(c) => {
                self.pos.update(c);
                Some(c)
            }
            None => None,
        }
    }
    fn read_until_line_end(&mut self) -> (String, P) {
        let (s, rn) = self.find_line_end();
        self.pos.update_str_no_newline(&s);
        let pos = self.pos;
        if let Some(rn) = rn {
            self.pos.update_newline(rn);
        }
        (s, pos)
    }
    fn trim_start(&mut self) {
        while let Some(c) = self.get_char() {
            if c == '\n' {
                self.pos.update_newline(false);
            } else if c == '\r' {
                match self.get_char() {
                    Some('\n') => {
                        self.pos.update_newline(true);
                    }
                    Some(c) => {
                        self.push_char(c);
                        self.pos.update_newline(false);
                    }
                    None => {
                        self.pos.update_newline(false);
                        break;
                    }
                }
            } else if c.is_whitespace() {
                self.pos.update(c);
            } else {
                self.push_char(c);
                break;
            }
        }
    }

    #[allow(clippy::unnecessary_map_or)]
    fn starts_with(&mut self, c: char) -> bool {
        self.get_char().map_or(false, |c2| {
            self.push_char(c2);
            c2 == c
        })
    }
    fn read_while(&mut self, mut pred: impl FnMut(char) -> bool) -> Self::Str {
        let mut ret = String::new();
        let mut rn = false;
        while let Some(c) = self.get_char() {
            if !pred(c) {
                self.push_char(c);
                break;
            }
            if rn && c == '\n' {
                self.pos.update_newline(true);
                rn = false;
                continue;
            }
            if rn {
                self.pos.update_newline(false);
                rn = false;
            } else if c == '\n' {
                self.pos.update_newline(false);
                ret.push('\n');
                continue;
            } else if c == '\r' {
                ret.push('\n');
                rn = true;
                continue;
            }
            self.pos.update(c);
            ret.push(c);
        }
        if rn {
            self.pos.update_newline(false);
        }
        ret
    }
    fn read_until_with_brackets(
        &mut self,
        open: char,
        close: char,
        mut pred: impl FnMut(char) -> bool,
    ) -> Self::Str {
        let mut ret = String::new();
        let mut depth = 0;
        let mut rn = false;
        while let Some(c) = self.get_char() {
            if c == open {
                depth += 1;
                self.pos.update(c);
                ret.push(c);
                continue;
            } else if c == close && depth > 0 {
                depth -= 1;
                self.pos.update(c);
                ret.push(c);
                continue;
            } else if depth > 0 {
                if rn && c == '\n' {
                    self.pos.update_newline(true);
                    rn = false;
                    continue;
                }
                if rn {
                    self.pos.update_newline(false);
                    rn = false;
                } else if c == '\n' {
                    self.pos.update_newline(false);
                    ret.push('\n');
                    continue;
                } else if c == '\r' {
                    ret.push('\n');
                    rn = true;
                    continue;
                }
                self.pos.update(c);
                ret.push(c);
                continue;
            }
            if pred(c) {
                self.push_char(c);
                break;
            }
            self.pos.update(c);
            ret.push(c);
        }
        if rn {
            self.pos.update_newline(false);
        }
        ret
    }
    fn peek_head(&mut self) -> Option<char> {
        self.get_char().inspect(|c| {
            self.push_char(*c);
        })
    }
    fn read_n(&mut self, i: usize) -> Self::Str {
        let mut ret = String::new();
        for _ in 0..i {
            if let Some(c) = self.pop_head() {
                ret.push(c);
            } else {
                break;
            }
        }
        ret
    }
    fn read_until_str(&mut self, s: &str) -> Self::Str {
        let mut ret = String::new();
        while let Some(c) = self.pop_head() {
            ret.push(c);
            if ret.ends_with(s) {
                for _ in 0..s.len() {
                    self.push_char(ret.pop().unwrap_or_else(|| unreachable!()));
                }
                return ret;
            }
        }
        ret
    }
}

impl<R: Read, P: SourcePos> ParseReader<R, P> {
    fn get_char(&mut self) -> Option<char> {
        self.buf.pop().or_else(|| self.read_char())
    }
    fn read_char(&mut self) -> Option<char> {
        let mut byte = [0u8];
        self.inner.read_exact(&mut byte).ok()?;
        let byte = byte[0];
        if byte & 224u8 == 192u8 {
            // a two byte unicode character
            let mut buf = [byte, 0];
            self.inner.read_exact(&mut buf[1..]).ok()?;
            Self::char_from_utf8(&buf)
        } else if byte & 240u8 == 224u8 {
            // a three byte unicode character
            let mut buf = [byte, 0, 0];
            self.inner.read_exact(&mut buf[1..]).ok()?;
            Self::char_from_utf8(&buf)
        } else if byte & 248u8 == 240u8 {
            // a four byte unicode character
            let mut buf = [byte, 0, 0, 0];
            self.inner.read_exact(&mut buf[1..]).ok()?;
            Self::char_from_utf8(&buf)
        } else {
            Some(byte as char)
        }
    }
    fn push_char(&mut self, c: char) {
        self.buf.push(c);
    }
    fn char_from_utf8(buf: &[u8]) -> Option<char> {
        std::str::from_utf8(buf).ok().and_then(|s| s.chars().next())
    }
    fn find_line_end(&mut self) -> (String, Option<bool>) {
        let mut ret = String::new();
        while let Some(c) = self.get_char() {
            if c == '\n' {
                return (ret, Some(false));
            }
            if c == '\r' {
                match self.get_char() {
                    Some('\n') => return (ret, Some(true)),
                    Some(c) => self.push_char(c),
                    None => (),
                }
                return (ret, Some(true));
            }
            ret.push(c);
        }
        (ret, None)
    }
}
