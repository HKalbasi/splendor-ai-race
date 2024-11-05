use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Stdio},
};

use clap_repl::ReadCommandOutput;
use enum_map::enum_map;
use game_def::{Action, Card, Nobel, Player, ResourceKind, ResourceMap, State};
use rand::seq::SliceRandom;

enum Agent {
    Human {
        name: String,
    },
    AI {
        #[allow(unused)]
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
    let deck0 = enum_map![
        ResourceKind::Black => vec![
            (0, "1w+1u+1g+1r"),
            (0, "1w+2u+1g+1r"),
            (0, "2w+2u+1r"),
            (0, "1g+3r+1k"),
            (0, "2g+1r"),
            (0, "2w+2g"),
            (0, "3g"),
            (1, "4u"),
        ],
        ResourceKind::Blue => vec![
            (0, "1w+1g+1r+1k"),
            (0, "1w+1g+2r+1k"),
            (0, "1w+2g+2r"),
            (0, "1u+3g+1r"),
            (0, "1w+2k"),
            (0, "2g+2k"),
            (0, "3k"),
            (1, "4r"),
        ],
        ResourceKind::White => vec![
            (0, "1u+1g+1r+1k"),
            (0, "1u+2g+1r+1k"),
            (0, "2u+2g+1k"),
            (0, "3w+1u+1k"),
            (0, "2r+1k"),
            (0, "2u+2k"),
            (0, "3u"),
            (1, "4g"),
        ],
        ResourceKind::Green => vec![
            (0, "1w+1u+1r+1k"),
            (0, "1w+1u+1r+2k"),
            (0, "1u+2r+2k"),
            (0, "1w+3u+1g"),
            (0, "2w+1u"),
            (0, "2u+2r"),
            (0, "3r"),
            (1, "4k"),
        ],
        ResourceKind::Red => vec![
            (0, "1w+1u+1g+1k"),
            (0, "2w+1u+1g+1k"),
            (0, "2w+1g+2k"),
            (0, "1w+1r+3k"),
            (0, "2u+1g"),
            (0, "2w+2r"),
            (0, "3w"),
            (1, "4w"),
        ],
    ];
    let deck1 = enum_map![
        ResourceKind::Black => vec![
            (1, "3w+2u+2g"),
            (1, "3w+3g+2k"),
            (2, "1u+4g+2r"),
            (2, "5g+3r"),
            (2, "5w"),
            (3, "6k"),
        ],
        ResourceKind::Blue => vec![
            (1, "2u+2g+3r"),
            (1, "2u+3g+3k"),
            (2, "5w+3u"),
            (2, "2w+1r+4k"),
            (2, "5u"),
            (3, "6u"),
        ],
        ResourceKind::White => vec![
            (1, "3g+2r+2k"),
            (1, "2w+3u+3r"),
            (2, "1g+4r+2k"),
            (2, "5r+3k"),
            (2, "5r"),
            (3, "6w"),
        ],
        ResourceKind::Green => vec![
            (1, "3w+2g+3r"),
            (1, "2w+3u+2k"),
            (2, "4w+2u+1k"),
            (2, "5u+3g"),
            (2, "5g"),
            (3, "6g"),
        ],
        ResourceKind::Red => vec![
            (1, "2w+2r+3k"),
            (1, "3u+2r+3k"),
            (2, "1w+4u+2g"),
            (2, "3w+5k"),
            (2, "5k"),
            (3, "6r"),
        ],
    ];
    let deck2 = enum_map![
        ResourceKind::Black => vec![
            (3, "3w+3u+5g+3r"),
            (4, "7r"),
            (4, "3g+6r+3k"),
            (5, "7r+3k"),
        ],
        ResourceKind::Blue => vec![
            (3, "3w+3g+3r+5k"),
            (4, "7w"),
            (4, "6w+3u+3k"),
            (5, "7w+3u"),
        ],
        ResourceKind::White => vec![
            (3, "3u+3g+5r+3k"),
            (4, "7k"),
            (4, "3w+3r+6k"),
            (5, "3w+7k"),
        ],
        ResourceKind::Green => vec![
            (3, "5w+3u+3r+3k"),
            (4, "7u"),
            (4, "3w+6u+3g"),
            (5, "7u+3g"),
        ],
        ResourceKind::Red => vec![
            (3, "3w+5u+3g+3k"),
            (4, "7g"),
            (4, "3u+6g+3r"),
            (5, "7g+3r"),
        ],
    ];
    let mut nobels = vec![
        "4r+4g", "4u+4w", "4k+4w", "4u+4g", "4k+4r", "3k+3r+3w", "3g+3u+3r", "3g+3u+3w",
        "3k+3u+3w", "3k+3r+3g",
    ];
    let decks = [deck0, deck1, deck2];
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
        decks: decks
            .into_iter()
            .map(|d| {
                let mut r: Vec<Card> = d
                    .into_iter()
                    .flat_map(|(c, l)| l.into_iter().map(move |(s, d)| (c, s, d)))
                    .map(|(c, s, d)| Card::new(c, s, ResourceMap::from_code(d)))
                    .collect();
                r.shuffle(&mut rand::thread_rng());
                r
            })
            .collect(),
        nobels: {
            nobels.shuffle(&mut rand::thread_rng());
            nobels[0..agents.len() + 1]
                .iter()
                .map(|x| Nobel {
                    cost: ResourceMap::from_code(x),
                    score: 3,
                })
                .collect()
        },
        players: agents.iter().map(|a| Player::new(&a.name())).collect(),
        coins: ResourceMap(enum_map! {
            ResourceKind::Red => 7,
            ResourceKind::Blue => 7,
            ResourceKind::Green => 7,
            ResourceKind::White => 7,
            ResourceKind::Black => 7,
        }),
        turn: 0,
        wilds: 5,
    };

    let mut ed = clap_repl::ClapEditor::<Action>::builder().build();
    state.print();
    loop {
        if state.is_finished() {
            println!("Game finished");
            break;
        }
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
