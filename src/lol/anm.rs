use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    collections::BTreeMap,
    f32,
    io::{Cursor, Read},
};

use crate::{
    gls::glam_read,
    lol::{hasher, Skeleton},
};

enum FrameDataType {
    Rotation = 0,
    Translation = 64,
    Scale = 128,
}

impl FrameDataType {
    fn from_u8(value: u8) -> FrameDataType {
        match value {
            0 => FrameDataType::Rotation,
            64 => FrameDataType::Translation,
            128 => FrameDataType::Scale,
            _ => panic!("Unknown ANM FrameDataType"),
        }
    }
}

struct FrameIndices {
    pub translation_index: u16,
    pub rotation_index: u16,
    pub scale_index: u16,
}

pub struct JointAnm {
    pub hash: u32,
    pub translations: Vec<(f32, glam::Vec3)>,
    pub rotations: Vec<(f32, glam::Quat)>,
    pub scales: Vec<(f32, glam::Vec3)>,
}

pub struct Animation {
    pub fps: f32,
    pub duration: f32,
    pub frame_delay: f32,
    pub joints: Vec<JointAnm>,
}

impl Animation {
    pub fn read(contents: &Vec<u8>) -> Animation {
        let mut reader = Cursor::new(contents);

        let mut signature = vec![0u8; 8];
        reader
            .read_exact(&mut signature)
            .expect("Could not read ANM signature");

        if signature == b"r3d2canm"[..] {
            Self::read_compressed(&mut reader)
        } else if signature == b"r3d2anmd"[..] {
            let version = reader
                .read_u32::<LittleEndian>()
                .expect("Could not read ANM version");

            if version == 5 {
                Self::read_v5(&mut reader)
            } else if version == 4 {
                Self::read_v4(&mut reader)
            } else {
                Self::read_legacy(&mut reader, version)
            }
        } else {
            panic!("ANM has no valid signature");
        }
    }

    fn read_compressed(reader: &mut Cursor<&Vec<u8>>) -> Animation {
        let version = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM version");

        reader.set_position(reader.position() + 12);

        let joint_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM joint count");
        let entry_count = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read ANM entry count");

        reader.set_position(reader.position() + 4);

        let duration = reader
            .read_f32::<LittleEndian>()
            .expect("Could not read ANM duration");
        let fps = reader
            .read_f32::<LittleEndian>()
            .expect("Could not read ANM fps");
        let frame_delay = 1.0f32 / fps;

        reader.set_position(reader.position() + 24);

        let translation_min = glam_read::vec3_f32::<LittleEndian>(reader);
        let translation_max = glam_read::vec3_f32::<LittleEndian>(reader);

        let scale_min = glam_read::vec3_f32::<LittleEndian>(reader);
        let scale_max = glam_read::vec3_f32::<LittleEndian>(reader);

        let entries_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM entries offset");

        reader.set_position(reader.position() + 4);

        let hashes_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM hashes offset");

        reader.set_position((hashes_offset + 12) as u64);

        let mut hash_entries: Vec<u32> = Vec::with_capacity(joint_count as usize);
        for _ in 0..joint_count {
            hash_entries.push(
                reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read ANM hash entry"),
            );
        }

        reader.set_position((entries_offset + 12) as u64);

        let mut compressed_translations: BTreeMap<u8, Vec<(u16, u64)>> = BTreeMap::new();
        let mut compressed_scales: BTreeMap<u8, Vec<(u16, u64)>> = BTreeMap::new();
        let mut compressed_rotations: BTreeMap<u8, Vec<(u16, u64)>> = BTreeMap::new();
        for _ in 0..entry_count {
            let compressed_time = reader
                .read_u16::<LittleEndian>()
                .expect("Could not read ANM compressed time");

            let joint_index = reader.read_u8().expect("Could not read ANM joint index");

            let data_type = reader.read_u8().expect("Could not read ANM data type");

            let compressed_data = reader
                .read_u48::<LittleEndian>()
                .expect("Could not read ANM compressed data");

            match FrameDataType::from_u8(data_type) {
                FrameDataType::Rotation => {
                    compressed_rotations
                        .entry(joint_index)
                        .or_default()
                        .push((compressed_time, compressed_data));
                }
                FrameDataType::Translation => {
                    compressed_translations
                        .entry(joint_index)
                        .or_default()
                        .push((compressed_time, compressed_data));
                }
                FrameDataType::Scale => {
                    compressed_scales
                        .entry(joint_index)
                        .or_default()
                        .push((compressed_time, compressed_data));
                }
            }
        }

        let mut joints: Vec<JointAnm> = Vec::with_capacity(joint_count as usize);
        for i in 0..joint_count {
            let mut joint_anm = JointAnm {
                hash: hash_entries[i as usize],
                translations: vec![],
                rotations: vec![],
                scales: vec![],
            };

            let compressed_translation = compressed_translations
                .get(&(i as u8))
                .expect("Could not find compressed translation");
            let compressed_scale = compressed_scales
                .get(&(i as u8))
                .expect("Could not find compressed scale");
            let compressed_rotation = compressed_rotations
                .get(&(i as u8))
                .expect("Could not find compressed rotation");

            for (compressed_time, compressed_data) in compressed_translation {
                let uncompressed_time = uncompress_time(*compressed_time, duration);
                let uncompressed_translation =
                    uncompress_vec3(translation_min, translation_max, *compressed_data);

                joint_anm
                    .translations
                    .push((uncompressed_time, uncompressed_translation));
            }

            for (compressed_time, compressed_data) in compressed_scale {
                let uncompressed_time = uncompress_time(*compressed_time, duration);
                let uncompressed_scale = uncompress_vec3(scale_min, scale_max, *compressed_data);

                joint_anm
                    .scales
                    .push((uncompressed_time, uncompressed_scale));
            }

            for (compressed_time, compressed_data) in compressed_rotation {
                let uncompressed_time = uncompress_time(*compressed_time, duration);
                let uncompressed_rotation = uncompress_quaternion(*compressed_data);

                joint_anm
                    .rotations
                    .push((uncompressed_time, uncompressed_rotation));
            }

            joints.push(joint_anm);
        }

        print!("ANM version {version} was succesfully loaded: ");
        print!("Type: r3d2canm ");
        print!("FPS: {fps} ");
        println!("Duration: {duration}");

        Animation {
            fps,
            duration,
            frame_delay,
            joints,
        }
    }

