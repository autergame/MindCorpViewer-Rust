use std::{borrow, collections::HashMap, fs, fs::File, io::Write, mem, path::Path};

use lol::{anm, Animation, Skeleton, Skin};

use gltf::{
    json::{
        accessor, animation, buffer, extensions, material, mesh::Primitive, scene, texture,
        validation::Checked::Valid, Accessor, Animation as GltfAnimation, Asset, Buffer, Image,
        Index, Material, Mesh, Node, Root, Scene, Skin as GltfSkin, Texture, Value,
    },
    material::AlphaMode,
    mesh::Mode,
    Semantic,
};

use gls::glam_read;

use crate::MindModel;

pub fn export_model(export_as: u8, model_name: &String, mind_model: &MindModel) {
    let export_path = format!("export/{model_name}");
    fs::create_dir_all(&export_path).expect("Could not create export dirs");

    let mut accessor_index = 0;
    let mut buffer_view_index = 0;
    let mut buffer_view_offset = 0;

    let (mut buffer_views, mut accessors, mesh, mesh_data) = make_mesh(
        &mind_model.skn,
        &mind_model.textures_selecteds,
        &mut accessor_index,
        &mut buffer_view_index,
        &mut buffer_view_offset,
    );

    let mut all_datas = vec![mesh_data];

    let (materials, textures, images, texture_data_buffer_views) = make_material(
        &mind_model.textures_paths,
        &export_path,
        export_as,
        &mut buffer_view_index,
        &mut buffer_view_offset,
    );

    if let Some((texture_data, texture_buffer_views)) = texture_data_buffer_views {
        all_datas.push(texture_data);
        buffer_views.extend_from_slice(&texture_buffer_views);
    }

    let texture_sampler = texture::Sampler {
        mag_filter: Some(Valid(texture::MagFilter::Linear)),
        min_filter: Some(Valid(texture::MinFilter::Linear)),
        name: None,
        wrap_s: Valid(texture::WrappingMode::Repeat),
        wrap_t: Valid(texture::WrappingMode::Repeat),
        extensions: None,
        extras: None,
    };

    let (nodes, gltf_skin, ibm_data, ibm_buffer_view, ibm_accessor) = make_skeleton(
        model_name,
        &mind_model.skl,
        &mut accessor_index,
        &mut buffer_view_index,
        &mut buffer_view_offset,
    );

    all_datas.push(ibm_data);

    accessors.push(ibm_accessor);
    buffer_views.push(ibm_buffer_view);

    let mut animations_gltf = vec![];

    for i in 0..mind_model.animations.len() {
        let (animation_gltf, animation_data, animation_buffer_view, animation_accessor) =
            make_animation(
                &mind_model.skl,
                &mind_model.animations[i],
                &mind_model.animations_file_names[i],
                &mut accessor_index,
                &mut buffer_view_index,
                &mut buffer_view_offset,
            );
        animations_gltf.push(animation_gltf);
        all_datas.push(animation_data);
        buffer_views.push(animation_buffer_view);
        accessors.extend_from_slice(&animation_accessor);
    }

    let all_data_1d = vec_2d_to_vec_1d(&all_datas);

    let buffer = Buffer {
        byte_length: all_data_1d.len() as u32,
        extensions: None,
        extras: None,
        name: None,
        uri: None,
    };

    let scene = Scene {
        extensions: None,
        extras: None,
        name: Some(String::from("Model")),
        nodes: vec![Index::new(mind_model.skl.bones.len() as u32)],
    };

    let asset = Asset {
        copyright: None,
        extensions: None,
        extras: None,
        generator: None,
        min_version: None,
        version: String::from("2.0"),
    };

    let mut root = Root {
        accessors,
        buffers: vec![buffer],
        buffer_views,
        meshes: vec![mesh],
        nodes,
        skins: vec![gltf_skin],
        animations: animations_gltf,
        materials,
        textures,
        samplers: vec![texture_sampler],
        images,
        scenes: vec![scene],
        extensions_required: vec![String::from("KHR_materials_unlit")],
        extensions_used: vec![String::from("KHR_materials_unlit")],
        asset,
        scene: Some(Index::new(0)),
        extensions: None,
        extras: None,
        cameras: vec![],
    };

    if export_as == 0 {
        root.buffers[0].uri = Some(format!("{model_name}_data.bin"));

        let json_string = root.to_string_pretty().expect("Could not serialize gltf");

        let output_gltf = format!("{export_path}/{model_name}.gltf");
        let mut writer_gltf = File::create(output_gltf).expect("Could not create gltf file");
        writer_gltf
            .write_all(json_string.as_bytes())
            .expect("Could not write gltf");

        let output_data_bin = format!("{export_path}/{model_name}_data.bin");
        let mut writer_data_bin =
            File::create(output_data_bin).expect("Could not create gltf bin file");
        writer_data_bin
            .write_all(&all_data_1d)
            .expect("Could not write gltf bin");
    } else {
        let json_string = root.to_string().expect("Could not serialize glb");

        let glb = gltf::binary::Glb {
            header: gltf::binary::Header {
                magic: *b"glTF",
                version: 2,
                length: 0,
            },
            bin: Some(borrow::Cow::Owned(all_data_1d)),
            json: borrow::Cow::Owned(json_string.into_bytes()),
        };

        let outputglb = format!("export/{model_name}.glb");
        let writerglb = File::create(outputglb).expect("Could not create glb file");
        glb.to_writer(writerglb).expect("Could not write glb");
    }
}

