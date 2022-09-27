extern crate byteorder;
extern crate glob;
extern crate image;
extern crate mime;
extern crate native_dialog;
extern crate serde;

extern crate gl;
extern crate glam;
extern crate glfw;
extern crate gltf;

extern crate imgui;
extern crate imgui_opengl_renderer;

use glfw::{Action, Context, Key};
use native_dialog::FileDialog;
use std::{env, fs::File, io::Read, ops::Div, path::Path, rc::Rc, sync};

mod config_json;

mod export;

mod g3d;
mod gls;
mod lol;

use g3d::{Floor, Joints, Lines, Model, Skybox};
use gls::{ImguiGLFW, Shader, Texture};
use lol::{Animation, Skeleton, Skin};

fn main() {
    let mut json_config = config_json::ConfigJson::read(Path::new("config.json"));

    let mut mind_models: Vec<MindModel> = Vec::with_capacity(json_config.paths.len());

    for i in 0..json_config.paths.len() {
        let mut skn = Skin::read(&read_to_u8(Path::new(&json_config.paths[i].skn)));
        let skl = Skeleton::read(&read_to_u8(Path::new(&json_config.paths[i].skl)));

        skn.apply_skeleton(&skl);

        let bones_transforms = vec![glam::Mat4::IDENTITY; skl.bones.len()];

        let mut show_meshes: Vec<bool> = vec![true; skn.meshes.len()];
        if skn.meshes.len() == json_config.meshes[i].len() {
            show_meshes.copy_from_slice(
                &json_config.meshes[i]
                    .iter()
                    .map(|x| x.show)
                    .collect::<Vec<bool>>(),
            );
        }

        let dds_paths = glob::glob(format!("{}/*.dds", json_config.paths[i].dds).as_str())
            .expect("Failed to read glob dds pattern")
            .filter_map(Result::ok);

        let mut textures_paths = vec![];
        let mut textures_file_names = vec![];

        for path in dds_paths {
            textures_paths.push(path.to_str().unwrap().to_owned());
            textures_file_names.push(path.file_stem().unwrap().to_str().unwrap().to_owned());
        }

        let mut textures_selecteds: Vec<usize> = vec![0; skn.meshes.len()];
        for j in 0..skn.meshes.len() {
            if let Some(mesh_json) = json_config.meshes[i]
                .iter()
                .find(|x| x.name_texture.get(&skn.meshes[j].submesh.name).is_some())
            {
                if let Some(texture_position) = textures_file_names
                    .iter()
                    .position(|x| x == mesh_json.name_texture.iter().next().unwrap().1)
                {
                    textures_selecteds[j] = texture_position;
                }
            }
        }

        let anm_paths = glob::glob(format!("{}/*.anm", json_config.paths[i].anm).as_str())
            .expect("Failed to read glob anm pattern")
            .filter_map(Result::ok);

        let mut animations = vec![];
        let mut animations_file_names = vec![];

        for path in anm_paths {
            animations.push(Animation::read(&read_to_u8(&path)));
            animations_file_names.push(path.file_stem().unwrap().to_str().unwrap().to_owned());
        }

        let animation_selected = if let Some(animation_position) = animations_file_names
            .iter()
            .position(|x| *x == json_config.options[i].selected_animation_path)
        {
            animation_position
        } else {
            0
        };

        mind_models.push(MindModel {
            skn,
            skl,
            show_meshes,
            bones_transforms,
            textures: vec![],
            textures_paths,
            textures_selecteds,
            textures_file_names,
            animation_selected,
            animations,
            animations_file_names,
        });
    }

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("Could not init GLFW");

    glfw.window_hint(glfw::WindowHint::Samples(Some(4)));
    glfw.window_hint(glfw::WindowHint::DoubleBuffer(true));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (mut width, mut height) = (1024i32, 576i32);

    let (mut window, events) = glfw
        .create_window(
            width as u32,
            height as u32,
            format!("MindCorpViewer-Rust v{}", env!("CARGO_PKG_VERSION")).as_str(),
            glfw::WindowMode::Windowed,
        )
        .expect("Could not create GLFW window");

    glfw.with_primary_monitor(|_, monitor| {
        let (xpos, ypos, monitor_width, monitor_height) =
            monitor.expect("Could not get GLFW monitor").get_workarea();
        window.set_pos(
            (monitor_width - xpos) / 2 - width / 2,
            (monitor_height - ypos) / 2 - height / 2,
        );
    });

    window.make_current();

    window.set_key_polling(true);
    window.set_char_polling(true);
    window.set_scroll_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_framebuffer_size_polling(true);

    glfw.set_swap_interval(glfw::SwapInterval::None);

    gl::load_with(|symbol| window.get_proc_address(symbol));

    let mut has_samples = false;
    let mut use_samples = false;
    unsafe {
        let mut samples: gl::types::GLint = 0;
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        if samples >= 1 {
            has_samples = true;
            use_samples = true;
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::MULTISAMPLE);
            gl::Enable(gl::SAMPLE_ALPHA_TO_COVERAGE);
        }
        gl::PointSize(4.0f32);
        gl::Enable(gl::LINE_SMOOTH);
        gl::ClearColor(0.5f32, 0.5f32, 0.5f32, 1.0f32);
    }

    let floor = Floor::new();
    let skybox = Skybox::new();

    let model_shader = Rc::new(Shader::create(
        include_str!("../assets/model/model.vert"),
        include_str!("../assets/model/model.frag"),
    ));
    let model_refs = model_shader.get_refs(&["MVP", "Diffuse", "UseBone"]);
    let model_ubo_ref = model_shader.get_ubo_ref("BonesTransformsBlock");

    let lines_shader = Rc::new(Shader::create(
        include_str!("../assets/lines/lines.vert"),
        include_str!("../assets/lines/lines.frag"),
    ));
    let lines_refs = lines_shader.get_refs(&["MVP"]);

    let joints_shader = Rc::new(Shader::create(
        include_str!("../assets/joints/joints.vert"),
        include_str!("../assets/joints/joints.frag"),
    ));
    let joints_refs = joints_shader.get_refs(&["MVP"]);

    let mut models: Vec<Model> = Vec::with_capacity(mind_models.len());
    let mut lines: Vec<Lines> = Vec::with_capacity(mind_models.len());
    let mut joints: Vec<Joints> = Vec::with_capacity(mind_models.len());

    for i in 0..mind_models.len() {
        let mind_model = &mut mind_models[i];
        let skl_bones_count = mind_model.skl.bones.len();

        let mut model = Model::new();
        let mut line = Lines::new();
        let mut joint = Joints::new();

        model.load(&mind_model.skn, skl_bones_count, Rc::clone(&model_shader));
        line.load(&mind_model.skl, Rc::clone(&lines_shader));
        joint.load(&mind_model.skl, Rc::clone(&joints_shader));

        model.set_shader_refs(&model_refs);
        model.bind_ubo(model_ubo_ref, skl_bones_count);

        line.set_shader_refs(&lines_refs);
        joint.set_shader_refs(&joints_refs);

        models.push(model);
        lines.push(line);
        joints.push(joint);

        for path in mind_model.textures_paths.iter() {
            mind_model
                .textures
                .push(Texture::load_texture(&read_to_u8(Path::new(path))));
        }
    }

    let mut imgui = imgui::Context::create();

    imgui.set_ini_filename(None);

    let style = imgui.style_mut();
    style.use_dark_colors();
    style.grab_rounding = 6.0f32;
    style.frame_rounding = 8.0f32;
    style.window_rounding = 8.0f32;
    style.frame_border_size = 1.0f32;
    style.window_border_size = 1.0f32;
    style.indent_spacing = style.frame_padding[0] * 3.0f32 - 2.0f32;
    style.window_menu_button_position = imgui::Direction::Right;

    imgui.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../assets/fonts/consola.ttf"),
            size_pixels: 13.0f32,
            config: None,
        },
        imgui::FontSource::TtfData {
            data: include_bytes!("../assets/fonts/fa-regular-400.ttf"),
            size_pixels: 13.0f32,
            config: Some(imgui::FontConfig {
                glyph_ranges: imgui::FontGlyphRanges::from_slice(&[0xf000, 0xf3ff, 0]),
                ..Default::default()
            }),
        },
    ]);

    let mut imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

    let mut frames = 0.0f32;
    let mut last_time = 0.0f32;
    let mut last_time_fps = 0.0f32;

    let center_y = if mind_models.len() > 0 {
        mind_models
            .iter()
            .map(|mind_model| mind_model.skn.center.y)
            .sum::<f32>()
            .div(mind_models.len() as f32)
    } else {
        0.0f32
    };

    let fov = 45.0f32.to_radians();
    let mut translation = glam::vec3(0.0f32, -center_y, 0.0f32);
    let mut yaw_pitch = glam::vec2(90.0f32, 70.0f32);

    let mut mouse = Mouse::new(500.0f32, [width as f32 / 2.0f32, height as f32 / 2.0f32]);

    let mut export_as = 0;

    let working_dir = env::current_dir().expect("Could not get current dir");

    let mut add_model_name = String::new();
    let mut add_model_skn = String::new();
    let mut add_model_skl = String::new();
    let mut add_model_dds = String::new();
    let mut add_model_anm = String::new();

    while !window.should_close() {
        let current_time = glfw.get_time() as f32;
        let delta_time_fps = current_time - last_time_fps;

        frames += 1.0f32;
        if delta_time_fps >= 1.0f32 {
            window.set_title(
                format!(
                    "MindCorpViewer-Rust - v{} - Fps: {:1.0} / Ms: {:1.3}",
                    env!("CARGO_PKG_VERSION"),
                    frames / delta_time_fps,
                    1000.0f32 / frames
                )
                .as_str(),
            );
            frames = 0.0f32;
            last_time_fps = current_time;
        }

        let delta_time = current_time - last_time;
        last_time = current_time;

        glfw.poll_events();

        process_events(
            &events,
            &mut window,
            &mut imgui_glfw,
            &mut imgui,
            &mut width,
            &mut height,
            &mut mouse,
        );

        let ui = imgui_glfw.frame(delta_time, &window, &mut imgui);

        imgui::Window::new("Main")
            .position([4.0f32, 4.0f32], imgui::Condition::Once)
            .bring_to_front_on_focus(false)
            .always_auto_resize(true)
            .build(&ui, || {
                if has_samples && ui.checkbox("Use MSAA", &mut use_samples) {
                    match use_samples {
                        true => unsafe {
                            gl::Enable(gl::BLEND);
                            gl::Enable(gl::MULTISAMPLE);
                            gl::Enable(gl::SAMPLE_ALPHA_TO_COVERAGE);
                        },
                        false => unsafe {
                            gl::Disable(gl::BLEND);
                            gl::Disable(gl::MULTISAMPLE);
                            gl::Disable(gl::SAMPLE_ALPHA_TO_COVERAGE);
                        },
                    }
                }
                if ui.checkbox("Enable Vsync", &mut json_config.vsync) {
                    glfw.set_swap_interval(match json_config.vsync {
                        true => glfw::SwapInterval::Sync(1),
                        false => glfw::SwapInterval::None,
                    });
                }
                ui.checkbox("Show Floor", &mut json_config.show_floor);
                ui.checkbox("Show Skybox", &mut json_config.show_skybox);
                ui.checkbox("Synchronized Time", &mut json_config.synchronized_time);
                if ui.is_item_hovered() {
                    ui.tooltip(|| {
                        ui.text("Synchronize all models to first model");
                    });
                }
                ui.separator();
                for i in 0..mind_models.len() {
                    let mind_model = &mut mind_models[i];

                    let _model_id = ui.push_id(i as i32);
                    ui.align_text_to_frame_padding();
                    ui.checkbox("##show", &mut json_config.options[i].show);
                    if ui.is_item_hovered() {
                        ui.tooltip(|| {
                            ui.text("Show / Hide model");
                        });
                    }
                    ui.same_line();
                    let tree_node = imgui::TreeNode::new(json_config.paths[i].name.to_owned())
                        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                        .flags(imgui::TreeNodeFlags::ALLOW_ITEM_OVERLAP)
                        .framed(true)
                        .push(&ui);
                    ui.same_line_with_pos(
                        ui.window_content_region_width() - ui.calc_text_size("\u{F014}")[0],
                    );
                    if confirm_delete_button(&ui) {
                        lines.remove(i);
                        joints.remove(i);
                        models.remove(i);
                        mind_models.remove(i);
                        json_config.paths.remove(i);
                        json_config.options.remove(i);
                        json_config.meshes.remove(i);
                        break;
                    }
                    if let Some(_node) = tree_node {
                        let options = &mut json_config.options[i];

                        ui.checkbox("Show Wireframe", &mut options.show_wireframe);
                        ui.checkbox("Show Skeleton Bones", &mut options.show_skeleton_bones);
                        ui.checkbox("Show Skeleton Joints", &mut options.show_skeleton_joints);
                        imgui::TreeNode::new("Animations")
                            .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                            .framed(true)
                            .build(&ui, || {
                                ui.checkbox("Use Animation", &mut options.use_animation);
                                ui.checkbox("Play / Stop", &mut options.play_animation);
                                ui.checkbox("Loop Animation", &mut options.loop_animation);
                                ui.checkbox("Next Animation", &mut options.next_animation);
                                ui.text("CTRL+Click Change To Input");
                                imgui::Slider::new("Speed", 0.00001f32, 10.0f32)
                                    .display_format("%.5f")
                                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                    .build(&ui, &mut options.animation_speed);
                                imgui::Slider::new(
                                    "Time",
                                    0.0f32,
                                    mind_model.animations[mind_model.animation_selected].duration,
                                )
                                .display_format("%.5f")
                                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                .build(&ui, &mut options.animation_time);
                                ui.combo_simple_string(
                                    "Animations",
                                    &mut mind_model.animation_selected,
                                    &mind_model.animations_file_names,
                                );
                            });
                        imgui::TreeNode::new("Meshes")
                            .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                            .framed(true)
                            .build(&ui, || {
                                for i in 0..mind_model.skn.meshes.len() {
                                    let _meshes_id = ui.push_id(i as i32);
                                    ui.checkbox(
                                        mind_model.skn.meshes[i].submesh.name.as_str(),
                                        &mut mind_model.show_meshes[i],
                                    );
                                    if mind_model.show_meshes[i] {
                                        ui.combo_simple_string(
                                            "##texture",
                                            &mut mind_model.textures_selecteds[i],
                                            &mind_model.textures_file_names,
                                        );
                                        imgui::Image::new(
                                            imgui::TextureId::new(
                                                mind_model.textures
                                                    [mind_model.textures_selecteds[i]]
                                                    .id
                                                    as usize,
                                            ),
                                            [64.0f32, 64.0f32],
                                        )
                                        .build(&ui);
                                    }
                                }
                            });
                        imgui::TreeNode::new("Export")
                            .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                            .framed(true)
                            .build(&ui, || {
                                ui.radio_button("Export as gltf", &mut export_as, 0);
                                ui.radio_button("Export as glb", &mut export_as, 1);
                                if ui.button("Export Model") {
                                    export::export_model(
                                        export_as,
                                        &json_config.paths[i].name,
                                        &mind_model,
                                    );
                                }
                            });
                    }
                }
                ui.separator();
                imgui::TreeNode::new("Add Model")
                    .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                    .framed(true)
                    .build(&ui, || {
                        ui.align_text_to_frame_padding();
                        ui.text("Name:");
                        ui.same_line();
                        ui.input_text("##name", &mut add_model_name).build();
                        ui.align_text_to_frame_padding();
                        ui.text("SKN: ");
                        ui.same_line();
                        ui.input_text("##skn", &mut add_model_skn).build();
                        ui.same_line();
                        if ui.button("Select##1") {
                            let file_dialog_path = FileDialog::new()
                                .set_location(&working_dir)
                                .add_filter("SKN", &["skn"])
                                .show_open_single_file()
                                .unwrap();
                            if let Some(path) = file_dialog_path {
                                add_model_skn.clear();
                                add_model_skn.insert_str(0, path.to_str().unwrap());
                            }
                        }
                        ui.align_text_to_frame_padding();
                        ui.text("SKL: ");
                        ui.same_line();
                        ui.input_text("##skl", &mut add_model_skl).build();
                        ui.same_line();
                        if ui.button("Select##2") {
                            let file_dialog_path = FileDialog::new()
                                .set_location(&working_dir)
                                .add_filter("SKL", &["skl"])
                                .show_open_single_file()
                                .unwrap();
                            if let Some(path) = file_dialog_path {
                                add_model_skl.clear();
                                add_model_skl.insert_str(0, path.to_str().unwrap());
                            }
                        }
                        ui.align_text_to_frame_padding();
                        ui.text("DDS: ");
                        ui.same_line();
                        ui.input_text("##dds", &mut add_model_dds).build();
                        ui.same_line();
                        if ui.button("Select##3") {
                            let path = FileDialog::new()
                                .set_location(&working_dir)
                                .add_filter("DDS", &["dds"])
                                .show_open_single_dir()
                                .unwrap();
                            if let Some(path) = path {
                                add_model_dds.clear();
                                add_model_dds.insert_str(0, path.to_str().unwrap());
                            }
                        }
                        ui.align_text_to_frame_padding();
                        ui.text("ANM: ");
                        ui.same_line();
                        ui.input_text("##anm", &mut add_model_anm).build();
                        ui.same_line();
                        if ui.button("Select##4") {
                            let file_dialog_path = FileDialog::new()
                                .set_location(&working_dir)
                                .add_filter("ANM", &["anm"])
                                .show_open_single_dir()
                                .unwrap();
                            if let Some(path) = file_dialog_path {
                                add_model_anm.clear();
                                add_model_anm.insert_str(0, path.to_str().unwrap());
                            }
                        }
                        if ui.button_with_size("Add", [ui.content_region_avail()[0], 0.0f32]) {
                            json_config.paths.push(config_json::PathJson {
                                name: add_model_name.to_owned(),
                                skn: add_model_skn.to_owned(),
                                skl: add_model_skl.to_owned(),
                                dds: add_model_dds.to_owned(),
                                anm: add_model_anm.to_owned(),
                            });
                            json_config.options.push(config_json::OptionsJson::new());
                            json_config.meshes.push(vec![]);

                            let mut skn = Skin::read(&read_to_u8(Path::new(&add_model_skn)));
                            let skl = Skeleton::read(&read_to_u8(Path::new(&add_model_skl)));

                            skn.apply_skeleton(&skl);
                            let skl_bones_count = skl.bones.len();

                            let bones_transforms = vec![glam::Mat4::IDENTITY; skl_bones_count];
                            let show_meshes = vec![true; skn.meshes.len()];

                            let dds_paths = glob::glob(format!("{}/*.dds", add_model_dds).as_str())
                                .expect("Failed to read glob dds pattern")
                                .filter_map(Result::ok);

                            let mut textures_paths = vec![];
                            let mut textures_file_names = vec![];

                            for path in dds_paths {
                                textures_paths.push(path.to_str().unwrap().to_owned());
                                textures_file_names
                                    .push(path.file_stem().unwrap().to_str().unwrap().to_owned());
                            }

                            let mut textures = vec![];
                            for path in textures_paths.iter() {
                                textures.push(Texture::load_texture(&read_to_u8(Path::new(path))));
                            }

                            let textures_selecteds = vec![0; skn.meshes.len()];

                            let anm_paths = glob::glob(format!("{}/*.anm", add_model_anm).as_str())
                                .expect("Failed to read glob anm pattern")
                                .filter_map(Result::ok);

                            let mut animations = vec![];
                            let mut animations_file_names = vec![];

                            for path in anm_paths {
                                animations.push(Animation::read(&read_to_u8(&path)));
                                animations_file_names
                                    .push(path.file_stem().unwrap().to_str().unwrap().to_owned());
                            }

                            let animation_selected = 0;

                            let mut model = Model::new();
                            let mut line = Lines::new();
                            let mut joint = Joints::new();

                            model.load(&skn, skl_bones_count, Rc::clone(&model_shader));
                            line.load(&skl, Rc::clone(&lines_shader));
                            joint.load(&skl, Rc::clone(&joints_shader));

                            model.set_shader_refs(&model_refs);
                            model.bind_ubo(model_ubo_ref, skl_bones_count);

                            line.set_shader_refs(&lines_refs);
                            joint.set_shader_refs(&joints_refs);

                            models.push(model);
                            lines.push(line);
                            joints.push(joint);

                            mind_models.push(MindModel {
                                skn,
                                skl,
                                show_meshes,
                                bones_transforms,
                                textures,
                                textures_paths,
                                textures_selecteds,
                                textures_file_names,
                                animation_selected,
                                animations,
                                animations_file_names,
                            });

                            add_model_name.clear();
                            add_model_skn.clear();
                            add_model_skl.clear();
                            add_model_dds.clear();
                            add_model_anm.clear();
                        }
                    });
                ui.separator();
                if ui.button_with_size("Save Configuration", [ui.content_region_avail()[0], 0.0f32])
                {
                    json_config.write(&mind_models);
                }
            });

        let view_matrix = compute_matrix_from_inputs(&mut translation, &mut yaw_pitch, &mut mouse);
        let projection_matrix =
            glam::Mat4::perspective_infinite_rh(fov, width as f32 / height as f32, 0.1f32);
        let projection_view_matrix = projection_matrix * view_matrix;

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        if json_config.show_skybox {
            skybox.render(&view_matrix, &projection_matrix);
        }

        if json_config.show_floor {
            floor.render(&projection_view_matrix);
        }

        for i in 0..mind_models.len() {
            let animation_synchronized_time = if json_config.synchronized_time && i != 0 {
                Some(json_config.options[0].animation_time)
            } else {
                None
            };
            let options = &mut json_config.options[i];

            if options.show {
                let mind_model = &mut mind_models[i];

                play_animation(options, mind_model, delta_time, animation_synchronized_time);

                models[i].render(&options, &projection_view_matrix, mind_model);

                if options.show_skeleton_bones {
                    lines[i].render(options.use_animation, &projection_view_matrix, mind_model);
                }

                if options.show_skeleton_joints {
                    joints[i].render(
                        options.use_animation,
                        use_samples,
                        &projection_view_matrix,
                        mind_model,
                    );
                }
            }
        }

        unsafe {
            gl::Disable(gl::MULTISAMPLE);
            imgui_glfw.draw(ui, &mut window);
            gl::Enable(gl::MULTISAMPLE);
        }

        window.swap_buffers();
    }
}

