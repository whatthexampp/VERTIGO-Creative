use bevy::prelude::*;
use crate::Components::VuisElement::{VuisNode, EditorCanvas};

#[derive(Component)]
pub enum CreationButton {
    SpawnNode,
    SpawnText,
    SpawnImage,
}

pub fn CreationSystem(
    mut Commands: Commands,
    mut Images: ResMut<Assets<Image>>,
    QueryInteractions: Query<(&Interaction, &CreationButton), Changed<Interaction>>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
) {
    let CanvasEntity = if let Ok(Ent) = QueryCanvas.single() { Ent } else { return; };
    for (InteractionState, ButtonType) in QueryInteractions.iter() {
        if *InteractionState == Interaction::Pressed {
            let mut ImageData = Option::None;
            let mut ImageHandle = Option::None;
            let mut IsImage = false;

            if matches!(ButtonType, CreationButton::SpawnImage) {
                let RawData = vec![255, 255, 255, 255];
                let LoadedImage = Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    RawData.clone(),
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                );
                ImageData = Option::Some(RawData);
                ImageHandle = Option::Some(Images.add(LoadedImage));
                IsImage = true;
            }

            let NodeName = match ButtonType {
                CreationButton::SpawnNode => "Node".to_string(),
                CreationButton::SpawnText => "Text".to_string(),
                CreationButton::SpawnImage => "Image".to_string(),
            };

            let NewNode = VuisNode {
                Id: NodeName,
                BackgroundColor: if IsImage { Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 }) } else { Color::LinearRgba(LinearRgba { red: 0.5, green: 0.5, blue: 0.5, alpha: 1.0 }) },
                WidthPx: if matches!(ButtonType, CreationButton::SpawnText) { 0.0 } else { 100.0 },
                HeightPx: if matches!(ButtonType, CreationButton::SpawnText) { 0.0 } else { 100.0 },
                IsImage,
                ImageData,
                PositionX: 100.0,
                PositionY: 100.0,
                AnimTargetX: 100.0,
                AnimTargetY: 100.0,
                BorderRadiusPx: 0.0,
                BorderWidthPx: 0.0,
                BorderColor: Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 }),
                ..default()
            };

            let mut EntityCommands = Commands.spawn((
                NewNode.clone(),
                Interaction::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(100.0),
                    width: if NewNode.WidthPx <= 0.0 { Val::Auto } else { Val::Px(NewNode.WidthPx) },
                    height: if NewNode.HeightPx <= 0.0 { Val::Auto } else { Val::Px(NewNode.HeightPx) },
                    align_items: if NewNode.HasText { AlignItems::Center } else { AlignItems::default() },
                    justify_content: if NewNode.HasText { JustifyContent::Center } else { JustifyContent::default() },
                    ..default()
                },
                BackgroundColor(NewNode.BackgroundColor),
                Transform::IDENTITY,
            ));

            if let Some(Handle) = ImageHandle {
                EntityCommands.insert(ImageNode::new(Handle));
            }

            let ChildEntity = EntityCommands.id();

            if matches!(ButtonType, CreationButton::SpawnText) {
                let TextEntity = Commands.spawn((
                    Text::new("New Text"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )).id();
                Commands.entity(ChildEntity).add_child(TextEntity);
            }

            Commands.entity(CanvasEntity).add_child(ChildEntity);
        }
    }
}