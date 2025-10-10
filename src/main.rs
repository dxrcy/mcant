use mcrs::{Block, Coordinate};

#[derive(Debug)]
struct Rule {
    from_block: &'static [Block],
    from_state: &'static [State],
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Searching,
    Up,
    Out,
}

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

fn main() -> Result<(), mcrs::Error> {
    let rules = [
        Rule {
            from_block: &[Block::AIR],
            from_state: &[State::Searching],
            to_block: Block::AIR,
            to_facing: Rotation::NegY,
            to_state: State::Searching,
        },
        Rule {
            from_block: &[Block::DIRT, Block::GRASS],
            from_state: &[State::Searching],
            to_block: Block::STONE,
            to_facing: Rotation::PosY,
            to_state: State::Up,
        },
        Rule {
            from_block: &[Block::AIR],
            from_state: &[State::Up],
            to_block: Block::STONE,
            to_facing: Rotation::PosX,
            to_state: State::Out,
        },
        Rule {
            from_block: &[Block::AIR],
            from_state: &[State::Out],
            to_block: Block::STONE,
            to_facing: Rotation::PosY,
            to_state: State::Up,
        },
    ];

    let mut mc = mcrs::Connection::new()?;

    let player = mc.get_player_position()?;

    let mut ant = Ant {
        position: player,
        facing: Rotation::PosX,
        state: State::Searching,
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
        ant.state = rule.to_state;
        ant.facing = rule.to_facing;
        ant.move_forward();

        sleep(200);
    }

    Ok(())
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
        if rule.from_block.contains(&block) && rule.from_state.contains(&ant.state) {
            return Some(rule);
        }
    }
    None
}

fn sleep(time_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(time_ms))
}
