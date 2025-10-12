use std::fmt;

#[derive(Debug)]
pub struct Token<'a> {
    pub string: &'a str,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind {
    Slash,
    Comma,
    Arrow,
    Semicolon,
    KwEnd,
    KwSet,
    KwDefine,
    KwAnt,
    KwRuleset,
    KwUse,
    KwOffset,
    Ident,
}

impl<'a> Token<'a> {
    pub fn from(string: &'a str) -> Self {
        Self {
            string,
            kind: TokenKind::from(string),
        }
    }
}

impl TokenKind {
    pub fn from(string: &str) -> Self {
        match string {
            "/" => Self::Slash,
            "," => Self::Comma,
            "->" => Self::Arrow,
            ";" => Self::Semicolon,
            "end" => Self::KwEnd,
            "set" => Self::KwSet,
            "define" => Self::KwDefine,
            "ant" => Self::KwAnt,
            "ruleset" => Self::KwRuleset,
            "use" => Self::KwUse,
            "offset" => Self::KwOffset,
            _ => Self::Ident,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Slash => write!(f, "`/`"),
            Self::Comma => write!(f, "`,`"),
            Self::Arrow => write!(f, "`->`"),
            Self::Semicolon => write!(f, "`;`"),
            Self::KwEnd => write!(f, "`end`"),
            Self::KwSet => write!(f, "`set`"),
            Self::KwDefine => write!(f, "`define`"),
            Self::KwAnt => write!(f, "`ant`"),
            Self::KwRuleset => write!(f, "`ruleset`"),
            Self::KwUse => write!(f, "`use`"),
            Self::KwOffset => write!(f, "`offset`"),
            Self::Ident => write!(f, "<identifier>"),
        }
    }
}

pub struct Tokens<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> Tokens<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn is_end(&self) -> bool {
        self.cursor >= self.text.len()
    }

    fn peek_char(&self) -> Option<char> {
        let mut chars = self.text[self.cursor..].chars();
        let ch = chars.next()?;
        Some(ch)
    }

    fn next_char(&mut self) -> Option<char> {
        let mut chars = self.text[self.cursor..].chars();
        let ch = chars.next()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn advance_until_next_line(&mut self) {
        while let Some(ch) = self.next_char() {
            if Self::is_linebreak(ch) {
                break;
            }
        }
    }

    fn advance_until_nonwhitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !Self::is_whitespace(ch) {
                break;
            }
            _ = self.next_char();
        }
    }

    fn try_atomic(&mut self) -> Option<&'a str> {
        assert!(!self.is_end());

        let first = self.peek_char().unwrap();
        assert!(!Self::is_whitespace(first));
        if !Self::is_atomic(first) {
            return None;
        }

        let start = self.cursor;
        _ = self.next_char().unwrap();

        Some(&self.text[start..self.cursor])
    }

    fn expect_combination(&mut self) -> &'a str {
        assert!(!self.is_end());

        let start = self.cursor;
        let first = self.next_char().unwrap();
        assert!(!Self::is_whitespace(first));

        // Treat a `-` before a digit as non-symbol, to support negative numeric literals
        let is_symbol = if first == '-' && self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            false
        } else {
            Self::is_symbol(first)
        };

        while let Some(ch) = self.peek_char() {
            if Self::is_whitespace(ch) || Self::is_atomic(ch) || Self::is_symbol(ch) != is_symbol {
                break;
            }
            _ = self.next_char().unwrap();
        }

        &self.text[start..self.cursor]
    }

    fn is_linebreak(ch: char) -> bool {
        ch == '\n'
    }
    fn is_whitespace(ch: char) -> bool {
        ch.is_ascii_whitespace()
    }
    fn is_atomic(ch: char) -> bool {
        matches!(ch, ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}')
    }
    fn is_symbol(ch: char) -> bool {
        !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.advance_until_nonwhitespace();

            if self.is_end() {
                return None;
            }

            let string = self
                .try_atomic()
                .unwrap_or_else(|| self.expect_combination());

            // Start of comment: skip rest of line and try again
            if string.starts_with("--") {
                self.advance_until_next_line();
                continue;
            }

            return Some(Token::from(string));
        }
    }
}