pub struct MindModel {
    pub skn: Skin,
    pub skl: Skeleton,

    pub show_meshes: Vec<bool>,
    pub bones_transforms: Vec<glam::Mat4>,

    pub textures: Vec<Texture>,
    pub textures_paths: Vec<String>,
    pub textures_selecteds: Vec<usize>,
    pub textures_file_names: Vec<String>,

    pub animation_selected: usize,
    pub animations: Vec<Animation>,
    pub animations_file_names: Vec<String>,
}

struct Mouse {
    last_offset: [f32; 2],
    last_pos: [f32; 2],
    offset: [f32; 2],
    state: u8,
    zoom: f32,
}

impl Mouse {
    fn new(zoom: f32, last: [f32; 2]) -> Mouse {
        Mouse {
            last_offset: last,
            last_pos: last,
            offset: [0.0f32, 0.0f32],
            state: 0u8,
            zoom,
        }
    }
}

fn process_events(
    events: &sync::mpsc::Receiver<(f64, glfw::WindowEvent)>,
    window: &mut glfw::Window,
    imgui_glfw: &mut ImguiGLFW,
    imgui: &mut imgui::Context,
    width: &mut i32,
    height: &mut i32,
    mouse: &mut Mouse,
) {
    for (_, event) in glfw::flush_messages(events) {
        imgui_glfw.handle_event(imgui, &event);
        match event {
            glfw::WindowEvent::FramebufferSize(frame_width, frame_height) => unsafe {
                gl::Viewport(0, 0, frame_width, frame_height);
                *width = frame_width;
                *height = frame_height;
            },
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                window.set_should_close(true)
            }
            glfw::WindowEvent::Close => window.set_should_close(true),
            glfw::WindowEvent::MouseButton(button, action, _) => {
                if (action == Action::Press || action == Action::Repeat)
                    && imgui_no_window_hovered()
                {
                    if button == glfw::MouseButtonLeft {
                        mouse.state = 1;
                    } else if button == glfw::MouseButtonRight {
                        mouse.state = 2;
                    }
                }
                if action == Action::Release
                    && (button == glfw::MouseButtonLeft || button == glfw::MouseButtonRight)
                {
                    mouse.state = 0;
                }
            }
            glfw::WindowEvent::CursorPos(xpos, ypos) => {
                let (xpos, ypos) = (xpos as f32, ypos as f32);

                mouse.offset[0] = xpos - mouse.last_pos[0];
                mouse.offset[1] = ypos - mouse.last_pos[1];

                mouse.last_pos[0] = xpos;
                mouse.last_pos[1] = ypos;
            }
            glfw::WindowEvent::Scroll(_, yoffset) => {
                if imgui_no_window_hovered() {
                    mouse.zoom -= yoffset as f32 * 30.0f32;
                    if mouse.zoom < 1.0f32 {
                        mouse.zoom = 1.0f32;
                    }
                }
            }
            _ => {}
        }
    }
}

