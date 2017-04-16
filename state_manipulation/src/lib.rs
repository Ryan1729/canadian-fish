extern crate rand;
extern crate common;

use common::*;
use common::Suit::*;
use common::Value::*;
use common::MenuState::*;
use common::Declaration::*;
use common::Opponent::*;
use common::Teammate::*;
use common::Player::*;
use common::AskVector::*;
use common::SubSuit::*;
use common::AllValues;

use rand::{StdRng, SeedableRng, Rng};

//NOTE(Ryan1729): debug_assertions only appears to work correctly when the
//crate is not a dylib. Assuming you make this crate *not* a dylib on release,
//these configs should work
#[cfg(debug_assertions)]
#[no_mangle]
pub fn new_state(size: Size) -> State {
    //skip the title screen
    println!("debug on");

    let seed: &[_] = &[42];
    let mut rng: StdRng = SeedableRng::from_seed(seed);

    make_state(size, false, rng)
}
#[cfg(not(debug_assertions))]
#[no_mangle]
pub fn new_state(size: Size) -> State {
    //show the title screen
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(42);

    println!("{}", timestamp);
    let seed: &[_] = &[timestamp as usize];
    let rng: StdRng = SeedableRng::from_seed(seed);

    make_state(size, true, rng)
}

fn make_state(size: Size, title_screen: bool, mut rng: StdRng) -> State {
    let mut deck = shuffled_deck(&mut rng);

    let mut player = Vec::new();
    let mut teammate_1 = Vec::new();
    let mut teammate_2 = Vec::new();
    let mut opponent_1 = Vec::new();
    let mut opponent_2 = Vec::new();
    let mut opponent_3 = Vec::new();
    for _ in 0..8 {
        player.push(deck.pop().unwrap());
        opponent_1.push(deck.pop().unwrap());
        teammate_1.push(deck.pop().unwrap());
        opponent_2.push(deck.pop().unwrap());
        teammate_2.push(deck.pop().unwrap());
        opponent_3.push(deck.pop().unwrap());
    }

    player.sort();

    //TODO let player decide at game start how first player will be determined
    let current_player = Some(rng.gen::<Player>());

    State {
        rng: rng,
        title_screen: title_screen,
        player: player,
        teammate_1: teammate_1,
        teammate_2: teammate_2,
        opponent_1: opponent_1,
        opponent_2: opponent_2,
        opponent_3: opponent_3,
        menu_state: Main,
        declaration: None,
        current_player: current_player,
        ui_context: UIContext {
            hot: 0,
            active: 0,
            next_hot: 0,
        },
        card_offset: 0,
        suits_in_play_bits: 0xFF,
        player_points: 0,
        opponent_points: 0,
    }
}

fn shuffled_deck(rng: &mut StdRng) -> Deck {
    let mut deck = Vec::new();

    for &suit in vec![Clubs, Diamonds, Hearts, Spades].iter() {
        for &value in vec![Ace,
                           Two,
                           Three,
                           Four,
                           Five,
                           Six,
                           Seven,
                           //Eight, //Canadian Fish doesn't use the Eights
                           Nine,
                           Ten,
                           Jack,
                           Queen,
                           King]
                    .iter() {
            deck.push(Card {
                          suit: suit,
                          value: value,
                      });
        }
    }

    rng.shuffle(&mut deck);

    deck
}

const CARD_OFFSET: i32 = 5;
const CARD_OFFSET_DELTA: i32 = 6;

const HAND_HEIGHT_OFFSET: i32 = 8;

pub fn hand_height(height: i32) -> i32 {
    height - HAND_HEIGHT_OFFSET
}

#[no_mangle]
//returns true if quit requested
pub fn update_and_render(platform: &Platform, state: &mut State, events: &mut Vec<Event>) -> bool {
    if state.title_screen {

        for event in events {
            cross_mode_event_handling(platform, state, event);
            match *event {
                Event::Close |
                Event::KeyPressed { key: KeyCode::Escape, ctrl: _, shift: _ } => return true,
                Event::KeyPressed { key: _, ctrl: _, shift: _ } => state.title_screen = false,
                _ => (),
            }
        }

        draw(platform, state);

        false
    } else {
        game_update_and_render(platform, state, events)
    }
}

