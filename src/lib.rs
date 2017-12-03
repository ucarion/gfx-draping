extern crate geo;
#[macro_use]
extern crate gfx;

mod polygon;
mod render;
mod vertex;

pub use polygon::{Polygon, PolygonBuffer, PolygonBufferIndices};
pub use render::{DrapingRenderer, RenderablePolygonBuffer, RenderablePolygonIndices};
