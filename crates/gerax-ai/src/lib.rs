#![allow(unused_imports)]

use gerax_core::Entity;

pub trait AiModel: Entity {
    fn model_name(&self) -> String;
}