fn make_animation(
    skeleton: &Skeleton,
    animation: &Animation,
    animations_file_name: &String,
    accessor_index: &mut u32,
    buffer_view_index: &mut u32,
    buffer_view_offset: &mut u32,
) -> (GltfAnimation, Vec<u8>, buffer::View, Vec<Accessor>) {
    let frame_count = (animation.duration / animation.frame_delay).ceil() as u32;
    let times: Vec<f32> = (0..frame_count)
        .map(|i| animation.frame_delay * i as f32)
        .collect();

    let times_length = (times.len() * mem::size_of::<f32>()) as u32;

    let filtered_animation_bones: Vec<(usize, &anm::BoneAnm)> = animation
        .bones
        .iter()
        .filter_map(|animation_bone| {
            let bone_index = skeleton
                .bones
                .iter()
                .position(|skeleton_bone| skeleton_bone.hash == animation_bone.hash);
            bone_index.map(|bone_index| (bone_index, animation_bone))
        })
        .collect();

    let translations_data: Vec<Vec<u8>> = filtered_animation_bones
        .iter()
        .map(|(_, animation_bone)| {
            any_vec_as_vec_u8(
                &times
                    .iter()
                    .map(|time| {
                        let (min, max, lerp_value) =
                            anm::find_in_nearest_time(&animation_bone.translations, *time);
                        min.lerp(max, lerp_value)
                    })
                    .collect(),
            )
        })
        .collect();
    let rotations_data: Vec<Vec<u8>> = filtered_animation_bones
        .iter()
        .map(|(_, animation_bone)| {
            any_vec_as_vec_u8(
                &times
                    .iter()
                    .map(|time| {
                        let (min, max, lerp_value) =
                            anm::find_in_nearest_time(&animation_bone.rotations, *time);
                        min.lerp(max, lerp_value)
                    })
                    .collect(),
            )
        })
        .collect();
    let scales_data: Vec<Vec<u8>> = filtered_animation_bones
        .iter()
        .map(|(_, animation_bone)| {
            any_vec_as_vec_u8(
                &times
                    .iter()
                    .map(|time| {
                        let (min, max, lerp_value) =
                            anm::find_in_nearest_time(&animation_bone.scales, *time);
                        min.lerp(max, lerp_value)
                    })
                    .collect(),
            )
        })
        .collect();

    let translations_1d = vec_2d_to_vec_1d(&translations_data);
    let rotations_1d = vec_2d_to_vec_1d(&rotations_data);
    let scales_1d = vec_2d_to_vec_1d(&scales_data);

    let animation_total_data = vec_2d_to_vec_1d(&[
        any_vec_as_vec_u8(&times),
        translations_1d,
        rotations_1d,
        scales_1d,
    ]);

    let animation_buffer_view =
        make_buffer_view(animation_total_data.len() as u32, *buffer_view_offset, None);
    let animation_buffer_view_index = *buffer_view_index;
    *buffer_view_index += 1;
    *buffer_view_offset += animation_total_data.len() as u32;

    let times_accessor = make_accessor(
        frame_count,
        0,
        animation_buffer_view_index,
        accessor::Type::Scalar,
        accessor::ComponentType::F32,
        Some(Value::from(vec![*times.first().unwrap()])),
        Some(Value::from(vec![*times.last().unwrap()])),
    );
    let accessor_times = *accessor_index;
    *accessor_index += 1;

    let mut sampler_index = 0;
    let mut animation_offset = times_length;

    let (translations_channels, translations_samplers, translations_accessors) = make_trs(
        &filtered_animation_bones,
        accessor::Type::Vec3,
        accessor::ComponentType::F32,
        animation::Property::Translation,
        frame_count,
        mem::size_of::<glam::Vec3>() as u32,
        accessor_times,
        &mut sampler_index,
        &mut animation_offset,
        animation_buffer_view_index,
        accessor_index,
    );

    let (rotations_channels, rotations_samplers, rotations_accessors) = make_trs(
        &filtered_animation_bones,
        accessor::Type::Vec4,
        accessor::ComponentType::F32,
        animation::Property::Rotation,
        frame_count,
        mem::size_of::<glam::Vec4>() as u32,
        accessor_times,
        &mut sampler_index,
        &mut animation_offset,
        animation_buffer_view_index,
        accessor_index,
    );

    let (scales_channels, scales_samplers, scales_accessors) = make_trs(
        &filtered_animation_bones,
        accessor::Type::Vec3,
        accessor::ComponentType::F32,
        animation::Property::Scale,
        frame_count,
        mem::size_of::<glam::Vec3>() as u32,
        accessor_times,
        &mut sampler_index,
        &mut animation_offset,
        animation_buffer_view_index,
        accessor_index,
    );

    let mut accessors = vec![times_accessor];
    accessors.extend_from_slice(&translations_accessors);
    accessors.extend_from_slice(&rotations_accessors);
    accessors.extend_from_slice(&scales_accessors);

    let mut channels = vec![];
    channels.extend_from_slice(&translations_channels);
    channels.extend_from_slice(&rotations_channels);
    channels.extend_from_slice(&scales_channels);

    let mut samplers = vec![];
    samplers.extend_from_slice(&translations_samplers);
    samplers.extend_from_slice(&rotations_samplers);
    samplers.extend_from_slice(&scales_samplers);

    let animation = GltfAnimation {
        extensions: None,
        extras: None,
        channels,
        name: Some(animations_file_name.to_owned()),
        samplers,
    };

    (
        animation,
        animation_total_data,
        animation_buffer_view,
        accessors,
    )
}

