//====================================================================

use std::collections::{HashMap, HashSet};

use common::GlobalTransform;
use hecs::Entity;
use renderer::{
    camera::{CameraWgpu, PerspectiveCamera},
    shared::Vertex,
    text_shared::{Metrics, TextBuffer, TextBufferDescriptor, TextResources, TextVertex, Wrap},
    texture::Texture,
    tools, Renderer,
};

//====================================================================

#[derive(Debug, Clone)]
pub struct Ui3d {
    pub menu_color: [f32; 4],
    pub selection_color: [f32; 4],

    pub options: Vec<String>,
    pub selected: u8,
    pub font_size: f32,
}

impl Default for Ui3d {
    fn default() -> Self {
        Self {
            menu_color: [0.5, 0.5, 0.5, 0.7],
            selection_color: [0.7, 0.7, 0.7, 0.8],
            options: Vec::new(),
            selected: 0,
            font_size: 30.,
        }
    }
}

#[derive(Debug)]
struct Ui3dData {
    ui_uniform_buffer: wgpu::Buffer,
    ui_uniform_bind_group: wgpu::BindGroup,

    ui_position_uniform_buffer: wgpu::Buffer,
    ui_position_uniform_bind_group: wgpu::BindGroup,
    size: [f32; 2],

    text_buffer: TextBuffer,
}

//====================================================================

pub struct Ui3dRenderer {
    ui_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,

    ui_uniform_bind_group_layout: wgpu::BindGroupLayout,
    ui_position_uniform_bind_group_layout: wgpu::BindGroupLayout,

    instances: HashMap<Entity, Ui3dData>,
}

impl Renderer for Ui3dRenderer {
    fn new(
        core: &renderer::RendererCore,
        shared: &mut renderer::shared::SharedRenderResources,
        _world: &mut hecs::World,
    ) -> Self
    where
        Self: Sized,
    {
        let ui_position_uniform_bind_group_layout =
            core.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Ui Instance Buffer Bind Group Layout"),
                    entries: &[tools::bgl_uniform_entry(0, wgpu::ShaderStages::VERTEX)],
                });

        let ui_uniform_bind_group_layout =
            core.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Ui Instance Buffer Bind Group Layout"),
                    entries: &[tools::bgl_uniform_entry(0, wgpu::ShaderStages::VERTEX)],
                });

        let ui_pipeline = tools::create_pipeline(
            core.device(),
            core.config(),
            "Ui Renderer",
            &[
                shared.camera_bind_group_layout(),
                &ui_uniform_bind_group_layout,
                &ui_position_uniform_bind_group_layout,
            ],
            &[],
            include_str!("shaders/ui3d.wgsl"),
            tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                fragment_targets: Some(&[Some(wgpu::ColorTargetState {
                    format: core.config().format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })]),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                ..Default::default()
            },
        );

        let text_pipeline = tools::create_pipeline(
            core.device(),
            core.config(),
            "Ui Text Renderer",
            &[
                shared.camera_bind_group_layout(),
                shared.text_resources().text_atlas.bind_group_layout(),
                &ui_position_uniform_bind_group_layout,
            ],
            &[TextVertex::desc()],
            include_str!("shaders/text.wgsl"),
            tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                fragment_targets: Some(&[Some(wgpu::ColorTargetState {
                    format: core.config().format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })]),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                ..Default::default()
            },
        );

        Self {
            ui_pipeline,
            text_pipeline,
            ui_uniform_bind_group_layout,
            ui_position_uniform_bind_group_layout,
            instances: HashMap::default(),
        }
    }

    fn prep(
        &mut self,
        core: &renderer::RendererCore,
        shared: &mut renderer::shared::SharedRenderResources,
        world: &mut hecs::World,
    ) {
        //--------------------------------------------------

        let camera_pos: glam::Vec3 = match world
            .query::<(&PerspectiveCamera, &GlobalTransform)>()
            .into_iter()
            .next()
        {
            Some((_, (_, transform))) => transform.translation(),
            None => return,
        };

        // Force all ui to look at camera
        world
            .query::<(&mut GlobalTransform, &Ui3d)>()
            .iter()
            .for_each(|(_, (transform, _))| {
                transform.0 =
                    glam::Affine3A::look_at_lh(transform.translation(), camera_pos, glam::Vec3::Y)
            });

        //--------------------------------------------------

        let mut previous = self.instances.keys().map(|id| *id).collect::<HashSet<_>>();

        // Prep all ui
        world
            .query_mut::<(&Ui3d, &GlobalTransform)>()
            .into_iter()
            .for_each(|(entity, (ui, transform))| {
                previous.remove(&entity);

                //--------------------------------------------------
                // Insert new text data

                if !self.instances.contains_key(&entity) {
                    self.insert_ui(core.device(), shared.text_resources_mut(), entity, ui)
                }

                let data = match self.instances.get_mut(&entity) {
                    Some(data) => data,
                    None => return,
                };

                //--------------------------------------------------
                // Build Text

                if let Some(rebuild) = renderer::text_shared::prep(
                    core.device(),
                    core.queue(),
                    shared.text_resources_mut(),
                    &mut data.text_buffer,
                ) {
                    log::trace!("Rebuilding text for ui entity {:?}", entity);
                    tools::update_instance_buffer(
                        core.device(),
                        core.queue(),
                        "UI3d Text Vertex Buffer",
                        &mut data.text_buffer.vertex_buffer,
                        &mut data.text_buffer.vertex_count,
                        &rebuild,
                    );
                }

                //--------------------------------------------------
                // Build Transform

                let position_raw = UiPositionUniformRaw {
                    transform: transform.to_matrix(),
                };

                core.queue()
                    .write_buffer_with(
                        &data.ui_position_uniform_buffer,
                        0,
                        wgpu::BufferSize::new(std::mem::size_of::<UiPositionUniformRaw>() as u64)
                            .unwrap(),
                    )
                    .unwrap()
                    .copy_from_slice(bytemuck::cast_slice(&[position_raw]));

                //--------------------------------------------------
                // Build UI background

                let longest_line = ui.options.iter().reduce(|a, b| match a.len() < b.len() {
                    true => a,
                    false => b,
                });

                let longest_line = match longest_line {
                    Some(val) => val,
                    None => return,
                };

                let selected = ui.selected.clamp(0, ui.options.len() as u8) as f32;

                let option_count = ui.options.len() as f32;
                let option_range = 1. / option_count;

                let ui_size = glam::vec2(
                    ui.font_size * longest_line.len() as f32,
                    ui.font_size * option_count,
                );

                data.size = ui_size.to_array();
                data.text_buffer.set_metrics(
                    &mut shared.text_resources_mut().font_system,
                    Metrics::new(ui.font_size, ui.font_size),
                );

                let ui_raw = UiUniformRaw {
                    size: ui_size,
                    menu_color: ui.menu_color.into(),
                    selection_color: ui.selection_color.into(),
                    selection_range_y: glam::vec2(
                        option_range * selected,
                        option_range * (selected + 1.),
                    ),

                    pad: [0.; 2],
                    pad2: [0.; 2],
                };

                core.queue()
                    .write_buffer_with(
                        &data.ui_uniform_buffer,
                        0,
                        wgpu::BufferSize::new(std::mem::size_of::<UiUniformRaw>() as u64).unwrap(),
                    )
                    .unwrap()
                    .copy_from_slice(bytemuck::cast_slice(&[ui_raw]));
            });

        // Remove unused text data
        previous.into_iter().for_each(|to_remove| {
            self.instances.remove(&to_remove);
        });
    }

    fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
        shared: &mut renderer::shared::SharedRenderResources,
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

        // Set camera (both pipelines)
        render_pass.set_bind_group(0, camera.bind_group(), &[]);

        // Draw UI background
        render_pass.set_pipeline(&self.ui_pipeline);

        self.instances.values().into_iter().for_each(|instance| {
            render_pass.set_bind_group(1, &instance.ui_uniform_bind_group, &[]);
            render_pass.set_bind_group(2, &instance.ui_position_uniform_bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        });

        // // Draw Text
        render_pass.set_pipeline(&self.text_pipeline);
        render_pass.set_bind_group(1, shared.text_resources().text_atlas.bind_group(), &[]);

        self.instances.values().into_iter().for_each(|instance| {
            render_pass.set_vertex_buffer(0, instance.text_buffer.vertex_buffer.slice(..));
            render_pass.set_bind_group(2, &instance.ui_position_uniform_bind_group, &[]);
            render_pass.draw(0..4, 0..instance.text_buffer.vertex_count);
        });
    }
}

