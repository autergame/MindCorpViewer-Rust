use gl::types::{GLfloat, GLint, GLsizeiptr, GLuint};

use std::{mem, os::raw::c_void, path::Path, ptr};

use gls::{shader, texture};

pub struct Skybox {
    shader: GLuint,
    texture: GLuint,
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
    mvp_ref: GLint,
}

impl Skybox {
	#[inline(never)]
    pub fn new() -> Skybox {
        let shader = shader::create_shader(
            Path::new("assets/skybox.vert"),
            Path::new("assets/skybox.frag"),
        );

        let texture = texture::load_cubemap(&[
            Path::new("assets/skybox/right.dds"),
            Path::new("assets/skybox/left.dds"),
            Path::new("assets/skybox/top.dds"),
            Path::new("assets/skybox/bottom.dds"),
            Path::new("assets/skybox/front.dds"),
            Path::new("assets/skybox/back.dds"),
        ]);

        let refs = shader::get_refs_shader(shader, &["Skybox", "MVP"]);

        #[rustfmt::skip]
		let skybox_vertices: [GLfloat; 24] = [
			-1.0f32, -1.0f32,  1.0f32,
			 1.0f32, -1.0f32,  1.0f32,
			 1.0f32, -1.0f32, -1.0f32,
			-1.0f32, -1.0f32, -1.0f32,
			-1.0f32,  1.0f32,  1.0f32,
			 1.0f32,  1.0f32,  1.0f32,
			 1.0f32,  1.0f32, -1.0f32,
			-1.0f32,  1.0f32, -1.0f32,
		];

        #[rustfmt::skip]
		let skybox_indices: [GLint; 36] = [
			1, 2, 6,
			6, 5, 1,
			0, 4, 7,
			7, 3, 0,
			4, 5, 6,
			6, 7, 4,
			0, 3, 2,
			2, 1, 0,
			0, 1, 5,
			5, 4, 0,
			3, 7, 6,
			6, 2, 3,
		];

        unsafe {
            gl::UseProgram(shader);
            gl::Uniform1i(refs[0], texture as GLint - 1);

            let mut vao: GLuint = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            let mut vbo: GLuint = 0;
            gl::GenBuffers(1, &mut vbo);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (skybox_vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                skybox_vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());

            let mut ebo: GLuint = 0;
            gl::GenBuffers(1, &mut ebo);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (skybox_indices.len() * mem::size_of::<GLint>()) as GLsizeiptr,
                skybox_indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindVertexArray(0);

            Skybox {
                shader,
                texture,
                vao,
                vbo,
                ebo,
                mvp_ref: refs[1],
            }
        }
    }

	#[inline(never)]
    pub fn render(&self, view_matrix: &glam::Mat4, projection_matrix: &glam::Mat4) {
        let skybox_view_matrix = glam::Mat4::from_mat3(glam::Mat3::from_mat4(*view_matrix));
        let skybox_projection_view_matrix = *projection_matrix * skybox_view_matrix;
        unsafe {
            gl::DepthFunc(gl::LEQUAL);
            gl::UseProgram(self.shader);
            gl::BindVertexArray(self.vao);
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                skybox_projection_view_matrix.as_ref() as *const GLfloat,
            );
            gl::DrawElements(gl::TRIANGLES, 36, gl::UNSIGNED_INT, ptr::null());
            gl::BindVertexArray(0);
            gl::DepthFunc(gl::LESS);
        }
    }

	#[inline(never)]
    pub fn destroy(&self) {
        unsafe {
            gl::DeleteProgram(self.shader);
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
