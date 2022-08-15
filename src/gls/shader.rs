use std::{ffi::CString, fs::File, io::Read, path::Path, ptr};

use gl::types::{GLchar, GLenum, GLint, GLuint};

fn load_shader(shader_type: GLenum, path: &Path) -> GLuint {
    println!("Reading shader file: {}", path.to_str().unwrap());
    let mut file = File::open(path).expect("Could not open shader file");
    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("Could not read shader file");

    unsafe {
        let c_str_source = CString::new(source).expect("Could not create source CString");

        let shader = gl::CreateShader(shader_type);
        gl::ShaderSource(shader, 1, &c_str_source.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        let mut success = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut info_len: GLint = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_len);

            let info_log = CString::from_vec_unchecked(vec![0u8; info_len as usize]);
            gl::GetShaderInfoLog(
                shader,
                info_len,
                ptr::null_mut(),
                info_log.as_ptr() as *mut GLchar,
            );

            println!("Could not load shader\n{}", info_log.to_string_lossy());
        }

        println!("Finished reading shader file");

        shader
    }
}

#[inline(never)]
pub fn create_shader(vertex_path: &Path, fragment_path: &Path) -> GLuint {
    let vertex_shader = load_shader(gl::VERTEX_SHADER, vertex_path);
    let fragment_shader = load_shader(gl::FRAGMENT_SHADER, fragment_path);

    unsafe {
        let shader_program = gl::CreateProgram();
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);
        gl::LinkProgram(shader_program);

        let mut success = gl::FALSE as GLint;
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut info_len: GLint = 0;
            gl::GetProgramiv(shader_program, gl::INFO_LOG_LENGTH, &mut info_len);

            let info_log = CString::from_vec_unchecked(vec![0u8; info_len as usize]);
            gl::GetProgramInfoLog(
                shader_program,
                info_len,
                ptr::null_mut(),
                info_log.as_ptr() as *mut GLchar,
            );

            println!("Could not create shader\n{}", info_log.to_string_lossy());
        }

        gl::UseProgram(0);
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);

        shader_program
    }
}

#[inline(never)]
pub fn get_refs_shader(shader_program: GLuint, names: &[&str]) -> Vec<GLint> {
    unsafe {
        let mut refs: Vec<GLint> = Vec::with_capacity(names.len());
        gl::UseProgram(shader_program);
        for name in names {
            let c_str_name = CString::new(*name).expect("Could not create ref CString");
            refs.push(gl::GetUniformLocation(shader_program, c_str_name.as_ptr()));
        }
        refs
    }
}
