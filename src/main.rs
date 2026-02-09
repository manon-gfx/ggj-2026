pub mod audio;
pub(crate) mod bitmap;
pub(crate) mod game;

use ash::{
    Entry,
    vk::{
        self, CommandBufferBeginInfo, CommandBufferResetFlags, CommandPoolCreateFlags,
        CommandPoolCreateInfo, FenceCreateFlags, MemoryAllocateInfo, MemoryMapFlags,
        MemoryPropertyFlags, SampleCountFlags,
    },
};
use bitmap::Bitmap;
use game::Game;

use gilrs::Gilrs;
use minifb::WindowOptions;
use raw_window_handle::HasDisplayHandle;

// Set to true to enable fullscreen mode
const FULLSCREEN: bool = false;

// Set to true to check if the vulkan calls we are making are correct
const ENABLE_VALIDATION_LAYER: bool = false;

// Double buffering, draw to one frame while the other frame is copied to the screen
// Set to 1 for single buffering or to 3 for triple buffering
const IN_FLIGHT_COUNT: u32 = 2;

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

    // Disable maximum FPS by sleeping the thread, aka we want ALL the frames (or maybe we dont :p )
    window.set_target_fps(60);

    let vulkan_init_start = std::time::Instant::now();
    // Load Vulkan dynamic library (.so/.dll)
    let entry = unsafe { Entry::load().expect("Failed to load vulkan :(") };

    // Setup Vulkan
    let app_info = vk::ApplicationInfo {
        api_version: vk::make_api_version(0, 1, 1, 0),
        ..Default::default()
    };
    let layers = if ENABLE_VALIDATION_LAYER {
        &[c"VK_LAYER_KHRONOS_validation".as_ptr()]
    } else {
        &[] as &[*const i8]
    };
    let window_extensions =
        ash_window::enumerate_required_extensions(window.display_handle().unwrap().into()).unwrap();
    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(layers)
        .enabled_extension_names(window_extensions);
    let vk_instance = unsafe { entry.create_instance(&create_info, None).unwrap() };

    // Query GPUs in the system
    let physical_devices = unsafe { vk_instance.enumerate_physical_devices() }.unwrap();
    if physical_devices.is_empty() {
        panic!("Failed to find a vulkan device :(");
    }

    let physical_device = physical_devices[0];

    // Find a suitible GPU submission queue (one that supports displaying graphics to the screen)
    let queue_family_index =
        unsafe { vk_instance.get_physical_device_queue_family_properties(physical_device) }
            .iter()
            .enumerate()
            .find_map(|(index, info)| {
                let supports_graphic_and_surface =
                    info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                if supports_graphic_and_surface {
                    Some(index as u32)
                } else {
                    None
                }
            });
    let queue_family_index = queue_family_index.unwrap();

    // Create a vulkan device to talk to the GPU
    let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&[0.5])];
    let device_extensions = vec![ash::khr::swapchain::NAME.as_ptr()];
    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions);
    let device =
        unsafe { vk_instance.create_device(physical_devices[0], &device_create_info, None) }
            .expect("Failed to create vulkan device");

    // Load functions from the surface and swapchain extensions to interface with the window
    let swapchain_loader = ash::khr::swapchain::Device::new(&vk_instance, &device);
    let surface_loader = ash::khr::surface::Instance::new(&entry, &vk_instance);

    // Create a vulkan surface for the minifb window
    let surface = unsafe {
        use raw_window_handle::*;
        let display_handle = window.display_handle().unwrap();
        let window_handle = window.window_handle().unwrap();
        ash_window::create_surface(
            &entry,
            &vk_instance,
            display_handle.into(),
            window_handle.into(),
            None,
        )
    }
    .unwrap();

    // Find our desired surface format. Every device should support `B8G8R8A8_UNORM``
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
            .unwrap();
    let surface_format = surface_formats
        .iter()
        .map(|sfmt| match sfmt.format {
            vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: sfmt.color_space,
            },
            _ => *sfmt,
        })
        .next()
        .expect("Unable to find suitable surface format.");

    // Set up a swapchain (an objecect that gives us images that can be shown on the screen)
    let surface_capabilities = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap()
    };

    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    }
    .unwrap();
    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::IMMEDIATE)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(IN_FLIGHT_COUNT)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(surface_capabilities.current_extent)
        .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);

    let swapchain =
        unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();

    // Get the images created by the swapchain
    let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();

    // Grab the device queue we asked for during device creation
    // This queue allows us to submit commnds to the GPU
    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    // Set up command buffers to record commands in, these can be submitted to the queue later.
    let command_pool = {
        let create_info =
            CommandPoolCreateInfo::default().flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        unsafe { device.create_command_pool(&create_info, None) }.unwrap()
    };
    let command_buffers = {
        let info: vk::CommandBufferAllocateInfo<'_> = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(IN_FLIGHT_COUNT)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        unsafe { device.allocate_command_buffers(&info) }.unwrap()
    };

    // Set up textures that we access on the CPU and copy to the swapchain images.
    let upload_textures: Vec<vk::Image> = (0..IN_FLIGHT_COUNT)
        .map(|_| {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .extent(
                    vk::Extent3D::default()
                        .width(render_width as u32)
                        .height(render_height as u32)
                        .depth(1),
                )
                .mip_levels(1)
                .array_layers(1)
                .samples(SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::LINEAR)
                .usage(vk::ImageUsageFlags::TRANSFER_SRC)
                .initial_layout(vk::ImageLayout::PREINITIALIZED);
            unsafe { device.create_image(&create_info, None) }.unwrap()
        })
        .collect::<Vec<_>>();

    let layout = unsafe {
        device.get_image_subresource_layout(
            upload_textures[0],
            vk::ImageSubresource {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                array_layer: 0,
            },
        )
    };
    let render_stride = layout.row_pitch / 4;

    // Find a compatible memory type for the upload textures
    let mem_props = unsafe { vk_instance.get_physical_device_memory_properties(physical_device) };
    let mut memory_type = (0..mem_props.memory_type_count).find(|&type_index| {
        mem_props.memory_types[type_index as usize].property_flags
            == MemoryPropertyFlags::HOST_VISIBLE
                | MemoryPropertyFlags::HOST_COHERENT
                | MemoryPropertyFlags::HOST_CACHED
    });
    if memory_type.is_none() {
        memory_type = (0..mem_props.memory_type_count).find(|&type_index| {
            mem_props.memory_types[type_index as usize]
                .property_flags
                .contains(MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT)
        })
    }
    let Some(memory_type) = memory_type else {
        panic!(
            "Failed to find a comptabile memory type even though the spec guarantees one to be there :("
        )
    };

    // keep alive for RAII purposes
    let upload_texture_allocations = upload_textures
        .iter()
        .map(|&image| {
            // ask what memory requirement the texture needs
            let requirements = unsafe { device.get_image_memory_requirements(image) };

            // allocate a dedicated block of memory for the texture
            let device_memory = unsafe {
                device
                    .allocate_memory(
                        &MemoryAllocateInfo::default()
                            .allocation_size(requirements.size)
                            .memory_type_index(memory_type),
                        None,
                    )
                    .unwrap()
            };

            // associate the memory with the texture and get a pointer to it we can write to :D
            unsafe { device.bind_image_memory(image, device_memory, 0) }.unwrap();
            let mapped_ptr = unsafe {
                device.map_memory(device_memory, 0, requirements.size, MemoryMapFlags::empty())
            }
            .unwrap();

            (device_memory, mapped_ptr)
        })
        .collect::<Vec<_>>();

    // Set up synchronization primitives.
    // The semaphores are for sync'ing work on the GPU.
    // The fences are for sync'ing the CPU and GPU
    let image_acquired_semaphores = (0..IN_FLIGHT_COUNT)
        .map(|_| {
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap()
        })
        .collect::<Vec<_>>();
    let rendering_complete_semaphores = (0..IN_FLIGHT_COUNT)
        .map(|_| {
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap()
        })
        .collect::<Vec<_>>();
    let fences = (0..IN_FLIGHT_COUNT)
        .map(|_| {
            unsafe {
                device.create_fence(
                    &vk::FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED),
                    None,
                )
            }
            .unwrap()
        })
        .collect::<Vec<_>>();

    let vulkan_init_end = std::time::Instant::now();
    println!(
        "Managed to initialize Vulkan! Enjoy :D. It took {:?}",
        vulkan_init_end - vulkan_init_start
    );

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

    let mut swap_index = 0;
    while window.is_open() {
        while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _code) => match button {
                    gilrs::Button::North => game.on_key_down(game::Key::Jump),
                    gilrs::Button::South => game.on_key_down(game::Key::MaskGreen),
                    gilrs::Button::East => game.on_key_down(game::Key::MaskRed),
                    gilrs::Button::West => game.on_key_down(game::Key::MaskBlue),

                    gilrs::Button::DPadUp => game.on_key_down(game::Key::MoveUp),
                    gilrs::Button::DPadDown => game.on_key_down(game::Key::MoveDown),
                    gilrs::Button::DPadLeft => game.on_key_down(game::Key::MoveLeft),
                    gilrs::Button::DPadRight => game.on_key_down(game::Key::MoveRight),
                    _ => {}
                },
                gilrs::EventType::ButtonReleased(button, _code) => match button {
                    gilrs::Button::North => game.on_key_up(game::Key::Jump),
                    gilrs::Button::South => game.on_key_up(game::Key::MaskGreen),
                    gilrs::Button::East => game.on_key_up(game::Key::MaskRed),
                    gilrs::Button::West => game.on_key_up(game::Key::MaskBlue),

                    gilrs::Button::DPadUp => game.on_key_up(game::Key::MoveUp),
                    gilrs::Button::DPadDown => game.on_key_up(game::Key::MoveDown),
                    gilrs::Button::DPadLeft => game.on_key_up(game::Key::MoveLeft),
                    gilrs::Button::DPadRight => game.on_key_up(game::Key::MoveRight),
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

        let mut handle_key_events = |minifb_key, key| {
            if window.is_key_pressed(minifb_key, minifb::KeyRepeat::No) {
                game.on_key_down(key);
            }
            if window.is_key_released(minifb_key) {
                game.on_key_up(key);
            }
        };

        // Movement: WASD or arrow keys
        // Jump: Space or Z
        // Level editor: E (to avoid jump conflict)
        // Saving in level editor = F (for "File") (Changed to avoid WASD movement conflict, ctrl+S might be nice in the future) 
        // Masks: J, K & L (or R, G & B)

        // Mute audio in game = M
        
        // music mode = V

        handle_key_events(minifb::Key::Up, game::Key::MoveUp);
        handle_key_events(minifb::Key::Down, game::Key::MoveDown);
        handle_key_events(minifb::Key::Left, game::Key::MoveLeft);
        handle_key_events(minifb::Key::Right, game::Key::MoveRight);

        handle_key_events(minifb::Key::W, game::Key::MoveUp);
        handle_key_events(minifb::Key::S, game::Key::MoveDown);
        handle_key_events(minifb::Key::A, game::Key::MoveLeft);
        handle_key_events(minifb::Key::D, game::Key::MoveRight);

        handle_key_events(minifb::Key::Z, game::Key::Jump);
        handle_key_events(minifb::Key::Space, game::Key::Jump);

        handle_key_events(minifb::Key::F, game::Key::SaveLevelEdit);
        handle_key_events(minifb::Key::S, game::Key::SaveLevelEdit);

        handle_key_events(minifb::Key::E, game::Key::EditMode);

        handle_key_events(minifb::Key::M, game::Key::MuteAudio);

        handle_key_events(minifb::Key::LeftBracket, game::Key::SelectPrev);
        handle_key_events(minifb::Key::RightBracket, game::Key::SelectNext);


        handle_key_events(minifb::Key::Equal, game::Key::EditorZoomIn);
        handle_key_events(minifb::Key::Minus, game::Key::EditorZoomOut);

        handle_key_events(minifb::Key::Key1, game::Key::Key1);
        handle_key_events(minifb::Key::Key2, game::Key::Key2);

        // Toggle masks
        handle_key_events(minifb::Key::R, game::Key::MaskRed);
        handle_key_events(minifb::Key::G, game::Key::MaskGreen);
        handle_key_events(minifb::Key::B, game::Key::MaskBlue);

        handle_key_events(minifb::Key::J, game::Key::MaskRed);
        handle_key_events(minifb::Key::K, game::Key::MaskBlue);
        handle_key_events(minifb::Key::L, game::Key::MaskGreen);

        // Music mode
        handle_key_events(minifb::Key::V, game::Key::MusicMode);

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

        // Wait for any previous work on the GPU to finish
        let fence = fences[swap_index];
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX).unwrap() };
        unsafe { device.reset_fences(&[fence]) }.unwrap();
        let image_acquired_semaphore = image_acquired_semaphores[swap_index];
        let (image_index, _suboptimal) = unsafe {
            swapchain_loader.acquire_next_image(
                swapchain,
                u64::MAX,
                image_acquired_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();
        let image_index = image_index as usize;

        // Set up a bitmap to write to the upload buffer
        let mapped_ptr = upload_texture_allocations[swap_index].1;
        let mut bitmap = Bitmap::new_borrowed(
            mapped_ptr as *mut _,
            render_width,
            render_height,
            render_stride as usize,
        );

        // Update the game
        let t = std::time::Instant::now();
        let delta_time = (t - prev_t).as_secs_f32();
        game.tick(delta_time, &mut bitmap);
        prev_t = t;

        let cmd = command_buffers[swap_index];
        let swapchain_image = swapchain_images[image_index];
        unsafe {
            // Record commands in the command buffer buffer
            device
                .reset_command_buffer(cmd, CommandBufferResetFlags::empty())
                .unwrap();
            device
                .begin_command_buffer(
                    cmd,
                    &CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();

            const SINGLE_IMAGE_SUBRESOURCE_RANGE: vk::ImageSubresourceRange =
                vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };
            const SINGLE_IMAGE_SUBRESOURCE_LAYERS: vk::ImageSubresourceLayers =
                vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                };

            let upload_texture = upload_textures[swap_index];
            // Transition upload buffer and swapchain in correct state for copying
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[
                    vk::ImageMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .image(upload_texture)
                        .subresource_range(SINGLE_IMAGE_SUBRESOURCE_RANGE),
                    vk::ImageMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .image(swapchain_image)
                        .subresource_range(SINGLE_IMAGE_SUBRESOURCE_RANGE),
                ],
            );

            let scale_x = window_width as f32 / render_width as f32;
            let scale_y = window_height as f32 / render_height as f32;
            let scale = scale_x.min(scale_y);

            let blit_width = (render_width as f32 * scale) as i32;
            let blit_height = (render_height as f32 * scale) as i32;

            let blit_x = (window_width as i32 - blit_width) / 2;
            let blit_y = (window_height as i32 - blit_height) / 2;

            // copy image to the swapchain
            device.cmd_blit_image(
                cmd,
                upload_texture,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::default()
                    .src_subresource(SINGLE_IMAGE_SUBRESOURCE_LAYERS)
                    .src_offsets([
                        vk::Offset3D::default(),
                        vk::Offset3D {
                            x: render_width as i32,
                            y: render_height as i32,
                            z: 1,
                        },
                    ])
                    .dst_subresource(SINGLE_IMAGE_SUBRESOURCE_LAYERS)
                    .dst_offsets([
                        vk::Offset3D {
                            x: blit_x,
                            y: blit_y,
                            z: 0,
                        },
                        vk::Offset3D {
                            x: blit_x + blit_width,
                            y: blit_y + blit_height,
                            z: 1,
                        },
                    ])],
                vk::Filter::NEAREST,
            );

            // transistion swapchain to correct state for presenting to the screen
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::empty())
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(swapchain_image)
                    .subresource_range(SINGLE_IMAGE_SUBRESOURCE_RANGE)],
            );

            device.end_command_buffer(cmd).unwrap();

            let rendering_complete_semaphore = rendering_complete_semaphores[swap_index];
            // Submit command buffer to the GPU Queue
            {
                let wait_semaphores = [image_acquired_semaphores[swap_index]]; // start rendering when swapchain image is free
                let wait_stages = [vk::PipelineStageFlags::ALL_COMMANDS];
                let command_buffers = [cmd];
                let signal_semaphores = [rendering_complete_semaphore];
                let submit_info = vk::SubmitInfo::default()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_stages)
                    .command_buffers(&command_buffers)
                    .signal_semaphores(&signal_semaphores);
                device.queue_submit(queue, &[submit_info], fence).unwrap();
            }

            // Submit swapchain present to the GPU queue
            {
                let wait_semaphores = [rendering_complete_semaphore]; // only present after rendering is done
                let swapchains = [swapchain];
                let image_indices = [image_index as u32];
                let mut results = [vk::Result::SUCCESS];
                let present_info = vk::PresentInfoKHR::default()
                    .wait_semaphores(&wait_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&image_indices)
                    .results(&mut results);
                swapchain_loader
                    .queue_present(queue, &present_info)
                    .unwrap();
            }
        };

        // Update the minifb window, get new input data
        window.update();

        // Move to the next buffer
        swap_index = (swap_index + 1) % IN_FLIGHT_COUNT as usize;
    }
}
