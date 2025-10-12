use std::time::Duration;

use mcrs::{Block, Coordinate};

pub const INITIAL_STATE: &str = "initial";

#[derive(Debug)]
pub struct Schema {
    pub ants: Vec<Ant>,
    pub rulesets: Vec<Ruleset>,
    pub properties: Properties,
}

#[derive(Clone, Debug)]
pub struct Ant {
    pub ruleset: String,
    pub offset: Coordinate,
    pub position: Coordinate,
    pub facing: Direction,
    pub state: State,
    pub halted: bool,
}

impl Ant {
    pub fn move_forward(&mut self) {
        let offset: Coordinate = self.facing.into_vec3().into();
        self.position = self.position + offset;
    }
}

#[derive(Debug)]
pub struct Ruleset {
    pub name: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Rule {
    pub from_state: Vec<State>,
    pub from_block: Vec<Block>,
    pub from_facing: Vec<Direction>,
    pub to_state: State,
    pub to_block: Option<Block>,
    pub to_facing: Option<Direction>,
    pub spawn: Option<Ant>,
}

#[derive(Debug, Default)]
pub struct Properties {
    pub delay: Option<Duration>,
}

pub type State = String;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    East,
    West,
    South,
    North,
    Up,
    Down,
}

impl Direction {
    pub fn into_vec3(self) -> [i32; 3] {
        match self {
            Direction::East => [1, 0, 0],
            Direction::West => [-1, 0, 0],
            Direction::South => [0, 0, 1],
            Direction::North => [0, 0, -1],
            Direction::Up => [0, 1, 0],
            Direction::Down => [0, -1, 0],
        }
    }
}