fn make_trs(
    animation_bones: &[(usize, &anm::BoneAnm)],
    type_: accessor::Type,
    component_type: accessor::ComponentType,
    animation_property: animation::Property,
    times_count: u32,
    trs_byte_stride: u32,
    accessor_times: u32,
    sampler_index: &mut u32,
    animation_offset: &mut u32,
    buffer_view_index: u32,
    accessor_index: &mut u32,
) -> (
    Vec<animation::Channel>,
    Vec<animation::Sampler>,
    Vec<accessor::Accessor>,
) {
    let mut channels = vec![];
    let mut samplers = vec![];
    let mut accessors = vec![];

    for (bone_index, _) in animation_bones {
        accessors.push(make_accessor(
            times_count,
            *animation_offset,
            buffer_view_index,
            type_,
            component_type,
            None,
            None,
        ));
        let accessor_trs = *accessor_index;
        *accessor_index += 1;

        *animation_offset += times_count * trs_byte_stride;

        channels.push(animation::Channel {
            sampler: Index::new(*sampler_index),
            target: animation::Target {
                extensions: None,
                extras: None,
                node: Index::new(*bone_index as u32),
                path: Valid(animation_property),
            },
            extensions: None,
            extras: None,
        });
        samplers.push(animation::Sampler {
            extensions: None,
            extras: None,
            input: Index::new(accessor_times),
            interpolation: Valid(animation::Interpolation::Linear),
            output: Index::new(accessor_trs),
        });
        *sampler_index += 1;
    }

    (channels, samplers, accessors)
}

