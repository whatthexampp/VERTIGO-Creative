use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, LinearGradient, ColorStop, InterpolationColorSpace};
use serde_json;
use base64::prelude::*;
use std::fs;
use std::io::{Read, Write};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use crate::Components::VuisElement::{VuisNode, VuisAnimationState, EditorCanvas, load_image_from_bytes};
use crate::Serialization::VuisFormat::{VuisFile, VuisDataNode};
use crate::Editor::EditorPlugin::EditorSelection;

pub struct SerializationPlugin;

impl Plugin for SerializationPlugin {
    fn build(&self, AppBuilder: &mut App) {
        AppBuilder.add_message::<SaveVuisEvent>();
        AppBuilder.add_message::<LoadVuisEvent>();
        AppBuilder.add_systems(Update, (SaveSystem, LoadSystem));
    }
}

#[derive(Message)]
pub struct SaveVuisEvent {
    pub FilePath: String,
}

#[derive(Message)]
pub struct LoadVuisEvent {
    pub FilePath: String,
}

pub fn SaveSystem(
    mut SaveEvents: MessageReader<SaveVuisEvent>,
    QueryNodes: Query<(&VuisNode, Option<&Children>)>,
    QueryText: Query<&Text>,
    QueryCanvas: Query<&Children, With<EditorCanvas>>,
) {
    for Event in SaveEvents.read() {
        if let Ok(CanvasChildren) = QueryCanvas.single() {
            let mut RootChildren = Vec::new();
            for ChildEntity in CanvasChildren.iter() {
                if let Some(NodeData) = BuildDataTree(ChildEntity, &QueryNodes, &QueryText) {
                    RootChildren.push(NodeData);
                }
            }
            let FileData = VuisFile {
                Version: "1.0".to_string(),
                Root: VuisDataNode {
                    Id: "RootCanvas".to_string(),
                    ColorRgba: [0.2, 0.2, 0.2, 1.0],
                    TextColorRgba: Some([1.0, 1.0, 1.0, 1.0]),
                    FontFamily: None,
                    FontSizePx: Some(16.0),
                    WidthPx: 0.0,
                    HeightPx: 0.0,
                    IsImage: false,
                    Base64Image: None,
                    HasText: false,
                    TextContent: None,
                    Base64Font: None,
                    AnimTargetWidth: 0.0,
                    AnimTargetHeight: 0.0,
                    AnimTargetX: Some(0.0),
                    AnimTargetY: Some(0.0),
                    AnimTargetRotation: Some(0.0),
                    AnimDuration: 0.0,
                    PositionX: 0.0,
                    PositionY: 0.0,
                    Rotation: 0.0,
                    BorderRadiusPx: 0.0,
                    BorderWidthPx: 0.0,
                    BorderColorRgba: [0.0, 0.0, 0.0, 0.0],
                    IsGradient: false,
                    GradientColor1Rgba: [1.0, 1.0, 1.0, 1.0],
                    GradientColor2Rgba: [0.0, 0.0, 0.0, 1.0],
                    IsInput: false,
                    IsHidden: false,
                    IsBold: false,
                    IsItalic: false,
                    Placeholder: "".to_string(),
                    HasShadow: Some(false),
                    ShadowColorRgba: Some([0.0, 0.0, 0.0, 0.5]),
                    ShadowOffsetX: Some(4.0),
                    ShadowOffsetY: Some(4.0),
                    ShadowBlur: Some(10.0),
                    ShadowSpread: Some(0.0),
                    IsGrid: Some(false),
                    GridColumns: Some(2),
                    GridRows: Some(2),
                    GridColumnGap: Some(0.0),
                    GridRowGap: Some(0.0),
                    Children: RootChildren,
                },
            };
            if let Ok(JsonString) = serde_json::to_string(&FileData) {
                if let Ok(File) = std::fs::File::create(&Event.FilePath) {
                    let mut Encoder = GzEncoder::new(File, Compression::default());
                    let _ = Encoder.write_all(JsonString.as_bytes());
                }
            }
        }
    }
}

