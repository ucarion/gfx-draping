use gfx;
use gfx::traits::FactoryExt;

use Vertex;
use render::*;

#[derive(Debug)]
pub struct PolygonBuffer {
    pub(crate) polyhedron_vertices: Vec<Vertex>,
    pub(crate) bounding_box_vertices: Vec<Vertex>,
}

impl PolygonBuffer {
    pub fn new() -> PolygonBuffer {
        PolygonBuffer {
            polyhedron_vertices: Vec::new(),
            bounding_box_vertices: Vec::new(),
        }
    }

    pub fn add(&mut self, polygon: &Polygon) -> PolygonBufferIndices {
        let polyhedron_offset = self.polyhedron_vertices.len() as u32;
        let bounding_box_offset = self.bounding_box_vertices.len() as u32;

        self.polyhedron_vertices.extend(polygon.polyhedron_vertices());
        self.bounding_box_vertices.extend(polygon.bounding_box_vertices());

        PolygonBufferIndices {
            polyhedron_indices: polygon.polyhedron_indices().iter().map(|i| i + polyhedron_offset).collect(),
            bounding_box_indices: polygon.bounding_box_indices().iter().map(|i| i + bounding_box_offset).collect()
        }
    }

    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> RenderablePolygonBuffer<R> {
        RenderablePolygonBuffer::new(factory, &self)
    }
}

#[derive(Debug)]
pub struct PolygonBufferIndices {
    pub(crate) polyhedron_indices: Vec<u32>,
    pub(crate) bounding_box_indices: Vec<u32>,
}

impl PolygonBufferIndices {
    // pub fn polyhedron_slice<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::Slice<R> {
    //     gfx::Slice {
    //         start: 0,
    //         end: self.polyhedron_indices.len() as u32,
    //         base_vertex: 0,
    //         instances: None,
    //         buffer: factory.create_index_buffer(&self.polyhedron_indices[..]),
    //     }
    // }

    // pub fn bounding_box_slice<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::Slice<R> {
    //     gfx::Slice {
    //         start: 0,
    //         end: self.bounding_box_indices.len() as u32,
    //         base_vertex: 0,
    //         instances: None,
    //         buffer: factory.create_index_buffer(&self.bounding_box_indices[..]),
    //     }
    // }

    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> RenderablePolygonIndices<R> {
        RenderablePolygonIndices::new(factory, &self)
    }
}

pub struct Polygon {
    pub bounds: [(f32, f32); 3],
    pub points: Vec<(f32, f32)>,
}

impl Polygon {
    fn bounding_box_vertices(&self) -> Vec<Vertex> {
        let bounding_ring = &[
            (self.bounds[0].0, self.bounds[1].0),
            (self.bounds[0].1, self.bounds[1].0),
            (self.bounds[0].1, self.bounds[1].1),
            (self.bounds[0].0, self.bounds[1].1),
            (self.bounds[0].0, self.bounds[1].0),
        ];

        Self::prism_vertices(bounding_ring, self.bounds[2].0, self.bounds[2].1)
    }

    fn bounding_box_indices(&self) -> Vec<u32> {
        Self::prism_indices(5)
    }

    fn polyhedron_vertices(&self) -> Vec<Vertex> {
        Self::prism_vertices(&self.points, self.bounds[2].0, self.bounds[2].1)
    }

    fn polyhedron_indices(&self) -> Vec<u32> {
        Self::prism_indices(self.points.len() as u32)
    }

    fn prism_vertices(
        points: &[(f32, f32)],
        height_lower_bound: f32,
        height_upper_bound: f32,
    ) -> Vec<Vertex> {
        points
            .iter()
            .flat_map(|&(x, y)| {
                let below = Vertex { position: [x, y, height_lower_bound] };
                let above = Vertex { position: [x, y, height_upper_bound] };
                vec![below, above]
            })
            .collect()
    }

    fn prism_indices(num_points: u32) -> Vec<u32> {
        (0..num_points)
            .flat_map(|index| {
                let below_index = 2 * index;
                let above_index = below_index + 1;
                let after_below_index = 2 * ((1 + index) % num_points);
                let after_above_index = after_below_index + 1;

                // When on an exterior ring, whose points are in counter-clockwise orientation,
                // this face should face outward.
                //
                // For interior rings, with clockwise orientation, this face should face inward.
                let mut indices = vec![
                    below_index, after_below_index, above_index,
                    after_below_index, after_above_index, above_index,
                ];

                if index != 0 && index != num_points - 1 {
                    // The top faces should face upward; the bottom faces, downward.
                    let cap_triangles = vec![
                        0, after_below_index, below_index,
                        1, above_index, after_above_index,
                    ];

                    indices.extend(cap_triangles);
                }

                indices
            })
            .collect()
    }
}

