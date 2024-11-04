use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Stdio},
};

use clap_repl::ReadCommandOutput;
use enum_map::enum_map;
use game_def::{Action, Card, Player, ResourceKind, ResourceMap, State};

enum Agent {
    Human {
        name: String,
    },
    AI {
        process: Child,
        reader: BufReader<ChildStdout>,
        writer: ChildStdin,
        name: String,
    },
}

impl Agent {
    fn name(&self) -> String {
        match self {
            Agent::Human { name } => format!("Human {name}"),
            Agent::AI { name, .. } => format!("AI {name}"),
        }
    }
}

fn main() {
    let mut agents = std::env::args()
        .skip(1)
        .map(|arg| {
            if let Some(name) = arg.strip_prefix("human-") {
                Agent::Human {
                    name: name.to_owned(),
                }
            } else {
                let mut process = std::process::Command::new(&arg)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                let reader = BufReader::new(process.stdout.take().unwrap());
                let writer = process.stdin.take().unwrap();
                Agent::AI {
                    process,
                    reader,
                    writer,
                    name: arg,
                }
            }
        })
        .collect::<Vec<_>>();
    if agents.len() < 2 {
        println!("{} agent is not enough", agents.len());
        return;
    }
    let mut state = State {
        decks: vec![
            vec![
                Card::new(
                    ResourceKind::Blue,
                    0,
                    ResourceMap(enum_map! {
                        ResourceKind::Red => 0,
                        ResourceKind::Blue => 0,
                        ResourceKind::Yellow => 1,
                        ResourceKind::White => 2,
                        ResourceKind::Brown => 0,
                    }),
                );
                10
            ],
            vec![
                Card::new(
                    ResourceKind::Blue,
                    1,
                    ResourceMap(enum_map! {
                        ResourceKind::Red => 0,
                        ResourceKind::Blue => 2,
                        ResourceKind::Yellow => 1,
                        ResourceKind::White => 2,
                        ResourceKind::Brown => 0,
                    }),
                );
                10
            ],
            vec![
                Card::new(
                    ResourceKind::Blue,
                    4,
                    ResourceMap(enum_map! {
                        ResourceKind::Red => 0,
                        ResourceKind::Blue => 7,
                        ResourceKind::Yellow => 0,
                        ResourceKind::White => 0,
                        ResourceKind::Brown => 0,
                    }),
                );
                10
            ],
        ],
        players: agents.iter().map(|a| Player::new(&a.name())).collect(),
        coins: ResourceMap(enum_map! {
            ResourceKind::Red => 5,
            ResourceKind::Blue => 5,
            ResourceKind::Yellow => 5,
            ResourceKind::White => 5,
            ResourceKind::Brown => 5,
        }),
        turn: 0,
        wilds: 5,
    };

    let mut ed = clap_repl::ClapEditor::<Action>::builder().build();
    state.print();
    loop {
        let agent = &mut agents[state.turn];
        match agent {
            Agent::Human { .. } => match ed.read_command() {
                ReadCommandOutput::Command(action) => {
                    let mut s = state.clone();
                    if let Err(e) = s.run(action) {
                        println!("Error: {e:?}");
                        continue;
                    }
                    state = s;
                    state.print();
                }
                ReadCommandOutput::EmptyLine => (),
                ReadCommandOutput::ClapError(e) => {
                    e.print().unwrap();
                }
                ReadCommandOutput::ShlexError => {
                    println!(
                        "{} input was not valid and could not be processed",
                        "Error:", //style("Error:").red().bold()
                    );
                }
                ReadCommandOutput::ReedlineError(e) => {
                    panic!("{e}");
                }
                ReadCommandOutput::CtrlC | ReadCommandOutput::CtrlD => {
                    println!("End game requested by human player");
                    break;
                }
            },
            Agent::AI { writer, reader, .. } => {
                println!("AI Thinking...");
                writeln!(writer, "{}", state.json()).unwrap();
                let mut result = String::new();
                reader.read_line(&mut result).unwrap();
                let action: Action = serde_json::from_str(&result).unwrap();
                println!("{} did {:?}", agent.name(), action);
                if let Err(e) = state.run(action) {
                    println!("AI did invalid action: {e:?}");
                    println!("Terminating game");
                    break;
                }
                state.print();
            }
        }
    }
}
