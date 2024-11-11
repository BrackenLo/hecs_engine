//====================================================================

use std::{marker::PhantomData, time::Duration};

use common::Size;
use hecs::{Entity, World};
use renderer::{camera::CameraUniform, RendererState};
use tools::{Input, KeyCode, Time};
use window::Window;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

mod runner;
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
    time: Time,
}

pub struct RendererAccess<'a>(&'a mut State);
impl<'a> RendererAccess<'a> {
    #[inline]
    pub fn add_renderer<R: renderer::Renderer>(&mut self, priority: usize) {
        self.0
            .renderer
            .add_pipeline::<R>(&mut self.0.world, priority);
    }

    #[inline]
    pub fn spawn_camera<C: CameraUniform + 'static + Send + Sync>(&mut self, camera: C) -> Entity {
        self.0.renderer.spawn_camera(&mut self.0.world, camera)
    }
}

impl State {
    #[inline]
    pub fn renderer<'a: 'b, 'b>(&'a mut self) -> RendererAccess<'b> {
        RendererAccess(self)
    }

    #[inline]
    pub fn keys(&self) -> &Input<KeyCode> {
        &self.keys
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
        let renderer = RendererState::new(window.0.clone(), window.size());

        let mut state = State {
            world: World::new(),
            window,
            target_fps: Duration::from_secs_f32(1. / 75.),
            renderer,
            keys: Input::default(),
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
                    tools::process_inputs(&mut self.state.keys, key, event.state.is_pressed())
                }
            }

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
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    #[inline]
    pub fn request_redraw(&self) {
        self.state.window.0.request_redraw();
    }

    pub fn tick(&mut self) {
        tools::tick_time(&mut self.state.time);

        self.app.update(&mut self.state);
        self.state.renderer.tick(&mut self.state.world);

        tools::reset_input(&mut self.state.keys);
    }
}

//====================================================================
