use std::time::Duration;

use glam::{Mat4, Vec2, Vec3};
use winit::dpi::PhysicalSize;

/// Cámara ortográfica orbital, independiente de eventos y recursos de GPU.
#[derive(Clone, Debug)]
pub struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    ortho_scale: f32,
    aspect_ratio: f32,
    isometric: bool,
}

impl Camera {
    /// Límite inferior de la altura visible de la proyección ortográfica.
    pub const MIN_ORTHO_SCALE: f32 = 0.5;
    /// Límite superior de la altura visible de la proyección ortográfica.
    pub const MAX_ORTHO_SCALE: f32 = 24.0;
    /// Inclinación máxima para mantener un vector vertical estable.
    pub const MAX_PITCH: f32 = 1.52;
    const INITIAL_POSITION: Vec3 = Vec3::new(4.0, 3.0, 5.0);
    const INITIAL_ORTHO_SCALE: f32 = 7.0;
    const NEAR_PLANE: f32 = 0.1;
    const FAR_PLANE: f32 = 100.0;

    /// Crea la cámara equivalente a la demostración fija del Sprint 2.
    pub fn new(size: PhysicalSize<u32>) -> Self {
        let position = Self::INITIAL_POSITION;
        let distance = position.length();
        let yaw = position.x.atan2(position.z);
        let pitch = (position.y / distance).asin();
        Self {
            target: Vec3::ZERO,
            yaw,
            pitch,
            distance,
            ortho_scale: Self::INITIAL_ORTHO_SCALE,
            aspect_ratio: aspect_ratio(size),
            isometric: false,
        }
    }

    /// Crea la vista ortográfica con igual acortamiento de X, Y y Z.
    pub fn isometric(size: PhysicalSize<u32>) -> Self {
        let mut camera = Self::new(size);
        camera.isometric = true;
        camera.yaw = std::f32::consts::FRAC_PI_4;
        camera.pitch = (1.0_f32 / 3.0_f32.sqrt()).asin();
        // La demostración está centrada en una esfera de diámetro 5.
        // El encuadre inicial conserva ese margen incluso en ventanas verticales.
        camera.ortho_scale /= camera.aspect_ratio.min(1.0);
        camera.ortho_scale = camera.ortho_scale.min(Self::MAX_ORTHO_SCALE);
        camera
    }

    /// Restaura el objetivo, ángulos, escala y distancia iniciales.
    pub fn reset(&mut self, size: PhysicalSize<u32>) {
        *self = if self.isometric {
            Self::isometric(size)
        } else {
            Self::new(size)
        };
    }

    /// Posición orbital calculada a partir de los parámetros angulares en radianes.
    pub fn position(&self) -> Vec3 {
        let horizontal = self.distance * self.pitch.cos();
        self.target
            + Vec3::new(
                horizontal * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                horizontal * self.yaw.cos(),
            )
    }

    /// Punto central que permanece observado por la cámara.
    pub fn target(&self) -> Vec3 {
        self.target
    }

    /// Ángulo horizontal en radianes.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Ángulo vertical limitado en radianes.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Escala vertical actual de la proyección ortográfica.
    pub fn ortho_scale(&self) -> f32 {
        self.ortho_scale
    }

    /// Relación ancho/alto actual.
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Actualiza únicamente la relación de aspecto de la proyección.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect_ratio = aspect_ratio(size);
    }

    /// Aplica una rotación orbital en radianes e ignora entradas no finitas.
    pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) -> bool {
        if !yaw_delta.is_finite() || !pitch_delta.is_finite() {
            return false;
        }
        if yaw_delta == 0.0 && pitch_delta == 0.0 {
            return false;
        }
        self.yaw = (self.yaw + yaw_delta).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch_delta).clamp(-Self::MAX_PITCH, Self::MAX_PITCH);
        true
    }

    /// Multiplica la escala ortográfica y la limita a un intervalo seguro.
    pub fn zoom_by_factor(&mut self, factor: f32) -> bool {
        if !factor.is_finite() || factor <= 0.0 {
            return false;
        }
        if factor == 1.0 {
            return false;
        }
        let scale = (self.ortho_scale * factor).clamp(Self::MIN_ORTHO_SCALE, Self::MAX_ORTHO_SCALE);
        if !scale.is_finite() {
            return false;
        }
        self.ortho_scale = scale;
        true
    }

    /// Desplaza el objetivo sobre el plano XZ en unidades de mundo.
    pub fn pan_world(&mut self, offset: Vec3) -> bool {
        if !offset.is_finite() {
            return false;
        }
        let horizontal_offset = Vec3::new(offset.x, 0.0, offset.z);
        if horizontal_offset == Vec3::ZERO {
            return false;
        }
        let target = self.target + horizontal_offset;
        if !target.is_finite() {
            return false;
        }
        self.target = target;
        true
    }

    /// Vector hacia la derecha de la pantalla proyectado en el plano XZ.
    pub fn horizontal_right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    /// Vector hacia delante de la pantalla proyectado en el plano XZ.
    pub fn horizontal_forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    /// Matriz de vista de mano derecha que apunta al objetivo.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    /// Proyección ortográfica con profundidad 0..1 compatible con wgpu.
    pub fn projection_matrix(&self) -> Mat4 {
        let half_height = self.ortho_scale / 2.0;
        let half_width = half_height * self.aspect_ratio;
        Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            Self::NEAR_PLANE,
            Self::FAR_PLANE,
        )
    }

    /// Matriz de vista-proyección enviada al shader.
    pub fn view_projection(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

/// Alias temporal que preserva la API pública del Sprint 2.
pub type FixedCamera = Camera;

/// Botones de ratón que afectan a la cámara, sin depender de winit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMouseButton {
    /// Orbita alrededor del objetivo.
    Left,
    /// Desplaza el objetivo sobre XZ.
    Middle,
}

