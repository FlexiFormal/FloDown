use std::borrow::Cow;

use crate::sourcerefs::SourcePos;

pub trait Str<'a>:
    AsRef<str>
    + From<&'a str>
    + std::fmt::Debug
    + std::fmt::Display
    + Eq
    + std::hash::Hash
    + Clone
    + for<'b> PartialEq<&'b str>
{
    /// # Errors
    ///
    /// Will return `Err` if self does not start with prefix.
    fn strip_prefix(self, s: &str) -> Result<Self, Self>;
    #[must_use]
    fn split_n(self, n: usize) -> (Self, Self);
    fn trim_ws(&mut self);
    fn split_noparens(
        &'a self,
        open: char,
        close: char,
        split_char: char,
    ) -> impl Iterator<Item = &'a str>;
    fn as_cow(&self) -> Cow<'a, str>;
}
impl<'a> Str<'a> for &'a str {
    #[inline]
    fn strip_prefix(self, s: &str) -> Result<Self, Self> {
        str::strip_prefix(self, s).map(str::trim_start).ok_or(self)
    }
    #[inline]
    fn split_n(self, n: usize) -> (Self, Self) {
        (&self[..n], &self[n..])
    }
    #[inline]
    fn trim_ws(&mut self) {
        *self = self.trim();
    }
    fn split_noparens(
        &'a self,
        open: char,
        close: char,
        split_char: char,
    ) -> impl Iterator<Item = &'a str> {
        let mut depth = 0;
        self.split(move |c: char| {
            if c == open {
                depth += 1;
                false
            } else if c == close && depth > 0 {
                depth -= 1;
                false
            } else if depth > 0 {
                false
            } else {
                c == split_char
            }
        })
    }
    #[inline]
    fn as_cow(&self) -> Cow<'a, str> {
        Cow::Borrowed(self)
    }
}
impl<'a> Str<'a> for String {
    #[allow(clippy::option_if_let_else)]
    fn strip_prefix(self, s: &str) -> Result<Self, Self> {
        match str::strip_prefix(&self, s) {
            Some(s) => Ok(s.trim_start().to_string()),
            None => Err(self),
        }
    }
    #[inline]
    fn trim_ws(&mut self) {
        *self = self.trim().to_string();
    }
    fn split_n(mut self, n: usize) -> (Self, Self) {
        let r = self.split_off(n);
        (self, r)
    }
    fn split_noparens(
        &'a self,
        open: char,
        close: char,
        split_char: char,
    ) -> impl Iterator<Item = &'a str> {
        let mut depth = 0;
        self.split(move |c: char| {
            if c == open {
                depth += 1;
                false
            } else if c == close && depth > 0 {
                depth -= 1;
                false
            } else if depth > 0 {
                false
            } else {
                c == split_char
            }
        })
    }
    #[inline]
    fn as_cow(&self) -> Cow<'a, str> {
        Cow::Owned(self.clone())
    }
}

pub trait ParseSource<'a>: 'a {
    type Pos: SourcePos;
    type Str: Str<'a>;
    type Source;
    fn source(&self) -> &Self::Source;
    fn curr_pos(&self) -> Self::Pos;
    fn pop_head(&mut self) -> Option<char>;
    fn read_until_line_end(&mut self) -> (Self::Str, Self::Pos);
    fn trim_start(&mut self);
    fn starts_with(&mut self, c: char) -> bool;
    fn peek_head(&mut self) -> Option<char>;
    fn read_n(&mut self, i: usize) -> Self::Str;
    fn read_while(&mut self, pred: impl FnMut(char) -> bool) -> Self::Str;
    #[inline]
    fn read_until(&mut self, mut pred: impl FnMut(char) -> bool) -> Self::Str {
        self.read_while(|c| !pred(c))
    }
    fn read_until_str(&mut self, s: &str) -> Self::Str;
    fn read_until_with_brackets(
        &mut self,
        open: char,
        close: char,
        pred: impl FnMut(char) -> bool,
    ) -> Self::Str;
    fn skip(&mut self, i: usize);
}