pub fn LoadSystem(
    mut LoadEvents: MessageReader<LoadVuisEvent>,
    mut Commands: Commands,
    mut Images: ResMut<Assets<Image>>,
    mut Fonts: ResMut<Assets<Font>>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    QueryCanvasChildren: Query<&Children, With<EditorCanvas>>,
    mut SelectedEntity: ResMut<EditorSelection>,
    mut RecordEvents: MessageWriter<crate::Editor::History::RecordHistoryEvent>,
) {
    for Event in LoadEvents.read() {
        let mut LoadedData = None;

        if let Ok(File) = std::fs::File::open(&Event.FilePath) {
            let mut Decoder = GzDecoder::new(File);
            let mut JsonString = String::new();
            if Decoder.read_to_string(&mut JsonString).is_ok() {
                if let Ok(FileData) = serde_json::from_str::<VuisFile>(&JsonString) {
                    LoadedData = Some(FileData);
                }
            }
        }

        if LoadedData.is_none() {
            if let Ok(RawString) = fs::read_to_string(&Event.FilePath) {
                if let Ok(FileData) = serde_json::from_str::<VuisFile>(&RawString) {
                    LoadedData = Some(FileData);
                }
            }
        }

        if let Some(FileData) = LoadedData {
            if let Ok(CanvasEntity) = QueryCanvas.single() {
                if let Ok(CanvasChildren) = QueryCanvasChildren.get(CanvasEntity) {
                    for ChildEntity in CanvasChildren.iter() {
                        Commands.entity(ChildEntity).despawn();
                    }
                }
                SelectedEntity.SelectedNode = None;
                for ChildData in &FileData.Root.Children {
                    SpawnDataTree(&mut Commands, &mut Images, &mut Fonts, CanvasEntity, ChildData);
                }
                RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
            }
        }
    }
}

pub fn BuildDataTree(
    CurrentEntity: Entity,
    QueryNodes: &Query<(&VuisNode, Option<&Children>)>,
    QueryText: &Query<&Text>,
) -> Option<VuisDataNode> {
    if let Ok((NodeComponent, ChildrenComponent)) = QueryNodes.get(CurrentEntity) {
        let mut ChildNodes = Vec::new();
        let mut ExtractedText = None;

        if let Some(ChildrenList) = ChildrenComponent {
            for ChildEntity in ChildrenList.iter() {
                if let Some(ChildData) = BuildDataTree(ChildEntity, QueryNodes, QueryText) {
                    ChildNodes.push(ChildData);
                } else if let Ok(TextComponent) = QueryText.get(ChildEntity) {
                    ExtractedText = Some(TextComponent.0.clone());
                }
            }
        }
        let Base64String = NodeComponent.ImageData.as_ref().map(|Data| BASE64_STANDARD.encode(Data));
        let Base64FontString = NodeComponent.FontData.as_ref().map(|Data| BASE64_STANDARD.encode(Data));
        let LinearColor = NodeComponent.BackgroundColor.to_linear();
        let LinearTextColor = NodeComponent.TextColor.to_linear();
        let LinearBorderColor = NodeComponent.BorderColor.to_linear();
        let LinearGrad1 = NodeComponent.GradientColor1.to_linear();
        let LinearGrad2 = NodeComponent.GradientColor2.to_linear();
        let LinearShadowColor = NodeComponent.ShadowColor.to_linear();
        Some(VuisDataNode {
            Id: NodeComponent.Id.clone(),
            ColorRgba: [LinearColor.red, LinearColor.green, LinearColor.blue, LinearColor.alpha],
            TextColorRgba: Some([LinearTextColor.red, LinearTextColor.green, LinearTextColor.blue, LinearTextColor.alpha]),
            FontFamily: Some(NodeComponent.FontFamily.clone()),
            FontSizePx: Some(NodeComponent.FontSizePx),
            WidthPx: NodeComponent.WidthPx,
            HeightPx: NodeComponent.HeightPx,
            IsImage: NodeComponent.IsImage,
            Base64Image: Base64String,
            HasText: NodeComponent.HasText,
            TextContent: ExtractedText,
            Base64Font: Base64FontString,
            AnimTargetWidth: NodeComponent.AnimTargetWidth,
            AnimTargetHeight: NodeComponent.AnimTargetHeight,
            AnimTargetX: Some(NodeComponent.AnimTargetX),
            AnimTargetY: Some(NodeComponent.AnimTargetY),
            AnimTargetRotation: Some(NodeComponent.AnimTargetRotation),
            AnimDuration: NodeComponent.AnimDuration,
            PositionX: NodeComponent.PositionX,
            PositionY: NodeComponent.PositionY,
            Rotation: NodeComponent.Rotation,
            BorderRadiusPx: NodeComponent.BorderRadiusPx,
            BorderWidthPx: NodeComponent.BorderWidthPx,
            BorderColorRgba: [LinearBorderColor.red, LinearBorderColor.green, LinearBorderColor.blue, LinearBorderColor.alpha],
            IsGradient: NodeComponent.IsGradient,
            GradientColor1Rgba: [LinearGrad1.red, LinearGrad1.green, LinearGrad1.blue, LinearGrad1.alpha],
            GradientColor2Rgba: [LinearGrad2.red, LinearGrad2.green, LinearGrad2.blue, LinearGrad2.alpha],
            IsInput: NodeComponent.IsInput,
            IsHidden: NodeComponent.IsHidden,
            IsBold: NodeComponent.IsBold,
            IsItalic: NodeComponent.IsItalic,
            Placeholder: NodeComponent.Placeholder.clone(),
            HasShadow: Some(NodeComponent.HasShadow),
            ShadowColorRgba: Some([LinearShadowColor.red, LinearShadowColor.green, LinearShadowColor.blue, LinearShadowColor.alpha]),
            ShadowOffsetX: Some(NodeComponent.ShadowOffsetX),
            ShadowOffsetY: Some(NodeComponent.ShadowOffsetY),
            ShadowBlur: Some(NodeComponent.ShadowBlur),
            ShadowSpread: Some(NodeComponent.ShadowSpread),
            IsGrid: Some(NodeComponent.IsGrid),
            GridColumns: Some(NodeComponent.GridColumns),
            GridRows: Some(NodeComponent.GridRows),
            GridColumnGap: Some(NodeComponent.GridColumnGap),
            GridRowGap: Some(NodeComponent.GridRowGap),
            Children: ChildNodes,
        })
    } else {
        None
    }
}