pub fn game_update_and_render(platform: &Platform,
                              state: &mut State,
                              events: &mut Vec<Event>)
                              -> bool {
    let mut left_mouse_pressed = false;
    let mut left_mouse_released = false;

    for event in events {
        cross_mode_event_handling(platform, state, event);

        match *event {
            Event::KeyPressed { key: KeyCode::MouseLeft, ctrl: _, shift: _ } => {
                left_mouse_pressed = true;
            }
            Event::KeyReleased { key: KeyCode::MouseLeft, ctrl: _, shift: _ } => {
                left_mouse_released = true;
            }
            Event::Close |
            Event::KeyReleased { key: KeyCode::Escape, ctrl: _, shift: _ } => {
                match state.menu_state {
                    Main => return true,
                    _ => state.menu_state = Main,
                }
            }

            _ => (),
        }
    }

    let size = (platform.size)();

    let outer = SpecRect {
        x: MENU_OFFSET,
        y: MENU_TOP_HEIGHT_OFFSET,
        w: size.width - 2 * MENU_OFFSET,
        h: size.height - (MENU_TOP_HEIGHT_OFFSET + MENU_BOTTOM_HEIGHT_OFFSET),
    };

    draw_double_line_rect(platform, outer.x, outer.y, outer.w, outer.h);

    let inner = SpecRect {
        x: outer.x + 1,
        y: outer.y + 1,
        w: outer.w - 2,
        h: outer.h - 2,
    };

    state.ui_context.frame_init();

    if let Some(declaration) = state.declaration {
        match declaration {
            DeclareStep1 => {
                draw_subsuit_menu(platform,
                                  state,
                                  inner,
                                  left_mouse_pressed,
                                  left_mouse_released,
                                  &|state, subsuit| {
                                       state.declaration = Some(DeclareStep2(subsuit,
                                                                             [ThePlayer; 6]));
                                   },
                                  true)
            }
            DeclareStep2(subsuit, teammates) => {
                draw_declare_radio_buttons(platform,
                                           state,
                                           inner,
                                           left_mouse_pressed,
                                           left_mouse_released,
                                           subsuit,
                                           teammates)
            }
            DeclareStep3(player, subsuit, teammates) => {
                draw_declare_result(platform,
                                    state,
                                    inner,
                                    left_mouse_pressed,
                                    left_mouse_released,
                                    subsuit,
                                    teammates,
                                    player)
            }

        }
    } else if let Some(current_player) = state.current_player {

        if get_hand(state, current_player).len() == 0 {
            match current_player {
                TeammatePlayer(ThePlayer) => {
                    draw_teammate_selection(platform,
                                            state,
                                            inner,
                                            left_mouse_pressed,
                                            left_mouse_released)
                }
                TeammatePlayer(teammate) => {
                    state.current_player =
                        or_available_opponent(state,
                                              get_available_teammate(state,
                                                                     Some(teammate),
                                                                     MostCards))
                }
                OpponentPlayer(opponent) => {
                    state.current_player =
                        or_available_teammate(state,
                                              get_available_opponent(state,
                                                                     Some(opponent),
                                                                     MostCards))
                }
            }

        } else {
            match state.menu_state {
                Main => {
                    draw_main_menu(platform,
                                   state,
                                   inner,
                                   left_mouse_pressed,
                                   left_mouse_released)
                }
                AskStep1 => {
                    draw_ask_opponent_menu(platform,
                                           state,
                                           inner,
                                           left_mouse_pressed,
                                           left_mouse_released)
                }
                AskStep2(opponent) => {
                    draw_subsuit_menu(platform,
                                      state,
                                      inner,
                                      left_mouse_pressed,
                                      left_mouse_released,
                                      &|state, subsuit| {
                                           state.menu_state = AskStep3(opponent, subsuit);
                                       },
                                      false)
                }
                AskStep3(opponent, subsuit) => {
                    draw_ask_suit_menu(platform,
                                       state,
                                       inner,
                                       left_mouse_pressed,
                                       left_mouse_released,
                                       opponent,
                                       subsuit)
                }
                AskStep4(ask_vector, suit, value) => {
                    draw_ask_result(platform,
                                    state,
                                    inner,
                                    left_mouse_pressed,
                                    left_mouse_released,
                                    ask_vector,
                                    suit,
                                    value)
                }

            }
        }
    } else {
        //TODO handle game end
    }

    let screen_rect = SpecRect {
        x: 0,
        y: 0,
        w: size.width,
        h: size.height,
    };

    if let Some(player) = state.current_player {
        let turn_string = match player {
            TeammatePlayer(ThePlayer) => "Your turn".to_string(),
            _ => player.to_string() + "'s turn",
        };

        print_horizontally_centered_line(platform, &screen_rect, &turn_string, 0);
    };

    draw(platform, state);

    if state.card_offset > 0 {
        let hand_window_left = ButtonSpec {
            x: 1,
            y: size.height - 5,
            w: HAND_ARROW_WIDTH,
            h: HAND_ARROW_HEIGHT,
            text: "←".to_string(),
            id: 1223,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &hand_window_left,
                     left_mouse_pressed,
                     left_mouse_released) {
            state.card_offset = state.card_offset.saturating_sub(1);
        }

    }

    if state.player.len() - state.card_offset > HAND_WINDOW_SIZE {
        let hand_window_right = ButtonSpec {
            x: size.width - (DECLARE_BUTTON_WIDTH + MENU_OFFSET + HAND_ARROW_WIDTH),
            y: size.height - (MENU_OFFSET + HAND_ARROW_HEIGHT),
            w: HAND_ARROW_WIDTH,
            h: HAND_ARROW_HEIGHT,
            text: "→".to_string(),
            id: 2334,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &hand_window_right,
                     left_mouse_pressed,
                     left_mouse_released) {
            if state.player.get(state.card_offset + 1).is_some() {
                state.card_offset += 1;
            }
        }
    }

    let show_declare_button = match state.declaration {
        Some(DeclareStep3(_, _, _)) => false,
        _ => teammate_hand(state, ThePlayer).len() > 0,
    };

    if show_declare_button {


        let declare_button = ButtonSpec {
            x: size.width - DECLARE_BUTTON_WIDTH - MENU_OFFSET,
            y: outer.y + outer.h + MENU_OFFSET,
            w: DECLARE_BUTTON_WIDTH,
            h: 5,
            text: "Declare".to_string(),
            id: 3445,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &declare_button,
                     left_mouse_pressed,
                     left_mouse_released) {
            state.declaration = Some(DeclareStep1);
        }
    }

    false
}

