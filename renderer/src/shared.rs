//====================================================================

use wgpu::util::DeviceExt;

use crate::{
    camera::{CameraUniform, CameraWgpu},
    text_shared::TextResources,
    WgpuWrapper,
};

use super::{texture::Texture, tools};

//====================================================================

pub trait Vertex: bytemuck::Pod {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

//====================================================================

pub struct SharedRenderResources {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group_layout: wgpu::BindGroupLayout,

    text_resources: TextResources,
}

impl SharedRenderResources {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shared Texture 3d Bind Group Layout"),
                entries: &[tools::bgl_texture_entry(0), tools::bgl_sampler_entry(1)],
            });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let text_resources = TextResources::new(device);

        Self {
            texture_bind_group_layout,
            camera_bind_group_layout,
            text_resources,
        }
    }
}

impl SharedRenderResources {
    #[inline]
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    #[inline]
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    #[inline]
    pub fn text_resources(&self) -> &TextResources {
        &self.text_resources
    }

    #[inline]
    pub fn text_resources_mut(&mut self) -> &mut TextResources {
        &mut self.text_resources
    }
}

impl SharedRenderResources {
    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture: &Texture,
        label: Option<&str>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }

    pub fn create_camera<C: CameraUniform>(&self, device: &wgpu::Device, camera: &C) -> CameraWgpu {
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[camera.get_camera_uniform(&glam::Affine3A::IDENTITY)]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &self.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(camera_buffer.as_entire_buffer_binding()),
            }],
        });

        CameraWgpu {
            camera_buffer: WgpuWrapper::new(camera_buffer),
            camera_bind_group: WgpuWrapper::new(camera_bind_group),
        }
    }
}

//====================================================================

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureRectVertex {
    pos: glam::Vec2,
    uv: glam::Vec2,
}

impl Vertex for TextureRectVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
                0 => Float32x2, 1 => Float32x2
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextureRectVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

pub const TEXTURE_RECT_VERTICES: [TextureRectVertex; 4] = [
    TextureRectVertex {
        pos: glam::vec2(-0.5, 0.5),
        uv: glam::vec2(0., 0.),
    },
    TextureRectVertex {
        pos: glam::vec2(-0.5, -0.5),
        uv: glam::vec2(0., 1.),
    },
    TextureRectVertex {
        pos: glam::vec2(0.5, 0.5),
        uv: glam::vec2(1., 0.),
    },
    TextureRectVertex {
        pos: glam::vec2(0.5, -0.5),
        uv: glam::vec2(1., 1.),
    },
];

pub const TEXTURE_RECT_INDICES: [u16; 6] = [0, 1, 3, 0, 3, 2];
pub const TEXTURE_RECT_INDEX_COUNT: u32 = TEXTURE_RECT_INDICES.len() as u32;

//====================================================================

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ModelVertex {
    pos: glam::Vec3,
    uv: glam::Vec2,
    normal: glam::Vec3,
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32x3
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

pub const CUBE_VERTICES: [ModelVertex; 24] = [
    // Back (-z)
    // Top Left - 0
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, -0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::NEG_Z,
    },
    // Top Right - 1
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, -0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::NEG_Z,
    },
    // Bottom Left - 2
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, -0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::NEG_Z,
    },
    // Bottom Right - 3
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, -0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::NEG_Z,
    },
    //
    // Right (+x)
    // Top Left - 4
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, -0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::X,
    },
    // Top Right - 5
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, 0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::X,
    },
    // Bottom Left - 6
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, -0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::X,
    },
    // Bottom Right - 7
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, 0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::X,
    },
    //
    // Front (+z)
    // Top Left - 8
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, 0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::Z,
    },
    // Top Right - 9
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, 0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::Z,
    },
    // Bottom Left - 10
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, 0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::Z,
    },
    // Bottom Right - 11
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, 0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::Z,
    },
    //
    // Left (-x)
    // Top Left - 12
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, 0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::NEG_X,
    },
    // Top Right - 13
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, -0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::NEG_X,
    },
    // Bottom Left - 14
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, 0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::NEG_X,
    },
    // Bottom Right - 15
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, -0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::NEG_X,
    },
    //
    // Top
    // Top Left - 16
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, -0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::Y,
    },
    // Top Right - 17
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, -0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::Y,
    },
    // Bottom Left - 18
    ModelVertex {
        pos: glam::vec3(0.5, 0.5, 0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::Y,
    },
    // Bottom Right - 19
    ModelVertex {
        pos: glam::vec3(-0.5, 0.5, 0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::Y,
    },
    //
    // Bottom
    // Top Left - 20
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, -0.5),
        uv: glam::vec2(0., 0.),
        normal: glam::Vec3::NEG_Y,
    },
    // Top Right - 21
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, -0.5),
        uv: glam::vec2(1., 0.),
        normal: glam::Vec3::NEG_Y,
    },
    // Bottom Left - 22
    ModelVertex {
        pos: glam::vec3(0.5, -0.5, 0.5),
        uv: glam::vec2(0., 1.),
        normal: glam::Vec3::NEG_Y,
    },
    // Bottom Right - 23
    ModelVertex {
        pos: glam::vec3(-0.5, -0.5, 0.5),
        uv: glam::vec2(1., 1.),
        normal: glam::Vec3::NEG_Y,
    },
];

pub const CUBE_INDICES: [u32; 36] = [
    0, 2, 3, 0, 3, 1, // Back
    4, 6, 7, 4, 7, 5, // Right
    8, 10, 11, 8, 11, 9, // Front
    12, 14, 15, 12, 15, 13, // Left
    16, 18, 19, 16, 19, 17, // Top
    20, 22, 23, 20, 23, 21, // Bottom
];

pub const CUBE_INDEX_COUNT: u32 = CUBE_INDICES.len() as u32;

//====================================================================
