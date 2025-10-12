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
    Plus,
    KwEnd,
    KwSet,
    KwDefine,
    KwAnt,
    KwRuleset,
    KwUse,
    KwOffset,
    KwDirection,
    KwSpawn,
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
            "+" => Self::Plus,
            ";" => Self::Semicolon,
            "end" => Self::KwEnd,
            "set" => Self::KwSet,
            "define" => Self::KwDefine,
            "ant" => Self::KwAnt,
            "ruleset" => Self::KwRuleset,
            "use" => Self::KwUse,
            "offset" => Self::KwOffset,
            "direction" => Self::KwDirection,
            "spawn" => Self::KwSpawn,
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
            Self::Plus => write!(f, "`+`"),
            Self::Semicolon => write!(f, "`;`"),
            Self::KwEnd => write!(f, "`end`"),
            Self::KwSet => write!(f, "`set`"),
            Self::KwDefine => write!(f, "`define`"),
            Self::KwAnt => write!(f, "`ant`"),
            Self::KwRuleset => write!(f, "`ruleset`"),
            Self::KwUse => write!(f, "`use`"),
            Self::KwOffset => write!(f, "`offset`"),
            Self::KwDirection => write!(f, "`direction`"),
            Self::KwSpawn => write!(f, "`spawn`"),
            Self::Ident => write!(f, "<identifier>"),
        }
    }
}

const COMMENT_START: &str = "--";

#[derive(Clone, Copy, Debug, PartialEq)]
enum CharKind {
    Whitespace,
    Atomic,
    Any,
    Combining { is_symbol: bool },
}

impl CharKind {
    fn from(ch: char) -> Self {
        match ch {
            _ if ch.is_ascii_whitespace() => Self::Whitespace,
            ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' => Self::Atomic,
            '-' => Self::Any,
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$' => Self::Combining { is_symbol: false },
            _ => Self::Combining { is_symbol: true },
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

    fn peek_is_comment(&self) -> bool {
        self.text[self.cursor..].starts_with(COMMENT_START)
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
            if CharKind::from(ch) != CharKind::Whitespace {
                break;
            }
            _ = self.next_char();
        }
    }

    fn try_atomic(&mut self) -> Option<&'a str> {
        assert!(!self.is_end());

        let first = self.peek_char().unwrap();
        match CharKind::from(first) {
            CharKind::Whitespace => unreachable!(),
            CharKind::Atomic => (),
            _ => return None,
        }

        let start = self.cursor;
        _ = self.next_char().unwrap();

        Some(&self.text[start..self.cursor])
    }

    fn expect_combination(&mut self) -> &'a str {
        assert!(!self.is_end());

        let start = self.cursor;
        let first = self.next_char().unwrap();

        let mut token_is_symbol = match CharKind::from(first) {
            CharKind::Whitespace => unreachable!(),
            CharKind::Atomic => unreachable!(),
            CharKind::Any => None,
            CharKind::Combining { is_symbol } => Some(is_symbol),
        };

        while let Some(ch) = self.peek_char() {
            match CharKind::from(ch) {
                CharKind::Whitespace => break,
                CharKind::Atomic => break,
                CharKind::Any => (),
                CharKind::Combining { is_symbol } => {
                    if let Some(token_is_symbol) = token_is_symbol {
                        if token_is_symbol != is_symbol {
                            break;
                        }
                    } else {
                        token_is_symbol = Some(is_symbol);
                    }
                }
            }

            _ = self.next_char().unwrap();
        }

        &self.text[start..self.cursor]
    }

    fn is_linebreak(ch: char) -> bool {
        ch == '\n'
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

            // Start of comment: skip rest of line and try again
            if self.peek_is_comment() {
                self.advance_until_next_line();
                continue;
            }

            let string = self
                .try_atomic()
                .unwrap_or_else(|| self.expect_combination());

            return Some(Token::from(string));
        }
    }
}
