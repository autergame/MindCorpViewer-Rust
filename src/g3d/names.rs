use gl::types::{GLfloat, GLint, GLsizeiptr, GLuint};
use glam::{FloatExt, Vec4Swizzles};
use std::{mem, os::raw::c_void, ptr, rc::Rc};

use crate::{
    gls::{Shader, Texture},
    lol::Skeleton,
    MindModel,
};

pub struct Names {
    vao: GLuint,
    bo: Vec<GLuint>,
    shader: Rc<Shader>,
    texture: Texture,

    mvp_ref: GLint,

    text_size_ref: GLint,
    text_scale_ref: GLint,
    text_offset_ref: GLint,
    text_offset_size_ref: GLint,
    text_position_ref: GLint,

    camera_up_ref: GLint,
    camera_right_ref: GLint,

    texts: Vec<Text>,
    texts_tpose: *const Text,
}

pub struct Text {
    size: glam::Vec2,
    offset: glam::Vec2,
    offset_size: glam::Vec2,
    position: glam::Vec3,
}

impl Names {
    #[rustfmt::skip]
    pub fn create(skl: &Skeleton, shader: Rc<Shader>) -> Names {
		let text_vertices: [GLfloat; 12] = [
			0.0f32, 1.0f32,
			0.0f32, 0.0f32,
			1.0f32, 0.0f32,
			0.0f32, 1.0f32,
			1.0f32, 0.0f32,
			1.0f32, 1.0f32,
		];

		let text_texture_ids: [GLint; 6] = [
			0, 2, 3, 0, 3, 1
		];

        let font = Rc::new(Vec::from(include_bytes!("../../assets/fonts/dejavusans.ttf")));

		let library = freetype::Library::init().expect("Could not init freetype library");

        let face = library
            .new_memory_face(font, 0)
            .expect("Could not load font");

        face.set_pixel_sizes(0, 48)
            .expect("Could not set pixel size");

        let glyph = face.glyph();

		let font_ascent = face.size_metrics().unwrap().ascender >> 6;

		let max_width = 1024;

		let mut size = glam::ivec2(0, 0);
		let mut texture_size = glam::ivec2(0, 0);

		for joint in skl.joints.iter() {
			for char in joint.name.chars() {
				face.load_char(char as usize, freetype::face::LoadFlag::DEFAULT)
					.expect("Could not load char");

				let bitmap = glyph.bitmap();
				let metrics = glyph.metrics();

				let width = bitmap.width() + (metrics.horiBearingX >> 6);
				let height = bitmap.rows() + (font_ascent - (metrics.horiBearingY >> 6));

				if size.x + width >= max_width {
					break;
				}

				size.x += width;
				size.y = size.y.max(height);
			}

			texture_size.x = texture_size.x.max(size.x);
			texture_size.y += size.y;
			size.x = 0;
			size.y = 0;
		}

		let mut texts: Vec<Text> = Vec::with_capacity(skl.joints.len());

		let texture = unsafe {
			let mut texture_id: GLuint = 0;
			gl::GenTextures(1, &mut texture_id);
			gl::BindTexture(gl::TEXTURE_2D, texture_id);
			gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::R8 as GLint,
				texture_size.x,
				texture_size.y,
				0,
				gl::RED,
				gl::UNSIGNED_BYTE,
				vec![0u8; (texture_size.x * texture_size.y) as usize].as_ptr() as *const c_void,
			);

			let mut y_max = 0;
			let mut x_offset = 0;
			let mut y_offset = 0;

			for joint in skl.joints.iter() {
				let offset = glam::vec2(
					x_offset as f32 / texture_size.x as f32,
					(y_offset + 2) as f32 / texture_size.y as f32
				);

				for char in joint.name.chars() {
					face.load_char(char as usize, freetype::face::LoadFlag::DEFAULT)
						.expect("Could not load char");

					glyph.render_glyph(freetype::RenderMode::Normal)
						.expect("Could not render glyph");

					let bitmap = glyph.bitmap();
					let metrics = glyph.metrics();

					let x_gap = metrics.horiBearingX >> 6;
					let y_gap = font_ascent - (metrics.horiBearingY >> 6);

					let x_size = bitmap.width() + x_gap;
					let y_size = bitmap.rows() + y_gap;

					if x_offset + x_size >= max_width {
						break;
					}

					x_offset += x_gap;
					let y_offset_gap = y_offset + y_gap;

					gl::TexSubImage2D(
						gl::TEXTURE_2D,
						0,
						x_offset,
						y_offset_gap,
						bitmap.width(),
						bitmap.rows(),
						gl::RED,
						gl::UNSIGNED_BYTE,
						bitmap.buffer().as_ptr() as *const c_void,
					);

					x_offset += bitmap.width();
					y_max = y_max.max(y_size);
				}

				let size = glam::vec2(x_offset as f32, y_max as f32);
				let offset_size = glam::vec2(
					size.x / texture_size.x as f32,
					size.y / texture_size.y as f32
				);
				let position = (joint.global_matrix * glam::Vec4::ONE).xyz();

				texts.push(Text {
					size,
					offset,
					offset_size,
					position,
				});

				y_offset += y_max;
				x_offset = 0;
				y_max = 0;
			}

			gl::BindTexture(gl::TEXTURE_2D, 0);

			Texture {
				id: texture_id,
				gltype: gl::TEXTURE_2D,
			}
		};

