use glam::{Vec2, Vec3};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::{
    compositor::{state::ClientDataWrapper, CompositorStateData},
    input::InputManager,
    render::RenderState,
    spatial::{intersect_ray_quad, Ray, WindowQuad},
};

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    pub compositor_state: Option<CompositorStateData>,
    pub display: Option<smithay::reexports::wayland_server::Display<CompositorStateData>>,
    pub socket: Option<smithay::reexports::wayland_server::ListeningSocket>,
    pub socket_name: Option<String>,
    pub quads: Vec<WindowQuad>,
    pub input_manager: InputManager,
    pub is_cursor_grabbed: bool,
    pub start_time: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            render_state: None,
            compositor_state: None,
            display: None,
            socket: None,
            socket_name: None,
            quads: Vec::new(),
            input_manager: InputManager::new(),
            is_cursor_grabbed: false,
            start_time: Instant::now(),
        }
    }

    pub fn set_cursor_grab(&mut self, grab: bool) {
        if let Some(window) = &self.window {
            if grab {
                let _ = window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                window.set_cursor_visible(false);
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
            self.is_cursor_grabbed = grab;
        }
    }

    pub fn current_time_ms(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    /// Perform raycasting from the crosshair (camera forward) against all 3D window quads
    pub fn raycast_crosshair(&self) -> Option<(usize, smithay::utils::Point<f64, smithay::utils::Logical>)> {
        let render_state = self.render_state.as_ref()?;
        let camera = &render_state.camera;
        let ray = Ray::from_camera_center(camera.eye, camera.forward());

        let mut closest_hit: Option<(usize, f32, Vec2)> = None;

        for (idx, quad) in self.quads.iter().enumerate() {
            if let Some(hit) = intersect_ray_quad(&ray, quad.model_matrix()) {
                if let Some((_, best_dist, _)) = closest_hit {
                    if hit.distance < best_dist {
                        closest_hit = Some((idx, hit.distance, hit.uv));
                    }
                } else {
                    closest_hit = Some((idx, hit.distance, hit.uv));
                }
            }
        }

        if let Some((idx, _, uv)) = closest_hit {
            let quad = &self.quads[idx];
            let (w, h) = if quad.dimensions.0 > 0 && quad.dimensions.1 > 0 {
                quad.dimensions
            } else {
                (800, 600)
            };

            let local_x = (uv.x as f64) * (w as f64);
            let local_y = (uv.y as f64) * (h as f64);

            Some((idx, smithay::utils::Point::from((local_x, local_y))))
        } else {
            None
        }
    }

    pub fn dispatch_clients(&mut self) {
        if let (Some(display), Some(compositor_state)) =
            (self.display.as_mut(), self.compositor_state.as_mut())
        {
            if let Some(socket) = self.socket.as_mut() {
                while let Ok(Some(stream)) = socket.accept() {
                    tracing::info!("Accepted new Wayland client connection");
                    let _ = display.handle().insert_client(
                        stream,
                        Arc::new(ClientDataWrapper::default()),
                    );
                }
            }

            let _ = display.dispatch_clients(compositor_state);
            let _ = display.flush_clients();

            // Synchronize newly added windows from CompositorStateData into 3D quads
            if let Some(render_state) = self.render_state.as_ref() {
                while compositor_state.windows.len() > self.quads.len() {
                    let idx = self.quads.len();
                    let window_elem = &compositor_state.windows[idx];
                    let offset_x = (idx as f32) * 2.5 - 1.0;
                    let position = Vec3::new(offset_x, 1.0, 0.0);

                    let quad = WindowQuad::new(
                        window_elem.toplevel.clone(),
                        position,
                        &render_state.device,
                    );
                    self.quads.push(quad);
                    tracing::info!(index = idx, "Added new 3D WindowQuad to virtual space");
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("QuadWM - 3D Spatial Wayland Compositor")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create winit window"),
        );

        let render_state = pollster::block_on(RenderState::new(window.clone()))
            .expect("Failed to initialize WGPU RenderState");

        let display = smithay::reexports::wayland_server::Display::new()
            .expect("Failed to create Wayland display");
        let compositor_state = CompositorStateData::new(&display);

        // Bind listening socket (e.g. wayland-1 or auto)
        let socket = smithay::reexports::wayland_server::ListeningSocket::bind_auto("wayland-", 1..10)
            .expect("Failed to bind Wayland listening socket");
        let socket_name = socket
            .socket_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "wayland-1".to_string());

        tracing::info!("Wayland compositor listening on WAYLAND_DISPLAY={}", socket_name);
        println!("============================================================");
        println!(" QuadWM 3D Spatial Compositor Ready!");
        println!(" Run clients in another terminal:");
        println!("   WAYLAND_DISPLAY={} foot (or alacritty / kitty)", socket_name);
        println!(" Navigation & Interaction:");
        println!("   - Click inside window to grab mouse (Crosshair targeting)");
        println!("   - Point the crosshair at a 3D window to aim");
        println!("   - Left Click while aiming: send click to Wayland app");
        println!("   - Type keys while aiming: forward input to Wayland app");
        println!("   - Press ESC to release mouse cursor");
        println!("   - WASD to fly, Space / Shift for Up / Down");
        println!("============================================================");

        self.window = Some(window);
        self.render_state = Some(render_state);
        self.compositor_state = Some(compositor_state);
        self.display = Some(display);
        self.socket = Some(socket);
        self.socket_name = Some(socket_name);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.dispatch_clients();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if self.is_cursor_grabbed {
            if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
                if let Some(render_state) = self.render_state.as_mut() {
                    render_state.camera_controller.process_mouse(
                        &mut render_state.camera,
                        dx as f32,
                        dy as f32,
                    );
                }

                // Check crosshair raycast and dispatch pointer motion
                if let Some((hit_idx, local_pos)) = self.raycast_crosshair() {
                    let hit_surface = self.quads[hit_idx].toplevel.wl_surface().clone();
                    let time = self.current_time_ms();
                    if let Some(comp_state) = self.compositor_state.as_mut() {
                        let seat = comp_state.seat.clone();
                        self.input_manager.send_pointer_motion(
                            &seat,
                            comp_state,
                            &hit_surface,
                            local_pos,
                            time,
                        );
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(render_state) = self.render_state.as_mut() {
                    render_state.resize((physical_size.width, physical_size.height));
                }
            }
            WindowEvent::MouseInput {
                button,
                state: button_state,
                ..
            } => {
                if !self.is_cursor_grabbed {
                    self.set_cursor_grab(true);
                    return;
                }

                let is_pressed = button_state == ElementState::Pressed;
                let time = self.current_time_ms();

                if let Some((hit_idx, _local_pos)) = self.raycast_crosshair() {
                    let hit_surface = self.quads[hit_idx].toplevel.wl_surface().clone();

                    if let Some(comp_state) = self.compositor_state.as_mut() {
                        let seat = comp_state.seat.clone();

                        // Set focus on click
                        if is_pressed {
                            self.input_manager.set_focus(&seat, comp_state, Some(&hit_surface));
                        }

                        // Send button event (0x110 = BTN_LEFT in Linux evdev)
                        let linux_button = match button {
                            MouseButton::Left => 0x110,
                            MouseButton::Right => 0x111,
                            MouseButton::Middle => 0x112,
                            _ => 0x110,
                        };

                        self.input_manager.send_pointer_button(
                            &seat,
                            comp_state,
                            linux_button,
                            is_pressed,
                            time,
                        );
                    }
                } else if is_pressed {
                    // Clicked in empty 3D space: remove focus
                    if let Some(comp_state) = self.compositor_state.as_mut() {
                        let seat = comp_state.seat.clone();
                        self.input_manager.set_focus(&seat, comp_state, None);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                let is_pressed = state == ElementState::Pressed;

                if is_pressed && key_code == KeyCode::Escape {
                    self.set_cursor_grab(false);
                    return;
                }

                // If focused on a window, forward typing keys to client!
                let has_focus = self.input_manager.focused_surface.is_some();

                // If not focused or if navigation mode active without target: move camera
                if !has_focus || !self.is_cursor_grabbed {
                    if let Some(render_state) = self.render_state.as_mut() {
                        let ctrl = &mut render_state.camera_controller;
                        match key_code {
                            KeyCode::KeyW => ctrl.move_forward = is_pressed,
                            KeyCode::KeyS => ctrl.move_backward = is_pressed,
                            KeyCode::KeyA => ctrl.move_left = is_pressed,
                            KeyCode::KeyD => ctrl.move_right = is_pressed,
                            KeyCode::Space => ctrl.move_up = is_pressed,
                            KeyCode::ShiftLeft => ctrl.move_down = is_pressed,
                            _ => {}
                        }
                    }
                }

                // If focused, forward scancode to client via Smithay keyboard
                if has_focus {
                    let evdev_code = key_code_to_evdev(key_code);
                    if let Some(code) = evdev_code {
                        let time = self.current_time_ms();
                        if let Some(comp_state) = self.compositor_state.as_mut() {
                            let seat = comp_state.seat.clone();
                            self.input_manager.send_key(&seat, comp_state, code, is_pressed, time);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.dispatch_clients();

                // Synchronize Wayland surface pixels into GPU textures
                if let Some(render_state) = self.render_state.as_ref() {
                    for quad in &mut self.quads {
                        quad.ensure_initialized(
                            &render_state.device,
                            &render_state.queue,
                            &render_state.sampler,
                            &render_state.model_bind_group_layout,
                        );
                        quad.sync_surface_buffer(
                            &render_state.device,
                            &render_state.queue,
                            &render_state.sampler,
                            &render_state.model_bind_group_layout,
                        );
                    }
                }

                if let Some(render_state) = self.render_state.as_mut() {
                    render_state.update(1.0 / 60.0);

                    // Collect active quad bind groups
                    let bind_groups: Vec<&wgpu::BindGroup> = self
                        .quads
                        .iter()
                        .filter_map(|q| q.bind_group.as_ref())
                        .collect();

                    if let Err(err) = render_state.render(&bind_groups) {
                        tracing::error!("Render error: {:?}", err);
                    }
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Convert Winit KeyCode to Linux Evdev scancodes
fn key_code_to_evdev(code: KeyCode) -> Option<u32> {
    match code {
        KeyCode::KeyA => Some(30),
        KeyCode::KeyB => Some(48),
        KeyCode::KeyC => Some(46),
        KeyCode::KeyD => Some(32),
        KeyCode::KeyE => Some(18),
        KeyCode::KeyF => Some(33),
        KeyCode::KeyG => Some(34),
        KeyCode::KeyH => Some(35),
        KeyCode::KeyI => Some(23),
        KeyCode::KeyJ => Some(36),
        KeyCode::KeyK => Some(37),
        KeyCode::KeyL => Some(38),
        KeyCode::KeyM => Some(50),
        KeyCode::KeyN => Some(49),
        KeyCode::KeyO => Some(24),
        KeyCode::KeyP => Some(25),
        KeyCode::KeyQ => Some(16),
        KeyCode::KeyR => Some(19),
        KeyCode::KeyS => Some(31),
        KeyCode::KeyT => Some(20),
        KeyCode::KeyU => Some(22),
        KeyCode::KeyV => Some(47),
        KeyCode::KeyW => Some(17),
        KeyCode::KeyX => Some(45),
        KeyCode::KeyY => Some(21),
        KeyCode::KeyZ => Some(44),
        KeyCode::Digit1 => Some(2),
        KeyCode::Digit2 => Some(3),
        KeyCode::Digit3 => Some(4),
        KeyCode::Digit4 => Some(5),
        KeyCode::Digit5 => Some(6),
        KeyCode::Digit6 => Some(7),
        KeyCode::Digit7 => Some(8),
        KeyCode::Digit8 => Some(9),
        KeyCode::Digit9 => Some(10),
        KeyCode::Digit0 => Some(11),
        KeyCode::Enter => Some(28),
        KeyCode::Escape => Some(1),
        KeyCode::Backspace => Some(14),
        KeyCode::Tab => Some(15),
        KeyCode::Space => Some(57),
        KeyCode::Minus => Some(12),
        KeyCode::Equal => Some(13),
        KeyCode::BracketLeft => Some(26),
        KeyCode::BracketRight => Some(27),
        KeyCode::Backslash => Some(43),
        KeyCode::Semicolon => Some(39),
        KeyCode::Quote => Some(40),
        KeyCode::Comma => Some(51),
        KeyCode::Period => Some(52),
        KeyCode::Slash => Some(53),
        KeyCode::ArrowUp => Some(103),
        KeyCode::ArrowDown => Some(108),
        KeyCode::ArrowLeft => Some(105),
        KeyCode::ArrowRight => Some(106),
        _ => None,
    }
}
