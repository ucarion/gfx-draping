#[macro_use]
extern crate gfx;

extern crate camera_controllers;
extern crate piston_window;
extern crate vecmath;

use camera_controllers::{CameraPerspective, OrbitZoomCamera, OrbitZoomCameraSettings};
use gfx::Factory;
use gfx::traits::FactoryExt;
use piston_window::{OpenGL, PistonWindow, RenderEvent, ResizeEvent, Window, WindowSettings};

gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
    tex_coords: [f32; 2] = "a_tex_coords",
});

// out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
// out_stencil: gfx::StencilTarget<gfx::format::DepthStencil> = gfx::state::Stencil::new(
//     gfx::state::Comparison::LessEqual,
//     0,
//     (gfx::state::StencilOp::IncrementClamp, gfx::state::StencilOp::Keep, gfx::state::StencilOp::Keep),
// ),
gfx_pipeline!(terrain_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::LESS_EQUAL_WRITE,
        gfx::state::Stencil::new(
            gfx::state::Comparison::Always,
            255,
            (
                gfx::state::StencilOp::Keep, // never happens if Comparison::Always
                gfx::state::StencilOp::Keep, // when depth test fails
                gfx::state::StencilOp::Keep, // when depth test passes
            ),
        ),
    ),
});

gfx_pipeline!(z_fail_polyhedron_pipeline {
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::LESS_EQUAL_TEST,
        gfx::state::Stencil {
            front: gfx::state::StencilSide {
                fun: gfx::state::Comparison::Always,
                mask_read: 255,
                mask_write: 255,
                op_fail: gfx::state::StencilOp::Keep,
                op_depth_fail: gfx::state::StencilOp::DecrementWrap,
                op_pass: gfx::state::StencilOp::Keep,
            },
            back: gfx::state::StencilSide {
                fun: gfx::state::Comparison::Always,
                mask_read: 255,
                mask_write: 255,
                op_fail: gfx::state::StencilOp::Keep,
                op_depth_fail: gfx::state::StencilOp::IncrementWrap,
                op_pass: gfx::state::StencilOp::Keep,
            },
        },
    ),
});

// Only draw back faces with this!
gfx_pipeline!(z_fail_bounding_box_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::PASS_TEST,
        gfx::state::Stencil::new(
            gfx::state::Comparison::NotEqual,
            255,
            (
                gfx::state::StencilOp::Keep, // never happens, depth test always passed
                gfx::state::StencilOp::Keep, // if it's not not equal to zero, cool, it's zero!
                gfx::state::StencilOp::Replace, // if it's not equal to zero, replace it with zero
            ),
        ),
    ),
});

// gfx_vertex_struct!(PlainVertex {
//     // this is to make rustfmt go away
//     coords: [f32; 2] = "a_coords",
// });

// gfx_pipeline!(just_texture_pipeline {
//     out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
//     out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
//     color_texture: gfx::TextureSampler<u32> = "t_texture",
//     vertex_buffer: gfx::VertexBuffer<PlainVertex> = (),
// });

// // XXX these only work for the z-fail approach
// gfx_pipeline!(back_face_pipeline {
//     polyhedron: gfx::VertexBuffer<Vertex> = (),
//     out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
//     out_depth: gfx::StencilTarget<gfx::format::DepthStencil> = gfx::state::Stencil::new(

//     ),
// });

