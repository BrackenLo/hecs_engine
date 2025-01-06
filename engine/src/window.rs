//====================================================================

use std::sync::Arc;

use common::Size;
use winit::{event_loop::ActiveEventLoop, window::WindowAttributes};

//====================================================================

pub struct Window(pub(crate) Arc<winit::window::Window>);
impl Window {
    pub(super) fn new(event_loop: &ActiveEventLoop) -> Self {
        log::info!("Creating new window");

        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        #[cfg(target_arch = "wasm32")]
        {
            use winit::{dpi::PhysicalSize, platform::web::WindowExtWebSys};

            log::info!("Adding canvas to window");

            if let None = window.request_inner_size(PhysicalSize::new(450, 400)) {
                log::warn!(
                    "Wasm Window Resize Warning: Got none when requesting window inner size"
                );
            }

            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("hecs_engine_wasm")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        Self(Arc::new(window))
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        let window_size = self.0.inner_size();

        Size {
            width: window_size.width,
            height: window_size.height,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub fn confine_cursor(&self, confined: bool) {
        log::trace!("Confining window cursor: {}", confined);

        self.0
            .set_cursor_grab(match confined {
                true => winit::window::CursorGrabMode::Confined,
                false => winit::window::CursorGrabMode::None,
            })
            .unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub fn confine_cursor(&self, _confined: bool) {
        log::trace!("Confining window cursor not supported");
    }

    #[inline]
    pub fn hide_cursor(&self, hidden: bool) {
        log::trace!("Hiding window cursor: {}", hidden);
        self.0.set_cursor_visible(!hidden);
    }

    #[inline]
    pub fn inner(&self) -> &winit::window::Window {
        &self.0
    }
}

//====================================================================
