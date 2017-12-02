use gfx;

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

        self.polyhedron_vertices.extend(
            polygon.polyhedron_vertices(),
        );
        self.bounding_box_vertices.extend(
            polygon.bounding_box_vertices(),
        );

        PolygonBufferIndices {
            polyhedron_indices: polygon
                .polyhedron_indices()
                .map(|i| i + polyhedron_offset)
                .collect(),
            bounding_box_indices: polygon
                .bounding_box_indices()
                .map(|i| i + bounding_box_offset)
                .collect(),
        }
    }

    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(
        &self,
        factory: &mut F,
    ) -> RenderablePolygonBuffer<R> {
        RenderablePolygonBuffer::new(factory, &self)
    }
}

#[derive(Debug)]
pub struct PolygonBufferIndices {
    pub(crate) polyhedron_indices: Vec<u32>,
    pub(crate) bounding_box_indices: Vec<u32>,
}

impl PolygonBufferIndices {
    pub fn new() -> PolygonBufferIndices {
        PolygonBufferIndices {
            polyhedron_indices: Vec::new(),
            bounding_box_indices: Vec::new(),
        }
    }

    pub fn extend(&mut self, other: &PolygonBufferIndices) {
        self.polyhedron_indices.extend_from_slice(&other.polyhedron_indices);
        self.bounding_box_indices.extend_from_slice(&other.bounding_box_indices);
    }

    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(
        &self,
        factory: &mut F,
    ) -> RenderablePolygonIndices<R> {
        RenderablePolygonIndices::new(factory, &self)
    }
}

pub struct Polygon {
    bounding_ring: [(f32, f32); 5],
    points: Vec<(f32, f32)>,
}

impl Polygon {
    pub fn new(bounds: [(f32, f32); 2], points: Vec<(f32, f32)>) -> Polygon {
        let bounding_ring = [
            (bounds[0].0, bounds[1].0),
            (bounds[0].1, bounds[1].0),
            (bounds[0].1, bounds[1].1),
            (bounds[0].0, bounds[1].1),
            (bounds[0].0, bounds[1].0),
        ];

        Polygon {
            bounding_ring: bounding_ring,
            points: points,
        }
    }

    fn bounding_box_vertices<'a>(&'a self) -> Box<'a + Iterator<Item = Vertex>> {
        Box::new(Self::prism_vertices(
            &self.bounding_ring,
        ))
    }

    fn bounding_box_indices(&self) -> Box<Iterator<Item = u32>> {
        Self::prism_indices(5)
    }

    fn polyhedron_vertices<'a>(&'a self) -> Box<'a + Iterator<Item = Vertex>> {
        Self::prism_vertices(&self.points)
    }

    fn polyhedron_indices(&self) -> Box<Iterator<Item = u32>> {
        Self::prism_indices(self.points.len() as u32)
    }

    fn prism_vertices<'a>(
        points: &'a [(f32, f32)],
    ) -> Box<'a + Iterator<Item = Vertex>> {
        Box::new(points.iter().flat_map(move |&(x, y)| {
            let below = Vertex { position: [x, y, 0.0] };
            let above = Vertex { position: [x, y, 1.0] };
            vec![below, above]
        }))
    }

    fn prism_indices(num_points: u32) -> Box<Iterator<Item = u32>> {
        Box::new((0..num_points).flat_map(move |index| {
            let below_index = 2 * index;
            let above_index = below_index + 1;
            let after_below_index = 2 * ((1 + index) % num_points);
            let after_above_index = after_below_index + 1;

            // When on an exterior ring, whose points are in counter-clockwise orientation,
            // this face should face outward.
            //
            // For interior rings, with clockwise orientation, this face should face inward.
            let mut indices = vec![
                below_index,
                after_below_index,
                above_index,
                after_below_index,
                after_above_index,
                above_index,
            ];

            if index != 0 && index != num_points - 1 {
                // The top faces should face upward; the bottom faces, downward.
                let cap_triangles = vec![
                    0,
                    after_below_index,
                    below_index,
                    1,
                    above_index,
                    after_above_index,
                ];

                indices.extend(cap_triangles);
            }

            indices
        }))
    }
}
