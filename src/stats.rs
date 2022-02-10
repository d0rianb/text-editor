use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub update_duration: Duration,
    pub draw_duration: Duration
}