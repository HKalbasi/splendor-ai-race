use game_def::{ai_from_function, Action, State};

fn logic(state: State) -> Action {
    for (deck, card) in state.card_iter() {
        let action = Action::Purchase { deck, card };
        if state.clone().run(action.clone()).is_ok() {
            return action;
        }
    }

    for index in 0..state.players[state.turn].reserved.len() {
        let action = Action::PurchaseReserved { index };
        if state.clone().run(action.clone()).is_ok() {
            return action;
        }
    }

    for (one, two, three) in state.pick_three_iter() {
        let action = Action::PickThree { one, two, three };
        if state.clone().run(action.clone()).is_ok() {
            return action;
        }
    }

    for color in state.pick_two_iter() {
        let action = Action::PickTwo { color };
        if state.clone().run(action.clone()).is_ok() {
            return action;
        }
    }

    for (deck, card) in state.card_iter() {
        let action = Action::Reserve { deck, card };
        if state.clone().run(action.clone()).is_ok() {
            return action;
        }
    }
    Action::Skip
}

fn main() {
    ai_from_function(logic);
}
