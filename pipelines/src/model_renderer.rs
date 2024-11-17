//====================================================================

use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicU32, Arc},
};

use common::GlobalTransform;
use renderer::{
    camera::{CameraWgpu, PerspectiveCamera},
    shared::{ModelVertex, Vertex},
    texture::{LoadedTexture, TextureId},
    tools::{self, InstanceBuffer},
    Renderer, WgpuWrapper,
};

//====================================================================

pub type MeshId = u32;

static CURRENT_MESH_ID: AtomicU32 = AtomicU32::new(0);

pub struct Mesh {
    id: MeshId,
    vertex_buffer: WgpuWrapper<wgpu::Buffer>,
    index_buffer: WgpuWrapper<wgpu::Buffer>,
    index_count: u32,
}

impl Mesh {
    pub fn load_mesh(device: &wgpu::Device, vertices: &[ModelVertex], indices: &[u32]) -> Self {
        let id = CURRENT_MESH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let vertex_buffer = tools::buffer(device, tools::BufferType::Vertex, "Mesh", vertices);
        let index_buffer = tools::buffer(device, tools::BufferType::Index, "Mesh", indices);
        let index_count = indices.len() as u32;

        Self {
            id,
            vertex_buffer: WgpuWrapper::new(vertex_buffer),
            index_buffer: WgpuWrapper::new(index_buffer),
            index_count,
        }
    }
}

pub struct Model {
    pub meshes: Vec<(Arc<Mesh>, Arc<LoadedTexture>)>,
    pub color: [f32; 4],
    pub scale: glam::Vec3,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct ModelInstance {
    pub transform: glam::Mat4,
    pub color: glam::Vec4,
    pub normal: glam::Mat3,
    pub scale: glam::Vec3,
}

impl Vertex for ModelInstance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
            3 => Float32x4, // Transform
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4, // Color
            8 => Float32x3, // Normal
            9 => Float32x3,
            10 => Float32x3,
            11 => Float32x3, // Scale
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

pub struct ModelRenderer {
    pipeline: wgpu::RenderPipeline,

    texture_storage: HashMap<u32, Arc<LoadedTexture>>,
    mesh_storage: HashMap<u32, Arc<Mesh>>,
    instances: HashMap<MeshId, HashMap<TextureId, tools::InstanceBuffer<ModelInstance>>>,
}

impl Renderer for ModelRenderer {
    fn new(
        core: &renderer::RendererCore,
        shared: &mut renderer::shared::SharedRenderResources,
        _world: &mut hecs::World,
    ) -> Self {
        let pipeline = tools::create_pipeline(
            core.device(),
            core.config(),
            "Model Pipeline",
            &[
                shared.camera_bind_group_layout(),
                shared.texture_bind_group_layout(),
            ],
            &[ModelVertex::desc(), ModelInstance::desc()],
            include_str!("shaders/model.wgsl"),
            tools::RenderPipelineDescriptor::default()
                .with_depth_stencil()
                .with_backface_culling(),
        );

        Self {
            pipeline,
            texture_storage: HashMap::default(),
            mesh_storage: HashMap::default(),
            instances: HashMap::default(),
        }
    }

    fn prep(
        &mut self,
        core: &renderer::RendererCore,
        _shared: &mut renderer::shared::SharedRenderResources,
        world: &mut hecs::World,
    ) {
        let mut previous = self
            .instances
            .iter()
            .flat_map(|(mesh_id, textures)| {
                textures.keys().map(|texture_id| (*mesh_id, *texture_id))
            })
            .collect::<HashSet<_>>();

        let mut meshes_used = HashSet::new();
        let mut textures_used = HashSet::new();

        let instances = world
            .query_mut::<(&GlobalTransform, &Model)>()
            .into_iter()
            .fold(HashMap::new(), |mut acc, (_, (transform, model))| {
                model.meshes.iter().for_each(|(mesh, texture)| {
                    let mesh_entry = acc.entry(mesh.id).or_insert_with(|| {
                        if !self.mesh_storage.contains_key(&mesh.id) {
                            self.mesh_storage.insert(mesh.id, mesh.clone());
                        }

                        meshes_used.insert(mesh.id);

                        HashMap::new()
                    });

                    let rotation = transform.to_scale_rotation_translation().1;
                    let normal_matrix = glam::Mat3::from_quat(rotation);

                    mesh_entry
                        .entry(texture.id())
                        .or_insert_with(|| {
                            if !self.texture_storage.contains_key(&texture.id()) {
                                self.texture_storage.insert(texture.id(), texture.clone());
                            }

                            textures_used.insert(texture.id());

                            Vec::new()
                        })
                        .push(ModelInstance {
                            transform: transform.to_matrix(),
                            color: model.color.into(),
                            normal: normal_matrix,
                            scale: model.scale,
                        });
                });

                acc
            });

        instances.into_iter().for_each(|(mesh_id, texture_data)| {
            texture_data.into_iter().for_each(|(texture_id, raw)| {
                previous.remove(&(mesh_id, texture_id));

                self.instances
                    .entry(mesh_id)
                    .or_insert(HashMap::default())
                    .entry(texture_id)
                    .and_modify(|instance| instance.update(core.device(), core.queue(), &raw))
                    .or_insert_with(|| InstanceBuffer::new(core.device(), &raw));
            });
        });

        previous.into_iter().for_each(|(mesh_id, texture_id)| {
            log::trace!("Removing model instance {} - {}", mesh_id, texture_id);
            self.instances
                .get_mut(&mesh_id)
                .unwrap()
                .remove(&texture_id);
        });

        self.texture_storage
            .retain(|texture_id, _| textures_used.contains(texture_id));

        self.mesh_storage
            .retain(|mesh_id, _| meshes_used.contains(mesh_id));
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

        self.instances.iter().for_each(|(mesh_id, instance)| {
            let mesh = self.mesh_storage.get(mesh_id).unwrap();

            pass.set_vertex_buffer(0, mesh.vertex_buffer.inner().slice(..));
            pass.set_index_buffer(
                mesh.index_buffer.inner().slice(..),
                wgpu::IndexFormat::Uint32,
            );

            instance.iter().for_each(|(texture_id, instance)| {
                let texture = self.texture_storage.get(texture_id).unwrap();

                pass.set_bind_group(1, texture.bind_group(), &[]);
                pass.set_vertex_buffer(1, instance.buffer().slice(..));
                pass.draw_indexed(0..mesh.index_count, 0, 0..instance.count());
            });
        });
    }
}

//====================================================================
