#[macro_use]
extern crate gfx;

mod render;

#[cfg_attr(rustfmt, rustfmt_skip)]
gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
});

pub use render::*;
