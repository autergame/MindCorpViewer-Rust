use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use gls::{glam_read, Shader};

use config_json;

use lol::Skin;

use crate::MindModel;

pub struct Model {
    vao: GLuint,
    bo: Vec<GLuint>,
    shader: Option<Rc<Shader>>,
    mvp_ref: GLint,
    use_bone_ref: GLint,
}

impl Model {
    pub fn create(skin: &Skin, skl_bones_count: usize, shader: Rc<Shader>) -> Model {
        unsafe {
            let mut vao: GLuint = 0;
            let mut bo: Vec<GLuint> = vec![0; 6];

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(6, bo.as_mut_ptr());

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.vertices.len() * mem::size_of::<glam::Vec3>()) as GLsizeiptr,
                skin.vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.uvs.len() * mem::size_of::<glam::Vec2>()) as GLsizeiptr,
                skin.uvs.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[2]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_indices.len() * mem::size_of::<glam_read::U16Vec4>()) as GLsizeiptr,
                skin.bone_indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(2);
            gl::VertexAttribIPointer(2, 4, gl::UNSIGNED_SHORT, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[3]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_weights.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                skin.bone_weights.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::UNIFORM_BUFFER, bo[4]);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                (skl_bones_count * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, bo[5]);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (skin.indices.len() * mem::size_of::<u16>()) as GLsizeiptr,
                skin.indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindVertexArray(0);

            Model {
                vao,
                bo,
                shader: Some(shader),
                mvp_ref: 0,
                use_bone_ref: 0,
            }
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
                gl::BindBuffer(gl::UNIFORM_BUFFER, self.bo[4]);
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
                    self.bo[4],
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
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.bo[5]);
            for i in 0..mind_model.skn.meshes.len() {
                if mind_model.show_meshes[i] {
                    mind_model.textures[mind_model.textures_selecteds[i]].bind();
                    gl::DrawElements(
                        gl::TRIANGLES,
                        mind_model.skn.meshes[i].submesh.indices_count as GLsizei,
                        gl::UNSIGNED_SHORT,
                        (mind_model.skn.meshes[i].submesh.indices_offset
                            * mem::size_of::<u16>() as u32)
                            as *const c_void,
                    );
                }
            }
            if options.show_wireframe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
            }
            gl::BindVertexArray(0);
        }
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint], ubo_ref: GLuint) {
        self.mvp_ref = refs[0];
        let diffuse_ref = refs[1];
        self.use_bone_ref = refs[2];

        let shader = self.shader.as_ref().unwrap();
        unsafe {
            shader.enable();
            gl::Uniform1i(diffuse_ref, 0);
            gl::UniformBlockBinding(shader.id, ubo_ref, 0);
        }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(6, self.bo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