enum PlayerChoiceHeuristic {
    MostCards,
    FewestCards,
}
use PlayerChoiceHeuristic::*;
fn get_available_teammate(state: &State,
                          exclude: Option<Teammate>,
                          heuristic: PlayerChoiceHeuristic)
                          -> Option<Teammate> {
    let mut teammates = Teammate::all_values();

    match heuristic {
        MostCards => teammates.sort_by_key(|&t| std::usize::MAX - teammate_hand(state, t).len()),
        FewestCards => teammates.sort_by_key(|&t| teammate_hand(state, t).len()),
    };

    //Yes there is some duplication, but it means we don't have to futz around
    //with trait object types.
    if let Some(excluded) = exclude {
        teammates.iter()
            .filter(|&&t| excluded != t)
            .filter(|&&t| teammate_hand(state, t).len() > 0)
            .next()
            .cloned()
    } else {
        teammates.iter()
            .filter(|&&t| teammate_hand(state, t).len() > 0)
            .next()
            .cloned()
    }

}
fn get_available_opponent(state: &State,
                          exclude: Option<Opponent>,
                          heuristic: PlayerChoiceHeuristic)
                          -> Option<Opponent> {
    let mut opponents = Opponent::all_values();

    match heuristic {
        MostCards => opponents.sort_by_key(|&o| std::usize::MAX - opponent_hand(state, o).len()),
        FewestCards => opponents.sort_by_key(|&o| opponent_hand(state, o).len()),
    };

    if let Some(excluded) = exclude {
        opponents.iter()
            .filter(|&&t| excluded != t)
            .filter(|&&t| opponent_hand(state, t).len() > 0)
            .next()
            .cloned()
    } else {
        opponents.iter()
            .filter(|&&t| opponent_hand(state, t).len() > 0)
            .next()
            .cloned()
    }

}

fn get_hand(state: &State, player: Player) -> &Hand {
    match player {
        TeammatePlayer(t) => teammate_hand(state, t),
        OpponentPlayer(o) => opponent_hand(state, o),
    }
}

fn opponent_hand(state: &State, opponent: Opponent) -> &Hand {
    match opponent {
        OpponentZero => &state.opponent_1,
        OpponentOne => &state.opponent_2,
        OpponentTwo => &state.opponent_3,
    }
}


const HAND_ARROW_WIDTH: i32 = 4;
const HAND_ARROW_HEIGHT: i32 = 3;
const DECLARE_BUTTON_WIDTH: i32 = 11;

fn cross_mode_event_handling(platform: &Platform, state: &mut State, event: &Event) {
    match *event {
        Event::KeyPressed { key: KeyCode::R, ctrl: true, shift: _ } => {
            println!("reset");
            *state = new_state((platform.size)());
        }
        _ => (),
    }
}

const MENU_OFFSET: i32 = 2;
const MENU_TOP_HEIGHT_OFFSET: i32 = 1;
const MENU_BOTTOM_HEIGHT_OFFSET: i32 = HAND_HEIGHT_OFFSET + 2;
const HAND_WINDOW_SIZE: usize = 8;

fn draw(platform: &Platform, state: &State) {
    let size = (platform.size)();

    let mut x = CARD_OFFSET;
    let y = hand_height(size.height);

    for i in 0..HAND_WINDOW_SIZE {
        let index = i + state.card_offset;

        if let Some(card) = state.player.get(index) {
            draw_card(platform, (x, y), card);
            x += CARD_OFFSET_DELTA;
        } else {
            break;
        }
    }
}

pub struct SpecRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl AsRef<SpecRect> for SpecRect {
    fn as_ref(&self) -> &SpecRect {
        self
    }
}

//NOTE(Ryan1729): The AsRef impl below relies on ButtonSpec having
// the same ordering for x,y,w and h as SpecRect, and that they are
// at the top!
pub struct ButtonSpec {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text: String,
    pub id: i32,
}

impl AsRef<SpecRect> for ButtonSpec {
    fn as_ref(&self) -> &SpecRect {
        unsafe { std::mem::transmute::<&ButtonSpec, &SpecRect>(self) }
    }
}

