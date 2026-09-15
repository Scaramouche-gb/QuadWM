use std::sync::Arc;
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    compositor::{state::ClientDataWrapper, CompositorStateData},
    render::RenderState,
};

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    pub compositor_state: Option<CompositorStateData>,
    pub display: Option<Display<CompositorStateData>>,
    pub socket: Option<ListeningSocket>,
    pub socket_name: Option<String>,
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
        println!("==> QuadWM is ready. Connect clients using: WAYLAND_DISPLAY={}", socket_name);

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
            WindowEvent::RedrawRequested => {
                self.dispatch_clients();

                if let Some(render_state) = self.render_state.as_mut() {
                    if let Err(err) = render_state.render() {
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