fn make_mesh(
    skin: &Skin,
    texture_selecteds: &[usize],
    accessor_index: &mut u32,
    buffer_view_index: &mut u32,
    buffer_view_offset: &mut u32,
) -> (Vec<buffer::View>, Vec<Accessor>, Mesh, Vec<u8>) {
    let vec2_length = mem::size_of::<glam::Vec2>() as u32;
    let vec3_length = mem::size_of::<glam::Vec3>() as u32;
    let vec4_length = mem::size_of::<glam::Vec4>() as u32;
    let u16vec4_length = mem::size_of::<glam_read::U16Vec4>() as u32;

    let vertex_count = skin.vertices.len();

    let vertices_length = (vertex_count as u32) * vec3_length;
    let normals_length = (vertex_count as u32) * vec3_length;
    let uvs_length = (vertex_count as u32) * vec2_length;
    let bone_indices_length = (vertex_count as u32) * u16vec4_length;
    let bone_weights_length = (vertex_count as u32) * vec4_length;
    let indices_length = (skin.indices.len() * mem::size_of::<u16>()) as u32;

    let mut bone_indices = skin.bone_indices.clone();
    for i in 0..vertex_count {
        for j in 0..4 {
            if skin.bone_weights[i][j] == 0.0f32 && skin.bone_indices[i][j] != 0 {
                bone_indices[i][j] = 0;
            }
        }
    }

    let mut indices_padded = any_vec_as_vec_u8(&skin.indices);
    vec_4_byte_padded(&mut indices_padded);

    let total_buffers = vec_2d_to_vec_1d(&[
        any_vec_as_vec_u8(&skin.vertices),
        any_vec_as_vec_u8(&skin.normals),
        any_vec_as_vec_u8(&skin.uvs),
        any_vec_as_vec_u8(&bone_indices),
        any_vec_as_vec_u8(&skin.bone_weights),
        indices_padded,
    ]);

    *buffer_view_offset = total_buffers.len() as u32;

    let mut total_buffers_offset = 0;

    let vertices_buffer_view =
        make_buffer_view(vertices_length, 0, Some(buffer::Target::ArrayBuffer));
    let vertices_accessor = make_accessor(
        vertex_count as u32,
        0,
        *buffer_view_index,
        accessor::Type::Vec3,
        accessor::ComponentType::F32,
        Some(Value::from(skin.bounding_box[0].to_array().to_vec())),
        Some(Value::from(skin.bounding_box[1].to_array().to_vec())),
    );
    *accessor_index += 1;
    *buffer_view_index += 1;

    total_buffers_offset += vertices_length;

    let normals_buffer_view = make_buffer_view(
        vertices_length,
        total_buffers_offset,
        Some(buffer::Target::ArrayBuffer),
    );
    let normals_accessor = make_accessor(
        vertex_count as u32,
        0,
        *buffer_view_index,
        accessor::Type::Vec3,
        accessor::ComponentType::F32,
        None,
        None,
    );
    *accessor_index += 1;
    *buffer_view_index += 1;

    total_buffers_offset += normals_length;

    let uvs_buffer_view = make_buffer_view(
        uvs_length,
        total_buffers_offset,
        Some(buffer::Target::ArrayBuffer),
    );
    let uvs_accessor = make_accessor(
        vertex_count as u32,
        0,
        *buffer_view_index,
        accessor::Type::Vec2,
        accessor::ComponentType::F32,
        None,
        None,
    );
    *accessor_index += 1;
    *buffer_view_index += 1;

    total_buffers_offset += uvs_length;

    let bone_indices_buffer_view = make_buffer_view(
        bone_indices_length,
        total_buffers_offset,
        Some(buffer::Target::ArrayBuffer),
    );
    let bone_indices_accessor = make_accessor(
        vertex_count as u32,
        0,
        *buffer_view_index,
        accessor::Type::Vec4,
        accessor::ComponentType::U16,
        None,
        None,
    );
    *accessor_index += 1;
    *buffer_view_index += 1;

    total_buffers_offset += bone_indices_length;

    let bone_weights_buffer_view = make_buffer_view(
        bone_weights_length,
        total_buffers_offset,
        Some(buffer::Target::ArrayBuffer),
    );
    let bone_weights_accessor = make_accessor(
        vertex_count as u32,
        0,
        *buffer_view_index,
        accessor::Type::Vec4,
        accessor::ComponentType::F32,
        None,
        None,
    );
    *accessor_index += 1;
    *buffer_view_index += 1;

    total_buffers_offset += bone_weights_length;

    let (indices_buffer_view, indices_accessors, primitives) = make_primitives(
        skin,
        texture_selecteds,
        indices_length,
        total_buffers_offset,
        buffer_view_index,
        accessor_index,
    );

    let buffer_views = vec![
        vertices_buffer_view,
        normals_buffer_view,
        uvs_buffer_view,
        bone_indices_buffer_view,
        bone_weights_buffer_view,
        indices_buffer_view,
    ];

    let mut accessors = vec![
        vertices_accessor,
        normals_accessor,
        uvs_accessor,
        bone_indices_accessor,
        bone_weights_accessor,
    ];
    accessors.extend_from_slice(&indices_accessors);

    let mesh = Mesh {
        extensions: None,
        extras: None,
        name: None,
        primitives,
        weights: None,
    };

    (buffer_views, accessors, mesh, total_buffers)
}

