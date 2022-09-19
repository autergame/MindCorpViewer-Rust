use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr};

use gls::Shader;

use lol::{Bone, Skeleton};

pub struct Joints {
    vao: GLuint,
    vbo: GLuint,
    pub shader: Shader,
    pub mvp_ref: GLint,
    pub joints: Vec<glam::Vec4>,
    pub joints_tpose: Vec<glam::Vec4>,
}

impl Joints {
    pub fn new(skl: &Skeleton) -> Joints {
        unsafe {
            let mut vao: GLuint = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo: GLuint = 0;
            gl::GenBuffers(1, &mut vbo);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);

            let mut joints: Vec<glam::Vec4> = Vec::with_capacity(skl.bones.len());

            for bone in &skl.bones {
                joints.push(bone.global_matrix * glam::Vec4::ONE);
            }

            let joints_tpose = joints.clone();

            Joints {
                vao,
                vbo,
                shader: Shader::new(),
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
        skl_bone: &[Bone],
        bones_transforms: &[glam::Mat4],
        projection_view_matrix: &glam::Mat4,
    ) {
        unsafe {
            let mut joints_ptr = self.joints.as_ptr();

            if use_animation {
                for i in 0..skl_bone.len() {
                    self.joints[i] =
                        bones_transforms[i] * skl_bone[i].global_matrix * glam::Vec4::ONE;
                }
            } else {
                joints_ptr = self.joints_tpose.as_ptr();
            }

            gl::Enable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);
            self.shader.enable();

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
            gl::Enable(gl::DEPTH_TEST);
            if !use_samples {
                gl::Disable(gl::BLEND);
            }
        }
    }

    pub fn set_shader_refs(&mut self, shader: Shader, refs: &[GLint]) {
        self.shader = shader;
        self.mvp_ref = refs[0];
    }

    pub fn destroy(&self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
