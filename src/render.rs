use gfx;
use gfx::traits::FactoryExt;

use Vertex;
use polygon::*;

gfx_pipeline!(z_fail_polyhedron_pipeline {
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_color: gfx::BlendTarget<gfx::format::Srgba8> = (
        "o_color",
        gfx::state::ColorMask::all(),
        gfx::preset::blend::ALPHA,
    ),
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

gfx_pipeline!(z_fail_bounding_box_pipeline {
    out_color: gfx::BlendTarget<gfx::format::Srgba8> = (
        "o_color",
        gfx::state::ColorMask::all(),
        gfx::preset::blend::ALPHA,
    ),
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    color: gfx::Global<[f32; 4]> = "u_color",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::PASS_TEST,
        gfx::state::Stencil::new(
            // A fragment is only "inside" the polyhedron, and thus supposed to be drawn, if the
            // stencil buffer is nonzero at that point.
            gfx::state::Comparison::NotEqual,
            255,
            (
                // An important property of the stencil is that it is all zeroes after this
                // pipeline runs, so that the next draw doesn't need to clear the stencil first.
                //
                // If the stencil test fails, the value is zero and thus should be kept.
                gfx::state::StencilOp::Keep,
                // This never happens, because the depth test always passes.
                gfx::state::StencilOp::Keep,
                // The stencil test passed, so the value should be reset to zero.
                gfx::state::StencilOp::Replace,
            ),
        ),
    ),
});

#[derive(Debug)]
pub struct DrapingRenderer<R: gfx::Resources> {
    polyhedron_pso: gfx::pso::PipelineState<R, z_fail_polyhedron_pipeline::Meta>,
    bounding_box_pso: gfx::pso::PipelineState<R, z_fail_bounding_box_pipeline::Meta>,
}

impl<R: gfx::Resources> DrapingRenderer<R> {
    /// Set up the pipeline state objects needed for rendering draped polygons.
    pub fn new<F: gfx::Factory<R>>(factory: &mut F) -> DrapingRenderer<R> {
        DrapingRenderer {
            polyhedron_pso: Self::polyhedron_pso(factory),
            bounding_box_pso: Self::bounding_box_pso(factory),
        }
    }

    /// Render a single `DrapeablePolygon` as `color`.
    ///
    /// *Note:* The depth buffer in `depth_stencil_target` should contain the depth values of your
    /// terrain -- in other words, draw your terrain just before you call this function, and make
    /// sure you don't clear the buffer until after rendering all the polygons you wish to draw.
    ///
    /// *Note:* In addition, the stencil buffer should be cleared to zero before calling this
    /// function. The stencil buffer is guaranteed to remain zero after each call, so there is no
    /// need to clear the stencil buffer between calls to this function.
    pub fn render<C: gfx::CommandBuffer<R>, F: gfx::Factory<R>>(
        &self,
        factory: &mut F,
        encoder: &mut gfx::Encoder<R, C>,
        render_target: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        depth_stencil_target: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        mvp: [[f32; 4]; 4],
        color: [f32; 4],
        buffer: &RenderablePolygonBuffer<R>,
        indices: &RenderablePolygonIndices<R>,
    ) {
        // let polyhedron_slice = indices.polyhedron_slice
        // let polyhedron_vertex_buffer = buffer.polyhedron_vertex_buffer(factory);

        // let bounding_box_slice = indices.bounding_box_slice(factory);
        // let bounding_box_vertex_buffer = buffer.bounding_box_vertex_buffer(factory);

        let polyhedron_bundle = gfx::Bundle {
            pso: self.polyhedron_pso.clone(),
            slice: indices.polyhedron_slice.clone(),
            data: z_fail_polyhedron_pipeline::Data {
                mvp: mvp,
                out_color: render_target.clone(),
                out_depth_stencil: (depth_stencil_target.clone(), (0, 0)),
                vertex_buffer: buffer.polyhedron_vertex_buffer.clone(),
            },
        };

        let bounding_box_bundle = gfx::Bundle {
            pso: self.bounding_box_pso.clone(),
            slice: indices.bounding_box_slice.clone(),
            data: z_fail_bounding_box_pipeline::Data {
                color: color,
                mvp: mvp,
                out_color: render_target.clone(),
                out_depth_stencil: (depth_stencil_target.clone(), (0, 0)),
                vertex_buffer: buffer.bounding_box_vertex_buffer.clone(),
            },
        };

        polyhedron_bundle.encode(encoder);
        bounding_box_bundle.encode(encoder);
    }

    fn polyhedron_pso<F: gfx::Factory<R>>(
        factory: &mut F,
    ) -> gfx::pso::PipelineState<R, z_fail_polyhedron_pipeline::Meta> {
        let shaders = factory
            .create_shader_set(
                include_bytes!("shaders/polyhedron.vert"),
                include_bytes!("shaders/polyhedron.frag"),
            )
            .unwrap();

        let rasterizer = gfx::state::Rasterizer::new_fill();

        factory
            .create_pipeline_state(
                &shaders,
                gfx::Primitive::TriangleList,
                rasterizer,
                z_fail_polyhedron_pipeline::new(),
            )
            .unwrap()
    }

    fn bounding_box_pso<F: gfx::Factory<R>>(
        factory: &mut F,
    ) -> gfx::pso::PipelineState<R, z_fail_bounding_box_pipeline::Meta> {
        let shaders = factory
            .create_shader_set(
                include_bytes!("shaders/bounding_box.vert"),
                include_bytes!("shaders/bounding_box.frag"),
            )
            .unwrap();

        let rasterizer = gfx::state::Rasterizer {
            cull_face: gfx::state::CullFace::Front,
            ..gfx::state::Rasterizer::new_fill()
        };

        factory
            .create_pipeline_state(
                &shaders,
                gfx::Primitive::TriangleList,
                rasterizer,
                z_fail_bounding_box_pipeline::new(),
            )
            .unwrap()
    }
}

pub struct RenderablePolygonBuffer<R: gfx::Resources> {
    polyhedron_vertex_buffer: gfx::handle::Buffer<R, Vertex>,
    bounding_box_vertex_buffer: gfx::handle::Buffer<R, Vertex>,
}

impl<R: gfx::Resources> RenderablePolygonBuffer<R> {
    pub fn new<F: gfx::Factory<R>>(factory: &mut F, buffer: &PolygonBuffer) -> RenderablePolygonBuffer<R> {
        RenderablePolygonBuffer {
            polyhedron_vertex_buffer: factory.create_vertex_buffer(&buffer.polyhedron_vertices),
            bounding_box_vertex_buffer: factory.create_vertex_buffer(&buffer.bounding_box_vertices),
        }
    }
}

pub struct RenderablePolygonIndices<R: gfx::Resources> {
    polyhedron_slice: gfx::Slice<R>,
    bounding_box_slice: gfx::Slice<R>,
}

impl<R: gfx::Resources> RenderablePolygonIndices<R> {
    pub fn new<F: gfx::Factory<R>>(factory: &mut F, indices: &PolygonBufferIndices) -> RenderablePolygonIndices<R> {
        RenderablePolygonIndices {
            polyhedron_slice: Self::create_slice(factory, &indices.polyhedron_indices),
            bounding_box_slice: Self::create_slice(factory, &indices.bounding_box_indices),
        }
    }

    fn create_slice<F: gfx::Factory<R>>(factory: &mut F, indices: &[u32]) -> gfx::Slice<R> {
        gfx::Slice {
            start: 0,
            end: indices.len() as u32,
            base_vertex: 0,
            instances: None,
            buffer: factory.create_index_buffer(indices),
        }
    }
}
