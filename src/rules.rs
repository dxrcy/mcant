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
    pub from_block: Vec<Block>,
    pub from_state: Vec<State>,
    pub to_block: Block,
    pub to_facing: Rotation,
    pub to_state: State,
}

pub type State = String;

#[derive(Clone, Copy, Debug)]
pub enum Rotation {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Rotation {
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
}
