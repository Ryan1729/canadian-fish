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

    player.sort();

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
        card_offset: 0,
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
        AskStep4(opponent, suit, value) => {
            draw_ask_result(platform,
                            state,
                            inner,
                            left_mouse_pressed,
                            left_mouse_released,
                            opponent,
                            suit,
                            value)
        }

    }

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
                 left_mouse_released) {}

    false
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
fn draw_ask_subsuit_menu(platform: &Platform,
                         state: &mut State,
                         rect: SpecRect,
                         left_mouse_pressed: bool,
                         left_mouse_released: bool,
                         opponent: Opponent) {

    let button_width = (rect.w / 4) - (MENU_OFFSET);
    let button_height = (rect.h / 2) - (MENU_OFFSET / 2);

    let lows = vec![LowClubs, LowDiamonds, LowHearts, LowSpades];

    for i in 0..4 {
        let subsuit = lows[i];

        if has_subsuit(&state.player, subsuit) {


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
                state.menu_state = AskStep3(opponent, subsuit);
            }
        }
    }

    let highs = vec![HighClubs, HighDiamonds, HighHearts, HighSpades];

    for i in 0..4 {
        let subsuit = highs[i];

        if has_subsuit(&state.player, subsuit) {

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
                state.menu_state = AskStep3(opponent, subsuit);
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

fn draw_ask_suit_menu(platform: &Platform,
                      state: &mut State,
                      rect: SpecRect,
                      left_mouse_pressed: bool,
                      left_mouse_released: bool,
                      opponent: Opponent,
                      subsuit: SubSuit) {
    let button_width = (rect.w / 6) - MENU_OFFSET;
    let pairs = pairs_from_subsuit(subsuit);

    for i in 0..6 {
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
                state.menu_state = AskStep4(opponent, suit, value);
            }
        }
    }
}


fn draw_ask_result(platform: &Platform,
                   state: &mut State,
                   rect: SpecRect,
                   left_mouse_pressed: bool,
                   left_mouse_released: bool,
                   opponent: Opponent,
                   suit: Suit,
                   value: Value) {
    let button_width = (rect.w / 3) - (MENU_OFFSET as f64 / 3.0).round() as i32;
    let button_height = rect.h / 5;


    let hand = match opponent {
        OpponentZero => &mut state.opponent_1,
        OpponentOne => &mut state.opponent_2,
        OpponentTwo => &mut state.opponent_3,
    };

    let question = &format!("\"{:?}, do you have a {} of {}?\"", opponent, value, suit);

    print_horizontally_centered_line(platform, &rect, question, rect.y + MENU_OFFSET);

    let opponent_has_card = has_card(hand, suit, value);
    if opponent_has_card {
        print_centered_line(platform, &rect, "\"Yes, I do. Here you go.\"");
    } else {
        print_centered_line(platform, &rect, "\"Nope! Now it's my turn!\"");
    }

    let spec = ButtonSpec {
        x: rect.x + MENU_OFFSET + (button_width + MENU_OFFSET),
        y: rect.y + rect.h - button_height,
        w: button_width,
        h: button_height,
        text: if opponent_has_card {
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

        if opponent_has_card {
            let mut index = 0;
            for card in hand.iter() {
                if card.suit == suit && card.value == value {
                    break;
                }

                index += 1;
            }

            if index < hand.len() {
                let taken_card = hand.swap_remove(index);

                if let Err(insertion_index) = state.player.binary_search(&taken_card) {
                    state.player.insert(insertion_index, taken_card);
                }
            }
        }
        state.menu_state = Main;
    }

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
