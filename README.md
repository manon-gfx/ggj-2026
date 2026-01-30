# PIXL
PIXL is a basic framework that allows easily manipulation of pixels that are blitted to the screen.

## What can I use PIXL for?
* Creating basic visualisations
* Building a simple game
* Unleashing your creativity

## What does PIXL provide?
* A screenbuffer that is displayed on the screen
* Parallel uploading and CPU execution (double buffering) using Vulkan
* Basic mouse input
* Very basic keyboard input (to be improved)
* Drawing functions for basic shapes, sprites and text

## How to run
To run PIXL, simply do these three simple steps:
1. Install Rust if not yet installed: https://rustup.rs/
2. Run `cargo run`
3. [There is no step 3, there is no step 3!](https://www.youtube.com/watch?v=rjY0xsoozs8)

## A tour through PIXL
`game.rs` is a great place to start writing code. The `Game::new` function is executed on initialization and allows you to set up the intial state. Every frame `Game::tick` gets called every frame. You are provided with a `delta_time` and a `screen` Bitmap. After `Bitmap::tick` has finished executing the contents of the `screen` bitmap will be blitted to the screen.

`bitmap/mod.rs` contains a Bitmap struct with functionality for modifying it. `bitmap/font.rs` contains a basic font for displaying text.

`main.rs` is the platform layer where platform specific code can be written to provide all the basic functionality.

## TODO List
- [ ] Audio Support
- [ ] Full Screen support
- [ ] Screen integer scaling support
- [ ] Game Controller support
- [ ] Test if it works correctly on Linux
- [ ] Test if it works correctly on Windows
