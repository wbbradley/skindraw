pub mod app;
pub mod atlas;
pub mod brush;
pub mod camera;
pub mod color;
pub mod model;
pub mod renderer;
pub mod skin;

pub use atlas::{AtlasRect, FaceRegion, face_region};
pub use brush::{BrushSize, StrokeBuilder, brush_footprint};
pub use camera::{
    Camera, LayerVisibility, ViewingSide, pick_model, pick_model_arranged, pick_model_part,
    pick_model_part_arranged,
};
pub use color::HsvJitter;
pub use model::{
    BodyPart, Face, Layer, ModelArrangement, ModelBox, ModelHit, ModelKind, Ray, Texel,
    arranged_model_boxes, model_boxes,
};
pub use skin::{Skin, SkinDocument, SkinError};