fn draw_main_menu(platform: &Platform,
                  state: &mut State,
                  rect: SpecRect,
                  left_mouse_pressed: bool,
                  left_mouse_released: bool) {
    let ask_button_spec = ButtonSpec {
        x: rect.x,
        y: rect.y,
        w: (rect.w / 2) - MENU_OFFSET,
        h: (rect.h / 2) - MENU_OFFSET,
        text: "Ask for card".to_string(),
        id: 123,
    };


    if do_button(platform,
                 &mut state.ui_context,
                 &ask_button_spec,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.menu_state = AskStep1;
    }
}

fn draw_ask_opponent_menu(platform: &Platform,
                          state: &mut State,
                          rect: SpecRect,
                          left_mouse_pressed: bool,
                          left_mouse_released: bool) {

    let button_width = (rect.w / 3) - (MENU_OFFSET as f32 / 3.0).round() as i32;

    let opponent_zero = ButtonSpec {
        x: rect.x,
        y: rect.y,
        w: button_width,
        h: rect.h,
        text: "OpponentZero".to_string(),
        id: 123,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &opponent_zero,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.menu_state = AskStep2(OpponentZero);
    }

    let opponent_one = ButtonSpec {
        x: rect.x + button_width + MENU_OFFSET,
        y: rect.y,
        w: button_width,
        h: rect.h,
        text: "OpponentOne".to_string(),
        id: 234,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &opponent_one,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.menu_state = AskStep2(OpponentOne);
    }

    let opponent_two = ButtonSpec {
        x: rect.x + (button_width + MENU_OFFSET) * 2,
        y: rect.y,
        w: button_width,
        h: rect.h,
        text: "OpponentTwo".to_string(),
        id: 345,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &opponent_two,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.menu_state = AskStep2(OpponentTwo);
    }
}


fn draw_teammate_selection(platform: &Platform,
                           state: &mut State,
                           rect: SpecRect,
                           left_mouse_pressed: bool,
                           left_mouse_released: bool) {

    let teammates = Teammate::all_values();

    let filtered_teammates: Vec<&Teammate> = teammates.iter()
        .filter(|&&t| ThePlayer != t)
        .filter(|&&t| teammate_hand(state, t).len() > 0)
        .collect();

    if filtered_teammates.len() > 1 {
        let button_width = (rect.w / 2) - (MENU_OFFSET / 2);

        let spec_one = ButtonSpec {
            x: rect.x,
            y: rect.y,
            w: button_width,
            h: rect.h,
            text: "TeammateOne".to_string(),
            id: 123,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &spec_one,
                     left_mouse_pressed,
                     left_mouse_released) {
            state.current_player = Some(TeammatePlayer(TeammateOne))
        }

        let spec_two = ButtonSpec {
            x: rect.x + button_width + MENU_OFFSET,
            y: rect.y,
            w: button_width,
            h: rect.h,
            text: "TeammateTwo".to_string(),
            id: 234,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &spec_two,
                     left_mouse_pressed,
                     left_mouse_released) {
            state.current_player = Some(TeammatePlayer(TeammateTwo))
        }
    } else {
        //no choice so need for buttons
        state.current_player = or_available_opponent(state,
                                                     filtered_teammates.get(0).cloned().cloned());
    }

}

fn or_available_teammate(state: &State, potential_opponent: Option<Opponent>) -> Option<Player> {
    if let Some(available_opponent) = potential_opponent {
        Some(OpponentPlayer(available_opponent))
    } else {
        if let Some(available_teammate) = get_available_teammate(state, None, FewestCards) {
            Some(TeammatePlayer(available_teammate))
        } else {
            None
        }
    }
}
fn or_available_opponent(state: &State, potential_teammate: Option<Teammate>) -> Option<Player> {
    if let Some(available_teammate) = potential_teammate {
        Some(TeammatePlayer(available_teammate))
    } else {
        if let Some(available_opponent) = get_available_opponent(state, None, FewestCards) {
            Some(OpponentPlayer(available_opponent))
        } else {
            None
        }
    }
}

fn draw_subsuit_menu(platform: &Platform,
                     state: &mut State,
                     rect: SpecRect,
                     left_mouse_pressed: bool,
                     left_mouse_released: bool,
                     action: &Fn(&mut State, SubSuit),
                     show_all: bool) {

    let button_width = (rect.w / 4) - (MENU_OFFSET);
    let button_height = (rect.h / 2) - (MENU_OFFSET / 2);

    let lows = vec![LowClubs, LowDiamonds, LowHearts, LowSpades];

    for i in 0..4 {
        let subsuit = lows[i];

        if subsuit_is_in_play(state, subsuit) && (has_subsuit(&state.player, subsuit) || show_all) {
            let index = i as i32;
            let spec = ButtonSpec {
                x: rect.x + MENU_OFFSET + (button_width + MENU_OFFSET) * index,
                y: rect.y,
                w: button_width,
                h: button_height,
                text: subsuit.to_string(),
                id: 1123 + index,
            };

            if do_button(platform,
                         &mut state.ui_context,
                         &spec,
                         left_mouse_pressed,
                         left_mouse_released) {
                action(state, subsuit);
            }
        }
    }

    let highs = vec![HighClubs, HighDiamonds, HighHearts, HighSpades];

    for i in 0..4 {
        let subsuit = highs[i];

        if subsuit_is_in_play(state, subsuit) && (has_subsuit(&state.player, subsuit) || show_all) {

            let index = i as i32;
            let spec = ButtonSpec {
                x: rect.x + MENU_OFFSET + (button_width + MENU_OFFSET) * index,
                y: rect.y + button_height + (MENU_OFFSET / 2),
                w: button_width,
                h: button_height,
                text: subsuit.to_string(),
                id: 2234 + index,
            };

            if do_button(platform,
                         &mut state.ui_context,
                         &spec,
                         left_mouse_pressed,
                         left_mouse_released) {
                action(state, subsuit);
            }
        }
    }
}

