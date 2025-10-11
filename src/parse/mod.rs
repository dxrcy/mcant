mod tokens;

use std::iter::Peekable;

use mcrs::{Block, Coordinate};

use self::tokens::{TokenKind, Tokens};
use crate::Ant;
use crate::parse::tokens::Token;
use crate::rules::{INITIAL_STATE, Rotation, Rule, Ruleset, Schema};

pub struct Parser<'a> {
    tokens: Peekable<Tokens<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            tokens: Tokens::new(text).peekable(),
        }
    }

    pub fn parse_schema(&mut self) -> Result<Schema, String> {
        let mut ants = Vec::<Ant>::new();
        let mut rulesets = Vec::<Ruleset>::new();

        while !self.is_end() {
            if let Some(ant) = self.try_ant()? {
                ants.push(ant);
                continue;
            }

            if let Some(ruleset) = self.try_ruleset()? {
                if rulesets
                    .iter()
                    .any(|other| other.name.eq_ignore_ascii_case(&ruleset.name))
                {
                    return Err(format!("duplicate ruleset `{}`", ruleset.name));
                }

                rulesets.push(ruleset);
                continue;
            }

            return Err(format!(
                "expected {} or {}, found {}",
                TokenKind::KwRuleset,
                TokenKind::KwAnt,
                self.tokens.peek().unwrap().kind,
            ));
        }

        for ant in &ants {
            if !rulesets
                .iter()
                .any(|ruleset| ruleset.name.eq_ignore_ascii_case(&ant.ruleset))
            {
                return Err(format!("unknown ruleset `{}`", ant.ruleset));
            }
        }

        Ok(Schema { ants, rulesets })
    }

    fn try_ant(&mut self) -> Result<Option<Ant>, String> {
        if self.try_token_kind(TokenKind::KwAnt).is_none() {
            return Ok(None);
        }

        let mut ruleset: Option<String> = None;
        let mut offset: Option<Coordinate> = None;

        while !self
            .tokens
            .peek()
            .is_none_or(|token| token.kind == TokenKind::KwEnd)
        {
            let next = self.tokens.next().unwrap();
            match next.kind {
                TokenKind::KwUse => {
                    let next = self.expect_token_kind(TokenKind::Ident)?;
                    self.expect_token_kind(TokenKind::Semicolon)?;
                    if ruleset.is_some() {
                        return Err(format!("cannot use multiple rulesets for ant"));
                    }
                    ruleset = Some(next.string.to_string());
                }

                TokenKind::KwOffset => {
                    let x = self.expect_number()?;
                    self.expect_token_kind(TokenKind::Comma)?;
                    let y = self.expect_number()?;
                    self.expect_token_kind(TokenKind::Comma)?;
                    let z = self.expect_number()?;
                    self.expect_token_kind(TokenKind::Semicolon)?;
                    if offset.is_some() {
                        return Err(format!("duplicate attribute `offset` for ant"));
                    }
                    offset = Some(Coordinate::new(
                        x.floor() as i32,
                        y.floor() as i32,
                        z.floor() as i32,
                    ));
                }

                _ => {
                    return Err(format!(
                        "expected {} or {}, found {}",
                        TokenKind::KwUse,
                        TokenKind::KwOffset,
                        next.kind,
                    ));
                }
            }
        }
        self.expect_token_kind(TokenKind::KwEnd)?;

        let ruleset = ruleset.ok_or_else(|| format!("missing ruleset for ant"))?;

        Ok(Some(Ant {
            ruleset,
            offset: offset.unwrap_or(Coordinate::new(0, 0, 0)),
            position: Coordinate::new(0, 0, 0),
            facing: Rotation::East,
            state: INITIAL_STATE.to_string(),
            halted: false,
        }))
    }

    fn try_ruleset(&mut self) -> Result<Option<Ruleset>, String> {
        if self.try_token_kind(TokenKind::KwRuleset).is_none() {
            return Ok(None);
        }

        let name = self.expect_token_kind(TokenKind::Ident)?.string.to_string();

        let mut rules = Vec::new();

        while !self
            .tokens
            .peek()
            .is_none_or(|token| token.kind == TokenKind::KwEnd)
        {
            let rule = self.expect_rule()?;
            rules.push(rule);
        }
        self.expect_token_kind(TokenKind::KwEnd)?;

        Ok(Some(Ruleset { name, rules }))
    }

    fn expect_rule(&mut self) -> Result<Rule, String> {
        assert!(!self.is_end());

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

        Ok(Rule {
            from_state,
            from_block,
            from_facing,
            to_state,
            to_block,
            to_facing,
        })
    }

    fn is_end(&mut self) -> bool {
        self.tokens.peek().is_none()
    }

    fn try_token_kind(&mut self, kind: TokenKind) -> Option<Token<'a>> {
        self.tokens.peek().filter(|token| token.kind == kind)?;
        let next = self.tokens.next().unwrap();
        return Some(next);
    }

    fn expect_token_kind(&mut self, kind: TokenKind) -> Result<Token<'a>, String> {
        let Some(next) = self.tokens.next() else {
            return Err(format!("expected {}, found eof", kind));
        };
        if next.kind != kind {
            return Err(format!("expected {}, found {}", kind, next.kind));
        }
        Ok(next)
    }

    fn expect_ident(&mut self) -> Result<&'a str, String> {
        Ok(self.expect_token_kind(TokenKind::Ident)?.string)
    }

    fn expect_number(&mut self) -> Result<f32, String> {
        let string = self.expect_ident()?;
        string
            .parse()
            .map_err(|_| format!("expected number, found non-numeric identifier"))
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
            ("east", Rotation::East),
            ("west", Rotation::West),
            ("south", Rotation::South),
            ("north", Rotation::North),
            ("up", Rotation::Up),
            ("down", Rotation::Down),
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
