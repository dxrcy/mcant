mod tokens;

use std::collections::HashMap;
use std::iter::Peekable;
use std::time::Duration;

use mcrs::{Block, Coordinate};

use self::tokens::{TokenKind, Tokens};
use crate::Ant;
use crate::parse::tokens::Token;
use crate::rules::{Direction, INITIAL_STATE, Properties, Rule, Ruleset, Schema};

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
        let mut properties = Properties::default();

        while !self.is_end() {
            if let Some((property, value)) = self.try_property_set()? {
                Self::update_property(&mut properties, property, value)?;
                continue;
            }

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
            Self::ensure_ruleset_exists(&rulesets, ant)?;
        }
        for ruleset in &rulesets {
            for rule in &ruleset.rules {
                if let Some(spawn) = &rule.spawn {
                    Self::ensure_ruleset_exists(&rulesets, spawn)?;
                }
            }
        }

        Ok(Schema {
            ants,
            rulesets,
            properties,
        })
    }

    fn ensure_ruleset_exists(rulesets: &[Ruleset], ant: &Ant) -> Result<(), String> {
        if !rulesets
            .iter()
            .any(|ruleset| ruleset.name.eq_ignore_ascii_case(&ant.ruleset))
        {
            return Err(format!("unknown ruleset `{}`", ant.ruleset));
        }
        Ok(())
    }

    fn update_property(
        properties: &mut Properties,
        property: &str,
        value: &str,
    ) -> Result<(), String> {
        if property.eq_ignore_ascii_case("delay") {
            let millis: u64 = Self::parse_numeric(value)?;
            if properties.delay.is_some() {
                return Err(format!("duplicate property `{}`", property));
            }
            properties.delay = Some(Duration::from_millis(millis));
            return Ok(());
        }

        if property.eq_ignore_ascii_case("cap") {
            let count: usize = Self::parse_numeric(value)?;
            if properties.cap.is_some() {
                return Err(format!("duplicate property `{}`", property));
            }
            properties.cap = Some(count);
            return Ok(());
        }

        Err(format!("unknown property `{}`", property))
    }

    fn try_property_set(&mut self) -> Result<Option<(&'a str, &'a str)>, String> {
        if self.try_token_kind(TokenKind::KwSet).is_none() {
            return Ok(None);
        }

        let property = self.expect_ident_no_expand()?;
        let value = self.expect_ident()?;

        Ok(Some((property, value)))
    }

    fn try_symbol_define(&mut self) -> Result<Option<(&'a str, &'a str)>, String> {
        if self.try_token_kind(TokenKind::KwDefine).is_none() {
            return Ok(None);
        }

        let symbol = self.expect_ident_no_expand()?;
        if !symbol.starts_with('$') {
            return Err(String::from("symbol name must begin with `$`"));
        }

        let symbol = remove_first_char(symbol);
        let definition = self.expect_ident()?;

        Ok(Some((symbol, definition)))
    }

    fn try_ant(&mut self) -> Result<Option<Ant>, String> {
        if self.try_token_kind(TokenKind::KwAnt).is_none() {
            return Ok(None);
        }

        let mut ruleset: Option<String> = None;
        let mut offset: Option<Coordinate> = None;
        let mut facing: Option<Direction> = None;

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
                        return Err(String::from("cannot use multiple rulesets for ant"));
                    }
                    ruleset = Some(next.string.to_string());
                }

                TokenKind::KwOffset => {
                    let x = self.expect_i32()?;
                    self.expect_token_kind(TokenKind::Comma)?;
                    let y = self.expect_i32()?;
                    self.expect_token_kind(TokenKind::Comma)?;
                    let z = self.expect_i32()?;
                    self.expect_token_kind(TokenKind::Semicolon)?;
                    if offset.is_some() {
                        return Err(String::from("duplicate attribute `offset` for ant"));
                    }
                    offset = Some(Coordinate::new(x, y, z));
                }

                TokenKind::KwFacing => {
                    let next = self.expect_token_kind(TokenKind::Ident)?;
                    self.expect_token_kind(TokenKind::Semicolon)?;
                    if facing.is_some() {
                        return Err(String::from("duplicate attribute `facing` for ant"));
                    }
                    facing = Some(
                        Self::parse_direction(next.string)
                            .ok_or_else(|| format!("unknown direction `{}`", next.string))?,
                    );
                    continue;
                }

                _ => {
                    return Err(format!(
                        "expected attribute or {}, found {}",
                        TokenKind::KwEnd,
                        next.kind,
                    ));
                }
            }
        }
        self.expect_token_kind(TokenKind::KwEnd)?;

        let ruleset = ruleset.ok_or_else(|| String::from("missing ruleset for ant"))?;

        const DEFAULT_DIRECTION: Direction = Direction::East;

        Ok(Some(Ant {
            ruleset,
            offset: offset.unwrap_or(Coordinate::new(0, 0, 0)),
            position: Coordinate::new(0, 0, 0),
            facing: facing.unwrap_or(DEFAULT_DIRECTION),
            state: INITIAL_STATE.to_string(),
            halted: false,
            id: 0,
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

        let spawn = if self
            .tokens
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Plus)
        {
            _ = self.tokens.next().unwrap();
            self.expect_token_kind(TokenKind::KwSpawn)?;
            let Some(ant) = self.try_ant()? else {
                return Err(format!("expected `{}`", TokenKind::KwAnt));
            };
            Some(ant)
        } else {
            None
        };

        self.expect_list_end(TokenKind::Semicolon)?;

        Ok(Rule {
            from_state,
            from_block,
            from_facing,
            to_state,
            to_block,
            to_facing,
            spawn,
        })
    }

    fn is_end(&mut self) -> bool {
        self.tokens.peek().is_none()
    }

    fn try_token_kind(&mut self, kind: TokenKind) -> Option<Token<'a>> {
        self.tokens.peek().filter(|token| token.kind == kind)?;
        let next = self.tokens.next().unwrap();
        Some(next)
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

    fn expect_i32(&mut self) -> Result<i32, String> {
        Self::parse_numeric(self.expect_ident()?)
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

    fn parse_numeric<T: std::str::FromStr>(string: &str) -> Result<T, String> {
        string.parse().map_err(|_| {
            if (string.parse() as Result<f64, _>).is_ok() {
                String::from("invalid number value")
            } else {
                String::from("expected number, found non-numeric identifier")
            }
        })
    }

    fn parse_block(string: &str) -> Option<Block> {
        for (name, block) in mcrs::BLOCKS {
            if name.eq_ignore_ascii_case(string) {
                return Some(block);
            }
        }
        None
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
        None
    }
}

fn remove_first_char(string: &str) -> &str {
    let mut chars = string.chars();
    chars.next();
    chars.as_str()
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
            return Some(Err(String::from("expected token, found eof")));
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
