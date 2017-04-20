extern crate rand;
extern crate common;

use common::*;
use common::Suit::*;
use common::Value::*;
use common::MenuState::*;
use common::Declaration::*;
use common::DeclarationInfo::*;
use common::Opponent::*;
use common::Teammate::*;
use common::Player::*;
use common::AskVector::*;
use common::SubSuit::*;
use common::Fact::*;
use common::ModelCard::*;
use common::AllValues;

use rand::{StdRng, SeedableRng, Rng};

use std::collections::HashMap;

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

    let teammate_1_memory = new_memory(TeammatePlayer(TeammateOne), &teammate_1);
    let teammate_2_memory = new_memory(TeammatePlayer(TeammateTwo), &teammate_2);
    let opponent_1_memory = new_memory(OpponentPlayer(OpponentZero), &opponent_1);
    let opponent_2_memory = new_memory(OpponentPlayer(OpponentOne), &opponent_2);
    let opponent_3_memory = new_memory(OpponentPlayer(OpponentTwo), &opponent_3);

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
        teammate_1_memory: teammate_1_memory,
        teammate_2_memory: teammate_2_memory,
        opponent_1_memory: opponent_1_memory,
        opponent_2_memory: opponent_2_memory,
        opponent_3_memory: opponent_3_memory,
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
    let mut deck = Card::all_values();

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
    // if state.title_screen {
    //
    //     for event in events {
    //         cross_mode_event_handling(platform, state, event);
    //         match *event {
    //             Event::Close |
    //             Event::KeyPressed { key: KeyCode::Escape, ctrl: _, shift: _ } => return true,
    //             Event::KeyPressed { key: _, ctrl: _, shift: _ } => state.title_screen = false,
    //             _ => (),
    //         }
    //     }
    //
    //     draw(platform, state);
    //
    //     false
    // } else {
    game_update_and_render(platform, state, events)
    // }
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
                    Quit => return true,
                    _ => state.menu_state = Quit,
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

    let on_quit_screen = match state.menu_state {
        Quit => true,
        _ => false,
    };

    if on_quit_screen {
        show_quit_screen(platform,
                         state,
                         left_mouse_pressed,
                         left_mouse_released,
                         &inner);
    } else if state.suits_in_play_bits == 0 {
        let mid_y = inner.y + (inner.h / 2);
        print_horizontally_centered_line(platform,
                                         &inner,
                                         if state.player_points > state.opponent_points {
                                             "Your team won"
                                         } else if state.player_points < state.opponent_points {
            "The other team won"
        } else {
            "It was a tie."
        },
                                         mid_y - 3);

        print_horizontally_centered_line(platform, &inner, "Final Score", mid_y - 1);
        print_horizontally_centered_line(platform,
                                         &inner,
                                         &format!("{}:{}",
                                                  state.player_points,
                                                  state.opponent_points),
                                         mid_y);
        print_horizontally_centered_line(platform, &inner, "  Us Them", mid_y + 1);


        let restart_button = ButtonSpec {
            x: inner.x + ((inner.w - 14) / 2),
            y: mid_y + 3,
            w: 14,
            h: 3,
            text: "Restart".to_string(),
            id: 1223,
        };

        if do_button(platform,
                     &mut state.ui_context,
                     &restart_button,
                     left_mouse_pressed,
                     left_mouse_released) {
            *state = new_state(size);
        }
    } else if let Some(declaration) = state.declaration {
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
            DeclareStep3(info) => {
                draw_declare_result(platform,
                                    state,
                                    inner,
                                    left_mouse_pressed,
                                    left_mouse_released,
                                    info)
            }

        }
    } else if let Some(current_player) = state.current_player {

        if player_hand(state, current_player).len() == 0 {
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
                Quit => {
                    show_quit_screen(platform,
                                     state,
                                     left_mouse_pressed,
                                     left_mouse_released,
                                     &inner);

                }
                Main => {
                    match current_player {
                        TeammatePlayer(ThePlayer) => {
                            draw_main_menu(platform,
                                           state,
                                           inner,
                                           left_mouse_pressed,
                                           left_mouse_released)
                        }
                        _ => {
                            if let Some((ask_vector, suit, value)) =
                                get_ask_info(state, current_player) {
                                state.menu_state = AskStep4(ask_vector, suit, value);
                            } else {
                                state.menu_state = Main;
                                //TODO give player chance to declare first
                                state.declaration = Some(guess_declaration(state));
                            }
                        }
                    }
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
        Some(DeclareStep3(_)) => false,
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

fn guess_declaration(state: &mut State) -> Declaration {

    let mut declarations = Vec::new();

    let available_subsuits: Vec<SubSuit> =
        SubSuit::all_values().into_iter().filter(|&s| subsuit_is_in_play(state, s)).collect();
    for &(hand, player) in cpu_hands(state).iter() {
        for &subsuit in available_subsuits.iter() {
            let mut guessed_owners = Vec::new();
            let pairs = pairs_from_subsuit(subsuit);
            'pairs: for (suit, value) in pairs {
                if hand.contains(&Card {
                                      suit: suit,
                                      value: value,
                                  }) {
                    guessed_owners.push(player);
                    continue;
                }
                for current_player in Player::all_values() {
                    if let Some(knowledge) =
                        get_memory(state, player).and_then(|m| m.get(&current_player)) {
                        if knowledge.model_hand.contains(&Known(suit, value)) {
                            guessed_owners.push(player);
                            continue 'pairs;
                        }
                    }
                }

                //TODO better guessing
                let guess = *guessed_owners.get(guessed_owners.len() - 1).unwrap_or(&player);
                guessed_owners.push(guess);
            }

            let possible_info = match player {
                TeammatePlayer(t) => {
                    if let Some(teammates) = get_teammate_declaration_array(guessed_owners) {
                        Some(DeclareStep3(TeammateDInfo(t, subsuit, teammates)))
                    } else {
                        None
                    }
                }
                OpponentPlayer(o) => {
                    if let Some(opponents) = get_opponent_declaration_array(guessed_owners) {
                        Some(DeclareStep3(OpponentDInfo(o, subsuit, opponents)))
                    } else {
                        None
                    }
                }
            };

            if let Some(actual_info) = possible_info {
                declarations.push(actual_info);
            }
        }
    }

    let len = declarations.len();
    if len > 0 {
        if let Some(&declaration) = declarations.get(state.rng.gen_range(0, len)) {
            declaration
        } else {
            DeclareStep3(OpponentDInfo(OpponentZero, LowClubs, [OpponentZero; 6]))
        }
    } else {
        DeclareStep3(OpponentDInfo(OpponentZero, HighSpades, [OpponentZero; 6]))
    }
}

fn show_quit_screen(platform: &Platform,
                    state: &mut State,
                    left_mouse_pressed: bool,
                    left_mouse_released: bool,
                    inner: &SpecRect) {
    let mid_y = inner.y + (inner.h / 2);
    print_horizontally_centered_line(platform, &inner, "Press Esc again to Quit", mid_y);

    let resume = ButtonSpec {
        x: inner.x + ((inner.w - 14) / 2),
        y: mid_y + 3,
        w: 14,
        h: 3,
        text: "Resume".to_string(),
        id: 1223,
    };

    if do_button(platform,
                 &mut state.ui_context,
                 &resume,
                 left_mouse_pressed,
                 left_mouse_released) {
        state.menu_state = Main;
    }
}

fn get_opposite_team(player: Player) -> Vec<Player> {
    match player {
        TeammatePlayer(_) => Opponent::all_values().iter().map(|&o| OpponentPlayer(o)).collect(),
        OpponentPlayer(_) => Teammate::all_values().iter().map(|&t| TeammatePlayer(t)).collect(),
    }
}

fn get_ask_info(state: &mut State, player: Player) -> Option<(AskVector, Suit, Value)> {
    let memory = match player {
        TeammatePlayer(ThePlayer) => {
            println!("Cannot return a reference to the player's memory");
            return Some((ToOpponent(ThePlayer, OpponentZero), Spades, Ace));
        }
        TeammatePlayer(TeammateOne) => &state.teammate_1_memory,
        TeammatePlayer(TeammateTwo) => &state.teammate_2_memory,
        OpponentPlayer(OpponentZero) => &state.opponent_1_memory,
        OpponentPlayer(OpponentOne) => &state.opponent_2_memory,
        OpponentPlayer(OpponentTwo) => &state.opponent_3_memory,
    };

    let mut other_team: Vec<Player> = get_opposite_team(player);
    other_team = other_team.iter()
        .map(|&p| p)
        .filter(|&p| player_hand(state, p).len() > 0)
        .collect();


    //what cards can I ask for?
    let possible_pairs = get_possible_target_pairs(player_hand(state, player));

    //of those, what do I know a particular opponent has
    for pair in possible_pairs.iter() {
        for &target_player in other_team.iter() {
            if known_to_have(memory, target_player, *pair) {
                return if let Some(vector) = make_ask_vector(player, target_player) {
                           Some((vector, pair.0, pair.1))
                       } else {
                           None
                       };
            }
        }
    }

    //TODO don't ask an opponent a quesion if they would win a suit,
    //i.e. you know that they know where all the cards of a suit are
    //but some are in one of your teammates hands. Instead try to ask
    //opponents that we know don't have any cards of that suit.

    if let (Some(default_player), Some(default_pair)) = (other_team.get(0), possible_pairs.get(0)) {

        let mut best_so_far = (*default_player, default_pair, -1);
        //which opponent has the most unknown cards, and I don't already
        //know doesn't have this card?
        for pair in possible_pairs.iter() {
            for &target_player in other_team.iter() {
                if not_known_not_to_have(memory, target_player, *pair) {
                    let unknown_pairs_count = get_unknown_pairs_count(memory, target_player);

                    if unknown_pairs_count > best_so_far.2 {
                        best_so_far = (target_player, pair, unknown_pairs_count)
                    }
                }
            }
        }

        Some((make_ask_vector(player, best_so_far.0).unwrap_or(ToOpponent(ThePlayer,
                                                                          OpponentZero)),
              (best_so_far.1).0,
              (best_so_far.1).1))
    } else {
        None
    }

}

fn known_to_have(memory: &Memory, target_player: Player, pair: (Suit, Value)) -> bool {
    if let Some(knowledge) = memory.get(&target_player) {
        for &card in knowledge.model_hand.iter() {
            match card {
                Known(suit, value) => {
                    if suit == pair.0 && value == pair.1 {
                        return true;
                    }
                }
                _ => {}
            }
        }
    };

    false

}

fn not_known_not_to_have(memory: &Memory, target_player: Player, pair: (Suit, Value)) -> bool {
    if let Some(knowledge) = memory.get(&target_player) {
        for &fact in knowledge.facts.iter() {
            match fact {
                KnownNotToHave(suit, value) => {
                    if suit == pair.0 && value == pair.1 {
                        return false;
                    }
                }
                _ => {}
            }
        }
    };

    true
}

fn get_unknown_pairs_count(memory: &Memory, target_player: Player) -> i32 {
    let mut result = 0;

    if let Some(knowledge) = memory.get(&target_player) {
        for &card in knowledge.model_hand.iter() {
            match card {
                Unknown => {
                    result += 1;
                }
                _ => {}
            }
        }
    };

    result
}


fn make_ask_vector(source: Player, target: Player) -> Option<AskVector> {
    match (source, target) {
        (TeammatePlayer(s), OpponentPlayer(t)) => Some(ToOpponent(s, t)),
        (OpponentPlayer(s), TeammatePlayer(t)) => Some(ToTeammate(s, t)),
        _ => None,
    }
}

fn get_possible_target_pairs(hand: &Hand) -> Vec<(Suit, Value)> {
    let mut result = Vec::new();
    let subsuits = SubSuit::all_values();

    for &subsuit in subsuits.iter() {
        let pairs = pairs_from_subsuit(subsuit);

        if has_subsuit(hand, subsuit) {
            result.extend(pairs.iter().filter(|&&(suit, value)| {

                for card in hand.iter() {
                    if card.suit == suit && card.value == value {
                        return false;
                    }
                }

                true
            }))
        }
    }

    result
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

fn player_hand(state: &State, player: Player) -> &Hand {
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

    (platform.print_xy)(size.width - 9,
                        size.height - 3,
                        &format!("{}:{}", state.player_points, state.opponent_points));
    (platform.print_xy)(size.width - 10, size.height - 2, "Us Them");
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

    print_horizontally_centered_line(platform,
                                     &opponent_zero,
                                     &format!("{} cards", state.opponent_1.len()),
                                     opponent_zero.y + (opponent_zero.h / 2) + 1);

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

    print_horizontally_centered_line(platform,
                                     &opponent_one,
                                     &format!("{} cards", state.opponent_2.len()),
                                     opponent_one.y + (opponent_one.h / 2) + 1);

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

    print_horizontally_centered_line(platform,
                                     &opponent_two,
                                     &format!("{} cards", state.opponent_3.len()),
                                     opponent_two.y + (opponent_two.h / 2) + 1);
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

    let (target_has_card, target_name, target_is_opponent) = match ask_vector {
        ToTeammate(_, target) => {
            (has_card(teammate_hand(state, target), suit, value), teammate_name(target), false)
        }
        ToOpponent(_, target) => {
            (has_card(opponent_hand(state, target), suit, value), opponent_name(target), true)
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
        text: if target_has_card == target_is_opponent {
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
                {
                    let asker_hand = match ask_vector {
                        ToTeammate(source, _) => opponent_hand_mut(state, source),
                        ToOpponent(source, _) => teammate_hand_mut(state, source),
                    };
                    if let Err(insertion_index) = asker_hand.binary_search(&card) {
                        asker_hand.insert(insertion_index, card);
                    }
                }

                note_successful_ask(state, ask_vector, suit, value);
            }

        } else {
            state.current_player = Some(match ask_vector {
                                            ToTeammate(_, target) => TeammatePlayer(target),
                                            ToOpponent(_, target) => OpponentPlayer(target),
                                        });

            note_unsuccessful_ask(state, ask_vector, suit, value);
        }

        state.menu_state = Main;
    }

}

//everyone now knows that `source` has this card and the target has one fewer
fn note_successful_ask(state: &mut State, ask_vector: AskVector, suit: Suit, value: Value) {
    let (source, target) = match ask_vector {
        ToTeammate(source, target) => (OpponentPlayer(source), TeammatePlayer(target)),
        ToOpponent(source, target) => (TeammatePlayer(source), OpponentPlayer(target)),
    };

    for memory in get_memories(state).iter_mut() {
        if let Some(target_knowledge) = memory.get_mut(&target) {
            let target_hand = &mut target_knowledge.model_hand;

            let mut card_was_not_found = true;

            for i in 0..target_hand.len() {
                if let Some(&Known(known_suit, known_value)) = target_hand.get(i) {
                    if known_suit == suit && known_value == value {
                        target_hand.swap_remove(i);

                        card_was_not_found = false;
                    }
                }
            }

            if card_was_not_found {
                for i in 0..target_hand.len() {
                    if let Some(&Unknown) = target_hand.get(i) {
                        target_hand.swap_remove(i);
                    }
                }
            }
        }
        if let Some(source_knowledge) = memory.get_mut(&source) {
            let ref mut source_hand = source_knowledge.model_hand;

            source_hand.push(Known(suit, value));
        }
    }

    infer(state);

    set_any_declarations(state);
}

//everyone now knows that neither `source` or `target` have this card
fn note_unsuccessful_ask(state: &mut State, ask_vector: AskVector, suit: Suit, value: Value) {
    let (source, target) = match ask_vector {
        ToTeammate(source, target) => (OpponentPlayer(source), TeammatePlayer(target)),
        ToOpponent(source, target) => (TeammatePlayer(source), OpponentPlayer(target)),
    };

    for memory in get_memories(state).iter_mut() {
        if let Some(knowledge) = memory.get_mut(&target) {
            note_does_not_have(knowledge, suit, value);
        }
        if let Some(knowledge) = memory.get_mut(&source) {
            note_does_not_have(knowledge, suit, value);
        }
    }

    infer(state);

    set_any_declarations(state);
}

fn infer(state: &mut State) {
    for memory in get_memories(state).iter_mut() {
        let mut eliminated_players_by_card = HashMap::new();

        for &player in Player::all_values().iter() {
            if let Some(knowledge) = memory.get(&player) {
                for &fact in knowledge.facts.iter() {
                    match fact {
                        KnownNotToHave(suit, value) => {
                            let mut eliminated_players = eliminated_players_by_card.entry((suit,
                                                                                           value))
                                .or_insert(Vec::new());

                            eliminated_players.push(player);
                        }
                    }
                }
            }
        }

        for (&(suit, value), players) in
            eliminated_players_by_card.iter().filter(|&(_, v)| v.len() == 5) {
            let other_player = get_other_player(players);

            if let Some(knowledge) = memory.get_mut(&other_player) {
                let mut model_hand = &mut knowledge.model_hand;
                for i in 0..model_hand.len() {
                    match model_hand[i] {
                        Unknown => {
                            model_hand[i] = Known(suit, value);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn get_other_player(players: &Vec<Player>) -> Player {
    let mut all_players = Player::all_values();

    for i in 0..all_players.len() - 1 {
        if let Some(&player) = players.get(i) {
            for j in 0..all_players.len() {
                if player == all_players[j] {
                    all_players.swap_remove(j);
                    break;
                }
            }
        }
    }

    if let Some(&p) = all_players.get(0) {
        p
    } else {
        TeammatePlayer(ThePlayer)
    }
}

fn set_any_declarations(state: &mut State) {
    let mut cpu_players = cpu_players();

    state.rng.shuffle(&mut cpu_players);

    for &player in cpu_players.iter() {
        let new_declaration = get_new_delcaration(state, player);

        if new_declaration.is_some() {
            state.declaration = new_declaration;
            return;
        }
    }
}

fn get_new_delcaration(state: &mut State, player: Player) -> Option<Declaration> {
    if let Some(memory) = get_memory_mut(state, player) {
        let same_team_players = get_same_team_players(player);

        'subsuits: for &subsuit in SubSuit::all_values().iter() {
            let mut known_owners = Vec::new();

            for (suit, value) in pairs_from_subsuit(subsuit) {
                match (player, known_owning_player(memory, &same_team_players, suit, value)) {
                    (TeammatePlayer(_), Some(TeammatePlayer(owner))) => {
                        known_owners.push(TeammatePlayer(owner))
                    }
                    (OpponentPlayer(_), Some(OpponentPlayer(owner))) => {
                        known_owners.push(OpponentPlayer(owner))
                    }
                    _ => {
                        continue 'subsuits;
                    }
                }
            }

            match player {
                TeammatePlayer(t) => {
                    if let Some(teammates) = get_teammate_declaration_array(known_owners) {
                        return Some(DeclareStep3(TeammateDInfo(t, subsuit, teammates)));
                    }
                }
                OpponentPlayer(o) => {
                    if let Some(opponents) = get_opponent_declaration_array(known_owners) {
                        return Some(DeclareStep3(OpponentDInfo(o, subsuit, opponents)));
                    }
                }
            }
        }
    }
    None

}

fn get_teammate_declaration_array(known_owners: Vec<Player>) -> Option<[Teammate; 6]> {
    match (known_owners.get(0),
           known_owners.get(1),
           known_owners.get(2),
           known_owners.get(3),
           known_owners.get(4),
           known_owners.get(5)) {
        (Some(&TeammatePlayer(a)),
         Some(&TeammatePlayer(b)),
         Some(&TeammatePlayer(c)),
         Some(&TeammatePlayer(d)),
         Some(&TeammatePlayer(e)),
         Some(&TeammatePlayer(f))) => Some([a, b, c, d, e, f]),
        _ => None,
    }

}

fn get_opponent_declaration_array(known_owners: Vec<Player>) -> Option<[Opponent; 6]> {
    match (known_owners.get(0),
           known_owners.get(1),
           known_owners.get(2),
           known_owners.get(3),
           known_owners.get(4),
           known_owners.get(5)) {
        (Some(&OpponentPlayer(a)),
         Some(&OpponentPlayer(b)),
         Some(&OpponentPlayer(c)),
         Some(&OpponentPlayer(d)),
         Some(&OpponentPlayer(e)),
         Some(&OpponentPlayer(f))) => Some([a, b, c, d, e, f]),
        _ => None,
    }

}

fn known_owning_player(memory: &Memory,
                       players: &Vec<Player>,
                       suit: Suit,
                       value: Value)
                       -> Option<Player> {
    for &player in players.iter() {
        if let Some(knowledge) = memory.get(&player) {
            for &card in knowledge.model_hand.iter() {
                match card {
                    Unknown => {}
                    Known(s, v) => {
                        if s == suit && v == value {
                            return Some(player);
                        }
                    }
                }
            }
        }
    }

    None
}

fn note_does_not_have(knowledge: &mut Knowledge, suit: Suit, value: Value) {
    let ref mut facts = knowledge.facts;

    //no need to add duplicate facts
    let mut not_already_known = true;
    for i in 0..facts.len() {
        if let Some(&KnownNotToHave(known_suit, known_value)) = facts.get(i) {
            if known_suit == suit && known_value == value {
                not_already_known = false;
                break;
            }
        }
    }
    if not_already_known {
        facts.push(KnownNotToHave(suit, value));
    }

    //we'll assume that the newest information is correct
    let ref mut hand = knowledge.model_hand;
    for i in 0..hand.len() {
        if let Some(&Known(known_suit, known_value)) = hand.get(i) {
            if known_suit == suit && known_value == value {
                hand[i] = Unknown;
            }
        }
    }
}

fn get_memories(state: &mut State) -> Vec<&mut Memory> {
    vec![&mut state.teammate_1_memory,
         &mut state.teammate_2_memory,
         &mut state.opponent_1_memory,
         &mut state.opponent_2_memory,
         &mut state.opponent_3_memory]
}

fn get_memory_mut(state: &mut State, player: Player) -> Option<&mut Memory> {
    match player {
        TeammatePlayer(ThePlayer) => None,
        TeammatePlayer(TeammateOne) => Some(&mut state.teammate_1_memory),
        TeammatePlayer(TeammateTwo) => Some(&mut state.teammate_2_memory),
        OpponentPlayer(OpponentZero) => Some(&mut state.opponent_1_memory),
        OpponentPlayer(OpponentOne) => Some(&mut state.opponent_2_memory),
        OpponentPlayer(OpponentTwo) => Some(&mut state.opponent_3_memory),
    }
}

fn get_memory(state: &State, player: Player) -> Option<&Memory> {
    match player {
        TeammatePlayer(ThePlayer) => None,
        TeammatePlayer(TeammateOne) => Some(&state.teammate_1_memory),
        TeammatePlayer(TeammateTwo) => Some(&state.teammate_2_memory),
        OpponentPlayer(OpponentZero) => Some(&state.opponent_1_memory),
        OpponentPlayer(OpponentOne) => Some(&state.opponent_2_memory),
        OpponentPlayer(OpponentTwo) => Some(&state.opponent_3_memory),
    }
}

fn get_same_team_players(player: Player) -> Vec<Player> {
    match player {
        TeammatePlayer(_) => {
            vec![TeammatePlayer(ThePlayer),
                 TeammatePlayer(TeammateOne),
                 TeammatePlayer(TeammateTwo)]
        }
        OpponentPlayer(_) => {
            vec![OpponentPlayer(OpponentZero),
                 OpponentPlayer(OpponentOne),
                 OpponentPlayer(OpponentTwo)]
        }
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
        state.declaration = Some(DeclareStep3(TeammateDInfo(ThePlayer, subsuit, teammates)));
    }
}

fn draw_declare_result(platform: &Platform,
                       state: &mut State,
                       rect: SpecRect,
                       left_mouse_pressed: bool,
                       left_mouse_released: bool,
                       info: DeclarationInfo) {
    let row_width = (rect.w / 6) - (MENU_OFFSET as f64 / 6.0).round() as i32;
    let subsuit = match info {
        TeammateDInfo(_, subsuit, _) => subsuit,
        OpponentDInfo(_, subsuit, _) => subsuit,
    };

    let pairs = pairs_from_subsuit(subsuit);
    for i in 0..6 {
        let (suit, value) = pairs[i];

        let y = rect.y + (i as i32 * row_width) / 6;

        match info {
            TeammateDInfo(declarer, _, teammates) => {
                let teammate = teammates[i];

                print_horizontally_centered_line(platform,
                                                 &rect,
                                                 &format!("{} said that {} had the {} of {}",
                                                          declarer.to_string(),
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
            OpponentDInfo(declarer, _, opponents) => {
                let opponent = opponents[i];

                print_horizontally_centered_line(platform,
                                                 &rect,
                                                 &format!("{} said that {} had the {} of {}",
                                                          declarer.to_string(),
                                                          opponent,
                                                          value,
                                                          suit),
                                                 y);

                let result_str = if has_card(opponent_hand(state, opponent), suit, value) {

                    "And they did have it!"

                } else {
                    //TODO say who did have it here
                    "But they didn't have it!"
                };

                print_horizontally_centered_line(platform, &rect, result_str, y + 1);
            }
        }


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
        let mut removed_cards = Vec::new();
        for i in 0..6 {
            let (suit, value) = pairs[i];


            let card_not_found = {
                let (hand, player) = match info {
                    TeammateDInfo(_, _, teammates) => {
                        (teammate_hand_mut(state, teammates[i]), TeammatePlayer(teammates[i]))
                    }
                    OpponentDInfo(_, _, opponents) => {
                        (opponent_hand_mut(state, opponents[i]), OpponentPlayer(opponents[i]))
                    }
                };

                if let Some(card) = remove_from_hand(hand, suit, value) {
                    removed_cards.push((card.suit, card.value, player));

                    false
                } else {
                    true
                }
            };

            if card_not_found {
                all_correct = false;
                for &mut (ref mut hand, player) in all_hands_mut(state).iter_mut() {
                    if let Some(card) = remove_from_hand(hand, suit, value) {
                        removed_cards.push((card.suit, card.value, player));

                        break;
                    }
                }
            }
        }

        match (info, all_correct) {
            (TeammateDInfo(_, _, _), true) |
            (OpponentDInfo(_, _, _), false) => {
                state.player_points += 1;
            }
            (TeammateDInfo(_, _, _), false) |
            (OpponentDInfo(_, _, _), true) => {
                state.opponent_points += 1;
            }
        }

        update_memories_after_suit_declared(state, removed_cards);

        state.declaration = None;
        state.suits_in_play_bits &= !u8::from(subsuit);

    }
}

fn update_memories_after_suit_declared(state: &mut State,
                                       located_cards: Vec<(Suit, Value, Player)>) {
    for memory in get_memories(state).iter_mut() {
        'located_cards: for &(suit, value, player) in located_cards.iter() {
            if let Some(knowledge) = memory.get_mut(&player) {
                let hand = &mut knowledge.model_hand;

                for i in 0..hand.len() {
                    match hand[i] {
                        Unknown => {}
                        Known(known_suit, known_value) => {
                            if known_suit == suit && known_value == value {
                                hand.swap_remove(i);

                                continue 'located_cards;
                            }
                        }
                    }
                }

                for i in 0..hand.len() {
                    match hand[i] {
                        Unknown => {
                            hand.swap_remove(i);

                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

    }
}

fn all_hands_mut(state: &mut State) -> Vec<(&mut Hand, Player)> {
    vec![(&mut state.player, TeammatePlayer(ThePlayer)),
         (&mut state.teammate_1, TeammatePlayer(TeammateOne)),
         (&mut state.teammate_2, TeammatePlayer(TeammateTwo)),
         (&mut state.opponent_1, OpponentPlayer(OpponentZero)),
         (&mut state.opponent_2, OpponentPlayer(OpponentOne)),
         (&mut state.opponent_3, OpponentPlayer(OpponentTwo))]
}

fn cpu_hands(state: &State) -> Vec<(&Hand, Player)> {
    vec![(&state.teammate_1, TeammatePlayer(TeammateOne)),
         (&state.teammate_2, TeammatePlayer(TeammateTwo)),
         (&state.opponent_1, OpponentPlayer(OpponentZero)),
         (&state.opponent_2, OpponentPlayer(OpponentOne)),
         (&state.opponent_3, OpponentPlayer(OpponentTwo))]
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
