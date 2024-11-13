//====================================================================

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use common::GlobalTransform;
use renderer::{
    camera::{CameraWgpu, PerspectiveCamera},
    shared::{
        TextureRectVertex, Vertex, TEXTURE_RECT_INDEX_COUNT, TEXTURE_RECT_INDICES,
        TEXTURE_RECT_VERTICES,
    },
    texture::LoadedTexture,
    tools, Renderer,
};

//====================================================================

pub struct Sprite {
    pub texture: Arc<LoadedTexture>,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

//====================================================================

pub struct TextureRenderer {
    pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    instances: HashMap<u32, TextureInstanceBuffer>,
}

impl Renderer for TextureRenderer {
    fn new(
        core: &renderer::RendererCore,
        shared: &mut renderer::shared::SharedRenderResources,
        _world: &mut hecs::World,
    ) -> Self {
        let pipeline = tools::create_pipeline(
            core.device(),
            core.config(),
            "Texture Pipeline",
            &[
                shared.camera_bind_group_layout(),
                shared.texture_bind_group_layout(),
            ],
            &[TextureRectVertex::desc(), InstanceTexture::desc()],
            include_str!("shaders/texture.wgsl"),
            tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                ..Default::default()
            }
            .with_depth_stencil(),
        );

        let vertex_buffer = tools::buffer(
            core.device(),
            tools::BufferType::Vertex,
            "Texture",
            &TEXTURE_RECT_VERTICES,
        );

        let index_buffer = tools::buffer(
            core.device(),
            tools::BufferType::Index,
            "Texture",
            &TEXTURE_RECT_INDICES,
        );
        let index_count = TEXTURE_RECT_INDEX_COUNT;

        let instances = HashMap::default();

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            instances,
        }
    }

    fn prep(
        &mut self,
        core: &renderer::RendererCore,
        _shared: &mut renderer::shared::SharedRenderResources,
        world: &mut hecs::World,
    ) {
        let mut previous = self.instances.keys().map(|id| *id).collect::<HashSet<_>>();
        let mut textures_to_add = HashMap::new();

        let instances = world
            .query_mut::<(&GlobalTransform, &Sprite)>()
            .into_iter()
            .fold(HashMap::new(), |mut acc, (_, (transform, sprite))| {
                let instance = InstanceTexture {
                    size: sprite.size,
                    pad: [0.; 2],
                    transform: transform.to_matrix(),
                    color: sprite.color.into(),
                };

                acc.entry(sprite.texture.id())
                    .or_insert_with(|| {
                        textures_to_add.insert(sprite.texture.id(), sprite.texture.clone());
                        Vec::new()
                    })
                    .push(instance);

                acc
            });

        instances.into_iter().for_each(|(id, raw)| {
            previous.remove(&id);

            self.instances
                .entry(id)
                .and_modify(|instance| {
                    instance.update(core.device(), core.queue(), raw.as_slice());
                })
                .or_insert_with(|| {
                    TextureInstanceBuffer::new(
                        core.device(),
                        textures_to_add.remove(&id).unwrap(),
                        raw.as_slice(),
                    )
                });
        });

        previous.into_iter().for_each(|to_remove| {
            log::trace!("Removing texture instance {}", to_remove);
            self.instances.remove(&to_remove);
        });
    }

    fn render(
        &mut self,
        pass: &mut wgpu::RenderPass,
        _shared: &mut renderer::shared::SharedRenderResources,
        world: &mut hecs::World,
    ) {
        let camera = match world
            .query_mut::<(&PerspectiveCamera, &CameraWgpu)>()
            .into_iter()
            .next()
        {
            Some((_, (_, camera))) => camera,
            None => {
                log::warn!("No perspective camera available for texture renderer");
                return;
            }
        };

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera.bind_group(), &[]);

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        self.instances.iter().for_each(|(_, instance)| {
            pass.set_bind_group(1, instance.texture.bind_group(), &[]);
            pass.set_vertex_buffer(1, instance.buffer.buffer().slice(..));
            pass.draw_indexed(0..self.index_count, 0, 0..instance.buffer.count());
        });
    }
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct InstanceTexture {
    pub size: glam::Vec2,
    pub pad: [f32; 2],
    pub transform: glam::Mat4,
    pub color: glam::Vec4,
}

impl Vertex for InstanceTexture {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            2 => Float32x4, // Transform
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4, // Color
            7 => Float32x4, // Size
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

struct TextureInstanceBuffer {
    texture: Arc<LoadedTexture>,
    buffer: tools::InstanceBuffer<InstanceTexture>,
}

impl TextureInstanceBuffer {
    #[inline]
    pub fn new(
        device: &wgpu::Device,
        texture: Arc<LoadedTexture>,
        data: &[InstanceTexture],
    ) -> Self {
        Self {
            texture,
            buffer: tools::InstanceBuffer::new(device, data),
        }
    }

    #[inline]
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[InstanceTexture]) {
        self.buffer.update(device, queue, data);
    }
}

//====================================================================
