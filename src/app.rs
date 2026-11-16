use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

#[cfg(feature = "diagnostics")]
use crate::diagnostics::Diagnostics;
use crate::geometry::primitives::{Cube, RectangularPrism};
use crate::meshing::{mesh_building, MeshingError};
use crate::procedural::{generate_house, HouseConfig, HouseGenerationError};
use crate::render::{
    CameraInput, CameraMouseButton, CameraMove, FacePalette, FrameOutcome, RenderContext,
    RenderError, RenderInitError, RenderMode, RenderSettingsError,
};
use crate::scene::{Scene, SceneError, Transform};

/// Error que impide iniciar o ejecutar correctamente la aplicación.
#[derive(Debug)]
pub enum AppError {
    /// No se pudo crear o ejecutar el bucle de eventos de la plataforma.
    EventLoop(winit::error::EventLoopError),
    /// No se pudo crear la ventana principal.
    WindowCreation(winit::error::OsError),
    /// No se pudo inicializar el contexto de renderizado.
    RenderInitialization(RenderInitError),
    /// No se pudo construir la escena determinista de demostración.
    DemoScene(SceneError),
    HouseGeneration(HouseGenerationError),
    HouseMeshing(MeshingError),
    RenderSettings(RenderSettingsError),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoop(error) => write!(f, "error en el bucle de eventos: {error}"),
            Self::WindowCreation(error) => write!(f, "no se pudo crear la ventana: {error}"),
            Self::RenderInitialization(error) => {
                write!(f, "no se pudo inicializar el renderizador: {error}")
            }
            Self::DemoScene(error) => {
                write!(f, "no se pudo construir la escena de demostración: {error}")
            }
            Self::HouseGeneration(error) => write!(f, "error al generar la vivienda: {error}"),
            Self::HouseMeshing(error) => write!(f, "error al mallar la vivienda: {error}"),
            Self::RenderSettings(error) => write!(f, "configuración gráfica inválida: {error}"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EventLoop(error) => Some(error),
            Self::WindowCreation(error) => Some(error),
            Self::RenderInitialization(error) => Some(error),
            Self::DemoScene(error) => Some(error),
            Self::HouseGeneration(error) => Some(error),
            Self::HouseMeshing(error) => Some(error),
            Self::RenderSettings(error) => Some(error),
        }
    }
}

/// Controlador principal del ciclo de vida, entradas y renderizado de la aplicación.
pub struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    scene: Scene,
    palette: FacePalette,
    render_mode: RenderMode,
    isometric: bool,
    frame_count: u64,
    max_frames: Option<u64>,
    startup_error: Option<AppError>,
    last_camera_update: Instant,
    #[cfg(feature = "diagnostics")]
    diagnostics: Diagnostics,
}

#[derive(Debug, PartialEq, Eq)]
enum FrameAction {
    Wait,
    Redraw,
    Exit(u64),
}