impl Ui3dRenderer {
    fn insert_ui(
        &mut self,
        device: &wgpu::Device,
        text_resources: &mut TextResources,
        entity: Entity,
        ui: &Ui3d,
    ) {
        log::trace!("Inserting new ui3d Data");

        let ui_uniform_buffer = tools::buffer(
            device,
            tools::BufferType::Uniform,
            "Ui",
            &[UiUniformRaw {
                size: glam::vec2(1., 1.),
                pad: [0.; 2],
                menu_color: glam::vec4(1., 1., 1., 1.),
                selection_color: glam::vec4(1., 0., 0., 1.),
                selection_range_y: glam::vec2(0., 0.),
                pad2: [0.; 2],
            }],
        );

        let ui_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ui Bind Group"),
            layout: &self.ui_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    ui_uniform_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let ui_position_uniform_buffer = tools::buffer(
            device,
            tools::BufferType::Uniform,
            "Ui Position",
            &[UiPositionUniformRaw {
                transform: glam::Mat4::default(),
            }],
        );

        let ui_position_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ui Position Bind Group"),
            layout: &self.ui_position_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    ui_position_uniform_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let text = ui
            .options
            .iter()
            .cloned()
            .reduce(|a, b| format!("{}\n{}", a, b))
            .unwrap_or(String::new());

        let text_buffer = TextBuffer::new(
            device,
            &mut text_resources.font_system,
            &TextBufferDescriptor {
                metrics: Metrics::new(10., 10.),
                word_wrap: Wrap::None,
                // attributes: todo!(),
                text: &text,
                // width: todo!(),
                // height: todo!(),
                // color: todo!(),
                ..Default::default()
            },
        );

        self.instances.insert(
            entity,
            Ui3dData {
                ui_uniform_buffer,
                ui_uniform_bind_group,
                ui_position_uniform_buffer,
                ui_position_uniform_bind_group,
                size: [1., 1.],
                text_buffer,
            },
        );
    }
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct UiPositionUniformRaw {
    transform: glam::Mat4,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct UiUniformRaw {
    pub size: glam::Vec2,
    pub pad: [f32; 2],

    pub menu_color: glam::Vec4,
    pub selection_color: glam::Vec4,
    pub selection_range_y: glam::Vec2,
    pub pad2: [f32; 2],
}

//====================================================================
