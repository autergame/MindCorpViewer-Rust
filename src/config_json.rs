use std::{fs::File, io::Read, io::Write, path::Path};

use json::{codegen::Generator, JsonValue};

use lol;

#[inline(never)]
pub fn read_config_json() -> ConfigsJson {
    let mut file = File::open(Path::new("config.json")).expect("Could not open config json file");
    let mut contents = String::new();
    println!("Reading config json file");
    file.read_to_string(&mut contents)
        .expect("Could not read config json file");
    println!("Finished reading config json file");

    let config_json = json::parse(&contents).expect("Could not parse config json");

    let vsync = config_json["Vsync"]
        .as_bool()
        .expect("Expected bool in Vsync");
    let show_floor = config_json["ShowFloor"]
        .as_bool()
        .expect("Expected bool in ShowFloor");
    let show_skybox = config_json["ShowSkybox"]
        .as_bool()
        .expect("Expected bool in ShowSkybox");
    let synchronized_time = config_json["SynchronizedTime"]
        .as_bool()
        .expect("Expected bool in SynchronizedTime");

    let paths_json = config_json["PATHS"].members();
    let paths_json_count = paths_json.len();

    let mut paths: Vec<PathJson> = Vec::with_capacity(paths_json_count);

    for path in paths_json {
        let name = path["Name"]
            .as_str()
            .expect("Expected string in Name PATHS")
            .to_string();
        let dds = path["Dds"]
            .as_str()
            .expect("Expected string in DdsPath PATHS")
            .to_string();
        let skn = path["Skn"]
            .as_str()
            .expect("Expected string in SknPath PATHS")
            .to_string();
        let skl = path["Skl"]
            .as_str()
            .expect("Expected string in SklPath PATHS")
            .to_string();
        let animations = path["Animations"]
            .as_str()
            .expect("Expected string in AnimationsPath PATHS")
            .to_string();
        paths.push(PathJson {
            name,
            dds,
            skn,
            skl,
            animations,
        });
    }

    let configs_json = config_json["CONFIGS"].members();
    let configs_json_count = configs_json.len();

    let mut configs: Vec<ConfigJson> = Vec::with_capacity(configs_json_count);

    for config in configs_json {
        let show = config["Show"]
            .as_bool()
            .expect("Expected bool in Show CONFIG");
        let show_wireframe = config["ShowWireframe"]
            .as_bool()
            .expect("Expected bool in ShowWireframe CONFIG");
        let show_skeleton = config["ShowSkeleton"]
            .as_bool()
            .expect("Expected bool in ShowSkeleton CONFIG");
        let use_animation = config["UseAnimation"]
            .as_bool()
            .expect("Expected bool in UseAnimation CONFIG");
        let play_animation = config["PlayAnimation"]
            .as_bool()
            .expect("Expected bool in PlayAnimation CONFIG");
        let loop_animation = config["LoopAnimation"]
            .as_bool()
            .expect("Expected bool in LoopAnimation CONFIG");
        let next_animation = config["NextAnimation"]
            .as_bool()
            .expect("Expected bool in NextAnimation CONFIG");
        let animation_time = config["AnimationTime"]
            .as_f32()
            .expect("Expected f32 in AnimationTime CONFIG");
        let animation_speed = config["AnimationSpeed"]
            .as_f32()
            .expect("Expected f32 in AnimationSpeed CONFIG");
        let selected_animation_path = config["SelectedAnimation"]
            .as_str()
            .expect("Expected string in SelectedAnimation CONFIG")
            .to_string();
        configs.push(ConfigJson {
            show,
            show_wireframe,
            show_skeleton,
            use_animation,
            play_animation,
            loop_animation,
            next_animation,
            animation_time,
            animation_speed,
            selected_animation_path,
        });
    }

    if paths_json_count != configs_json_count {
        panic!("PATHS and CONFIGS must have the same count");
    }

    let meshes_json = config_json["MESHES"].members();
    let meshes_json_count = meshes_json.len();

    let mut meshes: Vec<MeshesJson> = Vec::with_capacity(meshes_json_count);

    for model in meshes_json {
        let mut names: Vec<String> = Vec::new();
        let mut textures: Vec<String> = Vec::new();
        let mut shows: Vec<bool> = Vec::new();

        for mesh in model.members() {
            let mut iter = mesh.entries();
            shows.push(
                iter.next()
                    .expect("Expected name texture")
                    .1
                    .as_bool()
                    .expect("Expected bool in Show MESHES"),
            );
            let name_texture = iter.next().expect("Expected name texture MESHES");
            names.push(name_texture.0.to_string());
            textures.push(
                name_texture
                    .1
                    .as_str()
                    .expect("Expected string in texture MESHES")
                    .to_string(),
            );
        }

        meshes.push(MeshesJson {
            names,
            textures,
            shows,
        });
    }

    for _ in 0..paths_json_count - meshes_json_count {
        meshes.push(MeshesJson::new());
    }

    ConfigsJson {
        vsync,
        show_floor,
        show_skybox,
        synchronized_time,
        model_count: paths_json_count,
        paths,
        configs,
        meshes,
    }
}

