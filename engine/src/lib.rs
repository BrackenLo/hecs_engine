//====================================================================

use std::{marker::PhantomData, sync::Arc, time::Duration};

use common::Size;
use hecs::World;
use renderer::{camera::CameraUniform, texture::LoadedTexture, RendererState};
use tools::{Input, KeyCode, MouseButton, MouseInput, Time};
use window::Window;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

mod runner;
pub mod spatial;
pub mod tools;
pub mod window;

//====================================================================

pub struct Runner<A: App> {
    state: Option<OuterState>,
    default_app: PhantomData<A>,
}

impl<A: App> Runner<A> {
    #[inline]
    pub fn run() {
        winit::event_loop::EventLoop::new()
            .unwrap()
            .run_app(&mut Self {
                state: None,
                default_app: PhantomData,
            })
            .unwrap();
    }
}

//====================================================================

pub trait App: 'static {
    fn new(state: &mut State) -> Self
    where
        Self: Sized;

    fn resize(&mut self, state: &mut State, size: Size<u32>);
    fn update(&mut self, state: &mut State);
}

//====================================================================

pub struct State {
    world: World,
    window: Window,
    target_fps: Duration,
    renderer: RendererState,
    keys: Input<KeyCode>,
    mouse_buttons: Input<MouseButton>,
    mouse_input: MouseInput,
    time: Time,
}

impl State {
    #[inline]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.window
    }

    #[inline]
    pub fn renderer_mut<'a: 'b, 'b>(&'a mut self) -> RendererAccessMut<'b> {
        RendererAccessMut(self)
    }

    #[inline]
    pub fn renderer<'a: 'b, 'b>(&'a self) -> RendererAccess<'b> {
        RendererAccess(self)
    }

    #[inline]
    pub fn keys(&self) -> &Input<KeyCode> {
        &self.keys
    }

    #[inline]
    pub fn mouse_buttons(&self) -> &Input<MouseButton> {
        &self.mouse_buttons
    }

    #[inline]
    pub fn mouse_input(&self) -> &MouseInput {
        &self.mouse_input
    }

    #[inline]
    pub fn time(&self) -> &Time {
        &self.time
    }
}

pub struct RendererAccessMut<'a>(&'a mut State);
impl<'a> RendererAccessMut<'a> {
    #[inline]
    pub fn add_renderer<R: renderer::Renderer>(&mut self, priority: usize) -> &mut Self {
        self.0
            .renderer
            .add_pipeline::<R>(&mut self.0.world, priority);
        self
    }
}

pub struct RendererAccess<'a>(&'a State);
impl<'a> RendererAccess<'a> {
    #[inline]
    pub fn spawn_camera<C: CameraUniform + 'static + Send + Sync>(
        &self,
        builder: &mut hecs::EntityBuilder,
        camera: C,
    ) {
        self.0.renderer.spawn_camera(builder, camera)
    }

    #[inline]
    pub fn clone_default_texture(&self) -> Arc<LoadedTexture> {
        self.0.renderer.default_texture.clone()
    }

    pub fn core(&self) -> &renderer::RendererCore {
        &self.0.renderer.core()
    }
}

//====================================================================

struct OuterState {
    state: State,
    app: Box<dyn App>,
}

impl OuterState {
    pub(crate) fn new<A: App>(event_loop: &ActiveEventLoop) -> Self {
        let window = Window::new(event_loop);
        #[cfg(not(target_arch = "wasm32"))]
        let window_size = window.size();
        #[cfg(target_arch = "wasm32")]
        let window_size = Size::new(450, 400);

        let renderer = RendererState::new(window.0.clone(), window_size);

        let mut state = State {
            world: World::new(),
            window,
            target_fps: Duration::from_secs_f32(1. / 75.),
            renderer,
            keys: Input::default(),
            mouse_buttons: Input::default(),
            mouse_input: MouseInput::default(),
            time: Time::default(),
        };

        let app = Box::new(A::new(&mut state));

        Self { state, app }
    }

    pub fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    log::warn!(
                        "Window resized to invalid size ({}, {})",
                        new_size.width,
                        new_size.height
                    );
                    return;
                }

                let size = Size {
                    width: new_size.width,
                    height: new_size.height,
                };

                self.state.renderer.resize(size);
                self.app.resize(&mut self.state, size);
            }

            WindowEvent::CloseRequested => {
                log::info!("Window close requested. Closing App");
                event_loop.exit();
            }

            WindowEvent::Destroyed => log::error!("Window was destroyed."),

            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    tools::process_inputs(&mut self.state.keys, key, event.state.is_pressed());
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                tools::process_inputs(&mut self.state.mouse_buttons, button, state.is_pressed());
            }

            WindowEvent::CursorMoved { position, .. } => {
                tools::process_mouse_position(&mut self.state.mouse_input, position.into());
            }

            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    tools::process_mouse_scroll(&mut self.state.mouse_input, (x, y))
                }
                winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
                    tools::process_mouse_scroll(
                        &mut self.state.mouse_input,
                        (physical_position.x as f32, physical_position.y as f32),
                    )
                }
            },
            //
            WindowEvent::RedrawRequested => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::wait_duration(
                    self.state.target_fps,
                ));

                self.tick();
            }

            _ => {}
        }
    }

    pub fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                tools::process_mouse_motion(&mut self.state.mouse_input, delta);
            }
            _ => {}
        }
    }

    #[inline]
    pub fn request_redraw(&self) {
        self.state.window.0.request_redraw();
    }

    pub fn tick(&mut self) {
        tools::tick_time(&mut self.state.time);

        self.app.update(&mut self.state);

        spatial::process_global_transform(&mut self.state);
        spatial::process_transform_hierarchy(&mut self.state);

        self.state.renderer.tick(&mut self.state.world);

        tools::reset_input(&mut self.state.keys);
        tools::reset_input(&mut self.state.mouse_buttons);
        tools::reset_mouse_input(&mut self.state.mouse_input);
    }
}

//====================================================================
