use crate::gls::Shader;
use gl::types::{GLint, GLuint};
use std::rc::Rc;

pub struct Shaders {
    pub model: Rc<Shader>,
    pub names: Rc<Shader>,
    pub bones: Rc<Shader>,
    pub joints: Rc<Shader>,
}

impl Shaders {
    pub fn new() -> Shaders {
        let model = Rc::new(Shader::create(
            include_str!("../../assets/model/model.vert"),
            include_str!("../../assets/model/model.frag"),
        ));
        let bones = Rc::new(Shader::create(
            include_str!("../../assets/bones/bones.vert"),
            include_str!("../../assets/bones/bones.frag"),
        ));
        let joints = Rc::new(Shader::create(
            include_str!("../../assets/joints/joints.vert"),
            include_str!("../../assets/joints/joints.frag"),
        ));
        let names = Rc::new(Shader::create(
            include_str!("../../assets/names/names.vert"),
            include_str!("../../assets/names/names.frag"),
        ));

        Shaders {
            model,
            names,
            bones,
            joints,
        }
    }
}

pub struct Refs {
    pub model: Vec<GLint>,
    pub names: Vec<GLint>,
    pub bones: Vec<GLint>,
    pub joints: Vec<GLint>,
    pub model_ubo: GLuint,
}

impl Refs {
    pub fn new(shaders: &Shaders) -> Refs {
        let model = shaders.model.get_refs(&["MVP", "Diffuse", "UseBone"]);
        let model_ubo = shaders.model.get_ubo_ref("BonesTransformsBlock");
        let bones = shaders.bones.get_refs(&["MVP"]);
        let joints = shaders.joints.get_refs(&["MVP"]);
        let names = shaders.names.get_refs(&[
            "MVP",
            "TextSize",
            "TextScale",
            "TextOffset",
            "TextOffsetSize",
            "TextPosition",
            "CameraUp",
            "CameraRight",
            "TextTexture",
        ]);

        Refs {
            model,
            names,
            bones,
            joints,
            model_ubo,
        }
    }
}
