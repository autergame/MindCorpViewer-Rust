use byteorder::{LittleEndian, ReadBytesExt};
use gl::types::{GLenum, GLint, GLuint};
use std::{
    io::{Cursor, Read},
    os::raw::c_void,
};

pub struct Texture {
    pub id: GLuint,
    pub gltype: GLenum,
}

impl Texture {
    #[rustfmt::skip]
    pub fn load_texture(source: &[u8]) -> Texture {
		let (images, mut width, mut height) = load_source(&mut Cursor::new(source));

		unsafe {
			let mut texture_id: GLuint = 0;
			gl::GenTextures(1, &mut texture_id);
			gl::BindTexture(gl::TEXTURE_2D, texture_id);

			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_BASE_LEVEL, 0);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAX_LEVEL, 1.max(images.len() - 1) as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

			for i in 0..images.len() {
				let data = images[i].as_ptr() as *const c_void;

				gl::TexImage2D(
					gl::TEXTURE_2D,
					i as GLint,
					gl::RGBA8 as GLint,
					width,
					height,
					0,
					gl::RGBA,
					gl::UNSIGNED_BYTE,
					data
				);

				width /= 2;
				height /= 2;
			}

			if 1.max(images.len()) == 1 {
				gl::GenerateMipmap(gl::TEXTURE_2D);
			}

			gl::BindTexture(gl::TEXTURE_2D, 0);

			Texture { id: texture_id, gltype: gl::TEXTURE_2D }
		}
	}

	#[rustfmt::skip]
    pub fn load_cubemap(source: &[&[u8]; 6]) -> Texture {
		unsafe {
			let mut texture_id: GLuint = 0;
			gl::GenTextures(1, &mut texture_id);
			gl::BindTexture(gl::TEXTURE_CUBE_MAP, texture_id);

			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
			gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as GLint);

			for i in 0..6u32 {
				let (images, mut width, mut height) = load_source(&mut Cursor::new(source[i as usize]));

				for level in 0..images.len() {
					let data = images[level].as_ptr() as *const c_void;

                    gl::TexImage2D(
                        gl::TEXTURE_CUBE_MAP_POSITIVE_X + i,
                        level as GLint,
                        gl::RGBA8 as GLint,
                        width,
                        height,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        data,
                    );

					width /= 2;
					height /= 2;
				}
			}

			gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);

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

#[derive(Debug)]
enum Format {
    BC1DXT1,
    BC2DXT3,
    BC3DXT5,
    ETC1,
    ETC2EAC,
    RGBA8,
}

impl Format {
    fn bytes_per_block(&self) -> usize {
        match self {
            Format::BC1DXT1 => 8,
            Format::BC2DXT3 => 16,
            Format::BC3DXT5 => 16,
            Format::ETC1 => 8,
            Format::ETC2EAC => 16,
            Format::RGBA8 => 4,
        }
    }

    fn decode_function(&self) -> fn(&[u8], usize, usize, &mut [u32]) -> Result<(), &'static str> {
        match self {
            Format::BC1DXT1 => texture2ddecoder::decode_bc1,
            Format::BC2DXT3 => texture2ddecoder::decode_bc2,
            Format::BC3DXT5 => texture2ddecoder::decode_bc3,
            Format::ETC1 => texture2ddecoder::decode_etc1,
            Format::ETC2EAC => texture2ddecoder::decode_etc2_rgba8,
            Format::RGBA8 => unreachable!(),
        }
    }
}

fn decode_by_format(
    image_data: &[u8],
    format: &Format,
    width: i32,
    height: i32,
    mipmap_count: usize,
) -> Vec<Vec<u8>> {
    let mut offset = 0;
    let mut current_width = width as usize;
    let mut current_height = height as usize;

    let bytes_per_block = format.bytes_per_block();
    let decode_function = format.decode_function();

    let mut images = Vec::with_capacity(mipmap_count);

    for _ in 0..mipmap_count {
        let image_width = 1.max((current_width + 3) / 4);
        let image_height = 1.max((current_height + 3) / 4);

        let block_size = image_width * image_height * bytes_per_block;
        let image = &image_data[offset..offset + block_size];

        let image_size = current_width * current_height;
        let mut image_out = vec![0; image_size];

        decode_function(
            image,
            current_width,
            current_height,
            image_out.as_mut_slice(),
        )
        .unwrap_or_else(|err| panic!("Could not decode {:?} image data: {}", format, err));

        let image_converted = image_out
            .iter()
            .flat_map(|x| {
                let v = x.to_le_bytes();
                [v[2], v[1], v[0], v[3]]
            })
            .collect::<Vec<u8>>();

        images.push(image_converted);

        offset += block_size;
        current_width /= 2;
        current_height /= 2;
    }

    images
}

