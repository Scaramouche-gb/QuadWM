//! QuadWM - 3D Spatial Wayland Compositor

pub mod backend;
pub mod compositor;
pub mod input;
pub mod render;
pub mod spatial;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting QuadWM 3D Compositor...");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = backend::App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