fn has_subsuit(hand: &Hand, subsuit: SubSuit) -> bool {
    let pairs = pairs_from_subsuit(subsuit);

    for card in hand.iter() {
        for &(suit, value) in pairs.iter() {

            if card.suit == suit && card.value == value {
                return true;
            }
        }
    }

    false
}

fn subsuit_is_in_play(state: &State, subsuit: SubSuit) -> bool {
    state.suits_in_play_bits & (u8::from(subsuit)) != 0
}


fn draw_ask_suit_menu(platform: &Platform,
                      state: &mut State,
                      rect: SpecRect,
                      left_mouse_pressed: bool,
                      left_mouse_released: bool,
                      opponent: Opponent,
                      subsuit: SubSuit) {
    let button_width = (rect.w / 6) - MENU_OFFSET;
    let pairs = pairs_from_subsuit(subsuit);

    for i in 0..pairs.len() {
        let (suit, value) = pairs[i];

        if !has_card(&state.player, suit, value) {

            let index = i as i32;
            let spec = ButtonSpec {
                x: rect.x + MENU_OFFSET + (button_width + MENU_OFFSET) * index,
                y: rect.y,
                w: button_width,
                h: rect.h,
                text: format!("{} of {}", value, suit),
                id: 3345 + index,
            };

            if do_button(platform,
                         &mut state.ui_context,
                         &spec,
                         left_mouse_pressed,
                         left_mouse_released) {

                state.menu_state = if let Some(TeammatePlayer(teammate)) = state.current_player {
                    AskStep4(ToOpponent(teammate, opponent), suit, value)
                } else {
                    Main
                }
            }
        }
    }
}


fn draw_ask_result(platform: &Platform,
                   state: &mut State,
                   rect: SpecRect,
                   left_mouse_pressed: bool,
                   left_mouse_released: bool,
                   ask_vector: AskVector,
                   suit: Suit,
                   value: Value) {
    let button_width = (rect.w / 3) - (MENU_OFFSET as f64 / 3.0).round() as i32;
    let button_height = rect.h / 5;

    let (target_has_card, target_name) = match ask_vector {
        ToTeammate(_, target) => {
            (has_card(teammate_hand(state, target), suit, value), teammate_name(target))
        }
        ToOpponent(_, target) => {
            (has_card(opponent_hand(state, target), suit, value), opponent_name(target))
        }
    };

    let question = &format!("\"{}, do you have a {} of {}?\"", target_name, value, suit);

    print_horizontally_centered_line(platform, &rect, question, rect.y + MENU_OFFSET);

    if target_has_card {
        print_centered_line(platform, &rect, "\"Yes, I do. Here you go.\"");
    } else {
        print_centered_line(platform, &rect, "\"Nope! Now it's my turn!\"");
    }

    let spec = ButtonSpec {
        x: rect.x + MENU_OFFSET + (button_width + MENU_OFFSET),
        y: rect.y + rect.h - button_height,
        w: button_width,
        h: button_height,
        text: if target_has_card {
            "Aha!".to_string()
        } else {
            "Oh...".to_string()
        },
        id: 4456,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &spec,
                 left_mouse_pressed,
                 left_mouse_released) {

        if target_has_card {
            let taken_card = {
                let mut index = 0;
                let target_hand = match ask_vector {
                    ToTeammate(_, target) => teammate_hand_mut(state, target),
                    ToOpponent(_, target) => opponent_hand_mut(state, target),
                };
                for card in target_hand.iter() {
                    if card.suit == suit && card.value == value {
                        break;
                    }

                    index += 1;
                }

                if index < target_hand.len() {
                    Some(target_hand.swap_remove(index))
                } else {
                    None
                }
            };

            if let Some(card) = taken_card {
                let asker_hand = match ask_vector {
                    ToTeammate(source, _) => opponent_hand_mut(state, source),
                    ToOpponent(source, _) => teammate_hand_mut(state, source),
                };
                if let Err(insertion_index) = asker_hand.binary_search(&card) {
                    asker_hand.insert(insertion_index, card);
                }
            }

        }
        state.menu_state = Main;
    }

}

