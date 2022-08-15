use gls::glam_read;

use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use lol;

#[inline(never)]
pub fn read_anm(contents: &Vec<u8>) -> Animation {
    let mut reader = Cursor::new(contents);

    let mut signature = vec![0u8; 8];
    reader
        .read_exact(&mut signature)
        .expect("Could not read ANM signature");
    let signature = String::from_utf8(signature).expect("Invalid UTF-8 sequence");

    if signature != "r3d2anmd" && signature != "r3d2canm" {
        panic!("ANM has no valid signature");
    }

    let version = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM version");

    if signature == "r3d2canm" {
        read_compressed(&mut reader, version)
    } else if signature == "r3d2anmd" {
        if version == 5 {
            read_v5(&mut reader)
        } else if version == 4 {
            read_v4(&mut reader)
        } else {
            read_legacy(&mut reader, version)
        }
    } else {
        panic!("ANM has no valid signature");
    }
}

fn read_compressed(reader: &mut Cursor<&Vec<u8>>, version: u32) -> Animation {
    reader.set_position(reader.position() + 12);

    let bone_count = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM bone count");
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

    let mut hash_entries: Vec<u32> = Vec::with_capacity(bone_count as usize);
    for _ in 0..bone_count {
        hash_entries.push(
            reader
                .read_u32::<LittleEndian>()
                .expect("Could not read ANM hash entry"),
        );
    }

    reader.set_position((entries_offset + 12) as u64);

    let mut compressed_translations: HashMap<u8, Vec<(u16, u64)>> =
        HashMap::with_capacity(bone_count as usize);
    let mut compressed_scales: HashMap<u8, Vec<(u16, u64)>> =
        HashMap::with_capacity(bone_count as usize);
    let mut compressed_rotations: HashMap<u8, Vec<(u16, u64)>> =
        HashMap::with_capacity(bone_count as usize);
    for _ in 0..entry_count {
        let compressed_time = reader
            .read_u16::<LittleEndian>()
            .expect("Could not read ANM compressed time");

        let bone_index = reader.read_u8().expect("Could not read ANM bone index");

        let data_type = reader.read_u8().expect("Could not read ANM data type");

        let compressed_data = reader
            .read_u48::<LittleEndian>()
            .expect("Could not read ANM compressed data");

        match FrameDataType::from_u8(data_type) {
            FrameDataType::Rotation => {
                compressed_rotations
                    .entry(bone_index)
                    .or_insert_with(Vec::new)
                    .push((compressed_time, compressed_data));
            }
            FrameDataType::Translation => {
                compressed_translations
                    .entry(bone_index)
                    .or_insert_with(Vec::new)
                    .push((compressed_time, compressed_data));
            }
            FrameDataType::Scale => {
                compressed_scales
                    .entry(bone_index)
                    .or_insert_with(Vec::new)
                    .push((compressed_time, compressed_data));
            }
        }
    }

    let mut bones: Vec<BoneAnm> = Vec::with_capacity(bone_count as usize);
    for i in 0..bone_count {
        let mut bone_anm = BoneAnm {
            hash: hash_entries[i as usize],
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
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

            bone_anm
                .translations
                .push((uncompressed_time, uncompressed_translation));
        }

        for (compressed_time, compressed_data) in compressed_scale {
            let uncompressed_time = uncompress_time(*compressed_time, duration);

            let uncompressed_scale = uncompress_vec3(scale_min, scale_max, *compressed_data);

            bone_anm
                .scales
                .push((uncompressed_time, uncompressed_scale));
        }

        for (compressed_time, compressed_data) in compressed_rotation {
            let uncompressed_time = uncompress_time(*compressed_time, duration);

            let uncompressed_rotation = uncompress_quaternion(*compressed_data);

            bone_anm
                .rotations
                .push((uncompressed_time, uncompressed_rotation));
        }

        bones.push(bone_anm);
    }

    print!("ANM version {version} was succesfully loaded: ");
    print!("Type: r3d2canm ");
    print!("FPS: {fps} ");
    println!("Duration: {duration}");

    Animation {
        fps,
        duration,
        frame_delay,
        bones,
    }
}

