use byteorder::{ByteOrder, ReadBytesExt};
use std::io::Cursor;

pub fn vec2_f32<T: ByteOrder>(reader: &mut Cursor<&Vec<u8>>) -> glam::Vec2 {
    glam::vec2(
        reader.read_f32::<T>().expect("Could not read vec2 x"),
        reader.read_f32::<T>().expect("Could not read vec2 y"),
    )
}

pub fn vec3_f32<T: ByteOrder>(reader: &mut Cursor<&Vec<u8>>) -> glam::Vec3 {
    glam::vec3(
        reader.read_f32::<T>().expect("Could not read vec3 x"),
        reader.read_f32::<T>().expect("Could not read vec3 y"),
        reader.read_f32::<T>().expect("Could not read vec3 z"),
    )
}

pub fn vec4_f32<T: ByteOrder>(reader: &mut Cursor<&Vec<u8>>) -> glam::Vec4 {
    glam::vec4(
        reader.read_f32::<T>().expect("Could not read vec4 x"),
        reader.read_f32::<T>().expect("Could not read vec4 y"),
        reader.read_f32::<T>().expect("Could not read vec4 z"),
        reader.read_f32::<T>().expect("Could not read vec4 w"),
    )
}

pub fn quat_f32<T: ByteOrder>(reader: &mut Cursor<&Vec<u8>>) -> glam::Quat {
    glam::quat(
        reader.read_f32::<T>().expect("Could not read quat x"),
        reader.read_f32::<T>().expect("Could not read quat y"),
        reader.read_f32::<T>().expect("Could not read quat z"),
        reader.read_f32::<T>().expect("Could not read quat w"),
    )
}

pub fn uvec4_u8(reader: &mut Cursor<&Vec<u8>>) -> glam::UVec4 {
    glam::uvec4(
        reader.read_u8().expect("Could not read vec4 x") as u32,
        reader.read_u8().expect("Could not read vec4 y") as u32,
        reader.read_u8().expect("Could not read vec4 z") as u32,
        reader.read_u8().expect("Could not read vec4 w") as u32,
    )
}