    fn read_v5(reader: &mut Cursor<&Vec<u8>>) -> Animation {
        reader.set_position(reader.position() + 16);

        let joint_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM joint count");
        let frame_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM frame count");

        let frame_delay = reader
            .read_f32::<LittleEndian>()
            .expect("Could not read ANM frame delay");

        let duration = frame_count as f32 * frame_delay;
        let fps = frame_count as f32 / duration;

        let hashes_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM hashes offset");

        reader.set_position(reader.position() + 8);

        let vectors_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM translation offset");
        let rotations_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM rotation offset");
        let frame_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM frame offset");

        let hashes_count = (frame_offset - hashes_offset) / 4;
        let vectors_count = (rotations_offset - vectors_offset) / 12;
        let rotations_count = (hashes_offset - rotations_offset) / 6;

        reader.set_position((hashes_offset + 12) as u64);

        let mut hashes: Vec<u32> = Vec::with_capacity(hashes_count as usize);
        for _ in 0..hashes_count {
            hashes.push(
                reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read ANM hash"),
            );
        }

        reader.set_position((vectors_offset + 12) as u64);

        let mut vectors: Vec<glam::Vec3> = Vec::with_capacity(vectors_count as usize);
        for _ in 0..vectors_count {
            vectors.push(glam_read::vec3_f32::<LittleEndian>(reader))
        }

        reader.set_position((rotations_offset + 12) as u64);

        let mut rotations: Vec<u64> = Vec::with_capacity(rotations_count as usize);
        for _ in 0..rotations_count {
            rotations.push(
                reader
                    .read_u48::<LittleEndian>()
                    .expect("Could not read ANM rotation"),
            );
        }

        reader.set_position((frame_offset + 12) as u64);

        let mut joints: Vec<JointAnm> = Vec::with_capacity(joint_count as usize);
        for i in 0..joint_count {
            joints.push(JointAnm {
                hash: hashes[i as usize],
                translations: vec![],
                rotations: vec![],
                scales: vec![],
            })
        }

        let mut current_time = 0.0f32;
        for _ in 0..frame_count {
            for j in 0..joint_count {
                let translation_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM translation index");
                let scale_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM scale index");
                let rotation_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM rotation index");

                let rotation = uncompress_quaternion(rotations[rotation_index as usize]);

                joints[j as usize].rotations.push((current_time, rotation));
                joints[j as usize]
                    .scales
                    .push((current_time, vectors[scale_index as usize]));
                joints[j as usize]
                    .translations
                    .push((current_time, vectors[translation_index as usize]));
            }
            current_time += frame_delay;
        }

        print!("ANM version 5 was succesfully loaded: ");
        print!("Type: r3d2anmd ");
        print!("FPS: {fps} ");
        println!("Duration: {duration}");

        Animation {
            fps,
            duration,
            frame_delay,
            joints,
        }
    }

