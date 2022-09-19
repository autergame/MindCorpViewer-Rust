use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Read, io::Write, path::Path};

use lol::Skin;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathJson {
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "DDS")]
    pub dds: String,

    #[serde(rename = "SKN")]
    pub skn: String,

    #[serde(rename = "SKL")]
    pub skl: String,

    #[serde(rename = "Animations")]
    pub animations: String,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshJson {
    #[serde(rename = "Show")]
    pub show: bool,

    #[serde(flatten)]
    pub name_texture: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigsJson {
    #[serde(rename = "Vsync")]
    pub vsync: bool,

    #[serde(rename = "ShowFloor")]
    pub show_floor: bool,

    #[serde(rename = "ShowSkybox")]
    pub show_skybox: bool,

    #[serde(rename = "SynchronizedTime")]
    pub synchronized_time: bool,

    #[serde(skip)]
    pub model_count: usize,

    #[serde(rename = "PATHS")]
    pub paths: Vec<PathJson>,

    #[serde(rename = "OPTIONS")]
    pub options: Vec<OptionsJson>,

    #[serde(rename = "MESHES")]
    pub meshes: Vec<Vec<MeshJson>>,
}

impl ConfigsJson {
    pub fn read(path: &Path) -> ConfigsJson {
        let mut file = File::open(path).expect("Could not open config file");
        let mut contents = String::new();
        println!("Reading config file");
        file.read_to_string(&mut contents)
            .expect("Could not read config file");

        let mut config_json: ConfigsJson =
            serde_json::from_str(&contents).expect("Could not deserialize config");

        if config_json.options.len() < config_json.paths.len() {
            let diff = config_json.paths.len() - config_json.options.len();
            let options = vec![
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
                    selected_animation_path: "".to_string()
                };
                diff
            ];
            config_json.options.extend_from_slice(&options);
        }

        if config_json.meshes.len() < config_json.paths.len() {
            let diff = config_json.paths.len() - config_json.meshes.len();
            let meshes = vec![vec![]; diff];
            config_json.meshes.extend_from_slice(&meshes);
        }

        config_json.model_count = config_json.paths.len();

        config_json
    }

    pub fn write(
        &self,
        skns: &[Skin],
        show_mesh: &[Vec<bool>],
        animations_file_names: &[Vec<String>],
        selected_animation: &[usize],
        textures_file_names: &[Vec<String>],
        texture_selected: &[Vec<usize>],
    ) {
        let mut config_json = self.clone();

        config_json
            .options
            .iter_mut()
            .enumerate()
            .for_each(|(j, config)| {
                config.selected_animation_path =
                    animations_file_names[j][selected_animation[j]].to_string()
            });

        let mut model_meshes = Vec::with_capacity(config_json.model_count);
        for j in 0..config_json.model_count {
            let mut meshes = Vec::with_capacity(skns[j].meshes.len());
            for i in 0..skns[j].meshes.len() {
                let mut name_texture = HashMap::new();
                name_texture.insert(
                    skns[j].meshes[i].submesh.name.to_string(),
                    textures_file_names[j][texture_selected[j][i]].to_string(),
                );
                meshes.push(MeshJson {
                    show: show_mesh[j][i],
                    name_texture,
                });
            }
            model_meshes.push(meshes);
        }
        config_json.meshes = model_meshes;

        let contents = serde_json::to_string_pretty(&config_json).unwrap();

        let mut file =
            File::create(Path::new("config.json")).expect("Could not create config file");
        println!("Writing to config file");
        file.write_all(contents.as_bytes())
            .expect("Could not write to config file");
    }
}
