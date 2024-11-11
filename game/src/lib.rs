//====================================================================

use engine::{App, Runner};

//====================================================================

pub fn run() {
    Runner::<Game>::run();
}

pub struct Game;

impl App for Game {
    fn new(state: &mut engine::State) -> Self {
        Game
    }

    fn resize(&mut self, state: &mut engine::State, size: common::Size<u32>) {}

    fn update(&mut self, state: &mut engine::State) {}
}

//====================================================================
