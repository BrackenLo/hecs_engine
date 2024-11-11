//====================================================================

use std::sync::Arc;

use common::Size;
use winit::{event_loop::ActiveEventLoop, window::WindowAttributes};

//====================================================================

pub struct Window(pub Arc<winit::window::Window>);
impl Window {
    pub(super) fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

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
}

//====================================================================