fn get_projection(window: &PistonWindow) -> [[f32; 4]; 4] {
    let draw_size = window.window.draw_size();

    CameraPerspective {
        fov: 45.0,
        near_clip: 10.0,
        far_clip: 1000.0,
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
    let mut window: PistonWindow = WindowSettings::new("Shadow Volume Draping Demo", [800, 600])
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
                position: [x, -y, get_elevation(x, y)],
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
            gfx::state::Rasterizer::new_fill().with_cull_back(),
            terrain_pipeline::new(),
        )
        .unwrap();

    // let (_depth_stencil_texture, depth_stencil_srv, depth_stencil_rtv) = factory
    //     .create_depth_stencil::<gfx::format::DepthStencil>(800, 600)
    //     .unwrap();
    // let (_render_target_tex, _render_target_srv, render_target_view) = factory
    //     .create_render_target::<gfx::format::Srgba8>(800, 600)
    //     .unwrap();

    // let stencil_srv = factory
    //     .view_texture_as_shader_resource::<(gfx::format::D24_S8, gfx::format::Uint)>(
    //         &_depth_stencil_texture,
    //         (0, 0),
    //         gfx::format::Swizzle::new(),
    //     )
    //     .unwrap();

    let terrain_data = terrain_pipeline::Data {
        color_texture: (terrain_texture_view.clone(), terrain_sampler.clone()),
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth_stencil: (window.output_stencil.clone(), (255, 255)),
        vertex_buffer: terrain_vertex_buffer,
    };

    let mut terrain_bundle = gfx::Bundle {
        slice: terrain_slice,
        pso: terrain_pso,
        data: terrain_data,
    };

    let polygon_shader_set = factory
        .create_shader_set(
            include_bytes!("shaders/vector.vert"),
            include_bytes!("shaders/vector.frag"),
        )
        .unwrap();
    let polygon_pso = factory
        .create_pipeline_state(
            &polygon_shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            z_fail_polyhedron_pipeline::new(),
        )
        .unwrap();
    let polygon = vec![
        (40.0, -60.0),
        (60.0, -60.0),
        (60.0, -40.0),
        (40.0, -40.0),
        (40.0, -60.0),
    ];
    let (polygon_vertices, polygon_indices) = polygon_to_vertices_and_indices(&polygon);
    let (polygon_vertex_buffer, polygon_slice) =
        factory.create_vertex_buffer_with_slice(&polygon_vertices, &polygon_indices[..]);
    let polygon_data = z_fail_polyhedron_pipeline::Data {
        mvp: [[0.0; 4]; 4],
        out_depth_stencil: (window.output_stencil.clone(), (0, 0)),
        vertex_buffer: polygon_vertex_buffer,
    };
    let mut polygon_bundle = gfx::Bundle {
        slice: polygon_slice,
        pso: polygon_pso,
        data: polygon_data,
    };

    let mut bbox_rasterizer = gfx::state::Rasterizer::new_fill();
    bbox_rasterizer.cull_face = gfx::state::CullFace::Front;
    let bbox_pso = factory
        .create_pipeline_state(
            &terrain_shader_set,
            gfx::Primitive::TriangleList,
            bbox_rasterizer,
            z_fail_bounding_box_pipeline::new(),
        )
        .unwrap();
    let bbox_points = vec![
        (0.0, -(TERRAIN_SIDE_LENGTH as f32)),
        (TERRAIN_SIDE_LENGTH as f32, -(TERRAIN_SIDE_LENGTH as f32)),
        (TERRAIN_SIDE_LENGTH as f32, -0.0),
        (0.0, -0.0),
        (0.0, -(TERRAIN_SIDE_LENGTH as f32)),
    ];
    let (bbox_vertices, bbox_indices) = polygon_to_vertices_and_indices(&bbox_points);
    let (bbox_vertex_buffer, bbox_slice) =
        factory.create_vertex_buffer_with_slice(&bbox_vertices, &bbox_indices[..]);
    let bbox_data = z_fail_bounding_box_pipeline::Data {
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth_stencil: (window.output_stencil.clone(), (0, 0)),
        vertex_buffer: bbox_vertex_buffer,
        color_texture: (terrain_texture_view.clone(), terrain_sampler.clone()),
    };
    let mut bbox_bundle = gfx::Bundle {
        slice: bbox_slice,
        pso: bbox_pso,
        data: bbox_data,
    };

    let mut camera_controller =
        OrbitZoomCamera::new([0.0, 0.0, 0.0], OrbitZoomCameraSettings::default());

    while let Some(event) = window.next() {
        camera_controller.event(&event);

        event.resize(|height, width| {
            // just_texture_bundle.data.out_color = window.output_color.clone();
            // just_texture_bundle.data.out_depth = window.output_stencil.clone();

            // let (_depth_stencil_texture, depth_stencil_srv, depth_stencil_rtv) = factory
            //     .create_depth_stencil::<gfx::format::DepthStencil>(height as u16, width as u16)
            //     .unwrap();
            // let (_render_target_tex, _render_target_srv, render_target_view) = factory
            //     .create_render_target::<gfx::format::Srgba8>(height as u16, width as u16)
            //     .unwrap();

            // let stencil_srv = factory
            //     .view_texture_as_shader_resource::<(gfx::format::D24_S8, gfx::format::Uint)>(
            //         &_depth_stencil_texture,
            //         (0, 0),
            //         gfx::format::Swizzle::new(),
            //     )
            //     .unwrap();

            terrain_bundle.data.out_color = window.output_color.clone();
            terrain_bundle.data.out_depth_stencil.0 = window.output_stencil.clone();
            polygon_bundle.data.out_depth_stencil.0 = window.output_stencil.clone();
            bbox_bundle.data.out_color = window.output_color.clone();
            bbox_bundle.data.out_depth_stencil.0 = window.output_stencil.clone();

            // terrain_bundle.data.out_color = render_target_view.clone();
            // terrain_bundle.data.out_depth_stencil = (depth_stencil_rtv.clone(), (0, 0));
            // polygon_bundle.data.out_color = render_target_view.clone();
            // polygon_bundle.data.out_depth_stencil = (depth_stencil_rtv.clone(), (0, 0));

            // just_texture_bundle.data.color_texture.0 = stencil_srv;
            // just_texture_bundle.data.out_color = window.output_color.clone();
            // just_texture_bundle.data.out_depth = window.output_stencil.clone();
        });

        window.draw_3d(&event, |window| {
            let render_args = event.render_args().unwrap();

            window.encoder.clear(
                &window.output_color,
                [0.3, 0.3, 0.3, 1.0],
            );
            window.encoder.clear_depth(&window.output_stencil, 1.0);
            // window.encoder.clear_stencil(
            //     &terrain_bundle.data.out_depth_stencil.0,
            //     0,
            // );

            // window.encoder.clear(
            //     &terrain_bundle.data.out_color,
            //     [0.3, 0.3, 0.3, 1.0],
            // );

            // window.encoder.clear_depth(
            //     &terrain_bundle.data.out_depth_stencil.0,
            //     1.0,
            // );

            let mvp = camera_controllers::model_view_projection(
                vecmath::mat4_id(),
                camera_controller.camera(render_args.ext_dt).orthogonal(),
                get_projection(window),
            );

            terrain_bundle.data.mvp = mvp;
            terrain_bundle.encode(&mut window.encoder);

            polygon_bundle.data.mvp = mvp;
            polygon_bundle.encode(&mut window.encoder);

            bbox_bundle.data.mvp = mvp;
            bbox_bundle.encode(&mut window.encoder);
        });
    }
}