    fn read_v4(reader: &mut Cursor<&Vec<u8>>) -> Animation {
        reader.set_position(reader.position() + 16);

        let joint_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM joint count");
        let frame_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM frame count");

        let frame_delay = reader
            .read_f32::<LittleEndian>()
            .expect("Could not read ANM frame delay");

        reader.set_position(reader.position() + 12);

        let duration = frame_count as f32 * frame_delay;
        let fps = 1.0f32 / frame_delay;

        let vectors_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM translation offset");
        let rotations_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM rotation offset");
        let frame_offset = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM frame offset");

        let vectors_count = (rotations_offset - vectors_offset) / 12;
        let rotations_count = (frame_offset - rotations_offset) / 16;

        reader.set_position((vectors_offset + 12) as u64);

        let mut vectors: Vec<glam::Vec3> = Vec::with_capacity(vectors_count as usize);
        for _ in 0..vectors_count {
            vectors.push(glam_read::vec3_f32::<LittleEndian>(reader))
        }

        reader.set_position((rotations_offset + 12) as u64);

        let mut rotations: Vec<glam::Quat> = Vec::with_capacity(rotations_count as usize);
        for _ in 0..rotations_count {
            rotations.push(glam_read::quat_f32::<LittleEndian>(reader));
        }

        reader.set_position((frame_offset + 12) as u64);

        let mut joint_map: BTreeMap<u32, Vec<FrameIndices>> = BTreeMap::new();
        for _ in 0..joint_count {
            for _ in 0..frame_count {
                let joint_hash = reader
                    .read_u32::<LittleEndian>()
                    .expect("Could not read ANM joint hash");

                let translation_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM translation index");
                let scale_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM scale index");
                let rotation_index = reader
                    .read_u16::<LittleEndian>()
                    .expect("Could not read ANM rotation index");

                reader.set_position(reader.position() + 2);

                joint_map.entry(joint_hash).or_default().push(FrameIndices {
                    translation_index,
                    rotation_index,
                    scale_index,
                });
            }
        }

        let mut joints: Vec<JointAnm> = Vec::with_capacity(joint_count as usize);
        for (hash, frame_indices) in joint_map {
            let mut current_time = 0.0f32;

            let mut joint_anm = JointAnm {
                hash,
                translations: Vec::with_capacity(frame_indices.len()),
                rotations: Vec::with_capacity(frame_indices.len()),
                scales: Vec::with_capacity(frame_indices.len()),
            };

            for frame_index in frame_indices {
                let translation_index = frame_index.translation_index;
                let rotation_index = frame_index.rotation_index;
                let scale_index = frame_index.scale_index;

                let translation = vectors[translation_index as usize];
                let rotation = rotations[rotation_index as usize];
                let scale = vectors[scale_index as usize];

                joint_anm.translations.push((current_time, translation));
                joint_anm.rotations.push((current_time, rotation));
                joint_anm.scales.push((current_time, scale));

                current_time += frame_delay;
            }

            joints.push(joint_anm);
        }

        print!("ANM version 4 was succesfully loaded: ");
        print!("Type: r3d2anmd ");
        print!("FPS: {fps} ");
        println!("Duration: {duration}");

        Animation {
            fps,
            duration,
            frame_delay,
            joints,
        }
    }

    fn read_legacy(reader: &mut Cursor<&Vec<u8>>, version: u32) -> Animation {
        reader.set_position(reader.position() + 4);

        let joint_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM joint count");
        let frame_count = reader
            .read_u32::<LittleEndian>()
            .expect("Could not read ANM frame count");

        let fps = reader
            .read_i32::<LittleEndian>()
            .expect("Could not read ANM fps") as f32;

        let frame_delay = 1.0f32 / fps;
        let duration = frame_count as f32 * frame_delay;

        let mut joints: Vec<JointAnm> = Vec::with_capacity(joint_count as usize);
        for _ in 0..joint_count {
            let mut string = vec![0u8; 32];
            reader
                .read_exact(&mut string)
                .expect("Could not read ANM joint name");
            let name = String::from(
                String::from_utf8(string)
                    .expect("Invalid UTF-8 sequence")
                    .trim_end_matches('\0'),
            );
            let hash = hasher::string_to_hash(&name);

            reader.set_position(reader.position() + 4);

            let mut joint_anm = JointAnm {
                hash,
                translations: Vec::with_capacity(frame_count as usize),
                rotations: Vec::with_capacity(frame_count as usize),
                scales: Vec::with_capacity(frame_count as usize),
            };

            let mut current_time = 0.0f32;
            for _ in 0..frame_count {
                let rotation = glam_read::quat_f32::<LittleEndian>(reader);
                let translation = glam_read::vec3_f32::<LittleEndian>(reader);

                joint_anm.rotations.push((current_time, rotation));
                joint_anm.translations.push((current_time, translation));
                joint_anm.scales.push((current_time, glam::Vec3::ONE));

                current_time += frame_delay;
            }

            joints.push(joint_anm);
        }

        print!("ANM version {version} was succesfully loaded: ");
        print!("Type: r3d2anmd ");
        print!("FPS: {fps} ");
        println!("Duration: {duration}");

        Animation {
            fps,
            duration,
            frame_delay,
            joints,
        }
    }
}

