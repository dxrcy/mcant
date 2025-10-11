mod tokens;

use std::collections::HashMap;
use std::iter::Peekable;

use mcrs::{Block, Coordinate};

use self::tokens::{TokenKind, Tokens};
use crate::Ant;
use crate::parse::tokens::Token;
use crate::rules::{Direction, INITIAL_STATE, Rule, Ruleset, Schema};

pub struct Parser<'a> {
    tokens: Peekable<Tokens<'a>>,
    symbols: HashMap<&'a str, &'a str>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            tokens: Tokens::new(text).peekable(),
            symbols: HashMap::new(),
        }
    }

    pub fn parse_schema(&mut self) -> Result<Schema, String> {
        let mut ants = Vec::<Ant>::new();
        let mut rulesets = Vec::<Ruleset>::new();

        while !self.is_end() {
            if let Some((symbol, definition)) = self.try_symbol_define()? {
                if self.symbols.contains_key(&symbol) {
                    return Err(format!("redefinition of symbol `{}`", symbol));
                }
                self.symbols.insert(symbol, definition);
                continue;
            };

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

    fn try_symbol_define(&mut self) -> Result<Option<(&'a str, &'a str)>, String> {
        if self.try_token_kind(TokenKind::KwDefine).is_none() {
            return Ok(None);
        }

        let symbol = remove_first_char(self.expect_ident_no_expand()?);
        let definition = self.expect_ident()?;

        Ok(Some((symbol, definition)))
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
            facing: Direction::East,
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
        for item in ListParser::new(self) {
            let item = item?;
            from_state.push(item.to_string());
        }
        self.expect_list_end(TokenKind::Comma)?;

        let mut from_block = Vec::new();
        for item in ListParser::new(self) {
            let item = item?;
            let Some(block) = Self::parse_block(item) else {
                return Err(format!("unknown block `{}`", item));
            };
            from_block.push(block);
        }
        self.expect_list_end(TokenKind::Comma)?;

        let mut from_facing = Vec::new();
        for item in ListParser::new(self) {
            let item = item?;
            let Some(facing) = Self::parse_direction(item) else {
                return Err(format!("unknown direction `{}`", item));
            };
            from_facing.push(facing);
        }
        self.expect_list_end(TokenKind::Arrow)?;

        let to_state = self.expect_ident()?.to_string();
        self.expect_list_end(TokenKind::Comma)?;

        let to_block = match self.try_ident() {
            None => None,
            Some(ident) => {
                let ident = ident?;
                Some(Self::parse_block(ident).ok_or_else(|| format!("unknown block `{}`", ident))?)
            }
        };
        self.expect_list_end(TokenKind::Comma)?;

        let to_facing = match self.try_ident() {
            None => None,
            Some(ident) => {
                let ident = ident?;
                Some(
                    Self::parse_direction(ident)
                        .ok_or_else(|| format!("unknown direction `{}`", ident))?,
                )
            }
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

    fn try_ident(&mut self) -> Option<Result<&'a str, String>> {
        let ident = self.try_token_kind(TokenKind::Ident)?.string;
        Some(self.expand_ident(ident))
    }

    fn expect_ident(&mut self) -> Result<&'a str, String> {
        let ident = self.expect_ident_no_expand()?;
        self.expand_ident(ident)
    }

    fn expect_ident_no_expand(&mut self) -> Result<&'a str, String> {
        Ok(self.expect_token_kind(TokenKind::Ident)?.string)
    }

    fn expand_ident(&self, ident: &'a str) -> Result<&'a str, String> {
        if !ident.starts_with('$') {
            return Ok(ident);
        }

        let symbol = remove_first_char(ident);
        let Some(expansion) = self.symbols.get(symbol) else {
            return Err(format!("undefined symbol `{}`", symbol));
        };
        Ok(expansion)
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

    fn parse_direction(string: &str) -> Option<Direction> {
        const DIRECTIONS: &[(&str, Direction)] = &[
            ("east", Direction::East),
            ("west", Direction::West),
            ("south", Direction::South),
            ("north", Direction::North),
            ("up", Direction::Up),
            ("down", Direction::Down),
        ];

        for (name, direction) in DIRECTIONS {
            if name.eq_ignore_ascii_case(string) {
                return Some(*direction);
            }
        }
        return None;
    }
}

fn remove_first_char(string: &str) -> &str {
    let mut chars = string.chars();
    chars.next();
    return chars.as_str();
}

struct ListParser<'r, 'a> {
    // tokens: &'r mut Peekable<Tokens<'a>>,
    parser: &'r mut Parser<'a>,
    first: bool,
    end: bool,
}

impl<'r, 'a> ListParser<'r, 'a> {
    pub fn new(parser: &'r mut Parser<'a>) -> Self {
        Self {
            parser,
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

        let Some(peek) = self.parser.tokens.peek() else {
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

        let next = self.parser.tokens.next().unwrap();

        if self.parser.tokens.peek()?.kind == TokenKind::Slash {
            self.first = false;
            _ = self.parser.tokens.next().unwrap();
        } else {
            self.end = true;
        }

        Some(self.parser.expand_ident(next.string))
    }
}
