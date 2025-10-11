mod tokens;

use std::iter::Peekable;

use mcrs::Block;

use self::tokens::{TokenKind, Tokens};
use crate::rules::{Rotation, Rule};

pub struct Parser<'a> {
    tokens: Peekable<Tokens<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            tokens: Tokens::new(text).peekable(),
        }
    }

    pub fn next_rule(&mut self) -> Result<Option<Rule>, String> {
        if self.tokens.peek().is_none() {
            return Ok(None);
        }

        let mut from_state = Vec::new();
        for item in ListParser::new(&mut self.tokens) {
            let item = item?;
            from_state.push(item.to_string());
        }
        self.expect_list_end(TokenKind::Comma)?;

        let mut from_block = Vec::new();
        for item in ListParser::new(&mut self.tokens) {
            let item = item?;
            let Some(block) = Self::parse_block(item) else {
                return Err(format!("unknown block `{}`", item));
            };
            from_block.push(block);
        }
        self.expect_list_end(TokenKind::Comma)?;

        let mut from_facing = Vec::new();
        for item in ListParser::new(&mut self.tokens) {
            let item = item?;
            let Some(facing) = Self::parse_rotation(item) else {
                return Err(format!("unknown rotation `{}`", item));
            };
            from_facing.push(facing);
        }
        self.expect_list_end(TokenKind::Arrow)?;

        let to_state = self.expect_ident()?.to_string();
        self.expect_list_end(TokenKind::Comma)?;

        let to_block_str = self.expect_ident()?;
        let Some(to_block) = Self::parse_block(to_block_str) else {
            return Err(format!("unknown block `{}`", to_block_str));
        };
        self.expect_list_end(TokenKind::Comma)?;

        let to_facing_str = self.expect_ident()?;
        let Some(to_facing) = Self::parse_rotation(to_facing_str) else {
            return Err(format!("unknown rotation `{}`", to_facing_str));
        };
        self.expect_list_end(TokenKind::Semicolon)?;

        Ok(Some(Rule {
            from_state,
            from_block,
            // from_facing,
            to_state,
            to_block,
            to_facing,
        }))
    }

    fn expect_ident(&mut self) -> Result<&'a str, String> {
        let Some(next) = self.tokens.next() else {
            return Err(format!("expected ident, found eof"));
        };
        if next.kind != TokenKind::Ident {
            return Err(format!("expected ident, found {}", next.kind));
        }
        Ok(next.string)
    }

    fn expect_list_end(&mut self, end: TokenKind) -> Result<(), String> {
        let Some(next) = self.tokens.next() else {
            return Err(format!(
                "expected {} or {}, found eof",
                end,
                TokenKind::Slash,
            ));
        };
        if next.kind != end {
            return Err(format!(
                "expected {} or {}, found {}",
                end,
                TokenKind::Slash,
                next.kind
            ));
        }
        Ok(())
    }

    fn parse_block(string: &str) -> Option<Block> {
        for (name, block) in mcrs::BLOCKS {
            if name.eq_ignore_ascii_case(string) {
                return Some(block);
            }
        }
        return None;
    }

    fn parse_rotation(string: &str) -> Option<Rotation> {
        const ROTATIONS: &[(&str, Rotation)] = &[
            ("east", Rotation::PosX),
            ("west", Rotation::NegX),
            ("up", Rotation::PosY),
            ("down", Rotation::NegY),
            ("south", Rotation::PosZ),
            ("north", Rotation::NegZ),
        ];

        for (name, rotation) in ROTATIONS {
            if name.eq_ignore_ascii_case(string) {
                return Some(*rotation);
            }
        }
        return None;
    }
}

struct ListParser<'r, 'a> {
    tokens: &'r mut Peekable<Tokens<'a>>,
    first: bool,
    end: bool,
}

impl<'r, 'a> ListParser<'r, 'a> {
    pub fn new(tokens: &'r mut Peekable<Tokens<'a>>) -> Self {
        Self {
            tokens,
            first: true,
            end: false,
        }
    }
}

impl<'r, 'a> Iterator for ListParser<'r, 'a> {
    type Item = Result<&'a str, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end {
            return None;
        }

        let Some(peek) = self.tokens.peek() else {
            return Some(Err(format!("expected token, found eof")));
        };
        if peek.kind != TokenKind::Ident {
            if self.first {
                return None;
            }
            return Some(Err(format!(
                "expected {} or {}, found {}",
                TokenKind::Ident,
                TokenKind::Comma,
                peek.kind,
            )));
        }

        let next = self.tokens.next().unwrap();

        if self.tokens.peek()?.kind == TokenKind::Slash {
            self.first = false;
            _ = self.tokens.next().unwrap();
        } else {
            self.end = true;
        }

        Some(Ok(next.string))
    }
}
