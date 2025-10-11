use mcrs::{Block, Coordinate};

pub const INITIAL_STATE: &str = "initial";

#[derive(Debug)]
pub struct Schema {
    pub ants: Vec<Ant>,
    pub rulesets: Vec<Ruleset>,
}

#[derive(Clone, Debug)]
pub struct Ant {
    pub ruleset: String,
    pub offset: Coordinate,
    pub position: Coordinate,
    pub facing: Rotation,
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
    pub from_facing: Vec<Rotation>,
    pub to_state: State,
    pub to_block: Block,
    pub to_facing: Rotation,
}

pub type State = String;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rotation {
    East,
    West,
    South,
    North,
    Up,
    Down,
}

impl Rotation {
    pub fn into_vec3(self) -> [i32; 3] {
        match self {
            Rotation::East => [1, 0, 0],
            Rotation::West => [-1, 0, 0],
            Rotation::South => [0, 0, 1],
            Rotation::North => [0, 0, -1],
            Rotation::Up => [0, 1, 0],
            Rotation::Down => [0, -1, 0],
        }
    }
}
