use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint, GLushort};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use gls::Shader;

use config_json;

use lol::Skin;

use crate::{gls::glam_read, MindModel};

pub struct Model {
    vao: GLuint,
    vbo: Vec<GLuint>,
    ubo: GLuint,
    ebo: Vec<GLuint>,
    shader: Option<Rc<Shader>>,
    mvp_ref: GLint,
    diffuse_ref: GLint,
    use_bone_ref: GLint,
}

impl Model {
    pub fn new() -> Model {
        Model {
            vao: 0,
            vbo: vec![0; 4],
            ubo: 0,
            ebo: vec![],
            shader: None,
            mvp_ref: 0,
            diffuse_ref: 0,
            use_bone_ref: 0,
        }
    }

    pub fn load(&mut self, skin: &Skin, skl_bones_count: usize, shader: Rc<Shader>) {
        self.shader = Some(shader);

        unsafe {
            gl::GenVertexArrays(1, &mut self.vao);
            gl::BindVertexArray(self.vao);

            gl::GenBuffers(4, self.vbo.as_mut_ptr());

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.vertices.len() * mem::size_of::<glam::Vec3>()) as GLsizeiptr,
                skin.vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.uvs.len() * mem::size_of::<glam::Vec2>()) as GLsizeiptr,
                skin.uvs.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[2]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_indices.len() * mem::size_of::<glam_read::U16Vec4>()) as GLsizeiptr,
                skin.bone_indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(2);
            gl::VertexAttribIPointer(2, 4, gl::UNSIGNED_SHORT, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[3]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_weights.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                skin.bone_weights.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::GenBuffers(1, &mut self.ubo);

            gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                (skl_bones_count * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            self.ebo.resize(skin.meshes.len(), 0);
            gl::GenBuffers(skin.meshes.len() as GLsizei, self.ebo.as_mut_ptr());

            for i in 0..skin.meshes.len() {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo[i]);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (skin.meshes[i].submesh.indices_count * mem::size_of::<GLushort>() as u32)
                        as GLsizeiptr,
                    skin.indices
                        .as_ptr()
                        .offset(skin.meshes[i].submesh.indices_offset as isize)
                        as *const c_void,
                    gl::STATIC_DRAW,
                );
            }

            gl::BindVertexArray(0);
        }
    }

    pub fn render(
        &self,
        options: &config_json::OptionsJson,
        projection_view_matrix: &glam::Mat4,
        mind_model: &MindModel,
    ) {
        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::LineWidth(1.0f32);

            self.shader.as_ref().unwrap().enable();
            self.shader
                .as_ref()
                .unwrap()
                .set_uniform_1_int(self.diffuse_ref, 0);

            gl::ActiveTexture(gl::TEXTURE0);

            gl::BindVertexArray(self.vao);
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );
            if options.use_animation {
                gl::Uniform1i(self.use_bone_ref, 1);
                gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo);
                gl::BufferSubData(
                    gl::UNIFORM_BUFFER,
                    0,
                    (mind_model.bones_transforms.len() * mem::size_of::<glam::Mat4>())
                        as GLsizeiptr,
                    mind_model.bones_transforms.as_ptr() as *const c_void,
                );
                gl::BindBufferRange(
                    gl::UNIFORM_BUFFER,
                    0,
                    self.ubo,
                    0,
                    (mind_model.bones_transforms.len() * mem::size_of::<glam::Mat4>())
                        as GLsizeiptr,
                );
            } else {
                gl::Uniform1i(self.use_bone_ref, 0);
            }
            if options.show_wireframe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
            }
            for i in 0..mind_model.skn.meshes.len() {
                if mind_model.show_meshes[i] {
                    mind_model.textures[mind_model.textures_selecteds[i]].bind();
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo[i]);
                    gl::DrawElements(
                        gl::TRIANGLES,
                        mind_model.skn.meshes[i].submesh.indices_count as GLsizei,
                        gl::UNSIGNED_SHORT,
                        ptr::null(),
                    );
                }
            }
            if options.show_wireframe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
            }
            gl::BindVertexArray(0);
        }
    }

    pub fn bind_ubo(&self, ubo_ref: GLuint, bones_transforms_size: usize) {
        unsafe {
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo);
            self.shader.as_ref().unwrap().ubo_binding(ubo_ref, 0);
            gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.ubo);
            gl::BindBufferRange(
                gl::UNIFORM_BUFFER,
                0,
                self.ubo,
                0,
                (bones_transforms_size * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
            );
        }
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint]) {
        self.mvp_ref = refs[0];
        self.diffuse_ref = refs[1];
        self.use_bone_ref = refs[2];
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(4, self.vbo.as_ptr());
            gl::DeleteBuffers(1, &self.ubo);
            gl::DeleteBuffers(self.ebo.len() as GLsizei, self.ebo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