fn teammate_name(teammate: Teammate) -> String {
    match teammate {
        ThePlayer => "Player".to_string(),
        _ => teammate.to_string(),
    }
}

fn opponent_name(opponent: Opponent) -> String {
    opponent.to_string()
}

macro_rules! array_update {
    ($array:expr, $index: expr, $value: expr) => {{
        let mut new_array = $array.clone();

        new_array[$index] = $value;

        new_array
    }}
}

fn draw_declare_radio_buttons(platform: &Platform,
                              state: &mut State,
                              rect: SpecRect,
                              left_mouse_pressed: bool,
                              left_mouse_released: bool,
                              subsuit: SubSuit,
                              teammates: [Teammate; 6]) {
    let column_width = (rect.w / 5) - (MENU_OFFSET as f64 / 5.0).round() as i32;

    let labels = ["Player", "TeammateOne", "TeammateTwo"];
    for i in 0..labels.len() {
        let label = labels[i];
        (platform.print_xy)(rect.x + ((i + 1) as i32 * column_width) - (label.len() as i32 / 2),
                            rect.y,
                            label);

    }

    let pairs = pairs_from_subsuit(subsuit);
    let len = pairs.len();

    for i in 0..len {
        let (suit, value) = pairs[i];

        let y = rect.y + (((i + 1) as f32 / (len + 1) as f32) * rect.h as f32) as i32;

        (platform.print_xy)(rect.x, y, &(value.to_string() + &suit.to_string()));


        let (player, teammate_one, teammate_two) = match teammates[i] {
            ThePlayer => (true, false, false),
            TeammateOne => (false, true, false),
            TeammateTwo => (false, false, true),
        };


        let base_id = i as i32 * 12;
        if do_radio_button(platform,
                           &mut state.ui_context,
                           rect.x + 1 * column_width,
                           y,
                           base_id + 1,
                           player,
                           left_mouse_pressed,
                           left_mouse_released) {
            state.declaration = Some(DeclareStep2(subsuit, array_update!(teammates, i, ThePlayer)))
        };
        if do_radio_button(platform,
                           &mut state.ui_context,
                           rect.x + 2 * column_width,
                           y,
                           base_id + 2,
                           teammate_one,
                           left_mouse_pressed,
                           left_mouse_released) {
            state.declaration = Some(DeclareStep2(subsuit,
                                                  array_update!(teammates, i, TeammateOne)))
        };
        if do_radio_button(platform,
                           &mut state.ui_context,
                           rect.x + 3 * column_width,
                           y,
                           base_id + 3,
                           teammate_two,
                           left_mouse_pressed,
                           left_mouse_released) {
            state.declaration = Some(DeclareStep2(subsuit,
                                                  array_update!(teammates, i, TeammateTwo)))
        };
    }

    let button_width = column_width;
    let button_height = rect.h / 5;

    let spec = ButtonSpec {
        x: rect.x + rect.w - (button_width + button_width / 2),
        y: rect.y + rect.h - (button_height + button_height / 2),
        w: button_width,
        h: button_height,
        text: "Submit".to_string(),
        id: 5667,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &spec,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.declaration = Some(DeclareStep3(TeammatePlayer(ThePlayer), subsuit, teammates));
    }
}

fn draw_declare_result(platform: &Platform,
                       state: &mut State,
                       rect: SpecRect,
                       left_mouse_pressed: bool,
                       left_mouse_released: bool,
                       subsuit: SubSuit,
                       teammates: [Teammate; 6],
                       player: Player) {
    let pairs = pairs_from_subsuit(subsuit);
    let row_width = (rect.w / 6) - (MENU_OFFSET as f64 / 6.0).round() as i32;

    for i in 0..teammates.len() {
        let (suit, value) = pairs[i];

        let teammate = teammates[i];

        let y = rect.y + (i as i32 * row_width) / 6;

        print_horizontally_centered_line(platform,
                                         &rect,
                                         &format!("You said that {} had the {} of {}",
                                                  teammate,
                                                  value,
                                                  suit),
                                         y);

        let result_str = if has_card(teammate_hand(state, teammate), suit, value) {
            match teammate {
                ThePlayer => "And you did have it.",
                _ => "And they did have it!",
            }
        } else {
            match teammate {
                ThePlayer => "But you didn't have it?! Nice move, genius.",
                //TODO say who did have it here
                _ => "But they didn't have it!",
            }
        };

        print_horizontally_centered_line(platform, &rect, result_str, y + 1);
    }

    let button_width = (rect.w / 3) - (MENU_OFFSET as f64 / 3.0).round() as i32;
    let button_height = rect.h / 5;
    let spec = ButtonSpec {
        x: rect.x + (rect.w - button_width) / 2,
        y: rect.y + rect.h - button_height,
        w: button_width,
        h: button_height,
        text: "Okay".to_string(),
        id: 5667,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &spec,
                 left_mouse_pressed,
                 left_mouse_released) {
        let mut all_correct = true;
        for i in 0..teammates.len() {
            let (suit, value) = pairs[i];

            let teammate = teammates[i];

            if let Some(_) = remove_from_hand(teammate_hand_mut(state, teammate), suit, value) {
                //card was where it was expected to be
            } else {
                all_correct = false;
                for hand in all_hands_mut(state).iter_mut() {
                    if let Some(_) = remove_from_hand(hand, suit, value) {
                        break;
                    }
                }
            }
        }

        if all_correct {
            state.player_points += 1;
        } else {
            state.opponent_points += 1;
        }

        state.declaration = None;
    }
}