fn make_primitives(
    skin: &Skin,
    texture_selecteds: &[usize],
    byte_length: u32,
    byte_offset: u32,
    buffer_view_index: &mut u32,
    accessor_index: &mut u32,
) -> (buffer::View, Vec<Accessor>, Vec<Primitive>) {
    let indices_buffer_view = make_buffer_view(
        byte_length,
        byte_offset,
        Some(buffer::Target::ElementArrayBuffer),
    );
    let indices_buffer_view_index = *buffer_view_index;
    *buffer_view_index += 1;

    let mut primitives = vec![];
    let mut indices_accessors = vec![];

    for i in 0..skin.meshes.len() {
        indices_accessors.push(make_accessor(
            skin.meshes[i].submesh.indices_count,
            skin.meshes[i].submesh.indices_offset * 2,
            indices_buffer_view_index,
            accessor::Type::Scalar,
            accessor::ComponentType::U16,
            None,
            None,
        ));
        primitives.push(Primitive {
            attributes: {
                let mut map = HashMap::new();
                map.insert(Valid(Semantic::Positions), Index::new(0));
                map.insert(Valid(Semantic::Normals), Index::new(1));
                map.insert(Valid(Semantic::TexCoords(0)), Index::new(2));
                map.insert(Valid(Semantic::Joints(0)), Index::new(3));
                map.insert(Valid(Semantic::Weights(0)), Index::new(4));
                map
            },
            extensions: None,
            extras: None,
            indices: Some(Index::new(*accessor_index)),
            material: Some(Index::new(texture_selecteds[i] as u32)),
            mode: Valid(Mode::Triangles),
            targets: None,
        });
        *accessor_index += 1;
    }

    (indices_buffer_view, indices_accessors, primitives)
}

