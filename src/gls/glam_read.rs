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

pub fn vec4_u8(reader: &mut Cursor<&Vec<u8>>) -> U16Vec4 {
    U16Vec4::new(
        reader.read_u8().expect("Could not read vec4 x") as u16,
        reader.read_u8().expect("Could not read vec4 y") as u16,
        reader.read_u8().expect("Could not read vec4 z") as u16,
        reader.read_u8().expect("Could not read vec4 w") as u16,
    )
}

#[derive(Clone, Copy)]
pub struct U16Vec4 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub w: u16,
}

impl U16Vec4 {
    #[inline(always)]
    pub const fn new(x: u16, y: u16, z: u16, w: u16) -> Self {
        Self { x, y, z, w }
    }
}

impl std::ops::Index<usize> for U16Vec4 {
    type Output = u16;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.w,
            _ => panic!("index out of bounds"),
        }
    }
}

impl std::ops::IndexMut<usize> for U16Vec4 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.w,
            _ => panic!("index out of bounds"),
        }
    }
}
