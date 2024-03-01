use gl::types::{GLfloat, GLint, GLsizei, GLsizeiptr, GLuint};
use std::{mem, os::raw::c_void, ptr};

use crate::gls::{Shader, Texture};

pub struct Floor {
    shader: Shader,
    texture: Texture,
    vao: GLuint,
    bo: GLuint,
    mvp_ref: GLint,
}

impl Floor {
    pub fn new() -> Floor {
        let shader = Shader::create(
            include_str!("../../assets/floor/floor.vert"),
            include_str!("../../assets/floor/floor.frag"),
        );
        let refs = shader.get_refs(&["Diffuse", "MVP"]);

        let texture = Texture::load_texture(include_bytes!("../../assets/floor/floor.dds"));

        #[rustfmt::skip]
		let floor_vertices: [GLfloat; 30] = [
			 750.0f32, 0.0f32,  750.0f32, 0.0f32, 1.0f32,
			 750.0f32, 0.0f32, -750.0f32, 0.0f32, 0.0f32,
			-750.0f32, 0.0f32,  750.0f32, 1.0f32, 1.0f32,
			 750.0f32, 0.0f32, -750.0f32, 0.0f32, 0.0f32,
			-750.0f32, 0.0f32, -750.0f32, 1.0f32, 0.0f32,
			-750.0f32, 0.0f32,  750.0f32, 1.0f32, 1.0f32,
		];

        unsafe {
            shader.enable();
            gl::Uniform1i(refs[0], 0);

            let mut vao: GLuint = 0;
            let mut bo: GLuint = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut bo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, bo);
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

            gl::BindVertexArray(0);

            Floor {
                shader,
                texture,
                vao,
                bo,
                mvp_ref: refs[1],
            }
        }
    }

    pub fn render(&self, projection_view_matrix: &glam::Mat4) {
        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);

            self.shader.enable();
            gl::UniformMatrix4fv(
                self.mvp_ref,
                1,
                gl::FALSE,
                projection_view_matrix.as_ref() as *const GLfloat,
            );

            gl::ActiveTexture(gl::TEXTURE0);
            self.texture.bind();

            gl::BindVertexArray(self.vao);

            gl::DrawArrays(gl::TRIANGLES, 0, 6);

            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Floor {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.bo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
