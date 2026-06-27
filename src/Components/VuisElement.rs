use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component, Clone)]
pub struct VuisNode {
    pub Id: String,
    pub BackgroundColor: Color,
    pub TextColor: Color,
    pub FontFamily: String,
    pub FontSizePx: f32,
    pub WidthPx: f32,
    pub HeightPx: f32,
    pub IsImage: bool,
    pub ImageData: Option<Vec<u8>>,
    pub HasText: bool,
    pub FontData: Option<Vec<u8>>,
    pub AnimTargetWidth: f32,
    pub AnimTargetHeight: f32,
    pub AnimTargetX: f32,
    pub AnimTargetY: f32,
    pub AnimTargetRotation: f32,
    pub AnimDuration: f32,
    pub PositionX: f32,
    pub PositionY: f32,
    pub Rotation: f32,
    pub BorderRadiusPx: f32,
    pub BorderWidthPx: f32,
    pub BorderColor: Color,
    pub IsGradient: bool,
    pub GradientColor1: Color,
    pub GradientColor2: Color,
    pub IsInput: bool,
    pub IsHidden: bool,
    pub IsBold: bool,
    pub IsItalic: bool,
    pub Placeholder: String,
    pub HasShadow: bool,
    pub ShadowColor: Color,
    pub ShadowOffsetX: f32,
    pub ShadowOffsetY: f32,
    pub ShadowBlur: f32,
    pub ShadowSpread: f32,
    pub IsGrid: bool,
    pub GridColumns: u32,
    pub GridRows: u32,
    pub GridColumnGap: f32,
    pub GridRowGap: f32,
}

impl Default for VuisNode {
    fn default() -> Self {
        Self {
            Id: "Node".to_string(),
            BackgroundColor: Color::WHITE,
            TextColor: Color::WHITE,
            FontFamily: "".to_string(),
            FontSizePx: 16.0,
            WidthPx: 100.0,
            HeightPx: 100.0,
            IsImage: false,
            ImageData: None,
            HasText: false,
            FontData: None,
            AnimTargetWidth: 100.0,
            AnimTargetHeight: 100.0,
            AnimTargetX: 0.0,
            AnimTargetY: 0.0,
            AnimTargetRotation: 0.0,
            AnimDuration: 0.0,
            PositionX: 0.0,
            PositionY: 0.0,
            Rotation: 0.0,
            BorderRadiusPx: 0.0,
            BorderWidthPx: 0.0,
            BorderColor: Color::srgba(0.0, 0.0, 0.0, 0.0),
            IsGradient: false,
            GradientColor1: Color::WHITE,
            GradientColor2: Color::BLACK,
            IsInput: false,
            IsHidden: false,
            IsBold: false,
            IsItalic: false,
            Placeholder: "".to_string(),
            HasShadow: false,
            ShadowColor: Color::srgba(0.0, 0.0, 0.0, 0.5),
            ShadowOffsetX: 4.0,
            ShadowOffsetY: 4.0,
            ShadowBlur: 10.0,
            ShadowSpread: 0.0,
            IsGrid: false,
            GridColumns: 2,
            GridRows: 2,
            GridColumnGap: 0.0,
            GridRowGap: 0.0,
        }
    }
}

#[derive(Component, Clone, Default)]
pub struct VuisAnimationState {
    pub Timer: f32,
    pub Forward: bool,
    pub IsPlaying: bool,
}

#[derive(Component)]
pub struct EditorCanvas;

#[derive(Component)]
pub struct SelectedNode;

#[derive(Component)]
pub struct SelectedNodeInfoText;

pub fn load_image_from_bytes(bytes: &[u8]) -> Option<Image> {
    if let Ok(dyn_img) = image::load_from_memory(bytes) {
        let rgba_img = dyn_img.to_rgba8();
        let width = rgba_img.width();
        let height = rgba_img.height();
        let raw_pixels = rgba_img.into_raw();
        
        Some(Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            raw_pixels,
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        ))
    } else {
        None
    }
}