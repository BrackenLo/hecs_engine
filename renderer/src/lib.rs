//====================================================================

use std::sync::Arc;

use camera::CameraUniform;
use common::Size;
use hecs::World;
use shared::SharedRenderResources;
use texture::{LoadedTexture, Texture};
use wgpu::SurfaceTarget;

pub mod camera;
pub mod shared;
pub mod text_shared;
pub mod texture;
pub mod tools;

//====================================================================

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct WgpuWrapper<T>(T);

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct WgpuWrapper<T>(send_wrapper::SendWrapper<T>);

impl<T> WgpuWrapper<T> {
    #[inline]
    fn new(data: T) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Self(data);

        #[cfg(target_arch = "wasm32")]
        return Self(send_wrapper::SendWrapper::new(data));
    }

    #[inline]
    pub fn inner(&self) -> &T {
        &self.0
    }
}

//====================================================================

pub struct RendererState {
    core: RendererCore,
    depth_texture: Texture,

    shared_resources: SharedRenderResources,
    pub default_texture: Arc<LoadedTexture>,
    pub clear_color: wgpu::Color,

    pipelines: Vec<RendererData>,
}

impl RendererState {
    pub fn new(window: impl Into<SurfaceTarget<'static>>, window_size: Size<u32>) -> Self {
        let core = pollster::block_on(RendererCore::new(window, window_size));
        let depth_texture =
            Texture::create_depth_texture(&core.device, window_size, "Depth Texture");

        let shared_resources = SharedRenderResources::new(&core.device);

        let default_texture = Arc::new(LoadedTexture::load_texture(
            &core.device,
            &shared_resources,
            Texture::from_color(
                &core.device,
                &core.queue,
                [255; 3],
                Some("Default Texture"),
                None,
            ),
        ));

        let clear_color = wgpu::Color {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.,
        };

        Self {
            core,
            depth_texture,
            shared_resources,
            default_texture,
            clear_color,
            pipelines: Vec::new(),
        }
    }

    pub fn add_pipeline<R: Renderer>(&mut self, world: &mut World, priority: usize) {
        let pipeline = Box::new(R::new(&self.core, &mut self.shared_resources, world));

        self.pipelines.push(RendererData { priority, pipeline });
        self.pipelines.sort_by_key(|val| val.priority);
    }

    pub fn spawn_camera<C: CameraUniform + 'static + Send + Sync>(
        &self,
        builder: &mut hecs::EntityBuilder,
        camera: C,
    ) {
        let camera_wgpu = self
            .shared_resources
            .create_camera(&self.core.device, &camera);

        builder.add(camera).add(camera_wgpu);
    }

    pub fn resize(&mut self, new_size: Size<u32>) {
        self.core.config.width = new_size.width;
        self.core.config.height = new_size.height;
        self.core
            .surface
            .configure(&self.core.device, &self.core.config);

        self.depth_texture =
            Texture::create_depth_texture(&self.core.device, new_size, "Depth Texture");
    }

    pub fn tick(&mut self, world: &mut World) {
        camera::sys_prep_perspective_cameras(world, &self.core.queue);
        camera::sys_prep_orthographic_cameras(world, &self.core.queue);

        // Prep pipelines
        self.pipelines.iter_mut().for_each(|pipeline_data| {
            pipeline_data
                .pipeline
                .prep(&self.core, &mut self.shared_resources, world)
        });

        // Get and check surface
        let (surface_texture, surface_view) = match self.core.surface.get_current_texture() {
            Ok(texture) => {
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                (texture, view)
            }
            Err(_) => {
                log::warn!("Unable to get surface texture - skipping frame");
                return;
            }
        };

        // Create command encoder
        let mut encoder = self
            .core
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // Begin main render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],

            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),

            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render all pipelines
        self.pipelines.iter_mut().for_each(|pipeline_data| {
            pipeline_data
                .pipeline
                .render(&mut render_pass, &mut self.shared_resources, world)
        });

        std::mem::drop(render_pass);

        // Finish and submit
        self.core.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

//====================================================================

pub struct RendererCore {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl RendererCore {
    #[inline]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[inline]
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }
}

impl RendererCore {
    pub async fn new(window: impl Into<SurfaceTarget<'static>>, window_size: Size<u32>) -> Self {
        log::debug!("Creating core wgpu renderer components.");

        log::debug!("Window inner size = {:?}", window_size);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // let surface = instance.create_surface(window.0.clone()).unwrap();
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        log::debug!("Chosen device adapter: {:#?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    #[cfg(target_arch = "wasm32")]
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        log::debug!("Successfully created core wgpu components.");

        Self {
            device,
            queue,
            surface,
            config,
        }
    }
}

//====================================================================

struct RendererData {
    priority: usize,
    pipeline: Box<dyn Renderer>,
}

pub trait Renderer: 'static {
    fn new(core: &RendererCore, shared: &mut SharedRenderResources, world: &mut World) -> Self
    where
        Self: Sized;

    fn prep(&mut self, core: &RendererCore, shared: &mut SharedRenderResources, world: &mut World);
    fn resize(&mut self, core: &RendererCore) {
        let _ = core;
    }
    fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
        shared: &mut SharedRenderResources,
        world: &mut World,
    );
}

//====================================================================
