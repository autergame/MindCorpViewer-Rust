// https://github.com/K4ugummi/imgui-glfw-rs

struct GlfwClipboardBackend {
    window: *mut glfw::ffi::GLFWwindow,
}

impl GlfwClipboardBackend {
    fn new(window: *mut glfw::ffi::GLFWwindow) -> GlfwClipboardBackend {
        GlfwClipboardBackend { window }
    }
}

impl imgui::ClipboardBackend for GlfwClipboardBackend {
    fn get(&mut self) -> Option<String> {
        let char_ptr = unsafe { glfw::ffi::glfwGetClipboardString(self.window) };
        if !char_ptr.is_null() {
            let c_str = unsafe { std::ffi::CStr::from_ptr(char_ptr) };
            Some(c_str.to_str().unwrap().to_owned())
        } else {
            None
        }
    }
    fn set(&mut self, value: &str) {
        unsafe {
            glfw::ffi::glfwSetClipboardString(self.window, value.as_ptr() as *const i8);
        };
    }
}

pub struct ImguiGLFW {
    renderer: imgui_opengl_renderer::Renderer,
}

impl ImguiGLFW {
    pub fn new(imgui: &mut imgui::Context, window: &mut glfw::Window) -> Self {
        unsafe {
            imgui.set_clipboard_backend(GlfwClipboardBackend::new(
                glfw::ffi::glfwGetCurrentContext(),
            ));
        }

        let mut io_mut = imgui.io_mut();
        io_mut.key_map[imgui::Key::Tab as usize] = glfw::Key::Tab as u32;
        io_mut.key_map[imgui::Key::LeftArrow as usize] = glfw::Key::Left as u32;
        io_mut.key_map[imgui::Key::RightArrow as usize] = glfw::Key::Right as u32;
        io_mut.key_map[imgui::Key::UpArrow as usize] = glfw::Key::Up as u32;
        io_mut.key_map[imgui::Key::DownArrow as usize] = glfw::Key::Down as u32;
        io_mut.key_map[imgui::Key::PageUp as usize] = glfw::Key::PageUp as u32;
        io_mut.key_map[imgui::Key::PageDown as usize] = glfw::Key::PageDown as u32;
        io_mut.key_map[imgui::Key::Home as usize] = glfw::Key::Home as u32;
        io_mut.key_map[imgui::Key::End as usize] = glfw::Key::End as u32;
        io_mut.key_map[imgui::Key::Insert as usize] = glfw::Key::Insert as u32;
        io_mut.key_map[imgui::Key::Delete as usize] = glfw::Key::Delete as u32;
        io_mut.key_map[imgui::Key::Backspace as usize] = glfw::Key::Backspace as u32;
        io_mut.key_map[imgui::Key::Space as usize] = glfw::Key::Space as u32;
        io_mut.key_map[imgui::Key::Enter as usize] = glfw::Key::Enter as u32;
        io_mut.key_map[imgui::Key::Escape as usize] = glfw::Key::Escape as u32;
        io_mut.key_map[imgui::Key::A as usize] = glfw::Key::A as u32;
        io_mut.key_map[imgui::Key::C as usize] = glfw::Key::C as u32;
        io_mut.key_map[imgui::Key::V as usize] = glfw::Key::V as u32;
        io_mut.key_map[imgui::Key::X as usize] = glfw::Key::X as u32;
        io_mut.key_map[imgui::Key::Y as usize] = glfw::Key::Y as u32;
        io_mut.key_map[imgui::Key::Z as usize] = glfw::Key::Z as u32;

        let renderer =
            imgui_opengl_renderer::Renderer::new(imgui, |s| window.get_proc_address(s) as _);

        Self { renderer }
    }

    pub fn handle_event(&mut self, imgui: &mut imgui::Context, event: &glfw::WindowEvent) {
        match *event {
            glfw::WindowEvent::MouseButton(mouse_btn, action, _) => {
                let index = match mouse_btn {
                    glfw::MouseButton::Button1 => 0,
                    glfw::MouseButton::Button2 => 1,
                    glfw::MouseButton::Button3 => 2,
                    glfw::MouseButton::Button4 => 3,
                    glfw::MouseButton::Button5 => 4,
                    _ => 0,
                };
                imgui.io_mut().mouse_down[index] = action != glfw::Action::Release;
            }
            glfw::WindowEvent::CursorPos(x, y) => {
                imgui.io_mut().mouse_pos = [x as f32, y as f32];
            }
            glfw::WindowEvent::Scroll(_, d) => {
                imgui.io_mut().mouse_wheel = d as f32;
            }
            glfw::WindowEvent::Char(character) => {
                imgui.io_mut().add_input_character(character);
            }
            glfw::WindowEvent::Key(key, _, action, modifier) => {
                Self::set_mod(imgui, modifier);
                imgui.io_mut().keys_down[key as usize] = action != glfw::Action::Release;
            }
            _ => {}
        }
    }

    pub fn frame<'a>(
        &mut self,
        delta_time: f32,
        window: &glfw::Window,
        imgui: &'a mut imgui::Context,
    ) -> imgui::Ui<'a> {
        let io = imgui.io_mut();

        io.delta_time = delta_time;

        let window_size = window.get_size();
        io.display_size = [window_size.0 as f32, window_size.1 as f32];

        if window_size.0 > 0 && window_size.1 > 0 {
            let framebuffer_size = window.get_framebuffer_size();
            io.display_framebuffer_scale = [
                framebuffer_size.0 as f32 / io.display_size[0],
                framebuffer_size.1 as f32 / io.display_size[1],
            ];
        }

        imgui.frame()
    }

    pub fn draw<'ui>(&mut self, ui: imgui::Ui<'ui>, window: &mut glfw::Window) {
        let io = ui.io();
        if !io
            .config_flags
            .contains(imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE)
        {
            match ui.mouse_cursor() {
                Some(mouse_cursor) if !io.mouse_draw_cursor => {
                    window.set_cursor_mode(glfw::CursorMode::Normal);

                    let cursor = match mouse_cursor {
                        imgui::MouseCursor::TextInput => glfw::StandardCursor::IBeam,
                        imgui::MouseCursor::ResizeNS => glfw::StandardCursor::VResize,
                        imgui::MouseCursor::ResizeEW => glfw::StandardCursor::HResize,
                        imgui::MouseCursor::Hand => glfw::StandardCursor::Hand,
                        _ => glfw::StandardCursor::Arrow,
                    };
                    window.set_cursor(Some(glfw::Cursor::standard(cursor)));
                }
                _ => {
                    window.set_cursor_mode(glfw::CursorMode::Hidden);
                }
            }
        }
        self.renderer.render(ui);
    }

    fn set_mod(imgui: &mut imgui::Context, modifier: glfw::Modifiers) {
        imgui.io_mut().key_ctrl = modifier.intersects(glfw::Modifiers::Control);
        imgui.io_mut().key_alt = modifier.intersects(glfw::Modifiers::Alt);
        imgui.io_mut().key_shift = modifier.intersects(glfw::Modifiers::Shift);
        imgui.io_mut().key_super = modifier.intersects(glfw::Modifiers::Super);
    }
}
