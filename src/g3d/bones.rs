use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use crate::{gls::Shader, lol::Skeleton, MindModel};

pub struct Bones {
    vao: GLuint,
    bo: Vec<GLuint>,
    shader: Rc<Shader>,
    mvp_ref: GLint,
    bones: Vec<glam::Vec4>,
    bones_tpose: *const glam::Vec4,
}

impl Bones {
    pub fn create(skl: &Skeleton, shader: Rc<Shader>) -> Bones {
        let mut bones: Vec<glam::Vec4> = Vec::with_capacity(skl.joints.len() * 2);
        let mut colors: Vec<glam::Vec3> = Vec::with_capacity(skl.joints.len() * 2);

        for joint in skl.joints.iter() {
            let parent_id = joint.parent_id;
            if parent_id != -1 {
                bones.push(skl.joints[parent_id as usize].global_matrix * glam::Vec4::ONE);
                bones.push(joint.global_matrix * glam::Vec4::ONE);
                colors.push(glam::vec3(0.0f32, 1.0f32, 0.0f32));
                colors.push(glam::vec3(0.0f32, 0.0f32, 1.0f32));
            }
        }

        let bones_tpose = bones.as_ptr();

        unsafe {
            let mut vao: GLuint = 0;
            let mut bo: Vec<GLuint> = vec![0; 2];

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(2, bo.as_mut_ptr());

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                bones_tpose as *const c_void,
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (colors.len() * mem::size_of::<glam::Vec3>()) as GLsizeiptr,
                colors.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);

            Bones {
                vao,
                bo,
                shader,
                mvp_ref: 0,
                bones,
                bones_tpose,
            }
        }
    }

    pub fn render(
        &mut self,
        use_animation: bool,
        projection_view_matrix: &glam::Mat4,
        mind_model: &MindModel,
    ) {
        let bones_ptr = if use_animation {
            let mut line_index: usize = 0;

            for i in 0..mind_model.skeleton.joints.len() {
                let parent_id = mind_model.skeleton.joints[i].parent_id;

                if parent_id != -1 {
                    self.bones[line_index] = mind_model.joints_transforms[parent_id as usize]
                        * mind_model.skeleton.joints[parent_id as usize].global_matrix
                        * glam::Vec4::ONE;

                    self.bones[line_index + 1] = mind_model.joints_transforms[i]
                        * mind_model.skeleton.joints[i].global_matrix
                        * glam::Vec4::ONE;

                    line_index += 2;
                }
            }
            self.bones.as_ptr()
        } else {
            self.bones_tpose
        };

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::LineWidth(2.0f32);

            self.shader.as_ref().enable();
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );

            gl::BindVertexArray(self.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.bo[0]);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (self.bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                bones_ptr as *const c_void,
            );

            gl::DrawArrays(gl::LINES, 0, self.bones.len() as GLsizei);

            gl::BindVertexArray(0);
        }
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint]) {
        self.mvp_ref = refs[0];
    }
}

impl Drop for Bones {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(2, self.bo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
