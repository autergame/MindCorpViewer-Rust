use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

use gls::glam_read;

use lol::hasher;

pub struct Bone {
    pub name: String,
    pub hash: u32,
    pub id: i16,
    pub parent_id: i32,
    pub local_matrix: glam::Mat4,
    pub global_matrix: glam::Mat4,
    pub inverse_global_matrix: glam::Mat4,
    pub children: Vec<usize>,
}

pub enum Type {
    Classic = 0x746C6B73,
    Version2 = 0x22FD4FC3,
}

impl Type {
    fn from_u32(value: u32) -> Type {
        match value {
            0x746C6B73 => Type::Classic,
            0x22FD4FC3 => Type::Version2,
            _ => panic!("Unknown SKL version"),
        }
    }
}

pub struct Skeleton {
    pub stype: Type,
    pub version: u32,
    pub bones: Vec<Bone>,
    pub bone_indices: Vec<u32>,
}

impl Skeleton {
    pub fn read(contents: &Vec<u8>) -> Skeleton {
        let mut reader = Cursor::new(contents);

        reader.set_position(reader.position() + 4);

        let stype = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL type");
        let version = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL version");

        let mut skeleton = match Type::from_u32(stype) {
            Type::Classic => Self::read_classic(&mut reader, version),
            Type::Version2 => Self::read_version2(&mut reader, version),
        };

        for i in 0..skeleton.bones.len() {
            let parent_id = skeleton.bones[i].parent_id;
            if parent_id >= 0 {
                if let Some(parent) = skeleton.bones.get_mut(parent_id as usize) {
                    parent.children.push(i);
                }
            }
        }

        skeleton
    }

    fn read_classic(reader: &mut Cursor<&Vec<u8>>, version: u32) -> Skeleton {
        reader.set_position(reader.position() + 4);

        let bone_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL bone count");

        let mut bones: Vec<Bone> = Vec::with_capacity(bone_count as usize);
        for i in 0..bone_count {
            let mut string = vec![0u8; 32];
            reader
                .read_exact(&mut string)
                .expect("Could not read SKL bone name");
            let name = String::from_utf8(string)
                .expect("Invalid UTF-8 sequence")
                .trim_end_matches('\0')
                .to_string();
            let hash = hasher::string_to_hash(&name);

            let parent_id = reader
                .read_i32::<LittleEndian>()
                .expect("Could not read SKL bone parent id");

            reader.set_position(reader.position() + 4);

            let mut global_matrix = glam::Mat4::IDENTITY;
            for y in 0..3 {
                for x in 0..4 {
                    global_matrix.col_mut(x)[y] = reader
                        .read_f32::<LittleEndian>()
                        .expect("Could not read SKL bone global matrix");
                }
            }

            global_matrix.w_axis.w = 1.0f32;

            let inverse_global_matrix = global_matrix.inverse();

            bones.push(Bone {
                name,
                hash,
                id: i as i16,
                parent_id,
                local_matrix: glam::Mat4::IDENTITY,
                global_matrix,
                inverse_global_matrix,
                children: Vec::new(),
            });
        }

        let mut bone_indices: Vec<u32> = Vec::new();

        match version {
            0..=1 => {
                bone_indices = Vec::with_capacity(bone_count as usize);
                for i in 0..bone_count {
                    bone_indices.push(i as u32);
                }
            }
            2 => {
                let bone_index_count = reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read SKL bone index count");

                bone_indices = Vec::with_capacity(bone_index_count as usize);
                for _ in 0..bone_index_count {
                    bone_indices.push(
                        reader
                            .read_u32::<LittleEndian>()
                            .expect("Could not read SKL bone index"),
                    );
                }
            }
            _ => {}
        }

        for i in 0..bone_count as usize {
            if bones[i].parent_id < 0 {
                bones[i].local_matrix = bones[i].global_matrix;
            } else {
                let parent = &bones[bones[i].parent_id as usize];
                bones[i].local_matrix = bones[i].global_matrix * parent.inverse_global_matrix;
            }
        }

        print!("SKL version {version} was succesfully loaded: ");
        print!("Type: Classic ");
        print!("Bones count: {} ", bones.len());
        println!("Bones indices count: {}", bone_indices.len());

        Skeleton {
            stype: Type::Classic,
            version,
            bones,
            bone_indices,
        }
    }

    fn read_version2(reader: &mut Cursor<&Vec<u8>>, version: u32) -> Skeleton {
        reader.set_position(reader.position() + 2);

        let bone_count = reader
            .read_u16::<LittleEndian>()
            .expect("Could not read SKL bone count");
        let bone_index_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL bone index count");
        let bone_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL bone offset");

        reader.set_position(reader.position() + 4);

        let bone_index_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKL bone index offset");

        reader.set_position(bone_offset as u64);

        let mut bones: Vec<Bone> = Vec::with_capacity(bone_count as usize);
        for _ in 0..bone_count {
            reader.set_position(reader.position() + 2);

            let id = reader
                .read_i16::<LittleEndian>()
                .expect("Could not read SKL bone id");

            let parent_id = reader
                .read_i16::<LittleEndian>()
                .expect("Could not read SKL bone parent id");

            reader.set_position(reader.position() + 2);

            let hash = reader
                .read_u32::<LittleEndian>()
                .expect("Could not read SKL bone hash");

            reader.set_position(reader.position() + 4);

            let position = glam_read::vec3_f32::<LittleEndian>(reader);
            let scale = glam_read::vec3_f32::<LittleEndian>(reader);
            let rotation = glam_read::quat_f32::<LittleEndian>(reader);

            let local_matrix =
                glam::Mat4::from_scale_rotation_translation(scale, rotation, position);

            let inserve_position = glam_read::vec3_f32::<LittleEndian>(reader);
            let inserve_scale = glam_read::vec3_f32::<LittleEndian>(reader);
            let inserve_rotation = glam_read::quat_f32::<LittleEndian>(reader);

            let inverse_global_matrix = glam::Mat4::from_scale_rotation_translation(
                inserve_scale,
                inserve_rotation,
                inserve_position,
            );

            let global_matrix = inverse_global_matrix.inverse();

            let name_offset = reader
                .read_i32::<LittleEndian>()
                .expect("Could not read SKL bone name offset");
            let return_offset = reader.position();

            reader.set_position(return_offset - 4 + name_offset as u64);

            let mut string: Vec<u8> = Vec::new();
            loop {
                let byte = reader.read_u8().expect("Could not read SKL bone name");
                if byte == 0 {
                    break;
                }
                string.push(byte);
            }
            let name = String::from_utf8(string).expect("Invalid UTF-8 sequence");

            reader.set_position(return_offset as u64);

            bones.push(Bone {
                name,
                hash,
                id,
                parent_id: parent_id as i32,
                local_matrix,
                global_matrix,
                inverse_global_matrix,
                children: Vec::new(),
            });
        }

        reader.set_position(bone_index_offset as u64);

        let mut bone_indices = Vec::with_capacity(bone_index_count as usize);
        for _ in 0..bone_index_count {
            bone_indices.push(
                reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read SKL bone index") as u32,
            );
        }

        print!("SKL version {version} was succesfully loaded: ");
        print!("Type: Version2 ");
        print!("Bones count: {} ", bones.len());
        println!("Bones indices count: {}", bone_indices.len());

        Skeleton {
            stype: Type::Version2,
            version,
            bones,
            bone_indices,
        }
    }
}