fn uncompress_quaternion(data: u64) -> glam::Quat {
    let index = ((data >> 45) & 0x0003) as u16;
    let v_a = ((data >> 30) & 0x7FFF) as u16;
    let v_b = ((data >> 15) & 0x7FFF) as u16;
    let v_c = (data & 0x7FFF) as u16;

    let sqrt2 = f32::consts::SQRT_2;
    let a = (v_a as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2;
    let b = (v_b as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2;
    let c = (v_c as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2;
    let d = 0.0f32.max(1.0f32 - (a * a + b * b + c * c)).sqrt();

    match index {
        0 => glam::quat(d, a, b, c),
        1 => glam::quat(a, d, b, c),
        2 => glam::quat(a, b, d, c),
        _ => glam::quat(a, b, c, d),
    }
}

fn uncompress_time(compressed_time: u16, animation_length: f32) -> f32 {
    (compressed_time as f32 / 65535.0f32) * animation_length
}

fn uncompress_vec3(min: glam::Vec3, max: glam::Vec3, data: u64) -> glam::Vec3 {
    let c_x = (data & 0xFFFF) as u16;
    let c_y = ((data >> 16) & 0xFFFF) as u16;
    let c_z = ((data >> 32) & 0xFFFF) as u16;

    let mut uncompressed = max - min;

    uncompressed.x *= c_x as f32 / 65535.0f32;
    uncompressed.y *= c_y as f32 / 65535.0f32;
    uncompressed.z *= c_z as f32 / 65535.0f32;

    uncompressed + min
}

pub fn find_in_nearest_time<T: Copy + Default>(vector: &Vec<(f32, T)>, time: f32) -> (T, T, f32) {
    if vector.len() >= 2 {
        let mut min = vector.first().unwrap();
        let mut max = vector.last().unwrap();

        for current in vector.iter() {
            if current.0 <= time {
                min = current;
                continue;
            }
            max = current;
            break;
        }

        let div = max.0 - min.0;
        let lerp_value = if div != 0.0f32 {
            (time - min.0) / div
        } else {
            1.0f32
        };

        (min.1, max.1, lerp_value)
    } else if vector.len() == 1 {
        (vector[0].1, vector[0].1, 0.0f32)
    } else {
        (T::default(), T::default(), 0.0f32)
    }
}

pub fn run_animation(
    joint_transforms: &mut [glam::Mat4],
    animation: &Animation,
    skeleton: &Skeleton,
    time: f32,
) {
    if time <= animation.duration {
        let mut parent_transforms: Vec<glam::Mat4> = skeleton
            .joints
            .iter()
            .map(|joint| joint.local_matrix)
            .collect();
        for i in 0..skeleton.joints.len() {
            let skeleton_joint = &skeleton.joints[i];

            let mut global_transform = if skeleton_joint.parent_id != -1 {
                parent_transforms[skeleton_joint.parent_id as usize]
            } else {
                glam::Mat4::IDENTITY
            };

            let animation_joint = animation
                .joints
                .iter()
                .find(|&joint| joint.hash == skeleton_joint.hash);

            if let Some(joint) = animation_joint {
                let (translation_min, translation_max, translation_lerp_value) =
                    find_in_nearest_time(&joint.translations, time);
                let (rotation_min, rotation_max, rotation_lerp_value) =
                    find_in_nearest_time(&joint.rotations, time);
                let (scale_min, scale_max, scale_lerp_value) =
                    find_in_nearest_time(&joint.scales, time);

                let translation = translation_min.lerp(translation_max, translation_lerp_value);
                let rotation = rotation_min.lerp(rotation_max, rotation_lerp_value);
                let scale = scale_min.lerp(scale_max, scale_lerp_value);

                global_transform *=
                    glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
            } else {
                global_transform *= skeleton_joint.local_matrix;
            }

            parent_transforms[i] = global_transform;
            joint_transforms[i] = global_transform * skeleton_joint.inverse_global_matrix;
        }
    }
}