fn make_skeleton(
    model_name: &String,
    skeleton: &Skeleton,
    accessor_index: &mut u32,
    buffer_view_index: &mut u32,
    buffer_view_offset: &mut u32,
) -> (Vec<Node>, GltfSkin, Vec<u8>, buffer::View, Accessor) {
    let mut nodes = vec![];

    for i in 0..skeleton.bones.len() {
        let (scale, rotation, translation) = skeleton.bones[i]
            .local_matrix
            .to_scale_rotation_translation();
        let children = if !skeleton.bones[i].children.is_empty() {
            Some(
                skeleton.bones[i]
                    .children
                    .iter()
                    .map(|i| Index::new(*i as u32))
                    .collect(),
            )
        } else {
            None
        };
        nodes.push(Node {
            camera: None,
            children,
            extensions: None,
            extras: None,
            matrix: None,
            mesh: None,
            name: Some(skeleton.bones[i].name.to_owned()),
            rotation: Some(scene::UnitQuaternion(rotation.to_array())),
            scale: Some(scale.to_array()),
            translation: Some(translation.to_array()),
            skin: None,
            weights: None,
        });
    }

    nodes.push(Node {
        camera: None,
        children: Some(
            skeleton
                .bones
                .iter()
                .filter(|bone| bone.parent_id < 0)
                .map(|bone| Index::new(bone.id as u32))
                .collect(),
        ),
        extensions: None,
        extras: None,
        matrix: None,
        mesh: Some(Index::new(0)),
        name: Some(format!("RootMaster{model_name}")),
        rotation: None,
        scale: None,
        translation: None,
        skin: Some(Index::new(0)),
        weights: None,
    });

    let joints = (0..skeleton.bones.len())
        .map(|i| Index::new(i as u32))
        .collect();

    let ibm_data = any_vec_as_vec_u8(
        &skeleton
            .bones
            .iter()
            .map(|bone| {
                let mut igm = bone.inverse_global_matrix;
                igm.x_axis.w = 0.0f32;
                igm.y_axis.w = 0.0f32;
                igm.z_axis.w = 0.0f32;
                igm.w_axis.w = 1.0f32;
                igm
            })
            .collect(),
    );

    let ibm_buffer_view = make_buffer_view(ibm_data.len() as u32, *buffer_view_offset, None);
    let ibm_accessor = make_accessor(
        skeleton.bones.len() as u32,
        0,
        *buffer_view_index,
        accessor::Type::Mat4,
        accessor::ComponentType::F32,
        None,
        None,
    );
    let ibm_accessor_index = *accessor_index;
    *accessor_index += 1;
    *buffer_view_index += 1;

    *buffer_view_offset += ibm_data.len() as u32;

    let gltf_skin = GltfSkin {
        extensions: None,
        extras: None,
        inverse_bind_matrices: Some(Index::new(ibm_accessor_index)),
        joints,
        name: Some(model_name.to_owned()),
        skeleton: Some(Index::new(skeleton.bones.len() as u32)),
    };

    (nodes, gltf_skin, ibm_data, ibm_buffer_view, ibm_accessor)
}