#[inline(never)]
pub fn write_config_json(
    config: &ConfigsJson,
    skns: &[lol::skn::Skin],
    show_mesh: &[Vec<bool>],
    animations_file_names: &[Vec<String>],
    selected_animation: &[usize],
    textures_file_names: &[Vec<String>],
    texture_selected: &[Vec<usize>],
) {
    let mut config_json = JsonValue::new_object();

    config_json
        .insert("Vsync", JsonValue::Boolean(config.vsync))
        .unwrap();
    config_json
        .insert("ShowFloor", JsonValue::Boolean(config.show_floor))
        .unwrap();
    config_json
        .insert("ShowSkybox", JsonValue::Boolean(config.show_skybox))
        .unwrap();
    config_json
        .insert(
            "SynchronizedTime",
            JsonValue::Boolean(config.synchronized_time),
        )
        .unwrap();

    let mut paths = JsonValue::new_array();
    for j in 0..config.model_count {
        let mut object = JsonValue::new_object();
        object
            .insert("Name", JsonValue::String(config.paths[j].name.to_string()))
            .unwrap();
        object
            .insert("Dds", JsonValue::String(config.paths[j].dds.to_string()))
            .unwrap();
        object
            .insert("Skn", JsonValue::String(config.paths[j].skn.to_string()))
            .unwrap();
        object
            .insert("Skl", JsonValue::String(config.paths[j].skl.to_string()))
            .unwrap();
        object
            .insert(
                "Animations",
                JsonValue::String(config.paths[j].animations.to_string()),
            )
            .unwrap();
        paths.push(object).unwrap();
    }
    config_json.insert("PATHS", paths).unwrap();

    let mut configs = JsonValue::new_array();
    for j in 0..config.model_count {
        let mut object = JsonValue::new_object();
        object
            .insert("Show", JsonValue::Boolean(config.configs[j].show))
            .unwrap();
        object
            .insert(
                "ShowWireframe",
                JsonValue::Boolean(config.configs[j].show_wireframe),
            )
            .unwrap();
        object
            .insert(
                "ShowSkeleton",
                JsonValue::Boolean(config.configs[j].show_skeleton),
            )
            .unwrap();
        object
            .insert(
                "UseAnimation",
                JsonValue::Boolean(config.configs[j].use_animation),
            )
            .unwrap();
        object
            .insert(
                "PlayAnimation",
                JsonValue::Boolean(config.configs[j].play_animation),
            )
            .unwrap();
        object
            .insert(
                "LoopAnimation",
                JsonValue::Boolean(config.configs[j].loop_animation),
            )
            .unwrap();
        object
            .insert(
                "NextAnimation",
                JsonValue::Boolean(config.configs[j].next_animation),
            )
            .unwrap();
        object
            .insert("AnimationTime", from_f32(config.configs[j].animation_time))
            .unwrap();
        object
            .insert(
                "AnimationSpeed",
                from_f32(config.configs[j].animation_speed),
            )
            .unwrap();
        object
            .insert(
                "SelectedAnimation",
                JsonValue::String(animations_file_names[j][selected_animation[j]].to_string()),
            )
            .unwrap();
        configs.push(object).unwrap();
    }
    config_json.insert("CONFIGS", configs).unwrap();

    let mut meshes = JsonValue::new_array();
    for j in 0..config.model_count {
        let mut array = JsonValue::new_array();
        for i in 0..skns[j].meshes.len() {
            let mut object = JsonValue::new_object();
            object
                .insert("Show", JsonValue::Boolean(show_mesh[j][i]))
                .unwrap();
            object
                .insert(
                    skns[j].meshes[i].name.as_str(),
                    JsonValue::String(textures_file_names[j][texture_selected[j][i]].to_string()),
                )
                .unwrap();
            array.push(object).unwrap();
        }
        meshes.push(array).unwrap();
    }
    config_json.insert("MESHES", meshes).unwrap();

    let mut gen = MyPrettyGenerator::new();
    gen.write_json(&config_json).expect("Could not write json");
    let contents = gen.consume();

    let mut file =
        File::create(Path::new("config.json")).expect("Could not create config json file");
    println!("Writing to config json file");
    file.write_all(contents.as_bytes())
        .expect("Could not write to config json file");
    println!("Finished writing to config json file");
}

pub struct ConfigsJson {
    pub vsync: bool,
    pub show_floor: bool,
    pub show_skybox: bool,
    pub synchronized_time: bool,
    pub model_count: usize,
    pub paths: Vec<PathJson>,
    pub configs: Vec<ConfigJson>,
    pub meshes: Vec<MeshesJson>,
}

