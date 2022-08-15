use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};

use std::{mem, os::raw::c_void, path::Path, ptr};

use gls::{shader, texture};

pub struct Floor {
    shader: GLuint,
    texture: GLuint,
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
    mvp_ref: GLint,
}

impl Floor {
	#[inline(never)]
    pub fn new() -> Floor {
        let shader = shader::create_shader(
            Path::new("assets/floor.vert"),
            Path::new("assets/floor.frag"),
        );

        let texture = texture::load_dds_to_opengl(Path::new("assets/floor.dds"));

        let refs = shader::get_refs_shader(shader, &["Diffuse", "MVP"]);

        #[rustfmt::skip]
		let floor_vertices: [GLfloat; 20] = [
			 750.0f32, 0.0f32,  750.0f32, 0.0f32, 1.0f32,
			 750.0f32, 0.0f32, -750.0f32, 0.0f32, 0.0f32,
			-750.0f32, 0.0f32, -750.0f32, 1.0f32, 0.0f32,
			-750.0f32, 0.0f32,  750.0f32, 1.0f32, 1.0f32,
		];

        #[rustfmt::skip]
		let floor_indices: [GLint; 6] = [
			0, 1, 3,
			1, 2, 3
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
                (floor_vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                floor_vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            let stride = (5 * mem::size_of::<GLfloat>()) as GLsizei;
            let offset = (3 * mem::size_of::<GLfloat>()) as *const c_void;

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, offset);

            let mut ebo: GLuint = 0;
            gl::GenBuffers(1, &mut ebo);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (floor_indices.len() * mem::size_of::<GLint>()) as GLsizeiptr,
                floor_indices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindVertexArray(0);

            Floor {
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
    pub fn render(&self, projection_view_matrix: &glam::Mat4) {
        unsafe {
            gl::UseProgram(self.shader);
            gl::BindVertexArray(self.vao);
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
            gl::BindVertexArray(0);
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
