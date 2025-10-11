mod parse;
mod rules;

use std::fs;

use mcrs::{Block, Coordinate};

use self::parse::Parser;
use self::rules::{Rotation, Rule, State};

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

    let mut parser = Parser::new(&text);
    let schema = parser.parse_schema()?;

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

        let Some(rule) = find_rule(&schema.rulesets[0].rules, &ant, block) else {
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
