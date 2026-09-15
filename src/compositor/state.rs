use smithay::{
    delegate_compositor, delegate_output, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{Seat, SeatHandler, SeatState},
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        protocol::wl_surface::WlSurface,
        Client, Display, DisplayHandle, Resource,
    },
    utils::{Point, Size},
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        output::OutputHandler,
        shell::xdg::{
            PositionerState, PopupSurface, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
    },
};

pub struct WindowElement {
    pub toplevel: ToplevelSurface,
}

pub struct CompositorStateData {
    pub display_handle: DisplayHandle,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub seat: Seat<Self>,
    pub output: Output,
    pub windows: Vec<WindowElement>,
    pub is_running: bool,
}

impl CompositorStateData {
    pub fn new(display: &Display<Self>) -> Self {
        let display_handle = display.handle();

        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, "seat-0");
        let _ = seat.add_keyboard(Default::default(), 200, 25);
        let _ = seat.add_pointer();

        // Initialize virtual output for clients to recognize display capabilities
        let output = Output::new(
            "QuadWM-Virtual-1".to_string(),
            PhysicalProperties {
                size: Size::from((1920, 1080)),
                subpixel: Subpixel::Unknown,
                make: "QuadWM".to_string(),
                model: "Spatial Display".to_string(),
            },
        );

        let mode = Mode {
            size: Size::from((1920, 1080)),
            refresh: 60_000,
        };
        output.change_current_state(
            Some(mode),
            None,
            None,
            Some(Point::from((0, 0))),
        );
        output.set_preferred(mode);
        output.create_global::<Self>(&display_handle);

        Self {
            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            seat,
            output,
            windows: Vec::new(),
            is_running: true,
        }
    }
}

// Client tracking data
#[derive(Default)]
pub struct ClientDataWrapper {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientDataWrapper {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

// Smithay trait implementations
impl BufferHandler for CompositorStateData {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {}
}

impl CompositorHandler for CompositorStateData {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<ClientDataWrapper>()
            .expect("Missing ClientDataWrapper on client")
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        tracing::info!(surface = ?surface.id(), "Wayland surface commit received");
    }
}

impl XdgShellHandler for CompositorStateData {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        tracing::info!(surface = ?surface.wl_surface().id(), "New XDG toplevel client connected");
        
        // Send configure with initial size hint 800x600 so clients know their layout
        surface.with_pending_state(|state| {
            state.size = Some((800, 600).into());
        });
        surface.send_configure();
        
        self.windows.push(WindowElement { toplevel: surface });
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }

    fn grab(
        &mut self,
        _surface: PopupSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
    ) {
    }
}

impl ShmHandler for CompositorStateData {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for CompositorStateData {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

impl OutputHandler for CompositorStateData {}

// Macro delegations for Smithay protocols
delegate_compositor!(CompositorStateData);
delegate_xdg_shell!(CompositorStateData);
delegate_shm!(CompositorStateData);
delegate_seat!(CompositorStateData);
delegate_output!(CompositorStateData);
