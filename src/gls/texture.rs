use gl::types::{GLint, GLubyte, GLuint};
use std::{
    fs::File,
    io::{Cursor, Read},
    os::raw::c_void,
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Copy, Clone)]
pub struct Texture {
    texture: GLuint,
}

impl Texture {
    fn load_dds(path: &Path) -> (Vec<u8>, u32, i32, i32, i32) {
        let mut file = File::open(path).expect("Could not open DDS file");
        let mut contents: Vec<u8> = Vec::new();
        println!("Reading DDS file: {}", path.to_str().unwrap());
        file.read_to_end(&mut contents)
            .expect("Could not read DDS file");
        println!("Finished reading DDS file");

        let mut reader = Cursor::new(contents);

        let mut signature = vec![0u8; 4];
        reader
            .read_exact(&mut signature)
            .expect("Could not read DDS signature");
        let signature = String::from_utf8(signature).expect("Invalid UTF-8 sequence");

        if signature != "DDS " {
            panic!("Dds has no valid signature");
        }

        reader.set_position(12);

        let height = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS height");
        let width = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS width");

        reader.set_position(28);

        let mipmap_count = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read DDS mipmap count");

        reader.set_position(84);

        let mut ddspf_fourcc = vec![0u8; 4];
        reader
            .read_exact(&mut ddspf_fourcc)
            .expect("Could not read DDS pixel format fourcc");
        let ddspf_fourcc = String::from_utf8(ddspf_fourcc).expect("Invalid UTF-8 sequence");

        let format: u32;
        if ddspf_fourcc == "DXT5" {
            format = 0x83F3;
        } else if ddspf_fourcc == "DXT3" {
            format = 0x83F2;
        } else {
            format = 0x83F1;
        }

        reader.set_position(128);

        let mut image_data = Vec::new();
        reader
            .read_to_end(&mut image_data)
            .expect("Could not read DDS image data");

        (image_data, format, height, width, mipmap_count)
    }

	#[rustfmt::skip]
    pub fn load_texture(path: &Path) -> Texture {
		let (image_data, format, mut width, mut height, mipmap_count) = Self::load_dds(path);

		unsafe {
			let mut texture: GLuint = 0;
			gl::GenTextures(1, &mut texture);
			gl::BindTexture(gl::TEXTURE_2D, texture);
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

			gl::ActiveTexture(gl::TEXTURE0 + texture);

			Texture { texture: texture - 1 }
		}
	}

	#[rustfmt::skip]
    pub fn load_cubemap(path: &[&Path; 6]) -> Texture {
		unsafe {
			let mut texture: GLuint = 0;
			gl::GenTextures(1, &mut texture);
			gl::BindTexture(gl::TEXTURE_CUBE_MAP, texture);
			gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as GLint);

			for i in 0..6u32 {
				let (image_data, format, mut width, mut height, mipmap_count) =
					Self::load_dds(path[i as usize]);

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

			gl::ActiveTexture(gl::TEXTURE0 + texture);

			Texture { texture: texture - 1 }
		}
	}

    pub fn set_in_shader_ref(&self, texture_ref: GLint) {
        unsafe {
            gl::Uniform1i(texture_ref, self.texture as i32);
        }
    }

    pub fn unslot(&self) -> GLuint {
        self.texture + 1
    }

    pub fn destroy(&self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}
