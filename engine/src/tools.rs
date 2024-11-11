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

pub use winit::keyboard::KeyCode;

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

pub fn process_inputs<T>(input: &mut Input<T>, val: T, pressed: bool)
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

pub fn reset_input<T>(input: &mut Input<T>) {
    input.just_pressed.clear();
    input.released.clear();
}

//====================================================================
