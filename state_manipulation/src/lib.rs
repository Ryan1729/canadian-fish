extern crate rand;
extern crate common;

use common::*;
use common::Suit::*;
use common::Value::*;

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

    let mut row = Vec::new();

    for _ in 0..size.width {
        row.push(rng.gen::<u8>());
    }

    let mut deck = shuffled_deck(&mut rng);

    let mut player = Vec::new();
    let mut teammate_1 = Vec::new();
    let mut teammate_2 = Vec::new();
    let mut opponent_1 = Vec::new();
    let mut opponent_2 = Vec::new();
    let mut opponent_3 = Vec::new();
    for _ in 0..8 {
        player.push(deck.pop().unwrap());
        teammate_1.push(deck.pop().unwrap());
        teammate_2.push(deck.pop().unwrap());
        opponent_1.push(deck.pop().unwrap());
        opponent_2.push(deck.pop().unwrap());
        opponent_3.push(deck.pop().unwrap());
    }

    set_hand_positions(size.height, &mut player);

    State {
        rng: rng,
        title_screen: false,
        player: player,
        teammate_1: teammate_1,
        teammate_2: teammate_2,
        opponent_1: opponent_1,
        opponent_2: opponent_2,
        opponent_3: opponent_3,
    }
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

    for _ in 0..size.width {
        row.push(rng.gen::<u8>());
    }

    State {
        rng: rng,
        title_screen: true,
        player: Hand::new(),
        teammate_1: Hand::new(),
        teammate_2: Hand::new(),
        opponent_1: Hand::new(),
        opponent_2: Hand::new(),
        opponent_3: Hand::new(),
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
    for event in events {
        cross_mode_event_handling(platform, state, event);

        match *event {
            Event::Close |
            Event::KeyPressed { key: KeyCode::Escape, ctrl: _, shift: _ } => return true,
            _ => (),
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

fn draw(platform: &Platform, state: &State) {
    for card in state.player.iter() {
        draw_card(platform, card)
    }
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
