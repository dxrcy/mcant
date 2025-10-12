mod parse;
mod rules;

use std::fs;
use std::time::Duration;

use mcrs::{Block, Coordinate};

use self::parse::Parser;
use self::rules::{Ant, Rule, Ruleset, Schema};

const DEFAULT_DELAY: Duration = Duration::from_millis(100);
const DEAFULT_CAP: usize = 50;

const COLORS: &[(f32, f32, f32)] = &[
    (1.0, 0.0, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, 0.0, 1.0),
    (0.0, 1.0, 1.0),
    (1.0, 0.0, 1.0),
    (1.0, 1.0, 0.0),
    (1.0, 0.5, 0.5),
    (0.5, 1.0, 0.5),
    (0.5, 0.5, 1.0),
    (0.5, 1.0, 1.0),
    (1.0, 0.5, 1.0),
    (1.0, 1.0, 0.5),
    (0.5, 0.0, 0.0),
    (0.0, 0.5, 0.0),
    (0.0, 0.0, 0.5),
    (0.0, 0.5, 0.5),
    (0.5, 0.0, 0.5),
    (0.5, 0.5, 0.0),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    args.next();

    let filepath = args.next().ok_or("missing filepath")?;

    let text = fs::read_to_string(filepath)?;

    let mut parser = Parser::new(&text);
    let schema = parser.parse_schema()?;

    let mut mc = mcrs::Connection::new()?;

    let player = mc.get_player_position()?;

    let mut max_id = 0;

    let mut ants = schema.ants.clone();
    for ant in &mut ants {
        ant.position = player + ant.offset;
        ant.id = max_id;
        max_id += 1;
    }

    let delay = schema.properties.delay.unwrap_or(DEFAULT_DELAY);
    let cap = schema.properties.cap.unwrap_or(DEAFULT_CAP);

    while !ants.iter().all(|ant| ant.halted) {
        while ants.len() > cap {
            ants.remove(0);
        }

        for ant in ants.iter().filter(|ant| !ant.halted) {
            show_ant_indicator(&mut mc, ant, delay)?;
        }

        std::thread::sleep(delay);

        let len = ants.len();
        for i in 0..len {
            let ant = &mut ants[i];
            if ant.halted {
                continue;
            }

            let block = mc.get_block(ant.position)?;

            print!(
                "{:2} \t{} \t{} \t{:?} \t{} \t",
                i,
                ant.position,
                ant.state,
                ant.facing,
                block.get_name().unwrap_or("[unknown]"),
            );

            let Some(rule) = find_rule(&schema, ant, block) else {
                println!("====[ HALT ]====");
                ant.halted = true;
                break;
            };

            print!("{} \t", rule.to_state);
            if let Some(to_facing) = rule.to_facing {
                print!("{:?}", to_facing);
            } else {
                print!("-");
            }
            print!(" \t");
            if let Some(to_block) = rule.to_block {
                print!("{}", to_block.get_name().unwrap_or("[unknown]"));
            } else {
                print!("-");
            }
            println!();

            let previous_position = ant.position;
            if let Some(to_block) = rule.to_block {
                mc.set_block(ant.position, to_block)?;
            }
            ant.state = rule.to_state.clone();
            if let Some(to_facing) = rule.to_facing {
                ant.facing = to_facing;
            }
            ant.move_forward();

            if let Some(spawn) = &rule.spawn {
                let mut child = spawn.clone();
                child.position = previous_position;
                child.id = max_id;
                max_id += 1;
                ants.push(child);
            }
        }

        std::thread::sleep(schema.properties.delay.unwrap_or(DEFAULT_DELAY));
    }

    Ok(())
}

fn show_ant_indicator(
    mc: &mut mcrs::Connection,
    ant: &Ant,
    delay: Duration,
) -> Result<(), mcrs::Error> {
    let color = COLORS[ant.id % COLORS.len()];

    let modifier = delay.as_millis() as f32 * 0.010;

    create_block_particle(mc, ant.position, color, 4, 0.4, 0.5, 0.6 * modifier, false)?;
    create_block_particle(mc, ant.position, color, 2, 0.8, 0.5, 1.5 * modifier, true)?;

    Ok(())
}

fn create_block_particle(
    mc: &mut mcrs::Connection,
    position: Coordinate,
    // RGB
    color: (f32, f32, f32),
    // Number of particles in cube, per direction
    count: i32,
    // Size of block (half)
    radius: f32,
    // Offset fix in blocks
    correction: f32,
    // Larger particle size means longer duration
    size: f32,
    // Show particles as a sphere, not a cube
    round: bool,
) -> Result<(), mcrs::Error> {
    // Particle positions get rounded to nearest half-block by Minecraft

    for x in -count..=count {
        for y in -count..=count {
            for z in -count..=count {
                let offset = [
                    (x as f32 / count as f32) * radius,
                    (y as f32 / count as f32) * radius,
                    (z as f32 / count as f32) * radius,
                ];

                if round && (offset[0].powi(2) + offset[1].powi(2) + offset[2].powi(2)) > radius {
                    continue;
                }

                mc.do_command(format_args!(
                    // Indirect execution to stop errors being spammed to player's chat
                    "execute at @a run particle dust {r} {g} {b} {size} {x} {y} {z}",
                    r = color.0,
                    g = color.1,
                    b = color.2,
                    size = size,
                    x = position.x as f32 + offset[0] + correction,
                    y = position.y as f32 + offset[1] + correction,
                    z = position.z as f32 + offset[2] + correction,
                ))?;
            }
        }
    }

    Ok(())
}

fn find_rule<'a>(schema: &'a Schema, ant: &Ant, block: Block) -> Option<&'a Rule> {
    let ruleset = find_ruleset(schema, ant)?;
    ruleset.rules.iter().find(|rule| {
        (rule.from_state.is_empty() || rule.from_state.contains(&ant.state))
            && (rule.from_block.is_empty() || rule.from_block.contains(&block))
            && (rule.from_facing.is_empty() || rule.from_facing.contains(&ant.facing))
    })
}

fn find_ruleset<'a>(schema: &'a Schema, ant: &Ant) -> Option<&'a Ruleset> {
    schema
        .rulesets
        .iter()
        .find(|ruleset| ruleset.name.eq_ignore_ascii_case(&ant.ruleset))
}
