use speedy2d::window::UserEventSender;

use crate::EditorEvent;

#[allow(dead_code)]
pub enum EasingFunction {
    Linear,
    SmoothStep,
    SmootherStep,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInBack,
    EaseOutBack,
}

fn get_easing_fn(f: EasingFunction) -> Box<dyn Fn(f32) -> f32 + 'static> {
    let function = match f {
        EasingFunction::Linear => |t: f32| t,
        EasingFunction::SmoothStep => |t: f32| (3. - 2. * t) * t.powi(2),
        EasingFunction::SmootherStep => |t: f32| (6. * t * t - 15. * t + 10.) * t.powi(3),
        EasingFunction::EaseIn => |t: f32| t.powi(2),
        EasingFunction::EaseOut => |t: f32| 1. - (1. - t).powi(2),
        EasingFunction::EaseInOut => |t: f32| if t < 0.5 { 2. * t * t } else { 1. - (-2. * t + 2.).powi(2) / 2. },
        EasingFunction::EaseInBack => |t: f32| 2.70158 * t.powi(3) - 1.70158 * t.powi(2),
        EasingFunction::EaseOutBack => |t: f32| 1. + 1.70158 * (t - 1.).powi(3) + 2.70158 * (t - 1.).powi(2),
    };
    Box::new(function)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Animation {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    #[derivative(Debug = "ignore")]
    pub easing: Box<dyn Fn(f32) -> f32>,
    pub value: f32,
    pub has_started: bool,
    pub is_paused: bool,
    pub is_ended: bool,
    is_reversed: bool,
    speed: f32,
    pub last_t: f32,
    #[derivative(Debug = "ignore")]
    animation_event_sender: Option<UserEventSender<EditorEvent>>,
}

impl Animation {
    pub fn new(from: f32, to: f32, duration: f32, easing: EasingFunction, aes: Option<UserEventSender<EditorEvent>> ) -> Self {
        Self {
            from,
            to,
            duration,
            easing: get_easing_fn(easing),
            value: from,
            has_started: false,
            is_paused: false,
            is_ended: from == to,
            is_reversed: false,
            speed: (to - from).abs() as f32 / duration,
            last_t: 0.,
            animation_event_sender: aes,
        }
    }

    #[inline]
    pub fn start(&mut self) {
        self.is_ended = self.from == self.to;
        self.has_started = true;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.last_t = 0.;
        self.is_paused = false;
        self.has_started = false;
        self.is_ended = false;
    }

    #[inline]
    pub fn toggle(&mut self) {
        if self.is_paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    #[inline]
    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    #[inline]
    pub fn resume(&mut self) {
        self.is_paused = false
    }

    pub fn update(&mut self, delta_time: f32) {
        if !self.has_started || self.is_paused || self.is_ended {
            return;
        }
        // t in  range [0, 1]
        let t = (self.last_t + delta_time * self.speed / (self.to - self.from).abs()).clamp(0., 1.);
        if t >= 1. || t <= 0. {
            self.is_ended = true;
            self.on_finish();
            return;
        }
        self.last_t = t;
        self.value = self.from + (self.to - self.from) * (self.easing)(t);
        if let Some(aes) = &self.animation_event_sender {
            aes.send_event(EditorEvent::Redraw).unwrap();
        }
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        return self.has_started && !(self.is_ended || self.is_paused);
    }

    pub fn on_finish(&self) {}
}
