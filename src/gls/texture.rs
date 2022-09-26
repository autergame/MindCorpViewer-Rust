use byteorder::{LittleEndian, ReadBytesExt};
use gl::types::{GLenum, GLint, GLubyte, GLuint};
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    os::raw::c_void,
};

pub struct Texture {
    pub id: GLuint,
    gltype: GLenum,
}

impl Texture {
    fn load_dds(source: &[u8]) -> (Vec<u8>, u32, i32, i32, i32) {
        let mut reader = Cursor::new(source);

        let mut signature = vec![0u8; 4];
        reader
            .read_exact(&mut signature)
            .expect("Could not read DDS signature");
        if signature != b"DDS "[..] {
            panic!("DDS has no valid signature");
        }

        reader.seek(SeekFrom::Current(8)).unwrap();

        let height = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS height");
        let width = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS width");

        reader.seek(SeekFrom::Current(8)).unwrap();

        let mipmap_count = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS mipmap count");

        reader.seek(SeekFrom::Current(52)).unwrap();

        let mut ddspf_fourcc = vec![0u8; 4];
        reader
            .read_exact(&mut ddspf_fourcc)
            .expect("Could not read DDS pixel format fourcc");

        let format: u32;
        if ddspf_fourcc == b"DXT5"[..] {
            format = 0x83F3;
        } else if ddspf_fourcc == b"DXT3"[..] {
            format = 0x83F2;
        } else if ddspf_fourcc == b"DXT1"[..] {
            format = 0x83F1;
        } else {
            panic!("Unknown DDS pixel format fourcc");
        }

        reader.seek(SeekFrom::Start(128)).unwrap();

        let mut image_data = vec![];
        reader
            .read_to_end(&mut image_data)
            .expect("Could not read DDS image data");

        (image_data, format, height, width, mipmap_count)
    }

	#[rustfmt::skip]
    pub fn load_texture(source: &[u8]) -> Texture {
		let (image_data, format, mut width, mut height, mipmap_count) = Self::load_dds(source);

		unsafe {
			let mut texture_id: GLuint = 0;
			gl::GenTextures(1, &mut texture_id);
			gl::BindTexture(gl::TEXTURE_2D, texture_id);
			gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_BASE_LEVEL, 0);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAX_LEVEL, 1i32.max(mipmap_count - 1i32));
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

			let mut offset = 0i32;
			let block_size = if format == 0x83F1 { 8i32 } else { 16i32 };

			for i in 0..1i32.max(mipmap_count) {
				let size = 1i32.max(((width + 3) / 4) * ((height + 3) / 4)) * block_size;
				let data = &image_data[offset as usize] as *const GLubyte as *const c_void;

				gl::CompressedTexImage2D(gl::TEXTURE_2D, i, format, width, height, 0, size, data);

				offset += size;
				width /= 2;
				height /= 2;
			}

			if 1i32.max(mipmap_count) == 1i32 {
				gl::GenerateMipmap(gl::TEXTURE_2D);
			}

			Texture { id: texture_id, gltype: gl::TEXTURE_2D }
		}
	}

	#[rustfmt::skip]
    pub fn load_cubemap(source: &[&[u8]; 6]) -> Texture {
		unsafe {
			let mut texture_id: GLuint = 0;
			gl::GenTextures(1, &mut texture_id);
			gl::BindTexture(gl::TEXTURE_CUBE_MAP, texture_id);
			gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as GLint);

			for i in 0..6u32 {
				let (image_data, format, mut width, mut height, mipmap_count) =
					Self::load_dds(source[i as usize]);

				let mut offset = 0i32;
				let block_size = if format == 0x83F1 { 8i32 } else { 16i32 };

				for level in 0..1i32.max(mipmap_count) {
					let size = 1i32.max(((width + 3) / 4) * ((height + 3) / 4)) * block_size;
					let data = &image_data[offset as usize] as *const GLubyte as *const c_void;

					gl::CompressedTexImage2D(
						gl::TEXTURE_CUBE_MAP_POSITIVE_X + i,
						level,
						format,
						width,
						height,
						0,
						size,
						data,
					);

					offset += size;
					width /= 2;
					height /= 2;
				}
			}

			Texture { id: texture_id, gltype: gl::TEXTURE_CUBE_MAP }
		}
	}

    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(self.gltype, self.id);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}