fn compute_matrix_from_inputs(
    translation: &mut glam::Vec3,
    yaw_pitch: &mut glam::Vec2,
    mouse: &mut Mouse,
) -> glam::Mat4 {
    if mouse.state == 1 {
        if mouse.offset[0] != mouse.last_offset[0] {
            yaw_pitch[0] += mouse.offset[0] * 0.5f32;
        }
        if mouse.offset[1] != mouse.last_offset[1] {
            yaw_pitch[1] -= mouse.offset[1] * 0.5f32;
        }
        if yaw_pitch[0] > 360.0f32 {
            yaw_pitch[0] = 0.0f32
        } else if yaw_pitch[0] < -360.0f32 {
            yaw_pitch[0] = 0.0f32
        }
        if yaw_pitch[1] > 179.0f32 {
            yaw_pitch[1] = 179.0f32;
        } else if yaw_pitch[1] < 1.0f32 {
            yaw_pitch[1] = 1.0f32;
        }
    }

    let position = glam::vec3(
        yaw_pitch[1].to_radians().sin() * yaw_pitch[0].to_radians().cos(),
        yaw_pitch[1].to_radians().cos(),
        yaw_pitch[1].to_radians().sin() * yaw_pitch[0].to_radians().sin(),
    )
    .normalize();

    let right = position.cross(glam::Vec3::Y).normalize();
    let up = right.cross(position).normalize();

    if mouse.state == 2 {
        if mouse.offset[0] != mouse.last_offset[0] {
            translation.x -= right.x * (mouse.offset[0] * 0.35f32);
            translation.z -= right.z * (mouse.offset[0] * 0.35f32);
        }
        if mouse.offset[1] != mouse.last_offset[1] {
            translation.y -= mouse.offset[1] * 0.35f32;
        }
    }

    mouse.last_offset[0] = mouse.offset[0];
    mouse.last_offset[1] = mouse.offset[1];

    let mut viewmatrix = glam::Mat4::look_at_rh(position * mouse.zoom, glam::Vec3::ZERO, up);

    viewmatrix *= glam::Mat4::from_translation(*translation)
        * glam::Mat4::from_scale(glam::vec3(-1.0f32, 1.0f32, 1.0f32));

    viewmatrix
}

