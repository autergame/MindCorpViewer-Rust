use gl::types::{GLchar, GLenum, GLint, GLuint};
use std::{ffi::CString, ptr};

pub struct Shader {
    id: GLuint,
}

impl Shader {
    fn load_shader(shader_type: GLenum, source: &str) -> GLuint {
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

            shader
        }
    }

    pub fn create(vertex_data: &str, fragment_data: &str) -> Shader {
        let vertex = Self::load_shader(gl::VERTEX_SHADER, vertex_data);
        let fragment = Self::load_shader(gl::FRAGMENT_SHADER, fragment_data);

        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex);
            gl::AttachShader(program, fragment);
            gl::LinkProgram(program);

            let mut success = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut info_len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut info_len);

                let info_log = CString::from_vec_unchecked(vec![0u8; info_len as usize]);
                gl::GetProgramInfoLog(
                    program,
                    info_len,
                    ptr::null_mut(),
                    info_log.as_ptr() as *mut GLchar,
                );

                println!("Could not create shader\n{}", info_log.to_string_lossy());
            }

            gl::UseProgram(0);
            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);

            Shader { id: program }
        }
    }

    pub fn get_refs(&self, names: &[&str]) -> Vec<GLint> {
        unsafe {
            let mut refs: Vec<GLint> = Vec::with_capacity(names.len());
            self.enable();
            for name in names {
                let c_str_name = CString::new(*name).expect("Could not create ref CString");
                refs.push(gl::GetUniformLocation(self.id, c_str_name.as_ptr()));
            }
            refs
        }
    }

    pub fn get_ubo_ref(&self, name: &str) -> GLuint {
        unsafe {
            let c_str_name = CString::new(name).expect("Could not create ubo ref CString");
            self.enable();
            gl::GetUniformBlockIndex(self.id, c_str_name.as_ptr())
        }
    }

    pub fn ubo_binding(&self, ubo_ref: GLuint, binding: GLuint) {
        unsafe {
            self.enable();
            gl::UniformBlockBinding(self.id, ubo_ref, binding);
        }
    }

	pub fn set_uniform_1_int(&self, glref: GLint, unit: i32) {
        unsafe {
            gl::Uniform1i(glref, unit);
        }
    }

    pub fn enable(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