pub fn SpawnDataTree(Commands: &mut Commands, Images: &mut Assets<Image>, Fonts: &mut Assets<Font>, Parent: Entity, Data: &VuisDataNode) {
    let mut ImageData = None;
    let mut ImageHandle = None;
    let mut FontData = None;

    if let Some(Base64String) = &Data.Base64Image {
        if let Ok(DecodedBytes) = BASE64_STANDARD.decode(Base64String) {
            ImageData = Some(DecodedBytes.clone());
            if let Some(LoadedImage) = load_image_from_bytes(&DecodedBytes) {
                ImageHandle = Some(Images.add(LoadedImage));
            }
        }
    }

    if let Some(Base64FontString) = &Data.Base64Font {
        if let Ok(DecodedBytes) = BASE64_STANDARD.decode(Base64FontString) {
            if ttf_parser::Face::parse(&DecodedBytes, 0).is_ok() {
                FontData = Some(DecodedBytes);
            }
        }
    }

    let NewNode = VuisNode {
        Id: Data.Id.clone(),
        BackgroundColor: Color::LinearRgba(LinearRgba {
            red: Data.ColorRgba[0],
            green: Data.ColorRgba[1],
            blue: Data.ColorRgba[2],
            alpha: Data.ColorRgba[3],
        }),
        TextColor: if let Some(tc) = Data.TextColorRgba {
            Color::LinearRgba(LinearRgba {
                red: tc[0],
                green: tc[1],
                blue: tc[2],
                alpha: tc[3],
            })
        } else {
            Color::WHITE
        },
        FontFamily: Data.FontFamily.clone().unwrap_or_default(),
        FontSizePx: Data.FontSizePx.unwrap_or(16.0),
        WidthPx: Data.WidthPx,
        HeightPx: Data.HeightPx,
        IsImage: Data.IsImage,
        ImageData,
        HasText: Data.HasText,
        FontData: FontData.clone(),
        AnimTargetWidth: Data.AnimTargetWidth,
        AnimTargetHeight: Data.AnimTargetHeight,
        AnimTargetX: Data.AnimTargetX.unwrap_or(Data.PositionX),
        AnimTargetY: Data.AnimTargetY.unwrap_or(Data.PositionY),
        AnimTargetRotation: Data.AnimTargetRotation.unwrap_or(Data.Rotation),
        AnimDuration: Data.AnimDuration,
        PositionX: Data.PositionX,
        PositionY: Data.PositionY,
        Rotation: Data.Rotation,
        BorderRadiusPx: Data.BorderRadiusPx,
        BorderWidthPx: Data.BorderWidthPx,
        BorderColor: Color::LinearRgba(LinearRgba {
            red: Data.BorderColorRgba[0],
            green: Data.BorderColorRgba[1],
            blue: Data.BorderColorRgba[2],
            alpha: Data.BorderColorRgba[3],
        }),
        IsGradient: Data.IsGradient,
        GradientColor1: Color::LinearRgba(LinearRgba {
            red: Data.GradientColor1Rgba[0],
            green: Data.GradientColor1Rgba[1],
            blue: Data.GradientColor1Rgba[2],
            alpha: Data.GradientColor1Rgba[3],
        }),
        GradientColor2: Color::LinearRgba(LinearRgba {
            red: Data.GradientColor2Rgba[0],
            green: Data.GradientColor2Rgba[1],
            blue: Data.GradientColor2Rgba[2],
            alpha: Data.GradientColor2Rgba[3],
        }),
        IsInput: Data.IsInput,
        IsHidden: Data.IsHidden,
        IsBold: Data.IsBold,
        IsItalic: Data.IsItalic,
        Placeholder: Data.Placeholder.clone(),
        HasShadow: Data.HasShadow.unwrap_or(false),
        ShadowColor: if let Some(sc) = Data.ShadowColorRgba {
            Color::LinearRgba(LinearRgba {
                red: sc[0],
                green: sc[1],
                blue: sc[2],
                alpha: sc[3],
            })
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.5)
        },
        ShadowOffsetX: Data.ShadowOffsetX.unwrap_or(4.0),
        ShadowOffsetY: Data.ShadowOffsetY.unwrap_or(4.0),
        ShadowBlur: Data.ShadowBlur.unwrap_or(10.0),
        ShadowSpread: Data.ShadowSpread.unwrap_or(0.0),
        IsGrid: Data.IsGrid.unwrap_or(false),
        GridColumns: Data.GridColumns.unwrap_or(2),
        GridRows: Data.GridRows.unwrap_or(2),
        GridColumnGap: Data.GridColumnGap.unwrap_or(0.0),
        GridRowGap: Data.GridRowGap.unwrap_or(0.0),
    };

    let mut EntityCommands = Commands.spawn((
        NewNode.clone(),
        VuisAnimationState::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(Data.PositionX),
            top: Val::Px(Data.PositionY),
            width: if NewNode.WidthPx <= 0.0 { Val::Auto } else { Val::Px(NewNode.WidthPx) },
            height: if NewNode.HeightPx <= 0.0 { Val::Auto } else { Val::Px(NewNode.HeightPx) },
            border: UiRect::all(Val::Px(Data.BorderWidthPx)),
            border_radius: BorderRadius::all(Val::Px(Data.BorderRadiusPx)),
            align_items: if Data.HasText { AlignItems::Center } else { AlignItems::default() },
            justify_content: if Data.HasText { JustifyContent::Center } else { JustifyContent::default() },
            display: if NewNode.IsGrid { Display::Grid } else { Display::Flex },
            grid_template_columns: if NewNode.IsGrid { vec![RepeatedGridTrack::flex(NewNode.GridColumns as u16, 1.0)] } else { Vec::new() },
            grid_template_rows: if NewNode.IsGrid { vec![RepeatedGridTrack::flex(NewNode.GridRows as u16, 1.0)] } else { Vec::new() },
            column_gap: if NewNode.IsGrid { Val::Px(NewNode.GridColumnGap) } else { Val::Auto },
            row_gap: if NewNode.IsGrid { Val::Px(NewNode.GridRowGap) } else { Val::Auto },
            ..default()
        },
        BackgroundColor(NewNode.BackgroundColor),
        Transform::from_rotation(Quat::from_rotation_z(-Data.Rotation)),
    ));

    if Data.BorderWidthPx > 0.0 {
        EntityCommands.insert(BorderColor::all(NewNode.BorderColor));
    }

    if NewNode.HasShadow {
        EntityCommands.insert(BoxShadow::new(
            NewNode.ShadowColor,
            Val::Px(NewNode.ShadowOffsetX),
            Val::Px(NewNode.ShadowOffsetY),
            Val::Px(NewNode.ShadowSpread),
            Val::Px(NewNode.ShadowBlur),
        ));
    }

    if let Some(Handle) = ImageHandle {
        EntityCommands.insert(ImageNode::new(Handle));
    }

    if Data.IsGradient {
        EntityCommands.insert(BackgroundGradient::from(LinearGradient {
            color_space: InterpolationColorSpace::Srgba,
            angle: 0.0,
            stops: vec![
                ColorStop::percent(NewNode.GradientColor1, 0.0),
                ColorStop::percent(NewNode.GradientColor2, 100.0),
            ],
        }));
    }

    if Data.IsHidden {
        EntityCommands.insert(Visibility::Hidden);
    } else {
        EntityCommands.insert(Visibility::Inherited);
    }

    let ChildEntity = EntityCommands.id();

    if Data.HasText {
        let mut TextEntity = Commands.spawn((
            Text::new(Data.TextContent.clone().unwrap_or_default()),
            TextColor(NewNode.TextColor),
        ));
        
        if Data.IsInput {
            TextEntity.insert(bevy::text::EditableText::default());
        }
        
        if let Some(Bytes) = FontData {
            let LoadedFont = Font::from_bytes(Bytes);
            let Handle = Fonts.add(LoadedFont);
            TextEntity.insert(TextFont { font: FontSource::Handle(Handle), font_size: FontSize::Px(NewNode.FontSizePx), ..default() });
        } else {
            TextEntity.insert(TextFont { font_size: FontSize::Px(NewNode.FontSizePx), ..default() });
        }
        
        let Id = TextEntity.id();
        Commands.entity(ChildEntity).add_child(Id);
    }

    Commands.entity(Parent).add_child(ChildEntity);

    for ChildData in &Data.Children {
        SpawnDataTree(Commands, Images, Fonts, ChildEntity, ChildData);
    }
}