fn load_dds(reader: &mut Cursor<&[u8]>) -> (Vec<u8>, Format, i32, i32, usize) {
    reader.set_position(reader.position() + 8);

    let height = reader
        .read_i32::<LittleEndian>()
        .expect("Could not read DDS height");
    let width = reader
        .read_i32::<LittleEndian>()
        .expect("Could not read DDS width");

    reader.set_position(reader.position() + 8);

    let mipmap_count = reader
        .read_i32::<LittleEndian>()
        .expect("Could not read DDS mipmap count");
    let mipmap_count = 1.max(mipmap_count) as usize;

    reader.set_position(reader.position() + 52);

    let mut ddspf_fourcc = vec![0u8; 4];
    reader
        .read_exact(&mut ddspf_fourcc)
        .expect("Could not read DDS pixel format fourcc");

    let format = match ddspf_fourcc.as_slice() {
        b"DXT1" => Format::BC1DXT1,
        b"DXT3" => Format::BC2DXT3,
        b"DXT5" => Format::BC3DXT5,
        _ => panic!("Unknown DDS pixel format fourcc"),
    };

    reader.set_position(128);

    let mut image_data = vec![];
    reader
        .read_to_end(&mut image_data)
        .expect("Could not read DDS image data");

    (image_data, format, height, width, mipmap_count)
}

fn load_tex(reader: &mut Cursor<&[u8]>) -> (Vec<u8>, Format, i32, i32, usize) {
    let width = reader
        .read_u16::<LittleEndian>()
        .expect("Could not read TEX width") as i32;
    let height = reader
        .read_u16::<LittleEndian>()
        .expect("Could not read TEX height") as i32;

    reader.set_position(reader.position() + 1);

    let tex_format = reader.read_u8().expect("Could not read TEX format");

    let format = match tex_format {
        1 => Format::ETC1,
        2 => Format::ETC2EAC,
        10 | 11 => Format::BC1DXT1,
        12 => Format::BC3DXT5,
        20 => Format::RGBA8,
        _ => panic!("Unknown TEX format"),
    };

    reader.set_position(reader.position() + 1);

    let has_mipmap = reader.read_u8().expect("Could not read TEX mipmap count") != 0;

    let mipmap_count = if has_mipmap {
        32.min((height.max(width) as f32).log2().floor() as usize + 1)
    } else {
        1
    };

    let mut image_data = vec![];
    reader
        .read_to_end(&mut image_data)
        .expect("Could not read TEX image data");

    let mut offset = image_data.len();
    let mut current_width = width as usize;
    let mut current_height = height as usize;

    let bytes_per_block = format.bytes_per_block();

    let mut image_data_reversed = vec![];

    for _ in 0..mipmap_count {
        let image_width = 1.max((current_width + 3) / 4);
        let image_height = 1.max((current_height + 3) / 4);

        let block_size = image_width * image_height * bytes_per_block;
        let image = &image_data[offset - block_size..offset];

        image_data_reversed.extend_from_slice(image);

        offset -= block_size;
        current_width /= 2;
        current_height /= 2;
    }

    (image_data_reversed, format, height, width, mipmap_count)
}

pub fn load_source(reader: &mut Cursor<&[u8]>) -> (Vec<Vec<u8>>, i32, i32) {
    let mut signature = vec![0u8; 4];
    reader
        .read_exact(&mut signature)
        .expect("Could not read texture signature");

    let (image_data, format, width, height, mipmap_count) = match signature.as_slice() {
        b"DDS " => load_dds(reader),
        b"TEX\0" => load_tex(reader),
        _ => panic!("Unknown texture signature"),
    };

    let images = match format {
        Format::RGBA8 => vec![image_data],
        _ => decode_by_format(&image_data, &format, width, height, mipmap_count),
    };

    println!(
        "Texture {:?} mipmaps {} {}x{} was successfully loaded",
        format, mipmap_count, width, height
    );

    (images, width, height)
}
