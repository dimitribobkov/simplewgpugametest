// Systems for entities
pub mod movement_system;
pub mod player_movement_system;
pub mod systemmanager;

use crate::{Renderer, EntityManager, Rc, RefCell, InputManager};

pub trait SystemBase{
    fn execute(&mut self, renderer: &Renderer, entity_manager: &mut EntityManager, input_manager: &InputManager);
}