fn play_animation(
    options: &mut config_json::OptionsJson,
    mind_model: &mut MindModel,
    delta_time: f32,
    animation_synchronized_time: Option<f32>,
) {
    if options.use_animation {
        if options.play_animation {
            if options.animation_time
                < mind_model.animations[mind_model.animation_selected].duration
            {
                options.animation_time += delta_time * options.animation_speed;
            } else if options.next_animation {
                mind_model.animation_selected += 1;
                if mind_model.animation_selected == mind_model.animations.len() {
                    mind_model.animation_selected = 0;
                }
                options.animation_time = 0.0f32;
            } else if options.loop_animation {
                options.animation_time = 0.0f32;
            }
        }
        if let Some(animation_time) = animation_synchronized_time {
            options.animation_time = animation_time;
        }
        lol::anm::run_animation(
            &mut mind_model.bones_transforms,
            &mind_model.animations[mind_model.animation_selected],
            &mind_model.skl,
            options.animation_time,
        );
    }
}

fn confirm_delete_button(ui: &imgui::Ui) -> bool {
    let delete_button = ui.button("\u{F014}");
    if ui.is_item_hovered() {
        ui.tooltip(|| {
            ui.text("Delete item?");
        });
    }
    if delete_button {
        ui.open_popup("##deletepopup");
    }
    let mut delete = false;
    ui.popup("##deletepopup", || {
        ui.text("Are you sure?");
        if ui.button("Yes") {
            ui.close_current_popup();
            delete = true;
        }
        ui.same_line();
        if ui.button("No") {
            ui.close_current_popup();
        }
    });
    delete
}

fn imgui_no_window_hovered() -> bool {
    unsafe { !imgui::sys::igIsWindowHovered(imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32) }
}

fn read_to_u8(path: &Path) -> Vec<u8> {
    let mut file = File::open(path).expect("Could not open file");
    let mut contents: Vec<u8> = vec![];
    println!("Reading file: {}", path.to_str().unwrap());
    file.read_to_end(&mut contents)
        .expect("Could not read file");
    contents
}
