use glfw::Glfw;
use native_dialog::FileDialog;
use std::path::PathBuf;

use crate::{
    config_json::{ConfigJson, OptionsJson},
    export, MindModel,
};

pub fn settings(
    ui: &imgui::Ui,
    glfw: &mut Glfw,
    has_samples: bool,
    use_samples: &mut bool,
    config_json: &mut ConfigJson,
) {
    if has_samples && ui.checkbox("Use MSAA", use_samples) {
        match use_samples {
            true => unsafe {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
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

    if ui.checkbox("Enable Vsync", &mut config_json.vsync) {
        glfw.set_swap_interval(match config_json.vsync {
            true => glfw::SwapInterval::Sync(1),
            false => glfw::SwapInterval::None,
        });
    }

    ui.checkbox("Show Floor", &mut config_json.show_floor);
    ui.checkbox("Show Skybox", &mut config_json.show_skybox);

    ui.checkbox("Synchronized Time", &mut config_json.synchronized_time);
    if ui.is_item_hovered() {
        ui.tooltip(|| {
            ui.text("Synchronize all models to first model");
        });
    }
}

pub fn model(
    ui: &imgui::Ui,
    options: &mut OptionsJson,
    mind_model: &mut MindModel,
    export_as: &mut u8,
    name: &String,
) {
    ui.checkbox("Show Wireframe", &mut options.show_wireframe);
    ui.checkbox("Show Skeleton Names", &mut options.show_skeleton_names);
    ui.checkbox("Show Skeleton Bones", &mut options.show_skeleton_bones);
    ui.checkbox("Show Skeleton Joints", &mut options.show_skeleton_joints);

    ui.tree_node_config("Animations")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            ui.checkbox("Use Animation", &mut options.use_animation);
            ui.checkbox("Play / Stop", &mut options.play_animation);
            ui.checkbox("Loop Animation", &mut options.loop_animation);
            ui.checkbox("Next Animation", &mut options.next_animation);

            ui.text("CTRL+Click Change To Input");

            ui.align_text_to_frame_padding();
            ui.text("Speed:     ");
            ui.same_line();
            ui.slider_config("##speed", 0.00001f32, 10.0f32)
                .display_format("%.5f")
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .build(&mut options.animation_speed);

            ui.align_text_to_frame_padding();
            ui.text("Time:      ");
            ui.same_line();
            ui.slider_config(
                "##time",
                0.0f32,
                mind_model.animations[mind_model.animation_selected].duration,
            )
            .display_format("%.5f")
            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
            .build(&mut options.animation_time);

            ui.align_text_to_frame_padding();
            ui.text("Animations:");
            ui.same_line();
            ui.combo_simple_string(
                "##animations",
                &mut mind_model.animation_selected,
                &mind_model.animations_file_names,
            );
        });

    ui.tree_node_config("Meshes")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            for i in 0..mind_model.skin.meshes.len() {
                let _meshes_id = ui.push_id_usize(i);
                ui.checkbox(
                    mind_model.skin.meshes[i].submesh.name.as_str(),
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
                            mind_model.textures[mind_model.textures_selecteds[i]].id as usize,
                        ),
                        [64.0f32, 64.0f32],
                    )
                    .build(ui);
                }
            }
        });

    ui.tree_node_config("Export")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            ui.radio_button("Export as gltf", export_as, 0);
            ui.radio_button("Export as glb", export_as, 1);
            if ui.button_with_size("Export Model", [ui.content_region_avail()[0], 0.0f32]) {
                export::export_model(*export_as, name, mind_model);
            }
        });
}

pub struct AddModel {
    pub name: String,
    pub skin: String,
    pub skeleton: String,
    pub textures: String,
    pub animations: String,
}

impl AddModel {
	pub fn new() -> Self {
		Self {
			name: String::new(),
			skin: String::new(),
			skeleton: String::new(),
			textures: String::new(),
			animations: String::new(),
		}
	}
}

pub fn add_model<F>(
    ui: &imgui::Ui,
    working_dir: &PathBuf,
    add_model: &mut AddModel,
    mut add_funct: F,
) where
    F: FnMut(&mut AddModel),
{
    ui.tree_node_config("Add Model")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            ui.align_text_to_frame_padding();
            ui.text("Name:       ");
            ui.same_line();
            ui.input_text("##name", &mut add_model.name).build();

            ui.align_text_to_frame_padding();
            ui.text("Skin:       ");
            ui.same_line();
            ui.input_text("##skin", &mut add_model.skin).build();
            ui.same_line();
            if ui.button("Select##1") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("Skin", &["skn"])
                    .show_open_single_file()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.skin.clear();
                    add_model.skin.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("Skeleton:   ");
            ui.same_line();
            ui.input_text("##skeleton", &mut add_model.skeleton).build();
            ui.same_line();
            if ui.button("Select##2") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("Skeleton", &["skl"])
                    .show_open_single_file()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.skeleton.clear();
                    add_model.skeleton.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("Textures:   ");
            ui.same_line();
            ui.input_text("##textures", &mut add_model.textures).build();
            ui.same_line();
            if ui.button("Select##3") {
                let path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("Textures", &["dds", "tex"])
                    .show_open_single_dir()
                    .unwrap();
                if let Some(path) = path {
                    add_model.textures.clear();
                    add_model.textures.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("Animations: ");
            ui.same_line();
            ui.input_text("##animations", &mut add_model.animations)
                .build();
            ui.same_line();
            if ui.button("Select##4") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("Animations", &["anm"])
                    .show_open_single_dir()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.animations.clear();
                    add_model.animations.insert_str(0, path.to_str().unwrap());
                }
            }

            if ui.button_with_size("Add", [ui.content_region_avail()[0], 0.0f32]) {
                add_funct(add_model);

                add_model.name.clear();
                add_model.skin.clear();
                add_model.skeleton.clear();
                add_model.textures.clear();
                add_model.animations.clear();
            }
        });
}

pub fn screenshot(
    ui: &imgui::Ui,
    use_samples: bool,
    take_screenshot: &mut bool,
    screenshot: &mut super::Screenshot,
    config_json: &mut ConfigJson,
) {
    ui.tree_node_config("Screenshot")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            ui.align_text_to_frame_padding();
            ui.text("Resolution:");
            ui.same_line();
            ui.input_scalar_n("##resolution", &mut config_json.screen_shot_resolution)
                .build();

            if config_json.screen_shot_resolution[0] == 0 {
                config_json.screen_shot_resolution[0] = 1280;
            }
            if config_json.screen_shot_resolution[1] == 0 {
                config_json.screen_shot_resolution[1] = 720;
            }

            ui.align_text_to_frame_padding();
            ui.text("File name: ");
            ui.same_line();
            ui.input_text("##file_name", &mut screenshot.file_name)
                .build();

            ui.align_text_to_frame_padding();
            ui.text("Format:    ");
            ui.same_line();
            ui.combo_simple_string("##format", &mut screenshot.format, &FORMATS);

            if ui.button_with_size("Take", [ui.content_region_avail()[0], 0.0f32]) {
                *take_screenshot = true;
                screenshot.use_samples = use_samples;
                screenshot.resolution = config_json.screen_shot_resolution;
            }
        });
}

const FORMATS: [&str; 4] = ["PNG", "JPG", "BMP", "TIFF"];

pub fn confirm_delete_button(ui: &imgui::Ui) -> bool {
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

pub fn no_window_hovered() -> bool {
    unsafe { !imgui::sys::igIsWindowHovered(imgui::WindowHoveredFlags::ANY_WINDOW.bits() as i32) }
}