fn all_hands_mut(state: &mut State) -> Vec<&mut Vec<Card>> {
    vec![&mut state.player,
         &mut state.teammate_1,
         &mut state.teammate_2,
         &mut state.opponent_1,
         &mut state.opponent_2,
         &mut state.opponent_3]
}

fn remove_from_hand(hand: &mut Hand, suit: Suit, value: Value) -> Option<Card> {
    for i in 0..hand.len() {
        //using hand[i] twice rather than "let card  = hand[i];"
        //is done to appease the borrow checker.
        if hand[i].suit == suit && hand[i].value == value {
            //we do remove instead of swap_remove
            //because we want to keep the player's
            //hand sorted.
            return Some(hand.remove(i));
        }
    }

    None
}

fn teammate_hand(state: &State, teammate: Teammate) -> &Hand {
    match teammate {
        ThePlayer => &state.player,
        TeammateOne => &state.teammate_1,
        TeammateTwo => &state.teammate_2,
    }
}
fn teammate_hand_mut(state: &mut State, teammate: Teammate) -> &mut Hand {
    match teammate {
        ThePlayer => &mut state.player,
        TeammateOne => &mut state.teammate_1,
        TeammateTwo => &mut state.teammate_2,
    }
}
fn opponent_hand_mut(state: &mut State, opponent: Opponent) -> &mut Hand {
    match opponent {
        OpponentZero => &mut state.opponent_1,
        OpponentOne => &mut state.opponent_2,
        OpponentTwo => &mut state.opponent_3,
    }
}

fn do_radio_button(platform: &Platform,
                   context: &mut UIContext,
                   x: i32,
                   y: i32,
                   id: i32,
                   checked: bool,
                   left_mouse_pressed: bool,
                   left_mouse_released: bool)
                   -> bool {
    let mut result = false;

    let mouse_pos = (platform.mouse_position)();
    //the larger x window is to allow dquare checkbox graphics alongside narrow letters
    let inside = mouse_pos.y == y && (mouse_pos.x - x).abs() <= 2;

    if context.active == id {
        if left_mouse_released {
            result = context.hot == id && inside;

            context.set_not_active();
        }
    } else if context.hot == id {
        if left_mouse_pressed {
            context.set_active(id);
        }
    }

    if inside {
        context.set_next_hot(id);
    }

    (platform.print_xy)(x, y, if checked { "☑" } else { "☐" });

    result
}


fn has_card(hand: &Hand, suit: Suit, value: Value) -> bool {
    for card in hand.iter() {
        if card.suit == suit && card.value == value {
            return true;
        }
    }

    false
}

fn pairs_from_subsuit(subsuit: SubSuit) -> Vec<(Suit, Value)> {
    match subsuit {
        LowClubs => {
            vec![(Clubs, Two),
                 (Clubs, Three),
                 (Clubs, Four),
                 (Clubs, Five),
                 (Clubs, Six),
                 (Clubs, Seven)]
        }
        HighClubs => {
            vec![(Clubs, Nine),
                 (Clubs, Ten),
                 (Clubs, Jack),
                 (Clubs, Queen),
                 (Clubs, King),
                 (Clubs, Ace)]
        }
        LowDiamonds => {
            vec![(Diamonds, Two),
                 (Diamonds, Three),
                 (Diamonds, Four),
                 (Diamonds, Five),
                 (Diamonds, Six),
                 (Diamonds, Seven)]
        }
        HighDiamonds => {
            vec![(Diamonds, Nine),
                 (Diamonds, Ten),
                 (Diamonds, Jack),
                 (Diamonds, Queen),
                 (Diamonds, King),
                 (Diamonds, Ace)]
        }
        LowHearts => {
            vec![(Hearts, Two),
                 (Hearts, Three),
                 (Hearts, Four),
                 (Hearts, Five),
                 (Hearts, Six),
                 (Hearts, Seven)]
        }
        HighHearts => {
            vec![(Hearts, Nine),
                 (Hearts, Ten),
                 (Hearts, Jack),
                 (Hearts, Queen),
                 (Hearts, King),
                 (Hearts, Ace)]
        }
        LowSpades => {
            vec![(Spades, Two),
                 (Spades, Three),
                 (Spades, Four),
                 (Spades, Five),
                 (Spades, Six),
                 (Spades, Seven)]
        }
        HighSpades => {
            vec![(Spades, Nine),
                 (Spades, Ten),
                 (Spades, Jack),
                 (Spades, Queen),
                 (Spades, King),
                 (Spades, Ace)]
        }
    }
}