pub struct PathJson {
    pub name: String,
    pub dds: String,
    pub skn: String,
    pub skl: String,
    pub animations: String,
}

pub struct ConfigJson {
    pub show: bool,
    pub show_wireframe: bool,
    pub show_skeleton: bool,
    pub use_animation: bool,
    pub play_animation: bool,
    pub loop_animation: bool,
    pub next_animation: bool,
    pub animation_time: f32,
    pub animation_speed: f32,
    pub selected_animation_path: String,
}

pub struct MeshesJson {
    pub shows: Vec<bool>,
    pub names: Vec<String>,
    pub textures: Vec<String>,
}

impl MeshesJson {
    fn new() -> MeshesJson {
        MeshesJson {
            shows: Vec::new(),
            names: Vec::new(),
            textures: Vec::new(),
        }
    }
}

pub struct MyPrettyGenerator {
    buf: Vec<u8>,
    dent: u16,
}

impl MyPrettyGenerator {
    pub fn new() -> MyPrettyGenerator {
        MyPrettyGenerator {
            buf: Vec::with_capacity(1024),
            dent: 0,
        }
    }

    pub fn consume(self) -> String {
        String::from_utf8(self.buf).expect("JSON have invalid UTF-8")
    }
}

impl json::codegen::Generator for MyPrettyGenerator {
    type T = Vec<u8>;

    #[inline(always)]
    fn get_writer(&mut self) -> &mut Vec<u8> {
        &mut self.buf
    }

    #[inline(always)]
    fn write(&mut self, slice: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(&mut self.get_writer(), slice)
    }

    #[inline(always)]
    fn write_char(&mut self, ch: u8) -> std::io::Result<()> {
        self.write(&[ch])
    }

    #[inline(always)]
    fn write_min(&mut self, slice: &[u8], _: u8) -> std::io::Result<()> {
        self.write(slice)
    }

    #[inline(always)]
    fn new_line(&mut self) -> std::io::Result<()> {
        self.write_char(b'\n')?;
        for _ in 0..self.dent {
            self.write_char(b'\t')?;
        }
        Ok(())
    }

    #[inline(always)]
    fn indent(&mut self) {
        self.dent += 1;
    }

    #[inline(always)]
    fn dedent(&mut self) {
        self.dent -= 1;
    }

    #[inline(always)]
    fn write_number(&mut self, num: &json::number::Number) -> std::io::Result<()> {
        if num.is_nan() {
            return self.write(b"null");
        }
        let (positive, mantissa, exponent) = num.as_parts();
        if exponent >= 0 {
            if positive {
                self.write(format!("{}", mantissa).as_bytes())
            } else {
                self.write(format!("{}", -(mantissa as i64)).as_bytes())
            }
        } else {
            let float = f32::from_bits(mantissa as u32);
            let float_str = format!("{:1.5}", float);
            self.write(float_str.as_bytes())
        }
    }

    fn write_json(&mut self, json: &JsonValue) -> std::io::Result<()> {
        match *json {
            JsonValue::Null => self.write(b"null"),
            JsonValue::Short(ref short) => self.write_string(short.as_str()),
            JsonValue::String(ref string) => self.write_string(string),
            JsonValue::Number(ref number) => self.write_number(number),
            JsonValue::Boolean(true) => self.write(b"true"),
            JsonValue::Boolean(false) => self.write(b"false"),
            JsonValue::Array(ref array) => {
                self.write_char(b'[')?;
                let mut iter = array.iter();

                if let Some(item) = iter.next() {
                    self.indent();
                    self.new_line()?;
                    self.write_json(item)?;
                } else {
                    self.write_char(b']')?;
                    return Ok(());
                }

                for item in iter {
                    if let JsonValue::Number(number) = item {
                        self.write(b", ")?;
                        self.write_number(number)?;
                    } else {
                        self.write_char(b',')?;
                        self.new_line()?;
                        self.write_json(item)?;
                    }
                }

                self.dedent();
                self.new_line()?;
                self.write_char(b']')
            }
            JsonValue::Object(ref object) => {
                self.write_char(b'{')?;
                let mut iter = object.iter();

                if let Some((key, value)) = iter.next() {
                    self.indent();
                    self.new_line()?;
                    self.write_string(key)?;
                    self.write(b": ")?;
                    self.write_json(value)?;
                } else {
                    self.write_char(b'}')?;
                    return Ok(());
                }

                for (key, value) in iter {
                    self.write_char(b',')?;
                    self.new_line()?;
                    self.write_string(key)?;
                    self.write(b": ")?;
                    self.write_json(value)?;
                }

                self.dedent();
                self.new_line()?;
                self.write_char(b'}')
            }
        }
    }
}

fn from_f32(float: f32) -> JsonValue {
    JsonValue::Number(unsafe {
        json::number::Number::from_parts_unchecked(false, float.to_bits() as u64, -1)
    })
}
