use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use gls::Shader;

use lol::{Skeleton};

use crate::MindModel;

pub struct Joints {
    vao: GLuint,
    vbo: GLuint,
    shader: Option<Rc<Shader>>,
    mvp_ref: GLint,
    joints: Vec<glam::Vec4>,
    joints_tpose: Vec<glam::Vec4>,
}

impl Joints {
    pub fn new() -> Joints {
        Joints {
            vao: 0,
            vbo: 0,
            shader: None,
            mvp_ref: 0,
            joints: vec![],
            joints_tpose: vec![],
        }
    }

    pub fn load(&mut self, skl: &Skeleton, shader: Rc<Shader>) {
		self.joints.reserve_exact(skl.bones.len());

		for bone in skl.bones.iter() {
			self.joints.push(bone.global_matrix * glam::Vec4::ONE);
		}

		self.joints_tpose = self.joints.clone();
		self.shader = Some(shader);

        unsafe {
            gl::GenVertexArrays(1, &mut self.vao);
            gl::BindVertexArray(self.vao);

            gl::GenBuffers(1, &mut self.vbo);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                self.joints.as_ptr() as *const c_void,
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);
        }
    }

    pub fn render(
        &mut self,
        use_animation: bool,
        use_samples: bool,
        projection_view_matrix: &glam::Mat4,
		mind_model: &MindModel,
    ) {
		let mut joints_ptr = self.joints.as_ptr();

		if use_animation {
			for i in 0..mind_model.skl.bones.len() {
				self.joints[i] = mind_model.bones_transforms[i]
					* mind_model.skl.bones[i].global_matrix
					* glam::Vec4::ONE;
			}
		} else {
			joints_ptr = self.joints_tpose.as_ptr();
		}

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
