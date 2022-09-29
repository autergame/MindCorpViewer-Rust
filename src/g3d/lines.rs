use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use gls::Shader;

use lol::Skeleton;

use crate::MindModel;

pub struct Lines {
    vao: GLuint,
    vbo: Vec<GLuint>,
    shader: Option<Rc<Shader>>,
    mvp_ref: GLint,
    lines: Vec<glam::Vec4>,
    lines_tpose: *const glam::Vec4,
}

impl Lines {
    pub fn create(skl: &Skeleton, shader: Rc<Shader>) -> Lines {
        let mut lines: Vec<glam::Vec4> = Vec::with_capacity(skl.bones.len() * 2);
        let mut colors: Vec<glam::Vec3> = Vec::with_capacity(skl.bones.len() * 2);

        for bone in skl.bones.iter() {
            let parent_id = bone.parent_id;
            if parent_id != -1 {
                lines.push(skl.bones[parent_id as usize].global_matrix * glam::Vec4::ONE);
                lines.push(bone.global_matrix * glam::Vec4::ONE);
                colors.push(glam::vec3(0.0f32, 1.0f32, 0.0f32));
                colors.push(glam::vec3(0.0f32, 0.0f32, 1.0f32));
            }
        }

        let lines_tpose = lines.as_ptr();
        let shader = Some(shader);

        unsafe {
            let mut vao: GLuint = 0;
            let mut vbo: Vec<GLuint> = vec![0; 2];

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(2, vbo.as_mut_ptr());

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * 2 * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                lines.as_ptr() as *const c_void,
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * 2 * mem::size_of::<glam::Vec3>()) as GLsizeiptr,
                colors.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);

            Lines {
                vao,
                vbo,
                shader,
                mvp_ref: 0,
                lines,
                lines_tpose,
            }
        }
    }

    pub fn render(
        &mut self,
        use_animation: bool,
        projection_view_matrix: &glam::Mat4,
        mind_model: &MindModel,
    ) {
        let lines_ptr = if use_animation {
            let mut line_index: usize = 0;
            for i in 0..mind_model.skl.bones.len() {
                let parent_id = mind_model.skl.bones[i].parent_id;
                if parent_id != -1 {
                    self.lines[line_index] = mind_model.bones_transforms[parent_id as usize]
                        * mind_model.skl.bones[parent_id as usize].global_matrix
                        * glam::Vec4::ONE;
                    self.lines[line_index + 1] = mind_model.bones_transforms[i]
                        * mind_model.skl.bones[i].global_matrix
                        * glam::Vec4::ONE;
                    line_index += 2;
                }
            }
            self.lines.as_ptr()
        } else {
            self.lines_tpose
        };

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::LineWidth(2.0f32);

            self.shader.as_ref().unwrap().enable();
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );

            gl::BindVertexArray(self.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[0]);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (self.lines.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                lines_ptr as *const c_void,
            );

            gl::DrawArrays(gl::LINES, 0, self.lines.len() as GLsizei);

            gl::BindVertexArray(0);
        }
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint]) {
        self.mvp_ref = refs[0];
    }
}

impl Drop for Lines {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(2, self.vbo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
