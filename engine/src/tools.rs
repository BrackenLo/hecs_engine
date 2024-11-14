//====================================================================

use std::{
    collections::HashSet,
    hash::{BuildHasherDefault, Hash},
};

use rustc_hash::FxHasher;
use web_time::{Duration, Instant};

//====================================================================

type Hasher = BuildHasherDefault<FxHasher>;

//====================================================================

#[derive(Debug)]
pub struct Time {
    elapsed: Instant,

    last_frame: Instant,
    delta: Duration,
    delta_seconds: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            elapsed: Instant::now(),
            last_frame: Instant::now(),
            delta: Duration::ZERO,
            delta_seconds: 0.,
        }
    }
}

#[allow(dead_code)]
impl Time {
    #[inline]
    pub fn elapsed(&self) -> &Instant {
        &self.elapsed
    }

    #[inline]
    pub fn delta(&self) -> &Duration {
        &self.delta
    }

    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }
}

pub fn tick_time(time: &mut Time) {
    time.delta = time.last_frame.elapsed();
    time.delta_seconds = time.delta.as_secs_f32();

    time.last_frame = Instant::now();
}

//====================================================================

pub use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Debug)]
pub struct Input<T> {
    pressed: HashSet<T, Hasher>,
    just_pressed: HashSet<T, Hasher>,
    released: HashSet<T, Hasher>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            pressed: HashSet::default(),
            just_pressed: HashSet::default(),
            released: HashSet::default(),
        }
    }
}

#[allow(dead_code)]
impl<T> Input<T>
where
    T: Eq + Hash,
{
    #[inline]
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    #[inline]
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    #[inline]
    pub fn released(&self, input: T) -> bool {
        self.released.contains(&input)
    }
}

pub(crate) fn process_inputs<T>(input: &mut Input<T>, val: T, pressed: bool)
where
    T: Eq + Hash + Copy,
{
    match pressed {
        true => {
            input.pressed.insert(val);
            input.just_pressed.insert(val);
        }
        false => {
            input.pressed.remove(&val);
            input.released.insert(val);
        }
    }
}

pub(crate) fn reset_input<T>(input: &mut Input<T>) {
    input.just_pressed.clear();
    input.released.clear();
}

//--------------------------------------------------

#[derive(Debug, Default)]
pub struct MouseInput {
    position: glam::Vec2,
    screen_position: glam::Vec2,
    motion_delta: glam::Vec2,
    scroll: glam::Vec2,
}

impl MouseInput {
    #[inline]
    pub fn position(&self) -> glam::Vec2 {
        self.position
    }

    #[inline]
    pub fn screen_position(&self) -> glam::Vec2 {
        self.screen_position
    }

    #[inline]
    pub fn motion_delta(&self) -> glam::Vec2 {
        self.motion_delta
    }

    #[inline]
    pub fn scroll(&self) -> glam::Vec2 {
        self.scroll
    }
}

#[inline]
pub(crate) fn process_mouse_position(input: &mut MouseInput, position: (f64, f64)) {
    input.position = glam::vec2(position.0 as f32, position.1 as f32);
}

#[inline]
pub(crate) fn process_mouse_motion(input: &mut MouseInput, delta: (f64, f64)) {
    input.motion_delta += glam::vec2(delta.0 as f32, delta.1 as f32);
}

#[inline]
pub(crate) fn process_mouse_scroll(input: &mut MouseInput, delta: (f32, f32)) {
    input.scroll += glam::vec2(delta.0, delta.1);
}

pub(crate) fn reset_mouse_input(input: &mut MouseInput) {
    input.motion_delta = glam::Vec2::ZERO;
    input.scroll = glam::Vec2::ZERO;
}

//====================================================================
