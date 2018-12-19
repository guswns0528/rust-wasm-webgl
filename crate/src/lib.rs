#![feature(duration_as_u128)]
#[macro_use]
extern crate cfg_if;
extern crate web_sys;
extern crate js_sys;
extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        fn set_panic_hook() {}
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

type GlContext = web_sys::WebGl2RenderingContext;
type GlShader = web_sys::WebGlShader;
type GlProgram = web_sys::WebGlProgram;

fn create_shader(gl: &GlContext, shader_type: u32, source: &str) -> GlShader {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    console::log_1(&JsValue::from_str(&gl.get_shader_info_log(&shader).unwrap()));
    shader
}

fn create_program(gl: &GlContext, vertex_shader: &GlShader, fragment_shader: &GlShader) -> GlProgram {
    let program = gl.create_program().unwrap();
    gl.attach_shader(&program, vertex_shader);
    gl.attach_shader(&program, fragment_shader);
    gl.link_program(&program);
    program
}

fn resize_canvas(canvas: &web_sys::HtmlCanvasElement) {
    let display_width = canvas.client_width() as u32;
    let display_height = canvas.client_height() as u32;
    let width = canvas.width();
    let height = canvas.height();

    if width != display_width || height != display_height {
        canvas.set_width(display_width);
        canvas.set_height(display_height);
    }
}

fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}

fn request_animation_frame(f: &Closure<FnMut()>) {
    window().request_animation_frame(f.as_ref().unchecked_ref()).unwrap();
}

fn get_canvas() -> web_sys::HtmlCanvasElement {
    let window = window();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("webgl_canvas").unwrap();
    canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap()
}

fn gl_context(canvas: &web_sys::HtmlCanvasElement) -> GlContext {
    canvas.get_context("webgl2").unwrap().unwrap().dyn_into::<GlContext>().unwrap()
}

use js_sys::*;
use web_sys::console;

extern crate cgmath;
use cgmath::{Matrix4, ortho};
use cgmath::prelude::*;
use cgmath::conv::*;

use std::cell::RefCell;
use std::rc::Rc;

// Called by our JS entry point to run the example.
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();

    let canvas = get_canvas();

    resize_canvas(&canvas);
    let gl = gl_context(&canvas);
    let vertex_shader_code = r#"#version 300 es
in vec4 a_position;
in vec4 a_color;

uniform mat4 world;

out vec4 color;

void main() {
    gl_Position = world * a_position;
    color = a_color;
}"#;
    let fragment_shader_code = r#"#version 300 es
precision mediump float;

in vec4 color;
out vec4 outColor;

void main() {
    outColor = color;
}"#;

    let vertex_shader = create_shader(&gl, GlContext::VERTEX_SHADER, vertex_shader_code);
    let fragment_shader = create_shader(&gl, GlContext::FRAGMENT_SHADER, fragment_shader_code);

    let program = create_program(&gl, &vertex_shader, &fragment_shader);
    gl.use_program(Some(&program));

    let vertex_buffer = gl.create_buffer();
    gl.bind_buffer(GlContext::ARRAY_BUFFER, vertex_buffer.as_ref());

    let memory = wasm_bindgen::memory().dyn_into::<WebAssembly::Memory>()?.buffer();

    let sqrt3: f32 = 1.7320508075688772935274463415059;
    let tri_height: f32 = 0.7;

    let vertices: [f32; 7 * 6] = [
        -tri_height / sqrt3, tri_height, 1.0, 0.0, 0.0, 1.0,
        tri_height / sqrt3, tri_height, 1.0, 1.0, 0.0, 1.0,
        tri_height * 2.0 / sqrt3, 0.0, 0.0, 1.0, 0.0, 1.0,
        tri_height / sqrt3, -tri_height, 0.0, 1.0, 1.0, 1.0,
        -tri_height / sqrt3, -tri_height, 0.0, 0.0, 1.0, 1.0,
        -tri_height * 2.0 / sqrt3, 0.0, 1.0, 0.0, 1.0, 1.0,
        0.0, 0.0, 0.5, 0.5, 0.5, 1.0,
    ];
    let vert_loc = vertices.as_ptr() as u32 / 4;
    let vertex_data = Float32Array::new(&memory).subarray(vert_loc, vert_loc + vertices.len() as u32);

    gl.buffer_data_with_array_buffer_view(GlContext::ARRAY_BUFFER, &vertex_data, GlContext::STATIC_DRAW);

    let vao = gl.create_vertex_array();
    gl.bind_vertex_array(vao.as_ref());

    let pos_location = gl.get_attrib_location(&program, "a_position") as u32;
    gl.enable_vertex_attrib_array(pos_location);
    gl.vertex_attrib_pointer_with_i32(pos_location, 2, GlContext::FLOAT, false, 24, 0);

    let color_loc = gl.get_attrib_location(&program, "a_color") as u32;
    gl.enable_vertex_attrib_array(color_loc);
    gl.vertex_attrib_pointer_with_i32(color_loc, 4, GlContext::FLOAT, false, 24, 8);

    let viewport_matrix: Matrix4<f32> = ortho(-4.0 / 3.0, 4.0 / 3.0, -1.0, 1.0, -1.0, 1.0);
    let world_loc = gl.get_uniform_location(&program, "world");
    gl.uniform_matrix4fv_with_f32_array(world_loc.as_ref(), false, unsafe { std::slice::from_raw_parts_mut(array4x4(viewport_matrix)[0].as_mut_ptr(), 16)} );

    let index_buffer = gl.create_buffer();
    gl.bind_buffer(GlContext::ELEMENT_ARRAY_BUFFER, index_buffer.as_ref());

    let indices: [u32; 8] = [6, 0, 1, 2, 3, 4, 5, 0];
    let index_loc = indices.as_ptr() as u32 / 4;
    let index_data = Uint32Array::new(&memory).subarray(index_loc, index_loc + indices.len() as u32);

    gl.buffer_data_with_array_buffer_view(GlContext::ELEMENT_ARRAY_BUFFER, &index_data, GlContext::STATIC_DRAW);

    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(GlContext::COLOR_BUFFER_BIT);

    gl.draw_elements_with_i32(GlContext::TRIANGLE_FAN, 8, GlContext::UNSIGNED_INT, 0);

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    {
        let mut current: Matrix4<f32> = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );
        let mut prev = window().performance().unwrap().now();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let now = window().performance().unwrap().now();
            let duration = (now - prev) / 1000.0;
            prev = now;
            let rotation = Matrix4::from_angle_z(cgmath::Deg((-60.0 * duration) as f32));
            current = current * rotation;
            let world = viewport_matrix * current;
            let world_loc = gl.get_uniform_location(&program, "world");
            gl.uniform_matrix4fv_with_f32_array(world_loc.as_ref(), false, unsafe { std::slice::from_raw_parts_mut(array4x4(world)[0].as_mut_ptr(), 16)});

            gl.clear(GlContext::COLOR_BUFFER_BIT);
            gl.draw_elements_with_i32(GlContext::TRIANGLE_FAN, 8, GlContext::UNSIGNED_INT, 0);
            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<FnMut()>));
    }

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}
