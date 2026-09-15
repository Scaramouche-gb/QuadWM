//! QuadWM - 3D Spatial Wayland Compositor

pub mod backend;
pub mod compositor;
pub mod input;
pub mod render;
pub mod spatial;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting QuadWM 3D Compositor...");
    Ok(())
}
