//====================================================================

use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hash},
    ops::Deref,
};

use common::Transform;
use hecs::{Entity, World};
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

#[derive(Debug)]
pub struct LocalTransform {
    pub parent: Entity,
    pub transform: Transform,
}

pub(crate) fn process_transform_hierarchy(state: &mut crate::State) {
    #[derive(Default)]
    struct Hierarchy {
        entries: HashSet<Entity>,
        links: HashMap<Entity, Vec<Entity>>,
    }

    let hierarchy = state
        .world
        .query_mut::<(&Transform, &LocalTransform)>()
        .into_iter()
        .fold(Hierarchy::default(), |mut acc, (entity, (_, local))| {
            acc.entries.insert(entity);

            acc.links
                .entry(local.parent)
                .or_insert(Vec::new())
                .push(entity);

            acc
        });

    let roots = hierarchy
        .links
        .keys()
        .filter(|val| !hierarchy.entries.contains(val))
        .collect::<Vec<_>>();

    roots.into_iter().for_each(|root| {
        let root_transform = state
            .world
            .get::<&Transform>(*root)
            .unwrap()
            .deref()
            .clone();

        hierarchy
            .links
            .get(root)
            .unwrap()
            .into_iter()
            .for_each(|child| {
                cascade_transform(
                    &mut state.world,
                    &hierarchy.links,
                    *child,
                    root_transform.clone(),
                )
            });
    });
}

fn cascade_transform(
    world: &mut World,
    links: &HashMap<Entity, Vec<Entity>>,
    current: Entity,
    mut transform: Transform,
) {
    if let Ok(local) = world.get::<&LocalTransform>(current) {
        transform += &local.transform;
    }

    if let Ok(mut entity_transform) = world.get::<&mut Transform>(current) {
        *entity_transform = transform.clone();
    }

    if let Some(child_links) = links.get(&current) {
        child_links
            .into_iter()
            .for_each(|child| cascade_transform(world, links, *child, transform.clone()))
    }
}

//====================================================================
