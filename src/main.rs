use procedural_city_engine::app::{App, AppError};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> Result<(), AppError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Iniciando Procedural City Engine...");

    let event_loop = EventLoop::new().map_err(AppError::EventLoop)?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new()?;
    event_loop.run_app(&mut app).map_err(AppError::EventLoop)?;
    app.into_result()?;

    log::info!("Procedural City Engine finalizó correctamente.");
    Ok(())
}
