//! QuadWM - 3D Spatial Wayland Compositor

pub mod backend;
pub mod compositor;
pub mod input;
pub mod render;
pub mod spatial;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,quadwm=debug")),
        )
        .init();
    tracing::info!("Starting QuadWM 3D Compositor...");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = backend::App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
