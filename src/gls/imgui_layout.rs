use gl::types::GLsizei;
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
    json_config: &mut ConfigJson,
) {
    if has_samples && ui.checkbox("Use MSAA", use_samples) {
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
}

pub fn model(
    ui: &imgui::Ui,
    options: &mut OptionsJson,
    mind_model: &mut MindModel,
    export_as: &mut u8,
    name: &String,
) {
    ui.checkbox("Show Wireframe", &mut options.show_wireframe);
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
            for i in 0..mind_model.skn.meshes.len() {
                let _meshes_id = ui.push_id_usize(i);
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
    pub skn: String,
    pub skl: String,
    pub dds: String,
    pub anm: String,
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
            ui.text("Name:");
            ui.same_line();
            ui.input_text("##name", &mut add_model.name).build();

            ui.align_text_to_frame_padding();
            ui.text("SKN: ");
            ui.same_line();
            ui.input_text("##skn", &mut add_model.skn).build();
            ui.same_line();
            if ui.button("Select##1") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("SKN", &["skn"])
                    .show_open_single_file()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.skn.clear();
                    add_model.skn.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("SKL: ");
            ui.same_line();
            ui.input_text("##skl", &mut add_model.skl).build();
            ui.same_line();
            if ui.button("Select##2") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("SKL", &["skl"])
                    .show_open_single_file()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.skl.clear();
                    add_model.skl.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("DDS: ");
            ui.same_line();
            ui.input_text("##dds", &mut add_model.dds).build();
            ui.same_line();
            if ui.button("Select##3") {
                let path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("DDS", &["dds"])
                    .show_open_single_dir()
                    .unwrap();
                if let Some(path) = path {
                    add_model.dds.clear();
                    add_model.dds.insert_str(0, path.to_str().unwrap());
                }
            }

            ui.align_text_to_frame_padding();
            ui.text("ANM: ");
            ui.same_line();
            ui.input_text("##anm", &mut add_model.anm).build();
            ui.same_line();
            if ui.button("Select##4") {
                let file_dialog_path = FileDialog::new()
                    .set_location(&working_dir)
                    .add_filter("ANM", &["anm"])
                    .show_open_single_dir()
                    .unwrap();
                if let Some(path) = file_dialog_path {
                    add_model.anm.clear();
                    add_model.anm.insert_str(0, path.to_str().unwrap());
                }
            }

            if ui.button_with_size("Add", [ui.content_region_avail()[0], 0.0f32]) {
                add_funct(add_model);

                add_model.name.clear();
                add_model.skn.clear();
                add_model.skl.clear();
                add_model.dds.clear();
                add_model.anm.clear();
            }
        });
}

pub fn screenshot(
    ui: &imgui::Ui,
    use_samples: bool,
    take_screenshot: &mut bool,
    resolution: &mut [u32; 2],
    screenshot: &mut super::Screenshot,
) {
    ui.tree_node_config("Screenshot")
        .flags(imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH)
        .framed(true)
        .build(|| {
            ui.align_text_to_frame_padding();
            ui.text("Resolution:");
            ui.same_line();
            ui.input_scalar_n("##resolution", resolution).build();

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
                if resolution[0] == 0 {
                    resolution[0] = 1280;
                }
                if resolution[1] == 0 {
                    resolution[1] = 720;
                }

                let resolution = [resolution[0] as GLsizei, resolution[1] as GLsizei];

                if screenshot.resolution[..] != resolution[..]
                    || screenshot.use_samples != use_samples
                {
                    screenshot.resolution = resolution;
                    screenshot.use_samples = use_samples;
                    screenshot.update();
                }

                *take_screenshot = true;
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
