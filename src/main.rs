use std::{fmt, fs, iter::Peekable};

use mcrs::{Block, Coordinate};

#[derive(Debug)]
struct Rule {
    from_block: Vec<Block>,
    from_state: Vec<State>,
    to_block: Block,
    to_facing: Rotation,
    to_state: State,
}

#[derive(Clone, Copy, Debug)]
enum Rotation {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Rotation {
    pub fn add(self, rhs: Self) -> Self {
        Self::from_vec3(Self::rotate_vec3(rhs, self.into_vec3()))
    }

    pub fn into_vec3(self) -> [i32; 3] {
        match self {
            Rotation::PosX => [1, 0, 0],
            Rotation::NegX => [-1, 0, 0],
            Rotation::PosY => [0, 1, 0],
            Rotation::NegY => [0, -1, 0],
            Rotation::PosZ => [0, 0, 1],
            Rotation::NegZ => [0, 0, -1],
        }
    }

    fn from_vec3(vec: [i32; 3]) -> Self {
        match vec {
            [1, 0, 0] => Rotation::PosX,
            [-1, 0, 0] => Rotation::NegX,
            [0, 1, 0] => Rotation::PosY,
            [0, -1, 0] => Rotation::NegY,
            [0, 0, 1] => Rotation::PosZ,
            [0, 0, -1] => Rotation::NegZ,
            _ => unreachable!(),
        }
    }

    fn rotate_vec3(self, vec: [i32; 3]) -> [i32; 3] {
        match self {
            Rotation::PosY => [vec[2], vec[1], -vec[0]],
            Rotation::NegY => [-vec[2], vec[1], vec[0]],
            Rotation::PosX => [vec[0], -vec[2], vec[1]],
            Rotation::NegX => [vec[0], vec[2], -vec[1]],
            Rotation::PosZ => [-vec[1], vec[0], vec[2]],
            Rotation::NegZ => [vec[1], -vec[0], vec[2]],
        }
    }
}

type State = String;

#[derive(Debug)]
struct Ant {
    position: Coordinate,
    facing: Rotation,
    state: State,
}

impl Ant {
    pub fn move_forward(&mut self) {
        let offset: Coordinate = self.facing.into_vec3().into();
        self.position = self.position + offset;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filepath = "./rules";
    let text = fs::read_to_string(filepath)?;

    let mut rules = Vec::new();

    let mut parser = Parser::new(&text);
    while let Some(rule) = parser.next_rule()? {
        rules.push(rule);
    }

    let mut mc = mcrs::Connection::new()?;

    let player = mc.get_player_position()?;

    let mut ant = Ant {
        position: player,
        facing: Rotation::PosX,
        state: "initial".to_string(),
    };

    loop {
        show_ant_indicator(&mut mc, ant.position)?;

        let block = mc.get_block(ant.position)?;

        print!(
            "{} \t{:?} \t{:?} \t{} \t",
            ant.position, ant.state, ant.facing, block
        );

        let Some(rule) = find_rule(&rules, &ant, block) else {
            println!("** HALT **");
            break;
        };

        println!(
            "{:?} \t{:?} \t{}",
            rule.to_state, rule.to_facing, rule.to_block,
        );

        mc.set_block(ant.position, rule.to_block)?;
        ant.state = rule.to_state.clone();
        ant.facing = rule.to_facing;
        ant.move_forward();

        sleep(200);
    }

    Ok(())
}

struct Parser<'a> {
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

#[derive(Debug)]
struct Token<'a> {
    string: &'a str,
    kind: TokenKind,
}

impl<'a> Token<'a> {
    pub fn from(string: &'a str) -> Self {
        Self {
            string,
            kind: TokenKind::from(string),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TokenKind {
    Slash,
    Comma,
    Arrow,
    Semicolon,
    Ident,
}

impl TokenKind {
    pub fn from(string: &str) -> Self {
        match string {
            "/" => Self::Slash,
            "," => Self::Comma,
            "->" => Self::Arrow,
            ";" => Self::Semicolon,
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
            Self::Ident => write!(f, "<identifier>"),
        }
    }
}

struct Tokens<'a> {
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
        let is_symbol = Self::is_symbol(first);

        while let Some(ch) = self.peek_char() {
            if Self::is_whitespace(ch) || Self::is_atomic(ch) || Self::is_symbol(ch) != is_symbol {
                break;
            }
            _ = self.next_char().unwrap();
        }

        &self.text[start..self.cursor]
    }

    fn is_whitespace(ch: char) -> bool {
        ch.is_ascii_whitespace()
    }
    fn is_atomic(ch: char) -> bool {
        matches!(ch, ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}')
    }
    fn is_symbol(ch: char) -> bool {
        matches!(ch, '\x21'..='\x2f' | '\x3a'..='\x40'|'\x5b'..='\x60'|'\x7b'..='\x7e')
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance_until_nonwhitespace();

        if self.is_end() {
            return None;
        }

        let string = self
            .try_atomic()
            .unwrap_or_else(|| self.expect_combination());

        Some(Token::from(string))
    }
}

fn show_ant_indicator(mc: &mut mcrs::Connection, position: Coordinate) -> Result<(), mcrs::Error> {
    // Particle positions get rounded to nearest half-block by Minecraft
    let count: i32 = 1; // Number of particles in cube, per direction
    let radius = 1.0;
    let correction = 0.5; // Offset fix in blocks

    for x in -count..=count {
        for y in -count..=count {
            for z in -count..=count {
                let offset = [
                    (x as f32 / count as f32) * radius,
                    (y as f32 / count as f32) * radius,
                    (z as f32 / count as f32) * radius,
                ];

                if (offset[0].powi(2) + offset[1].powi(2) + offset[2].powi(2)) > radius {
                    continue;
                }

                mc.do_command(format_args!(
                    "particle cloud {} {} {}",
                    position.x as f32 + offset[0] + correction,
                    position.y as f32 + offset[1] + correction,
                    position.z as f32 + offset[2] + correction,
                ))?;
            }
        }
    }

    Ok(())
}

fn find_rule<'a>(rules: &'a [Rule], ant: &Ant, block: Block) -> Option<&'a Rule> {
    for rule in rules {
        if (rule.from_block.is_empty() || rule.from_block.contains(&block))
            && (rule.from_state.is_empty() || rule.from_state.contains(&ant.state))
        {
            return Some(rule);
        }
    }
    None
}

fn sleep(time_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(time_ms))
}
