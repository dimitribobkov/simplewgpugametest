//Component Base
use std::any::Any;

trait Component {}
pub trait ComponentBase{
    fn get_id(&self) -> u32;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut Any;
}