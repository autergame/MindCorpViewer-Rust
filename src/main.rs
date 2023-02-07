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

use glfw::{Action, Context, Key};
use std::{
    env,
    fs::File,
    io::Read,
    ops::{Div, Neg},
    path::Path,
    rc::Rc,
    sync,
};

mod config_json;

mod export;

mod g3d;
mod gls;
mod lol;

use g3d::{Floor, Joints, Lines, Model, Skybox};
use gls::{
    imgui_layout::{self, AddModel},
    ImguiGLFW, Screenshot, Shader, Texture,
};
use lol::{Animation, Skeleton, Skin};

fn main() {
    let cargo_pkg_version = env!("CARGO_PKG_VERSION");
    let working_dir = env::current_dir().expect("Could not get current dir");

    let mut json_config = config_json::ConfigJson::read(Path::new("config.json"));

    let mut mind_models: Vec<MindModel> = Vec::with_capacity(json_config.paths.len());

    for i in 0..json_config.paths.len() {
        mind_models.push(load_mind_model(
            &json_config.paths[i],
            &json_config.options[i],
            &json_config.meshes[i],
        ));
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

    let (mut window_width, mut window_height) = (1024i32, 576i32);

    let (mut window, events) = glfw
        .create_window(
            window_width as u32,
            window_height as u32,
            format!("MindCorpViewer-Rust v{}", cargo_pkg_version).as_str(),
            glfw::WindowMode::Windowed,
        )
        .expect("Could not create GLFW window");

    glfw.with_primary_monitor(|_, monitor| {
        let (xpos, ypos, monitor_width, monitor_height) =
            monitor.expect("Could not get GLFW monitor").get_workarea();
        window.set_pos(
            (monitor_width - xpos) / 2 - window_width / 2,
            (monitor_height - ypos) / 2 - window_height / 2,
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

    for mind_model in &mut mind_models {
        let skl_bones_count = mind_model.skl.bones.len();

        let mut model = Model::create(&mind_model.skn, skl_bones_count, Rc::clone(&model_shader));
        let mut line = Lines::create(&mind_model.skl, Rc::clone(&lines_shader));
        let mut joint = Joints::create(&mind_model.skl, Rc::clone(&joints_shader));

        model.set_shader_refs(&model_refs, model_ubo_ref);
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

    let mut resolution = [1280, 720];
    let mut screenshot = Screenshot::new(use_samples, resolution);

    let mut imgui_ctx = imgui::Context::create();

    imgui_ctx.set_ini_filename(None);

    let style = imgui_ctx.style_mut();
    style.use_dark_colors();

    style.grab_rounding = 6.0f32;
    style.frame_rounding = 8.0f32;
    style.window_rounding = 8.0f32;
    style.frame_border_size = 1.0f32;
    style.window_border_size = 2.0f32;
    style.indent_spacing = style.frame_padding[0] * 3.0f32 - 2.0f32;
    style.window_menu_button_position = imgui::Direction::Right;

    imgui_ctx.fonts().add_font(&[
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

    let mut imgui_glfw = ImguiGLFW::new(&mut imgui_ctx);

    let mut frames = 0.0f32;
    let mut last_time = 0.0f32;
    let mut last_time_fps = 0.0f32;

    let center_y = if !mind_models.is_empty() {
        mind_models
            .iter()
            .map(|mind_model| mind_model.skn.center.y)
            .sum::<f32>()
            .div(mind_models.len() as f32)
            .neg()
    } else {
        0.0f32
    };

    let fov = 45.0f32.to_radians();
    let mut yaw_pitch = glam::vec2(90.0f32, 70.0f32);
    let mut translation = glam::vec3(0.0f32, center_y, 0.0f32);

    let mut mouse = Mouse::new(500.0f32, [0.0f32, 0.0f32]);

    let mut export_as = 0;
    let mut take_screenshot = false;

    let mut add_model = AddModel {
        name: String::new(),
        skn: String::new(),
        skl: String::new(),
        dds: String::new(),
        anm: String::new(),
    };

    while !window.should_close() {
        let current_time = glfw.get_time() as f32;
        let delta_time_fps = current_time - last_time_fps;

        frames += 1.0f32;
        if delta_time_fps >= 1.0f32 {
            window.set_title(
                format!(
                    "MindCorpViewer-Rust v{} - Fps: {:1.0} / Ms: {:1.3}",
                    cargo_pkg_version,
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
            &mut imgui_ctx,
            &mut window_width,
            &mut window_height,
            &mut mouse,
        );

        imgui_glfw.update_imgui(delta_time, &window, &mut imgui_ctx);

        let ui = imgui_ctx.new_frame();

        ui.window("Main")
            .position([4.0f32, 4.0f32], imgui::Condition::Once)
            .bring_to_front_on_focus(false)
            .always_auto_resize(true)
            .build(|| {
                imgui_layout::settings(
                    ui,
                    &mut glfw,
                    has_samples,
                    &mut use_samples,
                    &mut json_config,
                );

                ui.separator();

                for i in 0..mind_models.len() {
                    let _model_id = ui.push_id_usize(i);

                    ui.align_text_to_frame_padding();
                    ui.checkbox("##show", &mut json_config.options[i].show);
                    if ui.is_item_hovered() {
                        ui.tooltip(|| {
                            ui.text("Show / Hide model");
                        });
                    }
                    ui.same_line_with_spacing(0.0f32, 3.0f32);
                    if imgui_layout::confirm_delete_button(ui) {
                        lines.remove(i);
                        joints.remove(i);
                        models.remove(i);
                        mind_models.remove(i);
                        json_config.paths.remove(i);
                        json_config.options.remove(i);
                        json_config.meshes.remove(i);
                        break;
                    }
                    ui.same_line_with_spacing(0.0f32, 6.0f32);
                    if let Some(_tree) = ui
                        .tree_node_config(&json_config.paths[i].name)
                        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                        .flags(imgui::TreeNodeFlags::ALLOW_ITEM_OVERLAP)
                        .framed(true)
                        .push()
                    {
                        {
                            imgui_layout::model(
                                ui,
                                &mut json_config.options[i],
                                &mut mind_models[i],
                                &mut export_as,
                                &json_config.paths[i].name,
                            );
                        }
                    }
                }

                ui.separator();

                imgui_layout::add_model(ui, &working_dir, &mut add_model, |add_model| {
                    let mut skn = Skin::read(&read_to_u8(Path::new(&add_model.skn)));
                    let skl = Skeleton::read(&read_to_u8(Path::new(&add_model.skl)));

                    skn.apply_skeleton(&skl);

                    let bones_transforms = vec![glam::Mat4::IDENTITY; skl.bones.len()];
                    let show_meshes = vec![true; skn.meshes.len()];

                    let dds_paths = glob::glob(format!("{}/*.dds", add_model.dds).as_str())
                        .expect("Failed to read glob dds pattern")
                        .filter_map(Result::ok);

                    let mut textures_paths = vec![];
                    let mut textures_file_names = vec![];

                    for path in dds_paths {
                        textures_paths.push(String::from(path.to_str().unwrap()));
                        textures_file_names
                            .push(String::from(path.file_stem().unwrap().to_str().unwrap()));
                    }

                    let mut textures = vec![];
                    for path in textures_paths.iter() {
                        textures.push(Texture::load_texture(&read_to_u8(Path::new(path))));
                    }

                    let textures_selecteds = vec![0; skn.meshes.len()];

                    let anm_paths = glob::glob(format!("{}/*.anm", add_model.anm).as_str())
                        .expect("Failed to read glob anm pattern")
                        .filter_map(Result::ok);

                    let mut animations = vec![];
                    let mut animations_file_names = vec![];

                    for path in anm_paths {
                        animations.push(Animation::read(&read_to_u8(&path)));
                        animations_file_names
                            .push(String::from(path.file_stem().unwrap().to_str().unwrap()));
                    }

                    let animation_selected = 0;

                    let mut model = Model::create(&skn, skl.bones.len(), Rc::clone(&model_shader));
                    let mut line = Lines::create(&skl, Rc::clone(&lines_shader));
                    let mut joint = Joints::create(&skl, Rc::clone(&joints_shader));

                    model.set_shader_refs(&model_refs, model_ubo_ref);
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

                    if add_model.name.is_empty() {
                        add_model.name = String::from("model");
                    }

                    json_config.paths.push(config_json::PathJson {
                        name: add_model.name.to_owned(),
                        skn: add_model.skn.to_owned(),
                        skl: add_model.skl.to_owned(),
                        dds: add_model.dds.to_owned(),
                        anm: add_model.anm.to_owned(),
                    });
                    json_config.options.push(config_json::OptionsJson::new());
                    json_config.meshes.push(vec![]);
                });

                ui.separator();

                imgui_layout::screenshot(
                    ui,
                    use_samples,
                    &mut take_screenshot,
                    &mut resolution,
                    &mut screenshot,
                );

                ui.separator();

                if ui.button_with_size("Save Configuration", [ui.content_region_avail()[0], 0.0f32])
                {
                    json_config.write(&mind_models);
                }
            });

        let view_matrix = compute_matrix_from_inputs(&mut translation, &mut yaw_pitch, &mut mouse);
        let projection_matrix = glam::Mat4::perspective_infinite_rh(
            fov,
            window_width as f32 / window_height as f32,
            0.1f32,
        );
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

        let projection_view_matrix_mod = if take_screenshot {
            screenshot.take(fov) * view_matrix
        } else {
            projection_view_matrix
        };

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

                models[i].render(options, &projection_view_matrix_mod, mind_model);

                if !take_screenshot {
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
        }

        if take_screenshot {
            take_screenshot = false;
            screenshot.save([window_width, window_height]);
        }

        unsafe {
            gl::Disable(gl::MULTISAMPLE);
            imgui_glfw.draw(&mut imgui_ctx, &mut window);
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

fn load_mind_model(
    json_config_path: &config_json::PathJson,
    json_config_options: &config_json::OptionsJson,
    json_config_meshes: &[config_json::MeshJson],
) -> MindModel {
    let mut skn = Skin::read(&read_to_u8(Path::new(&json_config_path.skn)));
    let skl = Skeleton::read(&read_to_u8(Path::new(&json_config_path.skl)));

    skn.apply_skeleton(&skl);

    let bones_transforms = vec![glam::Mat4::IDENTITY; skl.bones.len()];

    let mut show_meshes: Vec<bool> = vec![true; skn.meshes.len()];
    if skn.meshes.len() == json_config_meshes.len() {
        show_meshes.copy_from_slice(
            &json_config_meshes
                .iter()
                .map(|x| x.show)
                .collect::<Vec<bool>>(),
        );
    }

    let dds_paths = glob::glob(format!("{}/*.dds", json_config_path.dds).as_str())
        .expect("Failed to read glob dds pattern")
        .filter_map(Result::ok);

    let mut textures_paths = vec![];
    let mut textures_file_names = vec![];

    for path in dds_paths {
        textures_paths.push(String::from(path.to_str().unwrap()));
        textures_file_names.push(String::from(path.file_stem().unwrap().to_str().unwrap()));
    }

    let mut textures_selecteds: Vec<usize> = vec![0; skn.meshes.len()];
    for j in 0..skn.meshes.len() {
        if let Some(mesh_json) = json_config_meshes
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

    let anm_paths = glob::glob(format!("{}/*.anm", json_config_path.anm).as_str())
        .expect("Failed to read glob anm pattern")
        .filter_map(Result::ok);

    let mut animations = vec![];
    let mut animations_file_names = vec![];

    for path in anm_paths {
        animations.push(Animation::read(&read_to_u8(&path)));
        animations_file_names.push(String::from(path.file_stem().unwrap().to_str().unwrap()));
    }

    let animation_selected = if let Some(animation_position) = animations_file_names
        .iter()
        .position(|x| *x == json_config_options.selected_animation_path)
    {
        animation_position
    } else {
        0
    };

    MindModel {
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
    }
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
    imgui_ctx: &mut imgui::Context,
    window_width: &mut i32,
    window_height: &mut i32,
    mouse: &mut Mouse,
) {
    for (_, event) in glfw::flush_messages(events) {
        imgui_glfw.handle_event(imgui_ctx, &event);
        match event {
            glfw::WindowEvent::FramebufferSize(frame_width, frame_height) => unsafe {
                gl::Viewport(0, 0, frame_width, frame_height);
                *window_width = frame_width;
                *window_height = frame_height;
            },
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                window.set_should_close(true)
            }
            glfw::WindowEvent::Close => window.set_should_close(true),
            glfw::WindowEvent::MouseButton(button, action, _) => {
                if (action == Action::Press || action == Action::Repeat)
                    && imgui_layout::no_window_hovered()
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
                if imgui_layout::no_window_hovered() {
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
        if yaw_pitch[0] > 360.0f32 || yaw_pitch[0] < -360.0f32 {
            yaw_pitch[0] = 0.0f32
        }
        yaw_pitch[1] = yaw_pitch[1].clamp(1.0f32, 179.0f32);
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

fn read_to_u8(path: &Path) -> Vec<u8> {
    let mut file = File::open(path).expect("Could not open file");
    let mut contents: Vec<u8> = vec![];
    println!("Reading file: {}", path.to_str().unwrap());
    file.read_to_end(&mut contents)
        .expect("Could not read file");
    contents
}
