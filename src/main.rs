pub mod audio;
pub(crate) mod bitmap;
pub(crate) mod game;
pub(crate) mod vulkan;

use bitmap::Bitmap;
use game::Game;

use gilrs::Gilrs;
use minifb::WindowOptions;

use crate::vulkan::init_vulkan;

// Set to true to enable fullscreen mode
const FULLSCREEN: bool = false;

fn main() {
    // Tell Windows not to apply unnecessary DPI scaling to this application
    #[cfg(target_os = "windows")]
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
    };

    // Create a minifb window
    let (window_width, window_height) = if FULLSCREEN {
        // TODO(manon): Gather window resolution
        (1280, 720)
    } else {
        (1280, 720)
    };

    let (render_width, render_height) = (256, 208);

    let mut window = minifb::Window::new(
        "PIXL",
        window_width,
        window_height,
        WindowOptions {
            none: FULLSCREEN, // no window decorations for full screen
            ..WindowOptions::default()
        },
    )
    .expect("Failed to open a window :(");

    // Disable maximum FPS by sleeping the thread, aka we want ALL the frames
    window.set_target_fps(0);

    let vulkan_init_start = std::time::Instant::now();
    let mut vulkan_state = init_vulkan(&window, render_width, render_height);
    let vulkan_init_end = std::time::Instant::now();

    let mut minifb_bitmap = if vulkan_state.is_some() {
        println!(
            "Managed to initialize Vulkan! Enjoy :D. It took {:?}",
            vulkan_init_end - vulkan_init_start
        );
        None
    } else {
        println!("Failed to initialize vulkan, falling back on minifb pixel upload.");
        Some(Bitmap::new(render_width, render_height))
    };

    // Initialize the game!
    let game_init_start = std::time::Instant::now();
    let mut game = Box::new(Game::new());
    let game_init_end = std::time::Instant::now();
    println!(
        "Initializing game took {:?}",
        game_init_end - game_init_start
    );

    let mut gilrs = Gilrs::new().unwrap();

    // Mouse state to keep track of
    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    let mut mouse_state = [false; game::MouseButton::Count as usize];

    let mut prev_t = std::time::Instant::now();

    while window.is_open() {
        if game.reset_game_bool_hack {
            drop(game);
            game = Box::new(Game::new());
        }

        while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _code) => match button {
                    gilrs::Button::North => game.on_key_down(game::Key::Jump),
                    gilrs::Button::South => game.on_key_down(game::Key::MaskGreen),
                    gilrs::Button::East => game.on_key_down(game::Key::MaskRed),
                    gilrs::Button::West => game.on_key_down(game::Key::MaskBlue),

                    gilrs::Button::DPadUp => game.on_key_down(game::Key::Up),
                    gilrs::Button::DPadDown => game.on_key_down(game::Key::Down),
                    gilrs::Button::DPadLeft => game.on_key_down(game::Key::Left),
                    gilrs::Button::DPadRight => game.on_key_down(game::Key::Right),

                    gilrs::Button::RightTrigger => game.on_key_down(game::Key::Jump),
                    gilrs::Button::LeftTrigger => game.on_key_down(game::Key::Jump),
                    _ => {}
                },
                gilrs::EventType::ButtonReleased(button, _code) => match button {
                    gilrs::Button::North => game.on_key_up(game::Key::Jump),
                    gilrs::Button::South => game.on_key_up(game::Key::MaskGreen),
                    gilrs::Button::East => game.on_key_up(game::Key::MaskRed),
                    gilrs::Button::West => game.on_key_up(game::Key::MaskBlue),

                    gilrs::Button::DPadUp => game.on_key_up(game::Key::Up),
                    gilrs::Button::DPadDown => game.on_key_up(game::Key::Down),
                    gilrs::Button::DPadLeft => game.on_key_up(game::Key::Left),
                    gilrs::Button::DPadRight => game.on_key_up(game::Key::Right),

                    gilrs::Button::RightTrigger => game.on_key_up(game::Key::Jump),
                    gilrs::Button::LeftTrigger => game.on_key_up(game::Key::Jump),
                    _ => {}
                },
                gilrs::EventType::AxisChanged(axis, value, _code) => match axis {
                    gilrs::Axis::LeftStickX => game.on_axis_change(game::Axis::LeftStickX, value),
                    gilrs::Axis::LeftStickY => game.on_axis_change(game::Axis::LeftStickY, value),
                    gilrs::Axis::RightStickX => game.on_axis_change(game::Axis::RightStickX, value),
                    gilrs::Axis::RightStickY => game.on_axis_change(game::Axis::RightStickY, value),
                    _ => {}
                },
                // gilrs::EventType::ForceFeedbackEffectCompleted,
                _ => {} // ignore
            }
        }

        if let Some((x, y)) = window.get_mouse_pos(minifb::MouseMode::Clamp) {
            let scale_x = window_width as f32 / render_width as f32;
            let scale_y = window_height as f32 / render_height as f32;
            let scale = scale_x.min(scale_y);

            let blit_width = (render_width as f32 * scale) as i32;
            let blit_height = (render_height as f32 * scale) as i32;

            let blit_x = (window_width as i32 - blit_width) / 2;
            let blit_y = (window_height as i32 - blit_height) / 2;

            mouse_x = (x - blit_x as f32) / scale;
            mouse_y = (y - blit_y as f32) / scale;
            game.on_mouse_moved(mouse_x, mouse_y);
        }
        if let Some((scroll_x, scroll_y)) = window.get_scroll_wheel() {
            game.on_mouse_scrolled(scroll_x, scroll_y);
        }

        let mut handle_mouse_events = |minifb_button, button| {
            let old_state = mouse_state[button as usize];
            let new_state = window.get_mouse_down(minifb_button);

            if new_state != old_state {
                if new_state {
                    game.on_mouse_button_down(button, mouse_x, mouse_y);
                } else {
                    game.on_mouse_button_up(button, mouse_x, mouse_y);
                }
                mouse_state[button as usize] = new_state;
            }
        };
        handle_mouse_events(minifb::MouseButton::Left, game::MouseButton::Left);
        handle_mouse_events(minifb::MouseButton::Middle, game::MouseButton::Middle);
        handle_mouse_events(minifb::MouseButton::Right, game::MouseButton::Right);

        if window.is_key_pressed(minifb::Key::Escape, minifb::KeyRepeat::No) {
            return;
        }

        if window.is_key_pressed(minifb::Key::Enter, minifb::KeyRepeat::No) {
            game.reset_game_bool_hack = true;
        }
        let mut handle_key_events = |minifb_key, key| {
            if window.is_key_pressed(minifb_key, minifb::KeyRepeat::No) {
                game.on_key_down(key);
            }
            if window.is_key_released(minifb_key) {
                game.on_key_up(key);
            }
        };
        handle_key_events(minifb::Key::Up, game::Key::Up);
        handle_key_events(minifb::Key::Down, game::Key::Down);
        handle_key_events(minifb::Key::Left, game::Key::Left);
        handle_key_events(minifb::Key::Right, game::Key::Right);
        handle_key_events(minifb::Key::Z, game::Key::Jump);
        handle_key_events(minifb::Key::S, game::Key::S);
        handle_key_events(minifb::Key::Space, game::Key::Space);
        handle_key_events(minifb::Key::LeftBracket, game::Key::LeftBracket);
        handle_key_events(minifb::Key::RightBracket, game::Key::RightBracket);

        handle_key_events(minifb::Key::Equal, game::Key::EditorZoomIn);
        handle_key_events(minifb::Key::Minus, game::Key::EditorZoomOut);

        handle_key_events(minifb::Key::Key1, game::Key::Key1);
        handle_key_events(minifb::Key::Key2, game::Key::Key2);

        // Toggle masks
        handle_key_events(minifb::Key::R, game::Key::MaskRed);
        handle_key_events(minifb::Key::G, game::Key::MaskGreen);
        handle_key_events(minifb::Key::B, game::Key::MaskBlue);

        handle_key_events(minifb::Key::M, game::Key::M);
        handle_key_events(minifb::Key::A, game::Key::MusicC3);
        handle_key_events(minifb::Key::W, game::Key::MusicCs3);
        handle_key_events(minifb::Key::S, game::Key::MusicD3);
        handle_key_events(minifb::Key::E, game::Key::MusicDs3);
        handle_key_events(minifb::Key::D, game::Key::MusicE3);
        handle_key_events(minifb::Key::F, game::Key::MusicF3);
        handle_key_events(minifb::Key::T, game::Key::MusicFs3);
        handle_key_events(minifb::Key::G, game::Key::MusicG3);
        handle_key_events(minifb::Key::Y, game::Key::MusicGs3);
        handle_key_events(minifb::Key::H, game::Key::MusicA3);
        handle_key_events(minifb::Key::U, game::Key::MusicAs3);
        handle_key_events(minifb::Key::J, game::Key::MusicB3);
        handle_key_events(minifb::Key::K, game::Key::MusicC4);
        handle_key_events(minifb::Key::O, game::Key::MusicCs4);
        handle_key_events(minifb::Key::L, game::Key::MusicD4);
        handle_key_events(minifb::Key::P, game::Key::MusicDs4);
        handle_key_events(minifb::Key::Semicolon, game::Key::MusicE4);

        let mut bitmap = if let Some(vulkan_state) = &mut vulkan_state {
            vulkan_state.acquire_bitmap()
        } else {
            minifb_bitmap.take().unwrap()
        };

        // Update the game
        let t = std::time::Instant::now();
        let delta_time = (t - prev_t).as_secs_f32();
        game.tick(delta_time, &mut bitmap);
        prev_t = t;

        if let Some(vulkan_state) = &mut vulkan_state {
            // Upload pixels to the screen
            vulkan_state.blit_to_screen(bitmap);

            // Update the minifb window, get new input data
            window.update();
        } else {
            // Update the minifb window, get new input data, and upload pixels to the screen
            window
                .update_with_buffer(bitmap.pixels(), bitmap.width, bitmap.height)
                .expect("Failed to blit to screen in back-up mode =[");

            minifb_bitmap = Some(bitmap);
        }
    }
}
