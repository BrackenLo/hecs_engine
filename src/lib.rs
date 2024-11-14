//====================================================================

pub use common;
pub use engine;
pub use pipelines;
pub use renderer;

pub mod prelude {
    pub use common::{GlobalTransform, Size, Transform};
    pub use engine::{
        tools::{Input, Time},
        App, Runner, State,
    };
    pub use pipelines::texture_renderer::Sprite;
    pub use renderer::{camera::PerspectiveCamera, texture::LoadedTexture};
}

//====================================================================