/// Direcciones de desplazamiento continuo del teclado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMove {
    Forward,
    Backward,
    Left,
    Right,
}

/// Eventos de entrada ya traducidos por la capa de aplicación.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraInput {
    CursorMoved(Vec2),
    MouseButton {
        button: CameraMouseButton,
        pressed: bool,
    },
    ScrollLines(f32),
    ScrollPixels(f32),
    Move {
        direction: CameraMove,
        pressed: bool,
    },
    Reset,
    PointerLeft,
    FocusLost,
}

/// Interpreta entradas y modifica el estado matemático de una [`Camera`].
#[derive(Clone, Debug, Default)]
pub struct CameraController {
    rotating: bool,
    panning: bool,
    moving_forward: bool,
    moving_backward: bool,
    moving_left: bool,
    moving_right: bool,
    last_cursor: Option<Vec2>,
}

impl CameraController {
    const ORBIT_SENSITIVITY: f32 = 0.008;
    const ZOOM_FACTOR_PER_STEP: f32 = 0.88;
    const PIXELS_PER_SCROLL_STEP: f32 = 48.0;
    const KEYBOARD_PAN_SPEED: f32 = 1.4;

    /// Procesa una entrada discreta y devuelve si modificó la cámara.
    pub fn process_input(
        &mut self,
        camera: &mut Camera,
        input: CameraInput,
        viewport: PhysicalSize<u32>,
    ) -> bool {
        match input {
            CameraInput::CursorMoved(position) => self.cursor_moved(camera, position, viewport),
            CameraInput::MouseButton { button, pressed } => {
                self.set_mouse_button(button, pressed);
                false
            }
            CameraInput::ScrollLines(delta) => self.zoom(camera, delta),
            CameraInput::ScrollPixels(delta) => {
                self.zoom(camera, delta / Self::PIXELS_PER_SCROLL_STEP)
            }
            CameraInput::Move { direction, pressed } => {
                self.set_move(direction, pressed);
                false
            }
            CameraInput::Reset => {
                camera.reset(viewport);
                true
            }
            CameraInput::PointerLeft => {
                self.rotating = false;
                self.panning = false;
                self.last_cursor = None;
                false
            }
            CameraInput::FocusLost => {
                self.clear_input_state();
                false
            }
        }
    }

    /// Actualiza el pan continuo de teclado con un delta de tiempo limitado.
    pub fn update(
        &mut self,
        camera: &mut Camera,
        delta: Duration,
        _viewport: PhysicalSize<u32>,
    ) -> bool {
        if !self.has_continuous_motion() {
            return false;
        }
        let seconds = delta.as_secs_f32().min(0.1);
        if !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }

        let mut direction = Vec3::ZERO;
        if self.moving_forward {
            direction += camera.horizontal_forward();
        }
        if self.moving_backward {
            direction -= camera.horizontal_forward();
        }
        if self.moving_right {
            direction += camera.horizontal_right();
        }
        if self.moving_left {
            direction -= camera.horizontal_right();
        }
        if direction.length_squared() == 0.0 {
            return false;
        }
        let distance = Self::KEYBOARD_PAN_SPEED * camera.ortho_scale() * seconds;
        camera.pan_world(direction.normalize() * distance)
    }

    /// Indica si el teclado requiere actualizaciones temporizadas.
    pub fn has_continuous_motion(&self) -> bool {
        self.moving_forward || self.moving_backward || self.moving_left || self.moving_right
    }

    fn cursor_moved(
        &mut self,
        camera: &mut Camera,
        position: Vec2,
        viewport: PhysicalSize<u32>,
    ) -> bool {
        if !position.is_finite() {
            self.last_cursor = None;
            return false;
        }
        let previous = self.last_cursor.replace(position);
        let Some(previous) = previous else {
            return false;
        };
        let delta = position - previous;
        if !delta.is_finite() {
            return false;
        }
        if self.rotating {
            return camera.orbit(
                -delta.x * Self::ORBIT_SENSITIVITY,
                -delta.y * Self::ORBIT_SENSITIVITY,
            );
        }
        if self.panning {
            let height = viewport.height.max(1) as f32;
            let world_per_pixel = camera.ortho_scale() / height;
            let offset = -camera.horizontal_right() * (delta.x * world_per_pixel)
                + camera.horizontal_forward() * (delta.y * world_per_pixel);
            return camera.pan_world(offset);
        }
        false
    }

    fn zoom(&mut self, camera: &mut Camera, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let limited_delta = delta.clamp(-10.0, 10.0);
        camera.zoom_by_factor(Self::ZOOM_FACTOR_PER_STEP.powf(limited_delta))
    }

    fn set_mouse_button(&mut self, button: CameraMouseButton, pressed: bool) {
        match button {
            CameraMouseButton::Left => self.rotating = pressed,
            CameraMouseButton::Middle => self.panning = pressed,
        }
        self.last_cursor = None;
    }

    fn set_move(&mut self, direction: CameraMove, pressed: bool) {
        match direction {
            CameraMove::Forward => self.moving_forward = pressed,
            CameraMove::Backward => self.moving_backward = pressed,
            CameraMove::Left => self.moving_left = pressed,
            CameraMove::Right => self.moving_right = pressed,
        }
    }

    fn clear_input_state(&mut self) {
        self.rotating = false;
        self.panning = false;
        self.moving_forward = false;
        self.moving_backward = false;
        self.moving_left = false;
        self.moving_right = false;
        self.last_cursor = None;
    }
}

fn aspect_ratio(size: PhysicalSize<u32>) -> f32 {
    let width = size.width.max(1) as f32;
    let height = size.height.max(1) as f32;
    width / height
}
