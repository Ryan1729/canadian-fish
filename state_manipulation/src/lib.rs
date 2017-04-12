extern crate rand;
extern crate common;

use common::*;
use common::Suit::*;
use common::Value::*;
use common::MenuState::*;
use common::Opponent::*;
use common::SubSuit::*;

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

    set_hand_positions(size.height, &mut player);

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
        ui_context: UIContext {
            hot: 0,
            active: 0,
            next_hot: 0,
        },
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
                          location: Point { x: 0, y: 0 },
                          suit: suit,
                          value: value,
                      });
        }
    }

    rng.shuffle(&mut deck);

    deck
}

const CARD_OFFSET: i32 = 2;
const CARD_OFFSET_DELTA: i32 = 6;

const HAND_HEIGHT_OFFSET: i32 = 8;

fn set_hand_positions(height: i32, hand: &mut Hand) {
    let mut offset = CARD_OFFSET;
    for card in hand.iter_mut() {
        card.location.x = offset;
        card.location.y = hand_height(height);

        offset += CARD_OFFSET_DELTA;
    }
}

pub fn hand_height(height: i32) -> i32 {
    height - HAND_HEIGHT_OFFSET
}

fn collect_hand(cards: &mut Vec<Card>) {
    let mut offset = CARD_OFFSET;
    for card in cards.iter_mut() {
        card.location.x = offset;
        offset += CARD_OFFSET_DELTA;
    }
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
            Event::KeyPressed { key: KeyCode::Escape, ctrl: _, shift: _ } => return true,
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
            draw_ask_subsuit_menu(platform,
                                  state,
                                  inner,
                                  left_mouse_pressed,
                                  left_mouse_released,
                                  opponent)
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

    }

    draw(platform, state);

    false
}

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


fn draw(platform: &Platform, state: &State) {
    for card in state.player.iter() {
        draw_card(platform, card)
    }
}

pub struct SpecRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct ButtonSpec {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text: String,
    pub id: i32,
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
fn draw_ask_subsuit_menu(platform: &Platform,
                         state: &mut State,
                         rect: SpecRect,
                         left_mouse_pressed: bool,
                         left_mouse_released: bool,
                         opponent: Opponent) {

}
fn draw_ask_suit_menu(platform: &Platform,
                      state: &mut State,
                      rect: SpecRect,
                      left_mouse_pressed: bool,
                      left_mouse_released: bool,
                      opponent: Opponent,
                      subsuit: SubSuit) {

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

    let rect_middle = spec.x + (spec.w / 2);

    (platform.print_xy)(rect_middle - (spec.text.len() as i32 / 2),
                        spec.y + (spec.h / 2),
                        &spec.text);

    return result;
}

pub fn inside_rect(point: Point, x: i32, y: i32, w: i32, h: i32) -> bool {
    x <= point.x && y <= point.y && point.x < x + w && point.y < y + h
}

const CARD_WIDTH: i32 = 16;
const CARD_HEIGHT: i32 = 12;

const CARD_MOUSE_X_OFFSET: i32 = -CARD_WIDTH / 2;
const CARD_MOUSE_Y_OFFSET: i32 = 0;


fn draw_card(platform: &Platform, card: &Card) {
    draw_card_at(platform, card.location, card);
}

fn draw_card_at(platform: &Platform, location: Point, card: &Card) {
    let x = location.x;
    let y = location.y;

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
