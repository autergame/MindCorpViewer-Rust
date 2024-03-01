pub mod bones;
pub mod floor;
pub mod joints;
pub mod model;
pub mod names;
pub mod skybox;
pub mod refs_shaders;

pub use self::bones::Bones;
pub use self::floor::Floor;
pub use self::joints::Joints;
pub use self::model::Model;
pub use self::names::Names;
pub use self::skybox::Skybox;
pub use self::refs_shaders::{Refs, Shaders};

pub struct Character {
    pub model: Model,
    pub names: Names,
    pub bones: Bones,
    pub joints: Joints,
}
