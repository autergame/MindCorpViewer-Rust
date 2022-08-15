use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint, GLushort};

use std::{mem, os::raw::c_void, ptr};

use config_json;

use lol;

pub struct Model {
    vao: GLuint,
    vbo: Vec<GLuint>,
    ubo: GLuint,
    ebo: Vec<GLuint>,
    shader: GLuint,
    mvp_ref: GLint,
    diffuse_ref: GLint,
    use_bone_ref: GLint,
}

impl Model {
	#[inline(never)]
    pub fn new(skin: &lol::skn::Skin, skl_bones_count: usize) -> Model {
        unsafe {
            let mut vao: GLuint = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo: Vec<GLuint> = vec![0; 4];
			gl::GenBuffers(4, vbo.as_mut_ptr());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[0]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.positions.len() * mem::size_of::<glam::Vec3>()) as GLsizeiptr,
                skin.positions.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.uvs.len() * mem::size_of::<glam::Vec2>()) as GLsizeiptr,
                skin.uvs.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[2]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_indices.len() * mem::size_of::<glam::UVec4>()) as GLsizeiptr,
                skin.bone_indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo[3]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skin.bone_weights.len() * mem::size_of::<glam::Vec4>()) as GLsizeiptr,
                skin.bone_weights.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());

            let mut ubo: GLuint = 0;
            gl::GenBuffers(1, &mut ubo);

            gl::BindBuffer(gl::UNIFORM_BUFFER, ubo);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                (skl_bones_count * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            let mut ebo: Vec<GLuint> = vec![0; skin.meshes.len()];
            gl::GenBuffers(skin.meshes.len() as GLsizei, ebo.as_mut_ptr());

            for i in 0..skin.meshes.len() {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo[i]);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (skin.meshes[i].indices.len() * mem::size_of::<GLushort>()) as GLsizeiptr,
                    skin.meshes[i].indices.as_ptr() as *const c_void,
                    gl::STATIC_DRAW,
                );
            }

            gl::BindVertexArray(0);

            Model {
                vao,
                vbo,
                ubo,
                ebo,
                shader: 0,
                mvp_ref: 0,
                diffuse_ref: 0,
                use_bone_ref: 0,
            }
		}
	}

	#[inline(never)]
    pub fn render(
        &self,
        config: &config_json::ConfigJson,
        projection_view_matrix: &glam::Mat4,
        show_mesh: &[bool],
        skns_meshes: &[lol::skn::Mesh],
        texture_used: &[GLint],
        bones_transforms: &[glam::Mat4],
    ) {
        unsafe {
            gl::UseProgram(self.shader);
            gl::BindVertexArray(self.vao);
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );
            if config.use_animation {
                gl::Uniform1i(self.use_bone_ref, 1);
                gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo);
                gl::BufferSubData(
                    gl::UNIFORM_BUFFER,
                    0,
                    (bones_transforms.len() * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
                    bones_transforms.as_ptr() as *const c_void,
                );
                gl::BindBufferRange(
                    gl::UNIFORM_BUFFER,
                    0,
                    self.ubo,
                    0,
                    (bones_transforms.len() * mem::size_of::<glam::Mat4>()) as GLsizeiptr,
                );
            } else {
                gl::Uniform1i(self.use_bone_ref, 0);
            }
            if config.show_wireframe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
            }
            for i in 0..skns_meshes.len() {
                if show_mesh[i] {
                    gl::Uniform1i(self.diffuse_ref, texture_used[i]);
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo[i]);
                    gl::DrawElements(
                        gl::TRIANGLES,
                        skns_meshes[i].indices.len() as GLsizei,
                        gl::UNSIGNED_SHORT,
                        ptr::null(),
                    );
                }
            }
            if config.show_wireframe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
            }
            gl::BindVertexArray(0);
        }
    }

	#[inline(never)]
    pub fn bind_ubo(
        &self,
        model_shader: GLuint,
        model_ubo_ref: GLuint,
        bones_transforms_size: usize,
    ) {
        unsafe {
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo);
            gl::UniformBlockBinding(model_shader, model_ubo_ref, 0);
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

	#[inline(never)]
    pub fn set_shader_refs(&mut self, shader: GLuint, refs: &[GLint]) {
        self.shader = shader;
        self.mvp_ref = refs[0];
        self.diffuse_ref = refs[1];
        self.use_bone_ref = refs[2];
    }

	#[inline(never)]
    pub fn destroy(&self) {
        unsafe {
            gl::DeleteBuffers(4, self.vbo.as_ptr());
            gl::DeleteBuffers(1, &self.ubo);
            gl::DeleteBuffers(self.ebo.len() as GLsizei, self.ebo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
