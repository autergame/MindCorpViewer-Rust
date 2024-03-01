#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use glfw::{Action, Context, GlfwReceiver, Key};
use std::{
    env,
    fs::File,
    io::Read,
    ops::{Div, Neg},
    path::Path,
    rc::Rc,
};

mod config_json;
mod export;

mod g3d;
mod gls;
mod lol;

use g3d::{Bones, Character, Floor, Joints, Model, Names, Refs, Shaders, Skybox};
use gls::{
    imgui_layout::{self, AddModel},
    ImguiGLFW, Screenshot, Texture,
};
use lol::{Animation, Skeleton, Skin};

fn main() {
    let cargo_pkg_version = env!("CARGO_PKG_VERSION");
    let working_dir = env::current_dir().expect("Could not get current dir");

    let mut config_json = config_json::ConfigJson::read(Path::new("config.json"));

    let mut glfw = glfw::init(glfw::fail_on_errors).expect("Could not init GLFW");

    let msaa = config_json.msaa.unwrap_or(4);
    glfw.window_hint(glfw::WindowHint::Samples(Some(msaa)));
    glfw.window_hint(glfw::WindowHint::DoubleBuffer(true));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (monitor_xpos, monitor_ypos, monitor_width, monitor_height) =
        glfw.with_primary_monitor(|_, monitor| {
            monitor.expect("Could not get GLFW monitor").get_workarea()
        });

    let (mut window_width, mut window_height) = (
        (monitor_width as f32 * 0.75f32) as i32,
        (monitor_height as f32 * 0.75f32) as i32,
    );

    let (mut window, events) = glfw
        .create_window(
            window_width as u32,
            window_height as u32,
            format!("MindCorpViewer-Rust v{}", cargo_pkg_version).as_str(),
            glfw::WindowMode::Windowed,
        )
        .expect("Could not create GLFW window");

    window.set_pos(
        (monitor_width - monitor_xpos) / 2 - window_width / 2,
        (monitor_height - monitor_ypos) / 2 - window_height / 2,
    );

    window.make_current();

    window.set_key_polling(true);
    window.set_char_polling(true);
    window.set_scroll_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_framebuffer_size_polling(true);

    glfw.set_swap_interval(match config_json.vsync {
        true => glfw::SwapInterval::Sync(1),
        false => glfw::SwapInterval::None,
    });

    gl::load_with(|symbol| window.get_proc_address(symbol));

    let mut has_samples = false;
    let mut use_samples = config_json.msaa.is_some();

    unsafe {
        let mut samples: gl::types::GLint = 0;
        gl::GetIntegerv(gl::SAMPLES, &mut samples);

        if samples >= 1 {
            has_samples = true;

            if use_samples {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::Enable(gl::MULTISAMPLE);
                gl::Enable(gl::SAMPLE_ALPHA_TO_COVERAGE);
            }
        }

        gl::PointSize(4.0f32);
        gl::Enable(gl::LINE_SMOOTH);
        gl::ClearColor(0.5f32, 0.5f32, 0.5f32, 1.0f32);
    }

    let floor = Floor::new();
    let skybox = Skybox::new();

    let shaders = Shaders::new();
    let refs = Refs::new(&shaders);

    let mut characters: Vec<Character> = Vec::with_capacity(config_json.paths.len());
    let mut mind_models: Vec<MindModel> = Vec::with_capacity(config_json.paths.len());

    for i in 0..config_json.paths.len() {
        let json_config = JsonConfig {
            meshes: &config_json.meshes[i],
            selected_animation_path: config_json.paths[i].animations.to_owned(),
        };

        let add_model = AddModel {
            name: config_json.paths[i].name.to_owned(),
            skin: config_json.paths[i].skin.to_owned(),
            skeleton: config_json.paths[i].skeleton.to_owned(),
            textures: config_json.paths[i].textures.to_owned(),
            animations: config_json.paths[i].animations.to_owned(),
        };

        mind_models.push(load_mind_model(
            &add_model,
            Some(json_config),
            &mut characters,
            &shaders,
            &refs,
        ));
    }

    let mut screenshot = Screenshot::new(use_samples, config_json.screen_shot_resolution);

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
            data: include_bytes!("../assets/fonts/consolas.ttf"),
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
            .map(|mind_model| mind_model.skin.center.y)
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

    let mut add_model = AddModel::new();

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
                    &mut config_json,
                );

                ui.separator();

                for i in 0..mind_models.len() {
                    let _model_id = ui.push_id_usize(i);

                    ui.align_text_to_frame_padding();
                    ui.checkbox("##show", &mut config_json.options[i].show);
                    if ui.is_item_hovered() {
                        ui.tooltip(|| {
                            ui.text("Show / Hide model");
                        });
                    }
                    ui.same_line_with_spacing(0.0f32, 3.0f32);
                    if imgui_layout::confirm_delete_button(ui) {
                        mind_models.remove(i);
                        config_json.paths.remove(i);
                        config_json.options.remove(i);
                        config_json.meshes.remove(i);
                        break;
                    }
                    ui.same_line_with_spacing(0.0f32, 6.0f32);
                    if let Some(_tree) = ui
                        .tree_node_config(&format!("{}    ", config_json.paths[i].name))
                        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                        .flags(imgui::TreeNodeFlags::ALLOW_ITEM_OVERLAP)
                        .framed(true)
                        .push()
                    {
                        {
                            imgui_layout::model(
                                ui,
                                &mut config_json.options[i],
                                &mut mind_models[i],
                                &mut export_as,
                                &config_json.paths[i].name,
                            );
                        }
                    }
                }

                ui.separator();

                imgui_layout::add_model(ui, &working_dir, &mut add_model, |add_model| {
                    mind_models.push(load_mind_model(
                        add_model,
                        None,
                        &mut characters,
                        &shaders,
                        &refs,
                    ));

                    let name = if !add_model.name.is_empty() {
                        add_model.name.to_owned()
                    } else {
                        String::from("model")
                    };

                    config_json.paths.push(config_json::PathJson {
                        name,
                        skin: add_model.skin.to_owned(),
                        skeleton: add_model.skeleton.to_owned(),
                        textures: add_model.textures.to_owned(),
                        animations: add_model.animations.to_owned(),
                    });
                    config_json.options.push(config_json::OptionsJson::new());
                    config_json.meshes.push(vec![]);
                });

                ui.separator();

                imgui_layout::screenshot(
                    ui,
                    use_samples,
                    &mut take_screenshot,
                    &mut screenshot,
                    &mut config_json,
                );

                ui.separator();

                if ui.button_with_size("Save Configuration", [ui.content_region_avail()[0], 0.0f32])
                {
                    config_json.write(&mind_models);
                }
            });

        if take_screenshot {
            screenshot.update();
        }

        let (view_matrix, camera_pos) =
            compute_matrix_from_inputs(&mut translation, &mut yaw_pitch, &mut mouse);
        let projection_matrix = glam::Mat4::perspective_infinite_rh(
            fov,
            window_width as f32 / window_height as f32,
            0.1f32,
        );
        let projection_view_matrix = projection_matrix * view_matrix;

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        if config_json.show_skybox {
            skybox.render(&view_matrix, &projection_matrix);
        }

        if config_json.show_floor {
            floor.render(&projection_view_matrix);
        }

        let projection_view_matrix_mod = if take_screenshot {
            screenshot.take(fov) * view_matrix
        } else {
            projection_view_matrix
        };

        for i in 0..mind_models.len() {
            let animation_synchronized_time = if config_json.synchronized_time && i != 0 {
                Some(config_json.options[0].animation_time)
            } else {
                None
            };

            let options = &mut config_json.options[i];

            if options.show {
                let character = &mut characters[i];
                let mind_model = &mut mind_models[i];

                play_animation(options, mind_model, delta_time, animation_synchronized_time);

                character
                    .model
                    .render(options, &projection_view_matrix_mod, mind_model);

                if options.show_skeleton_bones {
                    character.bones.render(
                        options.use_animation,
                        &projection_view_matrix_mod,
                        mind_model,
                    );
                }

                if options.show_skeleton_names {
                    character.names.render(
                        options.use_animation,
                        &camera_pos,
                        &view_matrix,
                        &projection_view_matrix_mod,
                        mind_model,
                    );
                }

                if options.show_skeleton_joints {
                    character.joints.render(
                        options.use_animation,
                        use_samples,
                        &projection_view_matrix_mod,
                        mind_model,
                    );
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

pub struct JsonConfig<'a> {
    meshes: &'a [config_json::MeshJson],
    selected_animation_path: String,
}

pub struct MindModel {
    pub skin: Skin,
    pub skeleton: Skeleton,

    pub show_meshes: Vec<bool>,
    pub joints_transforms: Vec<glam::Mat4>,

    pub textures: Vec<Texture>,
    pub textures_paths: Vec<String>,
    pub textures_selecteds: Vec<usize>,
    pub textures_file_names: Vec<String>,

    pub animations: Vec<Animation>,
    pub animation_selected: usize,
    pub animations_file_names: Vec<String>,
}

fn load_mind_model(
    add_model: &AddModel,
    config_json: Option<JsonConfig>,
    characters: &mut Vec<Character>,
    shaders: &Shaders,
    refs: &Refs,
) -> MindModel {
    let mut skin = Skin::read(&read_to_u8(Path::new(&add_model.skin)));
    let skeleton = Skeleton::read(&read_to_u8(Path::new(&add_model.skeleton)));

    skin.apply_skeleton(&skeleton);

    let joints_transforms = vec![glam::Mat4::IDENTITY; skeleton.joints.len()];

    let mut show_meshes: Vec<bool> = vec![true; skin.meshes.len()];
    if let Some(config_json) = &config_json {
        if skin.meshes.len() == config_json.meshes.len() {
            show_meshes.copy_from_slice(
                &config_json
                    .meshes
                    .iter()
                    .map(|x| x.show)
                    .collect::<Vec<bool>>(),
            );
        }
    }

    let dds_paths = glob::glob(format!("{}/*.dds", add_model.textures).as_str())
        .expect("Failed to read glob dds pattern")
        .filter_map(Result::ok);

    let tex_paths = glob::glob(format!("{}/*.tex", add_model.textures).as_str())
        .expect("Failed to read glob tex pattern")
        .filter_map(Result::ok);

    let mut textures_paths = vec![];
    let mut textures_file_names = vec![];

    for path in dds_paths.chain(tex_paths) {
        textures_paths.push(String::from(path.to_str().unwrap()));
        textures_file_names.push(String::from(path.file_stem().unwrap().to_str().unwrap()));
    }

    let mut textures = vec![];
    for path in textures_paths.iter() {
        textures.push(Texture::load_texture(&read_to_u8(Path::new(path))));
    }

    let mut textures_selecteds: Vec<usize> = vec![0; skin.meshes.len()];
    if let Some(config_json) = &config_json {
        for j in 0..skin.meshes.len() {
            if let Some(mesh_json) = config_json
                .meshes
                .iter()
                .find(|x| x.name_texture.get(&skin.meshes[j].submesh.name).is_some())
            {
                if let Some(texture_position) = textures_file_names
                    .iter()
                    .position(|x| x == mesh_json.name_texture.iter().next().unwrap().1)
                {
                    textures_selecteds[j] = texture_position;
                }
            }
        }
    }

    let animations_paths = glob::glob(format!("{}/*.anm", add_model.animations).as_str())
        .expect("Failed to read glob anm pattern")
        .filter_map(Result::ok);

    let mut animations = vec![];
    let mut animations_file_names = vec![];

    for path in animations_paths {
        animations.push(Animation::read(&read_to_u8(&path)));
        animations_file_names.push(String::from(path.file_stem().unwrap().to_str().unwrap()));
    }

    let mut animation_selected = 0;
    if let Some(config_json) = &config_json {
        if let Some(animation_position) = animations_file_names
            .iter()
            .position(|x| *x == config_json.selected_animation_path)
        {
            animation_selected = animation_position;
        }
    }

    let mut model = Model::create(&skin, Rc::clone(&shaders.model));
    let mut names = Names::create(&skeleton, Rc::clone(&shaders.names));
    let mut bones = Bones::create(&skeleton, Rc::clone(&shaders.bones));
    let mut joints = Joints::create(&skeleton, Rc::clone(&shaders.joints));

    model.set_shader_refs(&refs.model, refs.model_ubo);
    names.set_shader_refs(&refs.names);
    bones.set_shader_refs(&refs.bones);
    joints.set_shader_refs(&refs.joints);

    characters.push(Character {
        bones,
        joints,
        model,
        names,
    });

    MindModel {
        skin,
        skeleton,
        show_meshes,
        joints_transforms,
        textures,
        textures_paths,
        textures_selecteds,
        textures_file_names,
        animations,
        animation_selected,
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
    events: &GlfwReceiver<(f64, glfw::WindowEvent)>,
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
                    mouse.zoom -= yoffset as f32 * 25.0f32;
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
) -> (glam::Mat4, glam::Vec3) {
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

    let eye = position * mouse.zoom;
    let camera_pos = eye - *translation;

    let mut view_matrix = glam::Mat4::look_to_rh(eye, -eye, up);
    view_matrix *= glam::Mat4::from_translation(*translation);

    (view_matrix, camera_pos)
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
            &mut mind_model.joints_transforms,
            &mind_model.animations[mind_model.animation_selected],
            &mind_model.skeleton,
            options.animation_time,
        );
    }
}

fn read_to_u8(path: &Path) -> Vec<u8> {
    println!("Reading file: {}", path.to_str().unwrap());
    let mut file = File::open(path).expect("Could not open file");
    let mut contents: Vec<u8> = vec![];
    file.read_to_end(&mut contents)
        .expect("Could not read file");
    contents
}
