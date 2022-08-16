use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};

use std::{mem, os::raw::c_void, ptr};

use lol;

pub struct LinesJoints {
    vao: GLuint,
    vbo: Vec<GLuint>,
    pub shader: GLuint,
    pub mvp_ref: GLint,
    pub color_ref: GLint,
    pub lines: Vec<glam::Vec4>,
    pub joints: Vec<glam::Vec4>,
    pub lines_tpose: Vec<glam::Vec4>,
    pub joints_tpose: Vec<glam::Vec4>,
}

impl LinesJoints {
    #[inline(never)]
    pub fn new(skl: &lol::skl::Skeleton) -> LinesJoints {
        unsafe {
            let mut vao: GLuint = 0;
            gl::GenVertexArrays(2, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo: Vec<GLuint> = vec![0; 2];
            gl::GenBuffers(2, vbo.as_mut_ptr());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * 2 * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(vao + 1);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skl.bones.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindVertexArray(0);

            let mut lines: Vec<glam::Vec4> = Vec::with_capacity(skl.bones.len() * 2);
            let mut joints: Vec<glam::Vec4> = Vec::with_capacity(skl.bones.len());

            for bone in &skl.bones {
                let parent_id = bone.parent_id;
                if parent_id != -1 {
                    lines.push(skl.bones[parent_id as usize].global_matrix * glam::Vec4::ONE);
                    lines.push(bone.global_matrix * glam::Vec4::ONE);
                }
                joints.push(bone.global_matrix * glam::Vec4::ONE);
            }

            let lines_tpose = lines.clone();
            let joints_tpose = joints.clone();

            LinesJoints {
                vao,
                vbo,
                shader: 0,
                mvp_ref: 0,
                color_ref: 0,
                lines,
                joints,
                lines_tpose,
                joints_tpose,
            }
        }
    }

    #[inline(never)]
    pub fn render(
        &mut self,
        use_animation: bool,
        skl_bone: &[lol::skl::Bone],
        bones_transforms: &[glam::Mat4],
        projection_view_matrix: &glam::Mat4,
    ) {
        unsafe {
            let mut lines_ptr = self.lines.as_ptr();
            let mut joints_ptr = self.joints.as_ptr();

            if use_animation {
                let mut line_index: usize = 0;
                for i in 0..skl_bone.len() {
                    let parent_id = skl_bone[i].parent_id;
                    if parent_id != -1 {
                        self.lines[line_index] = bones_transforms[parent_id as usize]
                            * skl_bone[parent_id as usize].global_matrix
                            * glam::Vec4::ONE;
                        self.lines[line_index + 1] =
                            bones_transforms[i] * skl_bone[i].global_matrix * glam::Vec4::ONE;
                        line_index += 2;
                    }
                    self.joints[i] =
                        bones_transforms[i] * skl_bone[i].global_matrix * glam::Vec4::ONE;
                }
            } else {
                lines_ptr = self.lines_tpose.as_ptr();
                joints_ptr = self.joints_tpose.as_ptr();
            }

            gl::Disable(gl::DEPTH_TEST);
            gl::UseProgram(self.shader);
            gl::Uniform1i(self.color_ref, 0);
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

            gl::Uniform1i(self.color_ref, 1);

            gl::BindVertexArray(self.vao + 1);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo[1]);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (self.joints.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                joints_ptr as *const c_void,
            );
            gl::DrawArrays(gl::POINTS, 0, self.joints.len() as GLsizei);
            gl::BindVertexArray(0);
            gl::Enable(gl::DEPTH_TEST);
        }
    }

    #[inline(never)]
    pub fn set_shader_refs(&mut self, shader: GLuint, refs: &[GLint]) {
        self.shader = shader;
        self.mvp_ref = refs[0];
        self.color_ref = refs[1];
    }

    #[inline(never)]
    pub fn destroy(&self) {
        unsafe {
            gl::DeleteBuffers(2, self.vbo.as_ptr());
            gl::DeleteVertexArrays(2, &self.vao);
        }
    }
}
