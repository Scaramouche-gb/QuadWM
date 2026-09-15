use smithay::{
    delegate_compositor, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{Seat, SeatHandler, SeatState},
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        protocol::wl_surface::WlSurface,
        Client, Display, DisplayHandle, Resource,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
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
        let seat = seat_state.new_wl_seat(&display_handle, "seat-0");

        Self {
            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            seat,
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
        tracing::trace!(surface = ?surface.id(), "Surface committed");
    }
}

impl XdgShellHandler for CompositorStateData {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        tracing::info!(surface = ?surface.wl_surface().id(), "New XDG toplevel client connected");
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

// Macro delegations for Smithay protocols
delegate_compositor!(CompositorStateData);
delegate_xdg_shell!(CompositorStateData);
delegate_shm!(CompositorStateData);
delegate_seat!(CompositorStateData);
