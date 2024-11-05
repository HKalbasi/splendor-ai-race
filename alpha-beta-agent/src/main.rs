use game_def::{ai_from_function, Action, Player, State};

fn moves(state: State) -> Vec<(State, Action)> {
    let mut r = vec![];
    for (deck, card) in state.card_iter() {
        let action = Action::Purchase { deck, card };
        let mut s = state.clone();
        if s.run(action.clone()).is_ok() {
            r.push((s, action));
        }
    }

    for index in 0..state.players[state.turn].reserved.len() {
        let action = Action::PurchaseReserved { index };
        let mut s = state.clone();
        if s.run(action.clone()).is_ok() {
            r.push((s, action));
        }
    }

    for (one, two, three) in state.pick_three_iter() {
        let action = Action::PickThree { one, two, three };
        let mut s = state.clone();
        if s.run(action.clone()).is_ok() {
            r.push((s, action));
        }
    }

    for color in state.pick_two_iter() {
        let action = Action::PickTwo { color };
        let mut s = state.clone();
        if s.run(action.clone()).is_ok() {
            r.push((s, action));
        }
    }

    for (deck, card) in state.card_iter() {
        let action = Action::Reserve { deck, card };
        let mut s = state.clone();
        if s.run(action.clone()).is_ok() {
            r.push((s, action));
        }
    }
    r
}

fn heuristic(state: &State) -> i32 {
    player_heuristic(&state.players[0]) - player_heuristic(&state.players[1])
}

fn player_heuristic(player: &Player) -> i32 {
    player.mortal.sum() + player.immortal.sum() * 100 + (1 << player.score)
}

fn max_score(state: State, depth: i32, mut alpha: i32, beta: i32) -> (i32, Action) {
    if state.is_finished() {
        if state.winner() == 0 {
            return (1_000_000_000, Action::Skip);
        } else {
            return (-1_000_000_000, Action::Skip);    
        }
    }
    if state.turn == 0 && depth <= 0 {
        return (heuristic(&state), Action::Skip)
    }
    let mut r = (-1_000_000_001, Action::Skip);
    for (st, ac) in moves(state) {
        let score = -max_score(st, depth - 1, -beta, -alpha).0;
        if r.0 < score {
            r = (score, ac);
            alpha = alpha.max(score);
            if score >= beta {
                break;
            }
        }
    }
    r
}

fn logic(state: State) -> Action {
    let (_, ac) = max_score(state, 3, -2_000_000_000, 2_000_000_000);
    ac
}

fn main() {
    ai_from_function(logic);
}