		let texts_tpose = texts.as_ptr();

		unsafe {
			let mut vao: GLuint = 0;
            let mut bo: Vec<GLuint> = vec![0; 2];

			gl::GenVertexArrays(1, &mut vao);
			gl::GenBuffers(2, bo.as_mut_ptr());

			gl::BindVertexArray(vao);

			gl::BindBuffer(gl::ARRAY_BUFFER, bo[0]);
			gl::BufferData(
				gl::ARRAY_BUFFER,
				(text_vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
				text_vertices.as_ptr() as *const c_void,
				gl::STATIC_DRAW,
			);

            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            gl::BindBuffer(gl::ARRAY_BUFFER, bo[1]);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (text_texture_ids.len() * mem::size_of::<GLint>()) as GLsizeiptr,
                text_texture_ids.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 1, gl::FLOAT, gl::FALSE, 0, ptr::null());

			gl::BindVertexArray(0);

			Names {
				vao,
				bo,
				shader,
				texture,

				mvp_ref: 0,

				text_size_ref: 0,
				text_scale_ref: 0,
				text_offset_ref: 0,
				text_offset_size_ref: 0,
				text_position_ref: 0,

				camera_up_ref: 0,
				camera_right_ref: 0,

				texts,
				texts_tpose,
			}
		}
    }

	#[rustfmt::skip]
    pub fn render(
        &mut self,
		use_animation: bool,
		camera_pos: &glam::Vec3,
        view_matrix: &glam::Mat4,
        projection_view_matrix: &glam::Mat4,
		mind_model: &MindModel,
    ) {
        let texts_ptr = if use_animation {
            for i in 0..mind_model.skeleton.joints.len() {
                self.texts[i].position = (mind_model.joints_transforms[i]
                    * mind_model.skeleton.joints[i].global_matrix
                    * glam::Vec4::ONE).xyz();
            }
            self.texts.as_ptr()
        } else {
            self.texts_tpose
        };
		let texts = unsafe { std::slice::from_raw_parts(texts_ptr, self.texts.len()) };

		let camera_up = view_matrix.row(1).xyz();
		let camera_right = view_matrix.row(0).xyz();

		unsafe {
			gl::Enable(gl::BLEND);
			gl::Disable(gl::DEPTH_TEST);

			self.shader.as_ref().enable();

			gl::ActiveTexture(gl::TEXTURE0);
			self.texture.bind();

			gl::BindVertexArray(self.vao);

			gl::Uniform3fv(self.camera_up_ref, 1, camera_up.as_ref() as *const GLfloat);
			gl::Uniform3fv(self.camera_right_ref, 1, camera_right.as_ref() as *const GLfloat);

			gl::UniformMatrix4fv(
				self.mvp_ref,
				1,
				gl::FALSE,
				projection_view_matrix.as_ref() as *const GLfloat,
			);
		}

		for text in texts {
			unsafe {
				let distance = camera_pos.distance(text.position);
				let text_scale = distance.clamp(1.0, 500.0).remap(1.0, 500.0, 0.005, 0.15);

				gl::Uniform1f(self.text_scale_ref, text_scale);
				gl::Uniform2fv(self.text_size_ref, 1, text.size.as_ref() as *const GLfloat);
				gl::Uniform2fv(self.text_offset_ref, 1, text.offset.as_ref() as *const GLfloat);
				gl::Uniform2fv(self.text_offset_size_ref, 1, text.offset_size.as_ref() as *const GLfloat);
				gl::Uniform3fv(self.text_position_ref, 1, text.position.as_ref() as *const GLfloat);

				gl::DrawArrays(gl::TRIANGLES, 0, 6);
			}
		}

		unsafe {
			gl::BindVertexArray(0);

			gl::Disable(gl::BLEND);
		}
    }

    pub fn set_shader_refs(&mut self, refs: &[GLint]) {
        self.mvp_ref = refs[0];
        self.text_size_ref = refs[1];
        self.text_scale_ref = refs[2];
        self.text_offset_ref = refs[3];
        self.text_offset_size_ref = refs[4];
        self.text_position_ref = refs[5];
        self.camera_up_ref = refs[6];
        self.camera_right_ref = refs[7];
        let text_texture = refs[8];

        let shader = self.shader.as_ref();
        unsafe {
            shader.enable();
            gl::Uniform1i(text_texture, 0);
        }
    }
}

impl Drop for Names {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(2, self.bo.as_ptr());
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