impl App {
    /// Crea una aplicación sin ventana ni recursos de GPU inicializados.
    pub fn new() -> Result<Self, AppError> {
        let max_frames = std::env::var("PCE_MAX_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());
        let mode = std::env::var("PCE_RENDER_MODE").ok();
        let scale = std::env::var("PCE_PIXEL_SCALE").ok();
        let render_mode = RenderMode::parse(mode.as_deref(), scale.as_deref())
            .map_err(AppError::RenderSettings)?;
        let isometric = std::env::var("PCE_SCENE").as_deref() == Ok("house");
        let (scene, palette) = if isometric {
            (house_scene()?, FacePalette::house())
        } else {
            (
                demo_scene().map_err(AppError::DemoScene)?,
                FacePalette::cube(),
            )
        };

        Ok(Self {
            window: None,
            render_context: None,
            scene,
            palette,
            render_mode,
            isometric,
            frame_count: 0,
            max_frames,
            startup_error: None,
            last_camera_update: Instant::now(),
            #[cfg(feature = "diagnostics")]
            diagnostics: Diagnostics::new(Instant::now()),
        })
    }

    /// Devuelve el error de inicio capturado por el controlador de eventos.
    pub fn into_result(self) -> Result<(), AppError> {
        self.startup_error.map_or(Ok(()), Err)
    }

    fn account_frame(
        &mut self,
        outcome: FrameOutcome,
        #[cfg(feature = "diagnostics")] render_started: Instant,
    ) -> FrameAction {
        if outcome == FrameOutcome::Skipped {
            return FrameAction::Wait;
        }

        #[cfg(feature = "diagnostics")]
        self.diagnostics
            .record_presented(Instant::now(), render_started.elapsed());

        self.frame_count += 1;
        match self.max_frames {
            Some(limit) if self.frame_count >= limit => FrameAction::Exit(limit),
            Some(_) => FrameAction::Redraw,
            None => FrameAction::Wait,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            log::info!("Creando ventana de la aplicación...");
            let attributes = WindowAttributes::default()
                .with_title("Procedural City Engine")
                .with_inner_size(PhysicalSize::new(1024, 768));

            let window = match event_loop.create_window(attributes) {
                Ok(w) => Arc::new(w),
                Err(err) => {
                    log::error!("No se pudo crear la ventana principal: {err}");
                    self.startup_error = Some(AppError::WindowCreation(err));
                    event_loop.exit();
                    return;
                }
            };

            log::info!("Inicializando contexto de renderizado wgpu...");
            match pollster::block_on(RenderContext::with_presentation(
                Arc::clone(&window),
                &self.scene,
                &self.palette,
                self.render_mode,
                self.isometric,
            )) {
                Ok(ctx) => {
                    log::info!("Contexto de renderizado inicializado correctamente.");
                    if ctx.has_drawable_surface() {
                        window.request_redraw();
                    }
                    self.render_context = Some(ctx);
                    self.window = Some(window);
                }
                Err(err) => {
                    log::error!("No se pudo inicializar el contexto de renderizado: {err}");
                    self.startup_error = Some(AppError::RenderInitialization(err));
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let (Some(window), Some(render_context)) =
            (self.window.as_ref(), self.render_context.as_mut())
        else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("El usuario solicitó cerrar la aplicación.");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    log::info!("El usuario solicitó cerrar la aplicación.");
                    event_loop.exit();
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    let input = match code {
                        KeyCode::KeyW => Some(CameraInput::Move {
                            direction: CameraMove::Forward,
                            pressed: event.state == ElementState::Pressed,
                        }),
                        KeyCode::KeyS => Some(CameraInput::Move {
                            direction: CameraMove::Backward,
                            pressed: event.state == ElementState::Pressed,
                        }),
                        KeyCode::KeyA => Some(CameraInput::Move {
                            direction: CameraMove::Left,
                            pressed: event.state == ElementState::Pressed,
                        }),
                        KeyCode::KeyD => Some(CameraInput::Move {
                            direction: CameraMove::Right,
                            pressed: event.state == ElementState::Pressed,
                        }),
                        KeyCode::KeyR if event.state == ElementState::Pressed => {
                            Some(CameraInput::Reset)
                        }
                        _ => None,
                    };
                    if let Some(input) = input {
                        let was_moving = render_context.has_continuous_camera_motion();
                        let camera_changed = render_context.handle_camera_input(input);
                        if !was_moving && render_context.has_continuous_camera_motion() {
                            self.last_camera_update = Instant::now();
                        }
                        if camera_changed && render_context.has_drawable_surface() {
                            window.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.x.is_finite() && position.y.is_finite() {
                    let input = CameraInput::CursorMoved(glam::Vec2::new(
                        position.x as f32,
                        position.y as f32,
                    ));
                    if render_context.handle_camera_input(input)
                        && render_context.has_drawable_surface()
                    {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                render_context.handle_camera_input(CameraInput::PointerLeft);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => Some(CameraMouseButton::Left),
                    MouseButton::Middle => Some(CameraMouseButton::Middle),
                    _ => None,
                };
                if let Some(button) = button {
                    render_context.handle_camera_input(CameraInput::MouseButton {
                        button,
                        pressed: state == ElementState::Pressed,
                    });
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let input = match delta {
                    MouseScrollDelta::LineDelta(_, vertical) => CameraInput::ScrollLines(vertical),
                    MouseScrollDelta::PixelDelta(position) => {
                        CameraInput::ScrollPixels(position.y as f32)
                    }
                };
                if render_context.handle_camera_input(input)
                    && render_context.has_drawable_surface()
                {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(false) => {
                render_context.handle_camera_input(CameraInput::FocusLost);
            }
            WindowEvent::Resized(new_size) => {
                log::debug!("Ventana redimensionada a: {:?}", new_size);
                render_context.resize(new_size);
                if render_context.has_drawable_surface() {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(feature = "diagnostics")]
                let render_started = Instant::now();
                let result = render_context.render(&self.scene);
                match result {
                    Ok(outcome) => match self.account_frame(
                        outcome,
                        #[cfg(feature = "diagnostics")]
                        render_started,
                    ) {
                        FrameAction::Wait => {}
                        FrameAction::Redraw => {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                        FrameAction::Exit(limit) => {
                            log::info!(
                                "Se alcanzó el límite de fotogramas ({}); se cerrará la aplicación.",
                                limit
                            );
                            event_loop.exit();
                        }
                    },
                    Err(RenderError::Surface(
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                    )) => {
                        log::warn!("La superficie se perdió o quedó obsoleta; se reconfigurará.");
                        render_context.reconfigure();
                        window.request_redraw();
                    }
                    Err(RenderError::Surface(wgpu::SurfaceError::OutOfMemory)) => {
                        log::error!("La GPU se quedó sin memoria durante el renderizado.");
                        event_loop.exit();
                    }
                    Err(RenderError::Surface(e)) => {
                        log::warn!("Advertencia de renderizado: {:?}", e);
                        if self.max_frames.is_some() {
                            window.request_redraw();
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "La escena dejó de ser compatible con los recursos GPU: {error}"
                        );
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(render_context)) =
            (self.window.as_ref(), self.render_context.as_mut())
        else {
            return;
        };

        if render_context.has_drawable_surface() && render_context.has_continuous_camera_motion() {
            let now = Instant::now();
            if render_context.update_camera(now.saturating_duration_since(self.last_camera_update))
            {
                window.request_redraw();
            }
            self.last_camera_update = now;
            let next_deadline = now + Duration::from_millis(16);
            #[cfg(feature = "diagnostics")]
            let next_deadline = {
                if now >= self.diagnostics.next_update_deadline() {
                    window.set_title(&Diagnostics::format_title(self.diagnostics.update(now)));
                }
                next_deadline.min(self.diagnostics.next_update_deadline())
            };
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_deadline));
        } else {
            self.last_camera_update = Instant::now();
            #[cfg(feature = "diagnostics")]
            {
                let now = Instant::now();
                if now >= self.diagnostics.next_update_deadline() {
                    window.set_title(&Diagnostics::format_title(self.diagnostics.update(now)));
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    self.diagnostics.next_update_deadline(),
                ));
            }
            #[cfg(not(feature = "diagnostics"))]
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

/// Construye una composición determinista de primitivas para validar la escena.
fn house_scene() -> Result<Scene, AppError> {
    let building = generate_house(&HouseConfig::default()).map_err(AppError::HouseGeneration)?;
    let mesh = mesh_building(&building)
        .map_err(AppError::HouseMeshing)?
        .mesh;
    let bounds = mesh
        .bounds()
        .ok_or(AppError::HouseMeshing(MeshingError::InvalidTopology))?;
    // El encuadre pertenece a la instancia de demostración.
    let size = bounds.max - bounds.min;
    let scale = 5.0 / size.length();
    let center = (bounds.min + bounds.max) / 2.0;
    let pose =
        transform(-center * scale, 0.0, glam::Vec3::splat(scale)).map_err(AppError::DemoScene)?;
    log::info!(
        "Demostración de vivienda: {} vértices, {} triángulos",
        mesh.vertices.len(),
        mesh.faces.len()
    );
    let mut scene = Scene::new();
    let mesh_id = scene.register_mesh(mesh).map_err(AppError::DemoScene)?;
    scene
        .create_object(mesh_id, pose)
        .map_err(AppError::DemoScene)?;
    Ok(scene)
}

fn demo_scene() -> Result<Scene, SceneError> {
    let mut scene = Scene::new();
    let materials = [0, 1, 2, 3, 4, 5];
    let cube = scene
        .register_mesh(Cube::with_materials(1.0, materials).map_err(SceneError::InvalidMesh)?)?;
    let tower = scene.register_mesh(
        RectangularPrism::with_materials(0.9, 2.8, 0.9, materials)
            .map_err(SceneError::InvalidMesh)?,
    )?;
    let slab = scene.register_mesh(
        RectangularPrism::with_materials(1.8, 0.7, 1.1, materials)
            .map_err(SceneError::InvalidMesh)?,
    )?;

    for (translation, rotation, scale) in [
        (glam::Vec3::new(0.0, 0.7, 0.0), 0.35, glam::Vec3::splat(1.4)),
        (
            glam::Vec3::new(-2.0, 0.5, -0.8),
            0.0,
            glam::Vec3::splat(1.0),
        ),
        (
            glam::Vec3::new(2.0, 0.45, -0.6),
            -0.55,
            glam::Vec3::splat(0.9),
        ),
        (glam::Vec3::new(0.0, 0.35, 2.0), 0.0, glam::Vec3::splat(0.7)),
    ] {
        scene.create_object(cube, transform(translation, rotation, scale)?)?;
    }
    for (translation, rotation, scale) in [
        (glam::Vec3::new(-2.6, 1.4, 1.4), 0.2, glam::Vec3::ONE),
        (
            glam::Vec3::new(2.6, 1.12, 1.5),
            -0.3,
            glam::Vec3::new(0.8, 0.8, 0.8),
        ),
    ] {
        scene.create_object(tower, transform(translation, rotation, scale)?)?;
    }
    for (translation, rotation) in [
        (glam::Vec3::new(0.0, 0.35, -2.1), 0.15),
        (glam::Vec3::new(1.5, 0.35, 2.7), -0.4),
    ] {
        scene.create_object(slab, transform(translation, rotation, glam::Vec3::ONE)?)?;
    }

    let hidden = scene.create_object(
        cube,
        transform(glam::Vec3::new(0.0, 0.5, -4.0), 0.0, glam::Vec3::ONE)?,
    )?;
    scene.set_visibility(hidden, false)?;
    Ok(scene)
}

fn transform(
    translation: glam::Vec3,
    yaw_radians: f32,
    scale: glam::Vec3,
) -> Result<Transform, SceneError> {
    Transform::new(translation, glam::Quat::from_rotation_y(yaw_radians), scale)
        .map_err(SceneError::InvalidTransform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipped_frames_wait_without_consuming_smoke_budget() {
        let mut app = App::new().expect("escena inicial válida");
        app.max_frames = Some(2);

        for (outcome, action, count) in [
            (FrameOutcome::Skipped, FrameAction::Wait, 0),
            (FrameOutcome::Presented, FrameAction::Redraw, 1),
            (FrameOutcome::Skipped, FrameAction::Wait, 1),
            (FrameOutcome::Presented, FrameAction::Exit(2), 2),
        ] {
            assert_eq!(
                app.account_frame(
                    outcome,
                    #[cfg(feature = "diagnostics")]
                    Instant::now(),
                ),
                action
            );
            assert_eq!(app.frame_count, count);
            #[cfg(feature = "diagnostics")]
            assert_eq!(app.diagnostics.update(Instant::now()).total_frames, count);
        }
    }

    #[test]
    fn presented_frames_wait_for_events_without_a_smoke_limit() {
        let mut app = App::new().expect("escena inicial válida");
        app.max_frames = None;
        assert_eq!(
            app.account_frame(
                FrameOutcome::Presented,
                #[cfg(feature = "diagnostics")]
                Instant::now(),
            ),
            FrameAction::Wait
        );
        assert_eq!(app.frame_count, 1);
    }
}
