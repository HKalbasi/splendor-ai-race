use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use anyhow::{bail, Context};
use clap::ValueEnum;
use enum_map::{enum_map, Enum, EnumMap};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Enum, ValueEnum, Serialize, Deserialize)]
enum ResourceKind {
    Red,
    Blue,
    Yellow,
    White,
    Brown,
}

#[derive(Clone, Serialize, Deserialize)]
struct ResourceMap(EnumMap<ResourceKind, usize>);

impl Debug for ResourceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut m = f.debug_map();
        for (r, v) in &self.0 {
            if *v == 0 {
                continue;
            }
            m.entry(&r, v);
        }
        m.finish()
    }
}

impl ResourceMap {
    fn new() -> Self {
        ResourceMap(enum_map! {
            ResourceKind::Red => 0,
            ResourceKind::Blue => 0,
            ResourceKind::Yellow => 0,
            ResourceKind::White => 0,
            ResourceKind::Brown => 0,
        })
    }
    
    fn add(&mut self, adds: &ResourceMap) {
        for (r, x) in &adds.0 {
            self.0[r] += *x;
        }
    }
}

impl Index<ResourceKind> for ResourceMap {
    type Output = usize;

    fn index(&self, index: ResourceKind) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<ResourceKind> for ResourceMap {
    fn index_mut(&mut self, index: ResourceKind) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Player {
    mortal: ResourceMap,
    immortal: ResourceMap,
    score: u8,
    reserved: Vec<Card>,
    wilds: usize,
}

impl Player {
    fn purchase(&mut self, cost: &ResourceMap) -> anyhow::Result<()> {
        for (r, &t) in &cost.0 {
            let t = t.saturating_sub(self.immortal[r]);
            if self.mortal[r] < t {
                bail!("Not enough resources");
            }
            self.mortal[r] -= t;
        }
        Ok(())
    }

    fn can_purchase(&self, cost: &ResourceMap) -> bool {
        cost.0
            .iter()
            .all(|(r, &t)| self.immortal[r] + self.mortal[r] >= t)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Card {
    cost: ResourceMap,
    score: u8,
    adds: ResourceMap,
}

impl Card {
    fn new(color: ResourceKind, score: u8, cost: ResourceMap) -> Self {
        Card {
            cost,
            score,
            adds: {
                let mut r = ResourceMap::new();
                r[color] = 1;
                r
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct State {
    decks: Vec<Vec<Card>>,
    players: Vec<Player>,
    coins: ResourceMap,
    turn: usize,
}

const MAX_DECK_SHOW: usize = 4;

impl State {
    fn run(&mut self, action: Action) -> anyhow::Result<()> {
        let player = &mut self.players[self.turn];
        match action {
            Action::PickThree { one, two, three } => {
                if one == two || one == three || two == three {
                    bail!("No duplicate code in pick-tree");
                }
                for item in [one, two, three] {
                    if self.coins[item] == 0 {
                        bail!("No coin of {item:?} exists");
                    }
                    self.coins[item] -= 1;
                    player.mortal[item] += 1;
                }
                self.change_player();
            },
            Action::PickTwo { color } => {
                if self.coins[color] < 4 {
                    bail!("At least two coin of {color:?} should remain");
                }
                self.coins[color] -= 2;
                player.mortal[color] += 2;
                self.change_player();
            },
            Action::Purchase { deck, card } => {
                let d = self.decks.get(deck).context("Invalid deck")?;
                if card >= MAX_DECK_SHOW {
                    bail!("Can not purchase invisible card");
                }
                let c = d.get(card).context("Invalid card")?;
                if !player.can_purchase(&c.cost) {
                    bail!("You don't have enough resources");
                }
                player.purchase(&c.cost)?;
                player.immortal.add(&c.adds);
                player.score += c.score;
                self.decks[deck].remove(card);
                self.change_player();
            },
            Action::Reserve { deck, card } => {
                let d = self.decks.get(deck).context("Invalid deck")?;
                if card >= MAX_DECK_SHOW {
                    bail!("Can not purchase invisible card");
                }
                _ = d.get(card).context("Invalid card")?;
                let c = self.decks[deck].remove(card);
                player.score += c.score;
                self.change_player();
            }
        }
        Ok(())
    }

    fn json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn print(&self) {
        let player = &self.players[self.turn];
        for (i, d) in self.decks.iter().enumerate() {
            println!("Deck {i}:");
            for (j, c) in d.iter().enumerate() {
                if j == MAX_DECK_SHOW {
                    break;
                }
                print!("   Card {j}: {c:?}");
                if player.can_purchase(&c.cost) {
                    println!(" (You can purchase)");
                } else {
                    println!();
                }
            }
        }
        println!("Coins: {:?}", self.coins);
        for (i, p) in self.players.iter().enumerate() {
            println!("Player {i}:");
            println!("   Score: {}", p.score);
            println!("   Resource Cards: {:?}", p.immortal);
            println!("   Resource Coins: {:?}", p.mortal);
            println!("   Wild Coins: {}", p.wilds);
            println!("   Reserved Cards:");
        }
        println!("Turn player {}", self.turn);
    }
    
    fn change_player(&mut self) {
        self.turn += 1;
        if self.turn == self.players.len() {
            self.turn = 0;
        }
    }
}

#[derive(Debug, clap::Parser)]
enum Action {
    PickThree {
        one: ResourceKind,
        two: ResourceKind,
        three: ResourceKind,
    },
    PickTwo {
        color: ResourceKind,
    },
    Purchase {
        deck: usize,
        card: usize,
    },
    Reserve {
        deck: usize,
        card: usize,
    },
}

fn main() {
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
        players: vec![
            Player {
                mortal: ResourceMap::new(),
                immortal: ResourceMap::new(),
                score: 0,
                reserved: vec![],
                wilds: 0,
            },
            Player {
                mortal: ResourceMap::new(),
                immortal: ResourceMap::new(),
                score: 0,
                reserved: vec![],
                wilds: 0,
            },
        ],
        coins: ResourceMap(enum_map! {
            ResourceKind::Red => 5,
            ResourceKind::Blue => 5,
            ResourceKind::Yellow => 5,
            ResourceKind::White => 5,
            ResourceKind::Brown => 5,
        }),
        turn: 0,
    };

    let ed = clap_repl::ClapEditor::<Action>::builder().build();
    state.print();
    ed.repl(|action| {
        let mut s = state.clone();
        if let Err(e) = s.run(action) {
            println!("Error: {e:?}");
            return;
        }
        state = s;
        state.print();
        println!("{}", state.json());
    });
}
