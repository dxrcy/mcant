mod parse;
mod rules;

use std::fs;
use std::time::Duration;

use mcrs::{Block, Coordinate};

use self::parse::Parser;
use self::rules::{Ant, Rule, Ruleset, Schema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    args.next();

    let filepath = args.next().ok_or("missing filepath")?;

    let text = fs::read_to_string(filepath)?;

    let mut parser = Parser::new(&text);
    let schema = parser.parse_schema()?;

    let mut mc = mcrs::Connection::new()?;

    let player = mc.get_player_position()?;

    let mut ants = schema.ants.clone();
    for ant in &mut ants {
        ant.position = player + ant.offset;
    }

    const DEFAULT_DELAY: Duration = Duration::from_millis(100);

    while !ants.iter().all(|ant| ant.halted) {

        let len = ants.len();

        for i in 0..len {
            let ant = &mut ants[i];
            if ant.halted {
                continue;
            }

            show_ant_indicator(&mut mc, i, ant.position)?;

            let block = mc.get_block(ant.position)?;

            print!(
                "{:2} \t{} \t{} \t{:?} \t{} \t",
                i,
                ant.position,
                ant.state,
                ant.facing,
                block.get_name().unwrap_or("[unknown]"),
            );

            let Some(rule) = find_rule(&schema, &ant, block) else {
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
                child.position = ant.position;
                ants.push(child);
            }
        }

        std::thread::sleep(schema.properties.delay.unwrap_or(DEFAULT_DELAY));
    }

    Ok(())
}

fn show_ant_indicator(
    mc: &mut mcrs::Connection,
    index: usize,
    position: Coordinate,
) -> Result<(), mcrs::Error> {
    // Particle positions get rounded to nearest half-block by Minecraft
    let count: i32 = 3; // Number of particles in cube, per direction
    let radius = 1.0;
    let correction = 0.5; // Offset fix in blocks
    let size = 1.5; // Larger particle size means longer duration

    let colors = [
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

    let color = colors[index % colors.len()];

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
    for rule in &ruleset.rules {
        if (rule.from_state.is_empty() || rule.from_state.contains(&ant.state))
            && (rule.from_block.is_empty() || rule.from_block.contains(&block))
            && (rule.from_facing.is_empty() || rule.from_facing.contains(&ant.facing))
        {
            return Some(rule);
        }
    }
    None
}

fn find_ruleset<'a>(schema: &'a Schema, ant: &Ant) -> Option<&'a Ruleset> {
    for ruleset in &schema.rulesets {
        if ruleset.name.eq_ignore_ascii_case(&ant.ruleset) {
            return Some(ruleset);
        }
    }
    None
}
