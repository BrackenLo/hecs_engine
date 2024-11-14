//====================================================================

use std::collections::{HashMap, HashSet};

use common::{GlobalTransform, Transform};
use hecs::{Entity, World};

//====================================================================

pub(crate) fn process_global_transform(state: &mut crate::State) {
    state
        .world
        .query_mut::<(&Transform, &mut GlobalTransform)>()
        .into_iter()
        .for_each(|(_, (transform, global))| global.0 = transform.to_affine());
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

    let hierarchy = state.world.query_mut::<&LocalTransform>().into_iter().fold(
        Hierarchy::default(),
        |mut acc, (entity, local)| {
            acc.entries.insert(entity);

            acc.links
                .entry(local.parent)
                .or_insert(Vec::new())
                .push(entity);

            acc
        },
    );

    let roots = hierarchy
        .links
        .keys()
        .filter(|val| !hierarchy.entries.contains(val))
        .collect::<Vec<_>>();

    roots.into_iter().for_each(|root| {
        let root_transform = match state.world.get::<&GlobalTransform>(*root) {
            Ok(transform) => transform.0,
            Err(_) => {
                log::warn!(
                    "Entity '{:?}' is a root transform for '{:?}' but is missing a GlobalTransform component.",
                    root, 
                    hierarchy.links.get(root)
                );
                glam::Affine3A::IDENTITY
            }
        };

        hierarchy
            .links
            .get(root)
            .unwrap()
            .into_iter()
            .for_each(|child| {
                cascade_transform(&mut state.world, &hierarchy.links, *child, root_transform);
            });
    });
}

fn cascade_transform(
    world: &mut World,
    links: &HashMap<Entity, Vec<Entity>>,
    current: Entity,
    mut transform: glam::Affine3A,
) {
    if let Ok(local) = world.get::<&LocalTransform>(current) {
        transform *= local.transform.to_affine();
    }

    if let Ok(mut entity_transform) = world.get::<&mut GlobalTransform>(current) {
        entity_transform.0 = transform;
    }

    if let Some(child_links) = links.get(&current) {
        child_links
            .into_iter()
            .for_each(|child| cascade_transform(world, links, *child, transform))
    }
}

//====================================================================
