use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use gls::Shader;

use lol::Skeleton;

use crate::MindModel;

pub struct Joints {
    vao: GLuint,
    vbo: GLuint,
    shader: Option<Rc<Shader>>,
    mvp_ref: GLint,
    joints: Vec<glam::Vec4>,
    joints_tpose: *const glam::Vec4,
}

impl Joints {
    pub fn create(skl: &Skeleton, shader: Rc<Shader>) -> Joints {
        let mut joints: Vec<glam::Vec4> = Vec::with_capacity(skl.bones.len());

        for bone in skl.bones.iter() {
            joints.push(bone.global_matrix * glam::Vec4::ONE);
        }

        let joints_tpose = joints.as_ptr();
        let shader = Some(shader);

        unsafe {
            let mut vao: GLuint = 0;
            let mut vbo: GLuint = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                joints.as_ptr() as *const c_void,
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);

            Joints {
                vao,
                vbo,
                shader,
                mvp_ref: 0,
                joints,
                joints_tpose,
            }
        }
    }

    pub fn render(
        &mut self,
        use_animation: bool,
        use_samples: bool,
        projection_view_matrix: &glam::Mat4,
        mind_model: &MindModel,
    ) {
        let joints_ptr = if use_animation {
            for i in 0..mind_model.skl.bones.len() {
                self.joints[i] = mind_model.bones_transforms[i]
                    * mind_model.skl.bones[i].global_matrix
                    * glam::Vec4::ONE;
            }
            self.joints.as_ptr()
        } else {
            self.joints_tpose
        };

        unsafe {
            gl::Enable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);

            self.shader.as_ref().unwrap().enable();
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );

            gl::BindVertexArray(self.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (self.joints.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                joints_ptr as *const c_void,
            );

            gl::DrawArrays(gl::POINTS, 0, self.joints.len() as GLsizei);

            gl::BindVertexArray(0);

            if !use_samples {
                gl::Disable(gl::BLEND);
            }
        }
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint]) {
        self.mvp_ref = refs[0];
    }
}

impl Drop for Joints {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
