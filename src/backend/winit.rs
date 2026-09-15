use glam::Vec3;
use std::sync::Arc;
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::{
    compositor::{state::ClientDataWrapper, CompositorStateData},
    render::RenderState,
    spatial::WindowQuad,
};

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    pub compositor_state: Option<CompositorStateData>,
    pub display: Option<Display<CompositorStateData>>,
    pub socket: Option<ListeningSocket>,
    pub socket_name: Option<String>,
    pub quads: Vec<WindowQuad>,
    pub is_cursor_grabbed: bool,
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
            is_cursor_grabbed: false,
        }
    }

    pub fn set_cursor_grab(&mut self, grab: bool) {
        if let Some(window) = &self.window {
            if grab {
                let _ = window.set_cursor_grab(CursorGrabMode::Confined)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                window.set_cursor_visible(false);
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
            self.is_cursor_grabbed = grab;
        }
    }

    pub fn dispatch_clients(&mut self) {
        if let (Some(display), Some(compositor_state)) =
            (self.display.as_mut(), self.compositor_state.as_mut())
        {
            if let Some(socket) = self.socket.as_mut() {
                if let Ok(Some(stream)) = socket.accept() {
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

        let display = Display::new().expect("Failed to create Wayland display");
        let compositor_state = CompositorStateData::new(&display);

        // Bind listening socket (e.g. wayland-1 or auto)
        let socket = ListeningSocket::bind_auto("wayland-", 1..10)
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
        println!(" Navigation:");
        println!("   - Click inside window to grab mouse for FPS look");
        println!("   - Press ESC to release mouse cursor");
        println!("   - WASD to move in 3D space, Space / LShift for Up / Down");
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
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                if !self.is_cursor_grabbed {
                    self.set_cursor_grab(true);
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