fn read_v5(reader: &mut Cursor<&Vec<u8>>) -> Animation {
    reader.set_position(reader.position() + 16);

    let bone_count = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM bone count");
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

    let mut bones: Vec<BoneAnm> = Vec::with_capacity(bone_count as usize);
    for i in 0..bone_count {
        bones.push(BoneAnm {
            hash: hashes[i as usize],
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
        })
    }

    let mut current_time = 0.0f32;
    for _ in 0..frame_count {
        for j in 0..bone_count {
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

            bones[j as usize].rotations.push((current_time, rotation));
            bones[j as usize]
                .scales
                .push((current_time, vectors[scale_index as usize]));
            bones[j as usize]
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
        bones,
    }
}

fn read_v4(reader: &mut Cursor<&Vec<u8>>) -> Animation {
    reader.set_position(reader.position() + 16);

    let bone_count = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM bone count");
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

    let mut bone_map: HashMap<u32, Vec<FrameIndices>> = HashMap::with_capacity(bone_count as usize);
    for _ in 0..bone_count {
        for _ in 0..frame_count {
            let bone_hash = reader
                .read_u32::<LittleEndian>()
                .expect("Could not read ANM bone hash");

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

            bone_map
                .entry(bone_hash)
                .or_insert_with(Vec::new)
                .push(FrameIndices {
                    translation_index,
                    rotation_index,
                    scale_index,
                });
        }
    }

    let mut bones: Vec<BoneAnm> = Vec::with_capacity(bone_count as usize);
    for (hash, frame_indices) in bone_map {
        let mut current_time = 0.0f32;

        let mut bone_anm = BoneAnm {
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

            bone_anm.translations.push((current_time, translation));
            bone_anm.rotations.push((current_time, rotation));
            bone_anm.scales.push((current_time, scale));

            current_time += frame_delay;
        }

        bones.push(bone_anm);
    }

    print!("ANM version 4 was succesfully loaded: ");
    print!("Type: r3d2anmd ");
    print!("FPS: {fps} ");
    println!("Duration: {duration}");

    Animation {
        fps,
        duration,
        frame_delay,
        bones,
    }
}

fn read_legacy(reader: &mut Cursor<&Vec<u8>>, version: u32) -> Animation {
    reader.set_position(reader.position() + 4);

    let bone_count = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM bone count");
    let frame_count = reader
        .read_u32::<LittleEndian>()
        .expect("Could not read ANM frame count");

    let fps = reader
        .read_i32::<LittleEndian>()
        .expect("Could not read ANM fps") as f32;

    let frame_delay = 1.0f32 / fps;
    let duration = frame_count as f32 * frame_delay;

    let mut bones: Vec<BoneAnm> = Vec::with_capacity(bone_count as usize);
    for _ in 0..bone_count {
        let mut string = vec![0u8; 32];
        reader
            .read_exact(&mut string)
            .expect("Could not read ANM bone name");
        let name = String::from_utf8(string)
            .expect("Invalid UTF-8 sequence")
            .trim_end_matches('\0')
            .to_string();
        let hash = lol::hasher::string_to_hash(&name);

        reader.set_position(reader.position() + 4);

        let mut bone_anm = BoneAnm {
            hash,
            translations: Vec::with_capacity(frame_count as usize),
            rotations: Vec::with_capacity(frame_count as usize),
            scales: Vec::with_capacity(frame_count as usize),
        };

        let mut current_time = 0.0f32;
        for _ in 0..frame_count {
            let rotation = glam_read::quat_f32::<LittleEndian>(reader);
            let translation = glam_read::vec3_f32::<LittleEndian>(reader);

            bone_anm.rotations.push((current_time, rotation));
            bone_anm.translations.push((current_time, translation));
            bone_anm.scales.push((current_time, glam::Vec3::ONE));

            current_time += frame_delay;
        }

        bones.push(bone_anm);
    }

    print!("ANM version {version} was succesfully loaded: ");
    print!("Type: r3d2anmd ");
    print!("FPS: {fps} ");
    println!("Duration: {duration}");

    Animation {
        fps,
        duration,
        frame_delay,
        bones,
    }
}

pub enum FrameDataType {
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

pub struct FrameIndices {
    pub translation_index: u16,
    pub rotation_index: u16,
    pub scale_index: u16,
}

pub struct BoneAnm {
    pub hash: u32,
    pub translations: Vec<(f32, glam::Vec3)>,
    pub rotations: Vec<(f32, glam::Quat)>,
    pub scales: Vec<(f32, glam::Vec3)>,
}

pub struct Animation {
    pub fps: f32,
    pub duration: f32,
    pub frame_delay: f32,
    pub bones: Vec<BoneAnm>,
}

fn uncompress_quaternion(data: u64) -> glam::Quat {
    let index = ((data >> 45) & 0x0003) as u16;
    let v_a = ((data >> 30) & 0x7FFF) as u16;
    let v_b = ((data >> 15) & 0x7FFF) as u16;
    let v_c = (data & 0x7FFF) as u16;

    let sqrt2 = std::f32::consts::SQRT_2;
    let a = ((v_a as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2) as f32;
    let b = ((v_b as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2) as f32;
    let c = ((v_c as f32 / 32767.0f32) * sqrt2 - 1.0f32 / sqrt2) as f32;
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

fn find_in_nearest_time<'a, T>(
    vector: &'a Vec<(f32, T)>,
    index: &mut usize,
    time: f32,
) -> (&'a T, &'a T, f32) {
    let mut min = vector.first().expect("Could not get first vector");
    let mut max = vector.last().expect("Could not get last vector");

    if time > max.0 {
        min = vector
            .get(vector.len() - 2)
            .expect("Could not get penultimate vector");
    } else {
        if time < vector[*index].0 {
            *index = 0;
        } else if *index > 0 {
            *index -= 1;
        }
        for current in vector.iter().skip(*index) {
            if current.0 <= time {
                min = current;
                continue;
            }
            max = current;
            break;
        }
    }

    let div = max.0 - min.0;
    let lerp_value = if div != 0.0f32 {
        (time - min.0) / div
    } else {
        1.0f32
    };

    (&min.1, &max.1, lerp_value)
}

fn run_hierarchy(
    bone_transforms: &mut Vec<glam::Mat4>,
    skeleton_bone: &lol::skl::Bone,
    skeleton_bones: &Vec<lol::skl::Bone>,
    parent_transform: &glam::Mat4,
    current_frame: &mut Vec<BoneFrameIndexCache>,
    animation: &Animation,
    time: f32,
) {
    let mut global_transform = *parent_transform;

    let animation_bone = animation
        .bones
        .iter()
        .find(|&bone| bone.hash == skeleton_bone.hash);
    if let Some(bone) = animation_bone {
        let translation = find_in_nearest_time(
            &bone.translations,
            &mut current_frame[skeleton_bone.id as usize].translation,
            time,
        );
        let rotation = find_in_nearest_time(
            &bone.rotations,
            &mut current_frame[skeleton_bone.id as usize].rotation,
            time,
        );
        let scale = find_in_nearest_time(
            &bone.scales,
            &mut current_frame[skeleton_bone.id as usize].scale,
            time,
        );

        let translation = translation.0.lerp(*translation.1, translation.2);
        let rotation = rotation.0.lerp(*rotation.1, rotation.2);
        let scale = scale.0.lerp(*scale.1, scale.2);

        global_transform = *parent_transform
            * glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
    }

    if let Some(transform) = bone_transforms.get_mut(skeleton_bone.id as usize) {
        *transform = global_transform * skeleton_bone.inverse_global_matrix;
    }

    for child_id in &skeleton_bone.children {
        run_hierarchy(
            bone_transforms,
            &skeleton_bones[*child_id],
            skeleton_bones,
            &global_transform,
            current_frame,
            animation,
            time,
        );
    }
}

#[inline(never)]
pub fn run_animation(
    bone_transforms: &mut Vec<glam::Mat4>,
    animation: &Animation,
    skeleton: &lol::skl::Skeleton,
    time: f32,
) {
    let mut current_frame = Vec::with_capacity(skeleton.bones.len());
    for _ in 0..skeleton.bones.len() {
        current_frame.push(BoneFrameIndexCache::new());
    }
    for animation_bone in &animation.bones {
        let skeleton_bone = skeleton
            .bones
            .iter()
            .find(|&bone| bone.hash == animation_bone.hash);
        if let Some(bone) = skeleton_bone {
            if bone.parent_id < 0 {
                run_hierarchy(
                    bone_transforms,
                    bone,
                    &skeleton.bones,
                    &glam::Mat4::IDENTITY,
                    &mut current_frame,
                    animation,
                    time,
                );
            }
        }
    }
}

struct BoneFrameIndexCache {
    translation: usize,
    rotation: usize,
    scale: usize,
}

impl BoneFrameIndexCache {
    fn new() -> BoneFrameIndexCache {
        BoneFrameIndexCache {
            translation: 0,
            rotation: 0,
            scale: 0,
        }
    }
}
