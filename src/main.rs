#[macro_use]
extern crate gfx;

extern crate camera_controllers;
extern crate piston_window;
extern crate sdl2_window;
extern crate vecmath;

use camera_controllers::{CameraPerspective, OrbitZoomCamera, OrbitZoomCameraSettings};
use gfx::Factory;
use gfx::traits::FactoryExt;
use piston_window::{OpenGL, PistonWindow, RenderEvent, ResizeEvent, Window, WindowSettings};
use sdl2_window::Sdl2Window;

gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
    tex_coords: [f32; 2] = "a_tex_coords",
});

gfx_pipeline!(terrain_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

// glClear(GL_STENCIL_BUFFER_BIT);
// glColorMask( GL_FALSE, GL_FALSE, GL_FALSE, GL_FALSE );
// glEnable( GL_CULL_FACE );
// glEnable(GL_DEPTH_TEST);
// glDepthMask(GL_FALSE);
// glDepthFunc(GL_GEQUAL);
// glEnable(GL_STENCIL_TEST);
// glStencilFunc(GL_ALWAYS, 0, 0);
// //set the stencil buffer operation
// glStencilOp(GL_KEEP, GL_KEEP,GL_INCR);
// //render the back-faces of the polyhedra
// glCullFace( GL_FRONT );
// DrawVectorPolyhedra();
gfx_pipeline!(vector_volume_forward_pipeline {
    depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::state::Depth {
            fun: gfx::state::Comparison::GreaterEqual,
            write: false,
        },
        gfx::state::Stencil::new(
            gfx::state::Comparison::Always,
            0,
            (
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::IncrementClamp
            ),
        ),
    ),
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

// //set the stencil buffer operation
// glStencilOp(GL_KEEP, GL_KEEP, GL_DECR);
// //render the front-faces of the polyhedra
// glCullFace( GL_BACK );
// DrawVectorPolyhedra();
gfx_pipeline!(vector_volume_backward_pipeline {
    depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::state::Depth {
            fun: gfx::state::Comparison::GreaterEqual,
            write: false,
        },
        gfx::state::Stencil::new(
            gfx::state::Comparison::Always,
            0,
            (
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::DecrementClamp,
            ),
        ),
    ),
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

// //draw the vector data
// //render the front-faces of the bounding box of the vector polyhedra
// glDepthMask( GL_TRUE );
// glColorMask( GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE );
// glCullFace( GL_FRONT );
// glDepthFunc(GL_GEQUAL);
// //set the stencil buffer operation
// glStencilFunc(GL_NOTEQUAL,0, 1);
// glStencilOp( GL_KEEP, GL_KEEP, GL_KEEP );
// DrawBoundingBoxofVectorPolyhedra();
// //resume the default setting
// glEnable( GL_CULL_FACE );
// glCullFace( GL_BACK );
// glDepthFunc(GL_LESS);
// glDisable(GL_STENCIL_TEST)
gfx_pipeline!(bounding_box_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::state::Depth {
            fun: gfx::state::Comparison::GreaterEqual,
            write: true,
        },
        gfx::state::Stencil::new(
            gfx::state::Comparison::NotEqual,
            0,
            (
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::Keep,
                gfx::state::StencilOp::Keep,
            ),
        ),
    ),
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

fn get_projection(window: &PistonWindow<Sdl2Window>) -> [[f32; 4]; 4] {
    let draw_size = window.window.draw_size();

    CameraPerspective {
        fov: 45.0,
        near_clip: 0.1,
        far_clip: 10000.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32),
    }.projection()
}

fn get_elevation(x: f32, y: f32) -> f32 {
    ((x / 10.0).sin() + (y / 5.0).sin()) * 10.0
}

const TERRAIN_SIDE_LENGTH: u16 = 100;

fn polygon_to_vertices_and_indices(polygon: &[(f32, f32)]) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (index, &(x, y)) in polygon.iter().enumerate() {
        let above = [x, y, 20.1];
        let below = [x, y, -20.1];

        vertices.push(Vertex {
            position: above,
            tex_coords: [0.0, 0.0],
        });
        vertices.push(Vertex {
            position: below,
            tex_coords: [0.0, 0.0],
        });

        let a = 2 * index as u16;
        let b = a + 1;
        let c = 2 * ((index as u16 + 1) % (polygon.len() as u16));
        let d = c + 1;

        indices.extend_from_slice(&[a, b, d, d, c, a]);

        if index != 0 && index != polygon.len() - 1 {
            indices.extend_from_slice(&[0, a, c, 1, d, b]);
        }
    }

    (vertices, indices)
}

fn main() {
    let mut window: PistonWindow<Sdl2Window> =
        WindowSettings::new("Shadow Volume Draping Demo", [800, 600])
            .exit_on_esc(true)
            .opengl(OpenGL::V3_2)
            .build()
            .unwrap();

    let mut factory = window.factory.clone();

    let mut terrain_vertices = Vec::new();
    let mut terrain_indices = Vec::new();
    let mut terrain_texture_data = Vec::new();

    for y in 0..TERRAIN_SIDE_LENGTH {
        for x in 0..TERRAIN_SIDE_LENGTH {
            let max_value = TERRAIN_SIDE_LENGTH - 1;
            if y != max_value && x != max_value {
                let a = (x + 0) + (y + 0) * TERRAIN_SIDE_LENGTH;
                let b = (x + 1) + (y + 0) * TERRAIN_SIDE_LENGTH;
                let c = (x + 0) + (y + 1) * TERRAIN_SIDE_LENGTH;
                let d = (x + 1) + (y + 1) * TERRAIN_SIDE_LENGTH;

                terrain_indices.extend_from_slice(&[a, c, b, b, c, d]);
            }

            let (x, y) = (x as f32, y as f32);
            let (u, v) = (x / max_value as f32, y / max_value as f32);
            terrain_vertices.push(Vertex {
                position: [x, y, get_elevation(x, y)],
                tex_coords: [u, v],
            });

            terrain_texture_data.push([(255.0 * u) as u8, (255.0 * v) as u8, 0, 255]);
        }
    }

    let (terrain_vertex_buffer, terrain_slice) =
        factory.create_vertex_buffer_with_slice(&terrain_vertices, terrain_indices.as_slice());
    let (_, terrain_texture_view) = factory
        .create_texture_immutable::<gfx::format::Srgba8>(
            gfx::texture::Kind::D2(
                TERRAIN_SIDE_LENGTH,
                TERRAIN_SIDE_LENGTH,
                gfx::texture::AaMode::Single,
            ),
            &[terrain_texture_data.as_slice()],
        )
        .unwrap();

    let terrain_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
        gfx::texture::FilterMethod::Bilinear,
        gfx::texture::WrapMode::Clamp,
    ));

    let terrain_shader_set = factory
        .create_shader_set(
            include_bytes!("shaders/terrain.vert"),
            include_bytes!("shaders/terrain.frag"),
        )
        .unwrap();

    let terrain_pso = factory
        .create_pipeline_state(
            &terrain_shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            terrain_pipeline::new(),
        )
        .unwrap();

    let mut terrain_data = terrain_pipeline::Data {
        color_texture: (terrain_texture_view, terrain_sampler),
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth: window.output_stencil.clone(),
        vertex_buffer: terrain_vertex_buffer,
    };

    // // share this between forward, backward, and bounding-box
    // let vector_shader_set = factory
    //     .create_shader_set(
    //         include_bytes!("shaders/vector.vert"),
    //         include_bytes!("shaders/vector.frag"),
    //     )
    //     .unwrap();

    // let vector_polygon = vec![
    //     (10.0, 10.0),
    //     (20.0, 10.0),
    //     (20.0, 20.0),
    //     (10.0, 20.0),
    //     (10.0, 10.0),
    // ];
    // let (vector_volume_vertices, vector_volume_indices) =
    //     polygon_to_vertices_and_indices(&vector_polygon);
    // let (vector_volume_vertex_buffer, vector_volume_slice) =
    //     factory.create_vertex_buffer_with_slice(
    //         &vector_volume_vertices,
    //         vector_volume_indices.as_slice(),
    //     );

    // // forward-specific stuff
    // let forward_pso = factory
    //     .create_pipeline_state(
    //         &vector_shader_set,
    //         gfx::Primitive::TriangleList,
    //         gfx::state::Rasterizer::new_fill().with_cull_back(),
    //         vector_volume_forward_pipeline::new(),
    //     )
    //     .unwrap();
    // let mut forward_data = vector_volume_forward_pipeline::Data {
    //     mvp: [[0.0; 4]; 4],
    //     out_color: window.output_color.clone(),
    //     depth_stencil: (window.output_stencil.clone(), (0, 0)),
    //     vertex_buffer: vector_volume_vertex_buffer,
    // };

    // // backward-specific stuff
    // let backward_pso = factory
    //     .create_pipeline_state(
    //         &vector_shader_set,
    //         gfx::Primitive::TriangleList,
    //         gfx::state::Rasterizer::new_fill().with_cull_back(),
    //         vector_volume_backward_pipeline::new(),
    //     )
    //     .unwrap();
    // let mut backward_data = vector_volume_backward_pipeline::Data {
    //     mvp: [[0.0; 4]; 4],
    //     out_color: window.output_color.clone(),
    //     depth_stencil: (window.output_stencil.clone(), (0, 0)),
    //     vertex_buffer: vector_volume_vertex_buffer,
    // };

    // // bounding-box stuff
    // let bounding_box_polygon = vec![
    //     (-1.0, -1.0),
    //     (TERRAIN_SIDE_LENGTH as f32 + 1.0, -1.0),
    //     (TERRAIN_SIDE_LENGTH as f32 + 1.0, TERRAIN_SIDE_LENGTH as f32 + 1.0),
    //     (-1.0, TERRAIN_SIDE_LENGTH as f32 + 1.0),
    //     (-1.0, -1.0),
    // ];
    // let (bounding_box_vertices, bounding_box_indices) =
    //     polygon_to_vertices_and_indices(&bounding_box_polygon);
    // let (bounding_box_vertex_buffer, bounding_box_slice) =
    //     factory.create_vertex_buffer_with_slice(
    //         &bounding_box_vertices,
    //         bounding_box_indices.as_slice(),
    //     );
    // let bounding_box_pso = bounding_box_pipeline::Data {
    //     mvp: [[0.0; 4]; 4],
    //     out_color: window.output_color.clone(),
    //     depth_stencil: (window.output_stencil.clone(), (0, 0)),
    //     vertex_buffer: bounding_box_vertex_buffer,
    // };




    let mut camera_controller =
        OrbitZoomCamera::new([0.0, 0.0, 0.0], OrbitZoomCameraSettings::default());

    while let Some(event) = window.next() {
        camera_controller.event(&event);

        window.draw_3d(&event, |window| {
            let render_args = event.render_args().unwrap();

            window.encoder.clear(
                &window.output_color,
                [0.3, 0.3, 0.3, 1.0],
            );
            window.encoder.clear_depth(&window.output_stencil, 1.0);

            let mvp = camera_controllers::model_view_projection(
                vecmath::mat4_id(),
                camera_controller.camera(render_args.ext_dt).orthogonal(),
                get_projection(window),
            );

            terrain_data.mvp = mvp;
            window.encoder.draw(
                &terrain_slice,
                &terrain_pso,
                &terrain_data,
            );

//             vo.mvp = mvp;
//             window.encoder.draw(
//                 &vector_slice,
//                 &vector_pso,
//                 &vector_data,
//             );
        });

        event.resize(|_, _| {
            terrain_data.out_color = window.output_color.clone();
            terrain_data.out_depth = window.output_stencil.clone();

            // vector_data.out_color = window.output_color.clone();
            // vector_data.out_depth = window.output_stencil.clone();
        });
    }
}
