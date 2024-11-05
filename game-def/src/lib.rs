use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use anyhow::{bail, Context};
use clap::ValueEnum;
use enum_map::{enum_map, Enum, EnumMap};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Enum, ValueEnum, Serialize, Deserialize)]
pub enum ResourceKind {
    Red,
    Blue,
    Green,
    White,
    Black,
}

impl ResourceKind {
    fn from_code(code: &str) -> Self {
        match code {
            "g" => ResourceKind::Green,
            "r" => ResourceKind::Red,
            "w" => ResourceKind::White,
            "k" => ResourceKind::Black,
            "u" => ResourceKind::Blue,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResourceMap(pub EnumMap<ResourceKind, usize>);

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
    pub fn new() -> Self {
        ResourceMap(enum_map! {
            ResourceKind::Red => 0,
            ResourceKind::Blue => 0,
            ResourceKind::Green => 0,
            ResourceKind::White => 0,
            ResourceKind::Black => 0,
        })
    }

    pub fn add(&mut self, adds: &ResourceMap) {
        for (r, x) in &adds.0 {
            self.0[r] += *x;
        }
    }

    pub fn from_code(code: &str) -> Self {
        let mut this = Self::new();
        for c in code.split("+") {
            let (num, color) = c.split_at(1);
            this.0[ResourceKind::from_code(color)] = num.parse().unwrap();
        }
        this
    }
    
    pub fn sum(&self) -> i32 {
        self.0.iter().map(|x| *x.1 as i32).sum()
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
pub struct Player {
    pub mortal: ResourceMap,
    pub immortal: ResourceMap,
    pub score: u8,
    pub reserved: Vec<Card>,
    pub wilds: usize,
    pub display_name: String,
}

impl Player {
    pub fn new(name: &str) -> Self {
        Player {
            mortal: ResourceMap::new(),
            immortal: ResourceMap::new(),
            score: 0,
            reserved: vec![],
            wilds: 0,
            display_name: name.to_owned(),
        }
    }
    pub fn purchase(
        &mut self,
        cost: &ResourceMap,
        state_coins: &mut ResourceMap,
        state_wilds: &mut usize,
    ) -> anyhow::Result<()> {
        for (r, &t) in &cost.0 {
            let t = t.saturating_sub(self.immortal[r]);
            if self.mortal[r] < t {
                if self.wilds < t - self.mortal[r] {
                    bail!("Not enough resources");
                }
                *state_wilds += t - self.mortal[r];
                self.wilds -= t - self.mortal[r];
                state_coins[r] += self.mortal[r];
                self.mortal[r] = 0;
            } else {
                self.mortal[r] -= t;
                state_coins[r] += t;
            }
        }
        Ok(())
    }

    pub fn can_purchase(&self, cost: &ResourceMap) -> bool {
        cost.0
            .iter()
            .map(|(r, &t)| t.saturating_sub(self.immortal[r] + self.mortal[r]))
            .sum::<usize>()
            <= self.wilds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    cost: ResourceMap,
    score: u8,
    adds: ResourceMap,
}

impl Card {
    pub fn new(color: ResourceKind, score: u8, cost: ResourceMap) -> Self {
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
pub struct State {
    pub decks: Vec<Vec<Card>>,
    pub players: Vec<Player>,
    pub coins: ResourceMap,
    pub wilds: usize,
    pub turn: usize,
}

const MAX_DECK_SHOW: usize = 4;

impl State {
    pub fn is_finished(&self) -> bool {
        self.turn == 0 && self.players.iter().any(|x| x.score > 14)
    }

    pub fn winner(&self) -> usize {
        self.players.iter().enumerate().max_by_key(|x| x.1.score).unwrap().0
    }

    pub fn run(&mut self, action: Action) -> anyhow::Result<()> {
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
            }
            Action::PickTwo { color } => {
                if self.coins[color] < 4 {
                    bail!("At least two coin of {color:?} should remain");
                }
                self.coins[color] -= 2;
                player.mortal[color] += 2;
                self.change_player();
            }
            Action::Purchase { deck, card } => {
                let d = self.decks.get(deck).context("Invalid deck")?;
                if card >= MAX_DECK_SHOW {
                    bail!("Can not purchase invisible card");
                }
                let c = d.get(card).context("Invalid card")?;
                if !player.can_purchase(&c.cost) {
                    bail!("You don't have enough resources");
                }
                player.purchase(&c.cost, &mut self.coins, &mut self.wilds)?;
                player.immortal.add(&c.adds);
                player.score += c.score;
                self.decks[deck].remove(card);
                self.change_player();
            }
            Action::PurchaseReserved { index } => {
                let c = player
                    .reserved
                    .get(index)
                    .context("Invalid reserved index")?;
                if !player.can_purchase(&c.cost) {
                    bail!("You don't have enough resources");
                }
                let c = player.reserved.remove(index);
                player.purchase(&c.cost, &mut self.coins, &mut self.wilds)?;
                player.immortal.add(&c.adds);
                player.score += c.score;
                self.change_player();
            }
            Action::Reserve { deck, card } => {
                let d = self.decks.get(deck).context("Invalid deck")?;
                if card >= MAX_DECK_SHOW {
                    bail!("Can not purchase invisible card");
                }
                _ = d.get(card).context("Invalid card")?;
                let c = self.decks[deck].remove(card);
                player.reserved.push(c);
                self.change_player();
            }
            Action::Skip => {
                self.change_player();
            }
        }
        Ok(())
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn print(&self) {
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
        for p in &self.players {
            println!("{}:", p.display_name);
            println!("   Score: {}", p.score);
            println!("   Resource Cards: {:?}", p.immortal);
            println!("   Resource Coins: {:?}", p.mortal);
            println!("   Wild Coins: {}", p.wilds);
            if !p.reserved.is_empty() {
                println!("   Reserved Cards:");
                for r in &p.reserved {
                    println!("        {:?}", r);
                }
            }
        }
        println!("Turn {}", self.players[self.turn].display_name);
    }

    pub fn change_player(&mut self) {
        self.turn += 1;
        if self.turn == self.players.len() {
            self.turn = 0;
        }
    }

    pub fn card_iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.decks
            .iter()
            .enumerate()
            .flat_map(|(x, t)| (0..MAX_DECK_SHOW.min(t.len())).map(move |y| (x, y)))
    }

    pub fn pick_two_iter(&self) -> impl Iterator<Item = ResourceKind> + '_ {
        self.coins.0.iter().filter(|x| *x.1 >= 4).map(|x| x.0)
    }

    pub fn pick_three_iter(
        &self,
    ) -> impl Iterator<Item = (ResourceKind, ResourceKind, ResourceKind)> + '_ {
        use ResourceKind::*;
        const CANDIDATES: [(ResourceKind, ResourceKind, ResourceKind); 10] = [
            (Red, Green, Blue),
            (Red, Green, White),
            (Red, Green, Black),
            (Red, Blue, White),
            (Red, Blue, Black),
            (Red, White, Black),
            (Green, Blue, White),
            (Green, Blue, Black),
            (Green, White, Black),
            (Blue, Black, White),
        ];
        CANDIDATES.into_iter()
    }
}

#[derive(Debug, Clone, clap::Parser, Serialize, Deserialize)]
pub enum Action {
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
    PurchaseReserved {
        index: usize,
    },
    Reserve {
        deck: usize,
        card: usize,
    },
    Skip,
}

pub fn ai_from_function(mut function: impl FnMut(State) -> Action) {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let state = serde_json::from_str(&line).unwrap();
        let action = function(state);
        println!("{}", serde_json::to_string(&action).unwrap());
    }
}
