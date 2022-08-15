extern crate byteorder;
extern crate clap;
extern crate dtoa;
extern crate glob;
extern crate json;

extern crate gl;
extern crate glam;
extern crate glfw;

extern crate imgui;

use glfw::{Action, Context, Key};

use gl::types::{GLint, GLsizei, GLuint};

use std::{ffi::CString, fs::File, io::Read, os::raw::c_void, path::Path, sync::mpsc::Receiver};

mod config_json;

mod g3d;
mod gls;
mod lol;

use gls::{imgui_glfw::ImguiGLFW, shader, texture};

use g3d::{floor::Floor, lines_joints::LinesJoints, model::Model, skybox::Skybox};

fn main() {
    let mut config = config_json::read_config_json();

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
            "MindCorpViewer-Rust",
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
    window.set_scroll_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_framebuffer_size_polling(true);

    glfw.set_swap_interval(glfw::SwapInterval::None);

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const c_void);

    let floor = Floor::new();
    let skybox = Skybox::new();

    let model_shader = shader::create_shader(
        Path::new("assets/model.vert"),
        Path::new("assets/model.frag"),
    );
    let model_refs = shader::get_refs_shader(model_shader, &["MVP", "Diffuse", "UseBone"]);
    let model_ubo_ref = unsafe {
        let c_str_btb =
            CString::new("BonesTransformsBlock").expect("Could not create ubo ref CString");
        gl::GetUniformBlockIndex(model_shader, c_str_btb.as_ptr())
    };

    let lines_joints_shader = shader::create_shader(
        Path::new("assets/lines_joints.vert"),
        Path::new("assets/lines_joints.frag"),
    );
    let lines_joints_refs = shader::get_refs_shader(lines_joints_shader, &["MVP", "Color"]);

    let mut skns: Vec<lol::skn::Skin> = Vec::with_capacity(config.model_count);
    let mut skls: Vec<lol::skl::Skeleton> = Vec::with_capacity(config.model_count);

    let mut models: Vec<Model> = Vec::with_capacity(config.model_count);
    let mut lines_joints: Vec<LinesJoints> = Vec::with_capacity(config.model_count);

    let mut bones_transforms: Vec<Vec<glam::Mat4>> = Vec::with_capacity(config.model_count);

    for j in 0..config.model_count {
        let mut skn = lol::skn::read_skn(&read_to_u8(Path::new(&config.paths[j].skn)));
        let skl = lol::skl::read_skl(&read_to_u8(Path::new(&config.paths[j].skl)));

        skn.apply_skeleton(&skl);

        let skl_bones_count = skl.bones.len();

        let mut model = Model::new(&skn, skl_bones_count);
        let mut line_joint = LinesJoints::new(&skl);

    	model.set_shader_refs(model_shader, &model_refs);
        model.bind_ubo(model_shader, model_ubo_ref, skl_bones_count);
        line_joint.set_shader_refs(lines_joints_shader, &lines_joints_refs);

        skns.push(skn);
        skls.push(skl);

        models.push(model);
        lines_joints.push(line_joint);

        bones_transforms.push(vec![glam::Mat4::IDENTITY; skl_bones_count]);
    }

    let mut textures: Vec<Vec<GLint>> = Vec::with_capacity(config.model_count);
    let mut textures_file_names: Vec<Vec<String>> = Vec::with_capacity(config.model_count);

    let mut texture_used: Vec<Vec<GLint>> = Vec::with_capacity(config.model_count);
    let mut texture_selected: Vec<Vec<usize>> = Vec::with_capacity(config.model_count);

    let mut animations: Vec<Vec<lol::anm::Animation>> = Vec::with_capacity(config.model_count);
    let mut animations_file_names: Vec<Vec<String>> = Vec::with_capacity(config.model_count);

    let mut selected_animation: Vec<usize> = vec![0; config.model_count];

    for j in 0..config.model_count {
        let dds_paths = glob::glob(format!("{}/*.dds", config.paths[j].dds).as_str())
            .expect("Failed to read glob pattern")
            .filter_map(Result::ok);

        let mut texture = Vec::new();
        let mut textures_file_name = Vec::new();

        for path in dds_paths {
            texture.push(texture::load_dds_to_opengl(&path) as GLint - 1);
            textures_file_name.push(path.file_stem().unwrap().to_str().unwrap().to_string());
        }

        let mut texture_useds: Vec<GLint> = vec![texture[0]; skns[j].meshes.len()];
        let mut texture_selecteds: Vec<usize> = vec![0; skns[j].meshes.len()];

        for i in 0..skns[j].meshes.len() {
            if let Some(name_position) = config.meshes[j]
                .names
                .iter()
                .position(|x| *x == skns[j].meshes[i].name)
            {
                let texture_name = config.meshes[j].textures[name_position].to_string();
                if let Some(texture_position) =
                    textures_file_name.iter().position(|x| *x == texture_name)
                {
                    texture_useds[i] = texture[texture_position];
                    texture_selecteds[i] = texture_position;
                }
            }
        }

        texture_used.push(texture_useds);
        texture_selected.push(texture_selecteds);

        textures.push(texture);
        textures_file_names.push(textures_file_name);

        let anm_paths = glob::glob(format!("{}/*.anm", config.paths[j].animations).as_str())
            .expect("Failed to read glob pattern")
            .filter_map(Result::ok);

        let mut animation = Vec::new();
        let mut animations_file_name = Vec::new();

        for path in anm_paths {
            animation.push(lol::anm::read_anm(&read_to_u8(&path)));
            animations_file_name.push(path.file_stem().unwrap().to_str().unwrap().to_string());
        }

        if let Some(animation_position) = animations_file_name
            .iter()
            .position(|x| *x == config.configs[j].selected_animation_path)
        {
            selected_animation[j] = animation_position;
        }

        animations.push(animation);
        animations_file_names.push(animations_file_name);
    }

    unsafe {
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Enable(gl::MULTISAMPLE);
        gl::Enable(gl::SAMPLE_ALPHA_TO_COVERAGE);
		gl::PointSize(3.0f32);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::ClearColor(0.5f32, 0.5f32, 0.5f32, 1.0f32);
    }

    let mut imgui = imgui::Context::create();

    imgui.set_ini_filename(None);

    let style = imgui.style_mut();
    style.use_dark_colors();
    style.grab_rounding = 4.0f32;
    style.frame_rounding = 4.0f32;
    style.window_rounding = 6.0f32;
    style.frame_border_size = 1.0f32;
    style.window_border_size = 1.0f32;
    style.indent_spacing = style.frame_padding[0] * 3.0f32 - 2.0f32;

    imgui.fonts().add_font(&[imgui::FontSource::TtfData {
        data: &read_to_u8(Path::new("assets/consola.ttf")),
        size_pixels: 13.0f32,
        config: None,
    }]);

    let mut imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

    let mut show_mesh: Vec<Vec<bool>> = Vec::with_capacity(config.model_count);

    for j in 0..config.model_count {
        let mut show_meshs: Vec<bool> = vec![true; skns[j].meshes.len()];
        show_meshs[..config.meshes[j].shows.len()].copy_from_slice(&config.meshes[j].shows[..]);
        show_mesh.push(show_meshs);
    }

    let mut frames = 0.0f32;
    let mut last_time = 0.0f32;
    let mut last_time_fps = 0.0f32;

    let fov = 45.0f32.to_radians();
    let mut translation = glam::Vec3::ONE;
    let mut yaw_pitch = glam::vec2(90.0f32, 70.0f32);

    let mut mouse = Mouse::new(700.0f32, [width as f32 / 2.0f32, height as f32 / 2.0f32]);

    while !window.should_close() {
        let current_time = glfw.get_time() as f32;
        let delta_time_fps = current_time - last_time_fps;

        frames += 1.0f32;
        if delta_time_fps >= 1.0f32 {
            window.set_title(
                format!(
                    "MindCorpViewer-Rust - Fps: {:1.0} / Ms: {:1.3}",
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
                if ui.checkbox("Enable Vsync", &mut config.vsync) {
                    glfw.set_swap_interval(match config.vsync {
                        true => glfw::SwapInterval::Sync(1),
                        false => glfw::SwapInterval::None,
                    });
                }
                ui.checkbox("Show Floor", &mut config.show_floor);
                ui.checkbox("Show Skybox", &mut config.show_skybox);
                ui.checkbox("Synchronized Time", &mut config.synchronized_time);
                ui.separator();
                for j in 0..config.model_count {
                    let _model_id = ui.push_id(j as i32);
                    ui.checkbox("##show", &mut config.configs[j].show);
                    ui.same_line();
                    imgui::TreeNode::new(config.paths[j].name.to_string())
                        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
                        .allow_item_overlap(true)
                        .framed(true)
                        .build(&ui, || {
                            ui.checkbox("Show Wireframe", &mut config.configs[j].show_wireframe);
                            ui.checkbox("Show Skeleton", &mut config.configs[j].show_skeleton);
                            ui.separator();
                            for i in 0..skns[j].meshes.len() {
                                let _meshes_id = ui.push_id(i as i32);
                                ui.checkbox(skns[j].meshes[i].name.as_str(), &mut show_mesh[j][i]);
                                if show_mesh[j][i] {
                                    ui.combo_simple_string(
                                        "##combo",
                                        &mut texture_selected[j][i],
                                        &textures_file_names[j],
                                    );
                                    texture_used[j][i] = textures[j][texture_selected[j][i]];
                                    imgui::Image::new(
                                        (texture_used[j][i] as usize + 1).into(),
                                        [64.0f32, 64.0f32],
                                    )
                                    .build(&ui);
                                }
                            }
                            ui.separator();
                            ui.text("Animation");
                            ui.checkbox("Use Animation", &mut config.configs[j].use_animation);
                            ui.checkbox("Play / Stop", &mut config.configs[j].play_animation);
                            ui.checkbox("Loop Animation", &mut config.configs[j].loop_animation);
                            ui.checkbox("Next Animation", &mut config.configs[j].next_animation);
                            ui.text("CTRL+Click Change To Input");
                            imgui::Slider::new("Speed", 0.00001f32, 10.0f32)
                                .display_format("%.5f")
                                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                .build(&ui, &mut config.configs[j].animation_speed);
                            imgui::Slider::new(
                                "Time",
                                0.0f32,
                                animations[j][selected_animation[j]].duration,
                            )
                            .display_format("%.5f")
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .build(&ui, &mut config.configs[j].animation_time);
                            ui.combo_simple_string(
                                "Animations",
                                &mut selected_animation[j],
                                &animations_file_names[j],
                            );
                        });
                }
                ui.separator();
                if ui.button("Save Configuration") {
                    config_json::write_config_json(
                        &config,
                        &skns,
                        &show_mesh,
                        &animations_file_names,
                        &selected_animation,
                        &textures_file_names,
                        &texture_selected,
                    );
                }
            });

        let view_matrix = compute_matrix_from_inputs(&mut translation, &mut yaw_pitch, &mut mouse);
        let projection_matrix =
            glam::Mat4::perspective_infinite_rh(fov, width as f32 / height as f32, 0.1f32);
        let projection_view_matrix = projection_matrix * view_matrix;

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            if config.show_skybox {
                skybox.render(&view_matrix, &projection_matrix);
            }

            if config.show_floor {
                floor.render(&projection_view_matrix);
            }

            for j in 0..config.model_count {
                if config.configs[j].show {
                    let animation_time_first = config.configs[0].animation_time;
                    play_animation(
                        &mut config.configs[j],
                        &skls[j],
                        j == 0,
                        delta_time,
                        config.synchronized_time,
                        animation_time_first,
                        &animations[j],
                        &mut selected_animation[j],
                        &mut bones_transforms[j],
                    );

                    models[j].render(
                        &config.configs[j],
                        &projection_view_matrix,
                        &show_mesh[j],
                        &skns[j].meshes,
                        &texture_used[j],
                        &bones_transforms[j],
                    );

                    if config.configs[j].show_skeleton {
                        lines_joints[j].render(
                            config.configs[j].use_animation,
                            &skls[j].bones,
                            &bones_transforms[j],
                            &projection_view_matrix,
                        );
                    }
                }
            }

            gl::Disable(gl::MULTISAMPLE);
            imgui_glfw.draw(ui, &mut window);
            gl::Enable(gl::MULTISAMPLE);
        }

        window.swap_buffers();
    }

    unsafe {
        floor.destroy();
        skybox.destroy();

        gl::DeleteProgram(model_shader);
        gl::DeleteProgram(lines_joints_shader);

        for j in 0..config.model_count {
            gl::DeleteTextures(
                textures[j].len() as GLsizei,
                textures[j].as_ptr() as *const GLuint,
            );

            models[j].destroy();
            lines_joints[j].destroy();
        }
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
    events: &Receiver<(f64, glfw::WindowEvent)>,
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
                if button == glfw::MouseButtonLeft && action == Action::Press {
                    mouse.state = 1;
                } else if button == glfw::MouseButtonRight && action == Action::Press {
                    mouse.state = 2;
                }
                if action == Action::Release {
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
                let hovered: bool = unsafe {
                    imgui::sys::igIsWindowHovered(
                        imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32
                    )
                };
                if !hovered {
                    mouse.zoom -= ((yoffset as f32) * 60.0f32) * 0.5f32;
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
        let hovered: bool = unsafe {
            imgui::sys::igIsWindowHovered(imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32)
        };
        let focused: bool = unsafe {
            imgui::sys::igIsWindowFocused(imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32)
        };
        if !hovered && !focused {
            if mouse.offset[0] != mouse.last_offset[0] {
                yaw_pitch[0] += mouse.offset[0] * 0.5f32;
            }
            if mouse.offset[1] != mouse.last_offset[1] {
                yaw_pitch[1] -= mouse.offset[1] * 0.5f32;
            }

            if yaw_pitch[0] > 360.0f32 {
                yaw_pitch[0] -= 360.0f32
            } else if yaw_pitch[0] < -360.0f32 {
                yaw_pitch[0] += 360.0f32
            }
            if yaw_pitch[1] > 179.0f32 {
                yaw_pitch[1] = 179.0f32;
            } else if yaw_pitch[1] < 1.0f32 {
                yaw_pitch[1] = 1.0f32;
            }
        }
    }

    let mut position = glam::vec3(
        yaw_pitch[1].to_radians().sin() * yaw_pitch[0].to_radians().cos(),
        yaw_pitch[1].to_radians().cos(),
        yaw_pitch[1].to_radians().sin() * yaw_pitch[0].to_radians().sin(),
    );
    position = position.normalize();

    let right = position.cross(glam::Vec3::Y).normalize();
    let up = right.cross(position).normalize();

    if mouse.state == 2 {
        let hovered: bool = unsafe {
            imgui::sys::igIsWindowHovered(imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32)
        };
        if !hovered {
            if mouse.offset[0] != mouse.last_offset[0] {
                translation.x -= right.x * (mouse.offset[0] * 0.35f32);
                translation.z -= right.z * (mouse.offset[0] * 0.35f32);
            }
            if mouse.offset[1] != mouse.last_offset[1] {
                translation.y -= mouse.offset[1] * 0.35f32;
            }
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
    config: &mut config_json::ConfigJson,
    skl: &lol::skl::Skeleton,
    first: bool,
    delta_time: f32,
    synchronized_time: bool,
    animation_time_first: f32,
    animations: &Vec<lol::anm::Animation>,
    selected_animation: &mut usize,
    bones_transforms: &mut Vec<glam::Mat4>,
) {
    if config.play_animation {
        if config.animation_time <= animations[*selected_animation].duration {
            config.animation_time += delta_time * config.animation_speed;
        } else if config.next_animation {
            *selected_animation += 1;
            if *selected_animation == animations.len() {
                *selected_animation = 0;
            }
            config.animation_time = 0.0f32;
        } else if config.loop_animation {
            config.animation_time = 0.0f32;
        }
        if synchronized_time && !first {
            config.animation_time = animation_time_first;
        }
    }
    if config.use_animation {
        lol::anm::run_animation(
            bones_transforms,
            &animations[*selected_animation],
            skl,
            config.animation_time,
        );
    }
}

fn read_to_u8(path: &Path) -> Vec<u8> {
    let mut file = File::open(path).expect("Could not open file");
    let mut contents: Vec<u8> = Vec::new();
    println!("Reading file: {}", path.to_str().unwrap());
    file.read_to_end(&mut contents)
        .expect("Could not read file");
    println!("Finished reading file");
    contents
}
