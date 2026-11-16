//! Métricas opcionales de desarrollo, activadas con la característica `diagnostics`.

use std::time::{Duration, Instant};

const TITLE_INTERVAL: Duration = Duration::from_secs(1);
const IDLE_INTERVAL: Duration = Duration::from_secs(1);

/// Estado reciente de presentaciones de fotogramas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityState {
    Active,
    Idle,
}

/// Instantánea de métricas de CPU; no representa tiempo de GPU.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub fps: u32,
    pub frame_time: Option<Duration>,
    pub total_frames: u64,
    pub activity: ActivityState,
}

/// Observa presentaciones exitosas sin controlar el motor ni solicitar renderizado.
#[derive(Debug)]
pub struct Diagnostics {
    total_frames: u64,
    frames_in_interval: u32,
    interval_start: Instant,
    next_title_update: Instant,
    last_presented: Option<Instant>,
    last_frame_time: Option<Duration>,
    fps: u32,
}

impl Diagnostics {
    /// Crea el observador a partir de un instante inyectable para pruebas deterministas.
    pub fn new(now: Instant) -> Self {
        Self {
            total_frames: 0,
            frames_in_interval: 0,
            interval_start: now,
            next_title_update: now,
            last_presented: None,
            last_frame_time: None,
            fps: 0,
        }
    }

    /// Registra solo un fotograma cuya presentación terminó correctamente.
    pub fn record_presented(&mut self, now: Instant, cpu_frame_time: Duration) {
        self.last_frame_time = Some(cpu_frame_time);
        self.total_frames = self.total_frames.saturating_add(1);
        self.frames_in_interval = self.frames_in_interval.saturating_add(1);
        self.last_presented = Some(now);
    }

    /// Próximo vencimiento de título; nunca implica solicitar un redibujado.
    pub fn next_update_deadline(&self) -> Instant {
        self.next_title_update
    }

    /// Devuelve métricas y avanza la ventana temporal cuando se actualiza el título.
    pub fn update(&mut self, now: Instant) -> Metrics {
        let elapsed = now.saturating_duration_since(self.interval_start);
        let active = self
            .last_presented
            .is_some_and(|presented| now.saturating_duration_since(presented) < IDLE_INTERVAL);
        if elapsed >= TITLE_INTERVAL {
            self.fps = if active {
                (self.frames_in_interval as f64 / elapsed.as_secs_f64()).round() as u32
            } else {
                0
            };
            self.frames_in_interval = 0;
            self.interval_start = now;
        }
        self.next_title_update = now + TITLE_INTERVAL;
        Metrics {
            fps: self.fps,
            frame_time: active.then_some(self.last_frame_time).flatten(),
            total_frames: self.total_frames,
            activity: if active {
                ActivityState::Active
            } else {
                ActivityState::Idle
            },
        }
    }

    /// Formatea el título de desarrollo a partir de una instantánea válida.
    pub fn format_title(metrics: Metrics) -> String {
        let frame = metrics
            .frame_time
            .map(|duration| format!("{:.2} ms", duration.as_secs_f64() * 1_000.0))
            .unwrap_or_else(|| "--".to_owned());
        let activity = match metrics.activity {
            ActivityState::Active => "ACTIVE",
            ActivityState::Idle => "IDLE",
        };
        format!(
            "Procedural City Engine | FPS: {} | Frame: {} | Total: {} | {}",
            metrics.fps, frame, metrics.total_frames, activity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_presented_frames_and_becomes_idle() {
        let start = Instant::now();
        let mut diagnostics = Diagnostics::new(start);
        assert_eq!(diagnostics.update(start).activity, ActivityState::Idle);
        diagnostics.record_presented(start + Duration::from_millis(10), Duration::from_millis(2));
        diagnostics.record_presented(start + Duration::from_millis(20), Duration::from_millis(3));
        let metrics = diagnostics.update(start + Duration::from_secs(1));
        assert_eq!(metrics.total_frames, 2);
        assert_eq!(metrics.fps, 2);
        assert_eq!(metrics.frame_time, Some(Duration::from_millis(3)));
        let idle = diagnostics.update(start + Duration::from_secs(3));
        assert_eq!(idle.fps, 0);
        assert_eq!(idle.frame_time, None);
        assert_eq!(idle.activity, ActivityState::Idle);
    }

    #[test]
    fn title_formats_unknown_and_known_frame_times() {
        let initial = Metrics {
            fps: 0,
            frame_time: None,
            total_frames: 0,
            activity: ActivityState::Idle,
        };
        assert!(Diagnostics::format_title(initial).contains("Frame: --"));
        let active = Metrics {
            fps: 60,
            frame_time: Some(Duration::from_micros(1_800)),
            total_frames: 450,
            activity: ActivityState::Active,
        };
        assert_eq!(
            Diagnostics::format_title(active),
            "Procedural City Engine | FPS: 60 | Frame: 1.80 ms | Total: 450 | ACTIVE"
        );
    }

    #[test]
    fn no_presentation_does_not_increment_counters() {
        let start = Instant::now();
        let mut diagnostics = Diagnostics::new(start);
        let metrics = diagnostics.update(start + Duration::from_secs(1));

        assert_eq!(metrics.fps, 0);
        assert_eq!(metrics.total_frames, 0);
        assert_eq!(metrics.frame_time, None);
        assert_eq!(metrics.activity, ActivityState::Idle);
    }

    #[test]
    fn accepts_zero_duration_and_schedules_one_title_update_per_interval() {
        let start = Instant::now();
        let mut diagnostics = Diagnostics::new(start);
        assert_eq!(diagnostics.next_update_deadline(), start);

        diagnostics.record_presented(start, Duration::ZERO);
        let metrics = diagnostics.update(start);

        assert_eq!(metrics.frame_time, Some(Duration::ZERO));
        assert_eq!(diagnostics.next_update_deadline(), start + TITLE_INTERVAL);
    }
}
