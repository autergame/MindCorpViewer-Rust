use std::{collections::BTreeMap, fs::File, io::Read, io::Write, path::Path};
use serde::{Deserialize, Serialize};

use crate::MindModel;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathJson {
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "SKN")]
    pub skn: String,

    #[serde(rename = "SKL")]
    pub skl: String,

    #[serde(rename = "DDS")]
    pub dds: String,

    #[serde(rename = "Animations")]
    pub anm: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionsJson {
    #[serde(rename = "Show")]
    pub show: bool,

    #[serde(rename = "ShowWireframe")]
    pub show_wireframe: bool,

    #[serde(rename = "ShowSkeletonBones")]
    pub show_skeleton_bones: bool,

    #[serde(rename = "ShowSkeletonJoints")]
    pub show_skeleton_joints: bool,

    #[serde(rename = "UseAnimation")]
    pub use_animation: bool,

    #[serde(rename = "PlayAnimation")]
    pub play_animation: bool,

    #[serde(rename = "LoopAnimation")]
    pub loop_animation: bool,

    #[serde(rename = "NextAnimation")]
    pub next_animation: bool,

    #[serde(rename = "AnimationTime")]
    pub animation_time: f32,

    #[serde(rename = "AnimationSpeed")]
    pub animation_speed: f32,

    #[serde(rename = "SelectedAnimation")]
    pub selected_animation_path: String,
}

impl OptionsJson {
    pub fn new() -> OptionsJson {
        OptionsJson {
            show: true,
            show_wireframe: false,
            show_skeleton_bones: false,
            show_skeleton_joints: false,
            use_animation: false,
            play_animation: false,
            loop_animation: true,
            next_animation: false,
            animation_time: 0.0f32,
            animation_speed: 1.0f32,
            selected_animation_path: "".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshJson {
    #[serde(rename = "Show")]
    pub show: bool,

    #[serde(flatten)]
    pub name_texture: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigJson {
    #[serde(rename = "Vsync")]
    pub vsync: bool,

    #[serde(rename = "ShowFloor")]
    pub show_floor: bool,

    #[serde(rename = "ShowSkybox")]
    pub show_skybox: bool,

    #[serde(rename = "SynchronizedTime")]
    pub synchronized_time: bool,

    #[serde(rename = "PATHS")]
    pub paths: Vec<PathJson>,

    #[serde(rename = "OPTIONS")]
    pub options: Vec<OptionsJson>,

    #[serde(rename = "MESHES")]
    pub meshes: Vec<Vec<MeshJson>>,
}

impl ConfigJson {
    pub fn read(path: &Path) -> ConfigJson {
        println!("Reading config file");
        match File::open(path) {
            Ok(mut file) => {
                let mut contents = String::new();
                match file.read_to_string(&mut contents) {
                    Ok(_) => {
                        let config_json: Result<ConfigJson, serde_json::Error> =
                            serde_json::from_str(&contents);
                        match config_json {
                            Ok(mut config_json) => {
                                if config_json.options.len() < config_json.paths.len() {
                                    let diff = config_json.paths.len() - config_json.options.len();
                                    let options = vec![OptionsJson::new(); diff];
                                    config_json.options.extend_from_slice(&options);
                                }
                                if config_json.meshes.len() < config_json.paths.len() {
                                    let diff = config_json.paths.len() - config_json.meshes.len();
                                    let meshes = vec![vec![]; diff];
                                    config_json.meshes.extend_from_slice(&meshes);
                                }
                                println!("Finished reading config file");
                                config_json
                            }
                            Err(error) => {
                                println!("Could not deserialize config: {}", error);
                                ConfigJson::new()
                            }
                        }
                    }
                    Err(error) => {
                        println!("Could not read config file: {}", error);
                        ConfigJson::new()
                    }
                }
            }
            Err(error) => {
                println!("Could not open config file: {}", error);
                ConfigJson::new()
            }
        }
    }

    pub fn write(&self, mind_models: &[MindModel]) {
        println!("Writing to config file");

        let mut config_json = self.clone();

        config_json
            .options
            .iter_mut()
            .enumerate()
            .for_each(|(i, config)| {
                config.selected_animation_path = mind_models[i].animations_file_names
                    [mind_models[i].animation_selected]
                    .to_owned()
            });

        config_json.meshes = Vec::with_capacity(config_json.paths.len());
        for i in 0..config_json.paths.len() {
            let mind_model = &mind_models[i];

            let mut meshes = Vec::with_capacity(mind_model.skn.meshes.len());
            for i in 0..mind_model.skn.meshes.len() {
                let mut name_texture = BTreeMap::new();
                name_texture.insert(
                    mind_model.skn.meshes[i].submesh.name.to_owned(),
                    mind_model.textures_file_names[mind_model.textures_selecteds[i]].to_owned(),
                );
                meshes.push(MeshJson {
                    show: mind_model.show_meshes[i],
                    name_texture,
                });
            }
            config_json.meshes.push(meshes);
        }

        let contents = serde_json::to_string_pretty(&config_json).unwrap();

        let mut file =
            File::create(Path::new("config.json")).expect("Could not create config file");
        file.write_all(contents.as_bytes())
            .expect("Could not write to config file");
        println!("Finished writing to config file");
    }

    pub fn new() -> ConfigJson {
        ConfigJson {
            vsync: false,
            show_floor: true,
            show_skybox: true,
            synchronized_time: false,
            paths: vec![],
            options: vec![],
            meshes: vec![],
        }
    }
}
