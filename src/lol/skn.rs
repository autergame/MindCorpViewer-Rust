use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

use gls::glam_read;

use lol::{hasher, Skeleton};

pub struct SubMeshHeader {
    pub name: String,
    pub indices_offset: u32,
    pub indices_count: u32,
}

pub struct Mesh {
    pub hash: u32,
    pub submesh: SubMeshHeader,
}

impl Mesh {
    fn new(submesh: SubMeshHeader) -> Mesh {
        Mesh {
            hash: hasher::fnv1a(&submesh.name),
            submesh,
        }
    }
}

pub struct Skin {
    pub major: u16,
    pub minor: u16,
    pub center: glam::Vec3,
    pub bounding_box: [glam::Vec3; 2],
    pub vertices: Vec<glam::Vec3>,
    pub normals: Vec<glam::Vec3>,
    pub uvs: Vec<glam::Vec2>,
    pub bone_indices: Vec<glam_read::U16Vec4>,
    pub bone_weights: Vec<glam::Vec4>,
    pub indices: Vec<u16>,
    pub meshes: Vec<Mesh>,
}

impl Skin {
    pub fn read(contents: &Vec<u8>) -> Skin {
        let mut reader = Cursor::new(contents);

        let mut signature = vec![0u8; 4];
        reader
            .read_exact(&mut signature)
            .expect("Could not read SKN signature");

        if signature != [0x33, 0x22, 0x11, 0x00] {
            panic!("SKN has no valid signature");
        }

        let major = reader
            .read_u16::<LittleEndian>()
            .expect("Could not read SKN major version");
        let minor = reader
            .read_u16::<LittleEndian>()
            .expect("Could not read SKN minor version");

        let mut submeshheader_count = 0u32;
        let mut submeshheaders: Vec<SubMeshHeader> = vec![];

        if major > 0 {
            submeshheader_count = reader
                .read_u32::<LittleEndian>()
                .expect("Could not read SKN SubMeshHeader count");

            for _ in 0..submeshheader_count {
                let mut string = vec![0u8; 64];
                reader
                    .read_exact(&mut string)
                    .expect("Could not read SKN SubMeshHeader name");
                let name = String::from_utf8(string)
                    .expect("Invalid UTF-8 sequence")
                    .trim_end_matches('\0')
                    .to_owned();

                reader.set_position(reader.position() + 8);

                let indices_offset = reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read SKN SubMeshHeader indices offset");
                let indices_count = reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read SKN SubMeshHeader indices count");

                submeshheaders.push(SubMeshHeader {
                    name,
                    indices_offset,
                    indices_count,
                });
            }

            if major == 4 {
                reader.set_position(reader.position() + 4);
            }
        }

        let indices_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKN indices count");
        let vertex_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read SKN vertex count");

        let mut bbmin = glam::Vec3::splat(std::f32::MAX);
        let mut bbmax = glam::Vec3::splat(std::f32::MIN);

        let mut vertex_type = 0u32;

        if major == 4 {
            reader.set_position(reader.position() + 4);

            vertex_type = reader
                .read_u32::<LittleEndian>()
                .expect("Could not read SKN tangent count");

            bbmin = glam_read::vec3_f32::<LittleEndian>(&mut reader);
            bbmax = glam_read::vec3_f32::<LittleEndian>(&mut reader);

            reader.set_position(reader.position() + 16);
        }

        let mut indices: Vec<u16> = Vec::with_capacity(indices_count as usize);
        for _ in 0..indices_count {
            indices.push(
                reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read SKN indices"),
            );
        }

        let mut vertices: Vec<glam::Vec3> = Vec::with_capacity(vertex_count as usize);
        let mut normals: Vec<glam::Vec3> = Vec::with_capacity(vertex_count as usize);
        let mut uvs: Vec<glam::Vec2> = Vec::with_capacity(vertex_count as usize);
        let mut bone_indices: Vec<glam_read::U16Vec4> = Vec::with_capacity(vertex_count as usize);
        let mut bone_weights: Vec<glam::Vec4> = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count as usize {
            vertices.push(glam_read::vec3_f32::<LittleEndian>(&mut reader));
            bone_indices.push(glam_read::vec4_u8(&mut reader));
            bone_weights.push(glam_read::vec4_f32::<LittleEndian>(&mut reader));
            normals.push(glam_read::vec3_f32::<LittleEndian>(&mut reader).normalize());
            uvs.push(glam_read::vec2_f32::<LittleEndian>(&mut reader));

            if vertex_type > 0 {
                reader.set_position(reader.position() + 4);
            }
        }

        if major != 4 {
            for pos in vertices.iter() {
                for i in 0..3 {
                    bbmin[i] = f32::min(bbmin[i], pos[i]);
                    bbmax[i] = f32::max(bbmax[i], pos[i]);
                }
            }
        }
        let bounding_box = [bbmin, bbmax];
        let center = (bbmin + bbmax) / 2.0f32;

        let meshes = if major > 0 {
            let mut meshes = Vec::with_capacity(submeshheader_count as usize);
            for submeshheader in submeshheaders {
                meshes.push(Mesh::new(submeshheader));
            }
            meshes
        } else {
            vec![Mesh::new(SubMeshHeader {
                name: "Base".to_owned(),
                indices_offset: 0,
                indices_count: indices.len() as u32,
            })]
        };

        print!("SKN version {major} {minor} was succesfully loaded: ");
        print!("SubMeshHeader count: {submeshheader_count} ");
        print!("indices count: {indices_count} ");
        println!("vertex count: {vertex_count} ");

        Skin {
            major,
            minor,
            center,
            bounding_box,
            vertices,
            normals,
            uvs,
            bone_indices,
            bone_weights,
            indices,
            meshes,
        }
    }

    pub fn apply_skeleton(&mut self, skeleton: &Skeleton) {
        for skin_bone_indice in self.bone_indices.iter_mut() {
            skin_bone_indice.x = skeleton.bone_indices[skin_bone_indice.x as usize];
            skin_bone_indice.y = skeleton.bone_indices[skin_bone_indice.y as usize];
            skin_bone_indice.z = skeleton.bone_indices[skin_bone_indice.z as usize];
            skin_bone_indice.w = skeleton.bone_indices[skin_bone_indice.w as usize];
        }
    }
}
