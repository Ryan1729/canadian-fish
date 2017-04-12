# Canadian Fish

This is a single-player only version of a six person card game with rules as described on [this page](http://bantha.org/~develin/cardgames.html#ch9).

##TODO
-> implement asking for a card
  -> menus
    -> suit selection
    -> value selection
  -> checking opponent's hand for card
    -> display answer
  -> moving card to player's hand
  -> menus
    -> breadcrumbs?


## Installation for Compilation

This program relies on `libBearLibTerminal.so` so that should be copied into `usr/local/lib` or another folder indicated by this command: `ldconfig -v 2>/dev/null | grep -v ^$'\t'`

then you should run `sudo ldconfig` to complete the installation.

Then the executable should run correctly.

Alternately if your OS has a package for BearLibTerminal, that may work as well.

## Compiling for Windows

Comment out the line containing `crate-type = ["dylib"]` in the `Cargo.toml` in the `state_manipulation` folder. (this is more or less a workaround for [this issue](https://github.com/rust-lang/rust/issues/18807), hopefully we will eventually be able to make this switch using the `cfg` attribute, but currently using the attribute doesn't appear to work correctly.)

Run `cargo build --release` then copy the exe in `./target/release` to the desired location as well as the following :

* a copy of the precompiled `BearLibTerminal.dll` and `BearLibTerminal.lib`.
* the `state_manipulation.dll` in `./target/release/deps`
* any necessary assets (graphics, sound, etc.).