fn make_material(
    textures_paths: &[String],
    export_path: &String,
    export_as: u8,
    buffer_view_index: &mut u32,
    buffer_view_offset: &mut u32,
) -> (
    Vec<Material>,
    Vec<Texture>,
    Vec<Image>,
    Option<(Vec<u8>, Vec<buffer::View>)>,
) {
    let mut images = vec![];
    let mut textures = vec![];
    let mut materials = vec![];

    let mut buffer_views = vec![];
    let mut total_buffers = vec![];

    let texture_export_path = format!("{export_path}/textures");
    if export_as == 0 {
        fs::create_dir_all(&texture_export_path).expect("Could not create texture export dirs");
    }

    for i in 0..textures_paths.len() {
        let texture_path = Path::new(&textures_paths[i]);

        let image = image::io::Reader::open(texture_path)
            .expect("Could not open image")
            .decode()
            .expect("Could not decode image");

        let mut uri = None;
        let mut buffer_view = None;

        if export_as == 0 {
            let texture_file_name =
                Path::new(&texture_path.file_stem().unwrap()).with_extension("png");
            let texture_save_path =
                format!("{texture_export_path}/{}", texture_file_name.display());

            image
                .save_with_format(texture_save_path, image::ImageFormat::Png)
                .expect("Could not save image");

            uri = Some(format!("textures/{}", texture_file_name.display()))
        } else {
            let mut buffer = vec![];
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            image::ImageEncoder::write_image(
                encoder,
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color(),
            )
            .expect("Could not encode image");

            buffer_views.push(make_buffer_view(
                buffer.len() as u32,
                *buffer_view_offset,
                None,
            ));

            buffer_view = Some(Index::new(*buffer_view_index));
            *buffer_view_index += 1;

            vec_4_byte_padded(&mut buffer);
            *buffer_view_offset += buffer.len() as u32;

            total_buffers.push(buffer);
        }

        images.push(Image {
            buffer_view,
            mime_type: Some(gltf::json::image::MimeType(String::from(
                mime::IMAGE_PNG.as_ref(),
            ))),
            name: None,
            uri,
            extensions: None,
            extras: None,
        });
        textures.push(Texture {
            name: None,
            sampler: Some(Index::new(0)),
            source: Index::new(i as u32),
            extensions: None,
            extras: None,
        });
        materials.push(Material {
            alpha_cutoff: None,
            alpha_mode: Valid(AlphaMode::Opaque),
            double_sided: true,
            name: None,
            pbr_metallic_roughness: material::PbrMetallicRoughness {
                base_color_factor: material::PbrBaseColorFactor([1.0f32, 1.0f32, 1.0f32, 1.0f32]),
                base_color_texture: Some(texture::Info {
                    index: Index::new(i as u32),
                    tex_coord: 0,
                    extensions: None,
                    extras: None,
                }),
                metallic_factor: material::StrengthFactor(0.0f32),
                roughness_factor: material::StrengthFactor(0.0f32),
                metallic_roughness_texture: None,
                extensions: None,
                extras: None,
            },
            normal_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            emissive_factor: material::EmissiveFactor([0.0f32, 0.0f32, 0.0f32]),
            extensions: Some(extensions::material::Material {
                unlit: Some(extensions::material::Unlit {}),
            }),
            extras: None,
        });
    }

    let total_buffers_buffer_views = if export_as == 1 {
        Some((vec_2d_to_vec_1d(&total_buffers), buffer_views))
    } else {
        None
    };

    (materials, textures, images, total_buffers_buffer_views)
}

fn make_buffer_view(
    byte_length: u32,
    byte_offset: u32,
    target: Option<buffer::Target>,
) -> buffer::View {
    buffer::View {
        buffer: Index::new(0),
        byte_length,
        byte_offset: Some(byte_offset),
        byte_stride: None,
        extensions: None,
        extras: None,
        name: None,
        target: target.map(Valid),
    }
}

fn make_accessor(
    count: u32,
    byte_offset: u32,
    buffer_view_index: u32,
    type_: accessor::Type,
    component_type: accessor::ComponentType,
    min: Option<Value>,
    max: Option<Value>,
) -> Accessor {
    Accessor {
        buffer_view: Some(Index::new(buffer_view_index)),
        byte_offset,
        count,
        component_type: Valid(accessor::GenericComponentType(component_type)),
        extensions: None,
        extras: None,
        type_: Valid(type_),
        min,
        max,
        name: None,
        normalized: false,
        sparse: None,
    }
}

fn any_vec_as_vec_u8<T: Clone>(vec: &Vec<T>) -> Vec<u8> {
    let byte_length = vec.len() * mem::size_of::<T>();
    let byte_capacity = vec.capacity() * mem::size_of::<T>();
    let ptr = Box::<[T]>::into_raw(vec.to_vec().into_boxed_slice()) as *mut u8;
    unsafe { Vec::from_raw_parts(ptr, byte_length, byte_capacity) }
}

fn vec_2d_to_vec_1d(vecs: &[Vec<u8>]) -> Vec<u8> {
    let mut new_vec = vec![];
    for vec in vecs {
        new_vec.extend_from_slice(vec);
    }
    new_vec
}

fn vec_4_byte_padded(vec: &mut Vec<u8>) {
    while vec.len() % 4 != 0 {
        vec.push(0);
    }
}