//calling this once will swallow multiple clicks on the button. We could either
//pass in and return the number of clicks to fix that, or this could simply be
//called multiple times per frame (once for each click).
fn do_button(platform: &Platform,
             context: &mut UIContext,
             spec: &ButtonSpec,
             left_mouse_pressed: bool,
             left_mouse_released: bool)
             -> bool {
    let mut result = false;

    let mouse_pos = (platform.mouse_position)();
    let inside = inside_rect(mouse_pos, spec.x, spec.y, spec.w, spec.h);
    let id = spec.id;

    if context.active == id {
        if left_mouse_released {
            result = context.hot == id && inside;

            context.set_not_active();
        }
    } else if context.hot == id {
        if left_mouse_pressed {
            context.set_active(id);
        }
    }

    if inside {
        context.set_next_hot(id);
    }

    if context.active == id && (platform.key_pressed)(KeyCode::MouseLeft) {
        draw_rect_with(platform,
                       spec.x,
                       spec.y,
                       spec.w,
                       spec.h,
                       ["╔", "═", "╕", "║", "│", "╙", "─", "┘"]);
    } else if context.hot == id {
        draw_rect_with(platform,
                       spec.x,
                       spec.y,
                       spec.w,
                       spec.h,
                       ["┌", "─", "╖", "│", "║", "╘", "═", "╝"]);
    } else {
        draw_rect(platform, spec.x, spec.y, spec.w, spec.h);
    }

    print_centered_line(platform, spec, &spec.text);

    return result;
}

fn print_centered_line<T: AsRef<SpecRect>>(platform: &Platform, thing: &T, text: &str) {
    print_line_in_rect(platform, thing, text, None, None)
}

fn print_horizontally_centered_line<T: AsRef<SpecRect>>(platform: &Platform,
                                                        thing: &T,
                                                        text: &str,
                                                        y: i32) {
    print_line_in_rect(platform, thing, text, None, Some(y))
}
fn print_vertically_centered_line<T: AsRef<SpecRect>>(platform: &Platform,
                                                      thing: &T,
                                                      text: &str,
                                                      x: i32) {
    print_line_in_rect(platform, thing, text, Some(x), None)
}
fn print_line_in_rect<T: AsRef<SpecRect>>(platform: &Platform,
                                          thing: &T,
                                          text: &str,
                                          x: Option<i32>,
                                          y: Option<i32>) {
    let rect = thing.as_ref();

    let x_ = if let Some(given_x) = x {
        given_x
    } else {
        let rect_middle = rect.x + (rect.w / 2);

        rect_middle - (text.len() as f32 / 2.0) as i32
    };

    let y_ = if let Some(given_y) = y {
        given_y
    } else {
        rect.y + (rect.h / 2)

    };

    (platform.print_xy)(x_, y_, &text);
}



pub fn inside_rect(point: Point, x: i32, y: i32, w: i32, h: i32) -> bool {
    x <= point.x && y <= point.y && point.x < x + w && point.y < y + h
}

const CARD_WIDTH: i32 = 16;
const CARD_HEIGHT: i32 = 12;

const CARD_MOUSE_X_OFFSET: i32 = -CARD_WIDTH / 2;
const CARD_MOUSE_Y_OFFSET: i32 = 0;


fn draw_card(platform: &Platform, (x, y): (i32, i32), card: &Card) {
    draw_rect(platform, x, y, CARD_WIDTH, CARD_HEIGHT);

    (platform.print_xy)(x + 1, y + 1, &card.value.to_string());
    (platform.print_xy)(x + 1, y + 2, &card.suit.to_string());
}



fn draw_rect(platform: &Platform, x: i32, y: i32, w: i32, h: i32) {
    draw_rect_with(platform,
                   x,
                   y,
                   w,
                   h,
                   ["┌", "─", "┐", "│", "│", "└", "─", "┘"]);
}

fn draw_double_line_rect(platform: &Platform, x: i32, y: i32, w: i32, h: i32) {
    draw_rect_with(platform,
                   x,
                   y,
                   w,
                   h,
                   ["╔", "═", "╗", "║", "║", "╚", "═", "╝"]);
}

fn draw_rect_with(platform: &Platform, x: i32, y: i32, w: i32, h: i32, edges: [&str; 8]) {
    (platform.clear)(Some(Rect::from_values(x, y, w, h)));

    let right = x + w - 1;
    let bottom = y + h - 1;
    // top
    (platform.print_xy)(x, y, edges[0]);
    for i in (x + 1)..right {
        (platform.print_xy)(i, y, edges[1]);
    }
    (platform.print_xy)(right, y, edges[2]);

    // sides
    for i in (y + 1)..bottom {
        (platform.print_xy)(x, i, edges[3]);
        (platform.print_xy)(right, i, edges[4]);
    }

    //bottom
    (platform.print_xy)(x, bottom, edges[5]);
    for i in (x + 1)..right {
        (platform.print_xy)(i, bottom, edges[6]);
    }
    (platform.print_xy)(right, bottom, edges[7]);
}
