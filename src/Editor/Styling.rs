use bevy::prelude::*;
use crate::Components::VuisElement::{VuisNode, SelectedNode, SelectedNodeInfoText};

#[derive(Component)]
pub enum StyleButton {
    ColorRed,
    ColorGreen,
    ColorBlue,
    IncreaseWidth,
    DecreaseWidth,
    IncreaseHeight,
    DecreaseHeight,
}

pub fn StylingSystem(
    mut QuerySelected: Query<(&mut VuisNode, &mut BackgroundColor, &mut Node), With<SelectedNode>>,
    QueryInteractions: Query<(&Interaction, &StyleButton), Changed<Interaction>>,
) {
    for (InteractionState, ButtonType) in QueryInteractions.iter() {
        if *InteractionState == Interaction::Pressed {
            if let Ok((mut VuisNodeComponent, mut BgColor, mut NodeComponent)) = QuerySelected.single_mut() {
                match ButtonType {
                    StyleButton::ColorRed => {
                        let NewColor = Color::Srgba(Srgba::new(1.0, 0.0, 0.0, 1.0));
                        VuisNodeComponent.BackgroundColor = NewColor;
                        BgColor.0 = NewColor;
                    }
                    StyleButton::ColorGreen => {
                        let NewColor = Color::Srgba(Srgba::new(0.0, 1.0, 0.0, 1.0));
                        VuisNodeComponent.BackgroundColor = NewColor;
                        BgColor.0 = NewColor;
                    }
                    StyleButton::ColorBlue => {
                        let NewColor = Color::Srgba(Srgba::new(0.0, 0.0, 1.0, 1.0));
                        VuisNodeComponent.BackgroundColor = NewColor;
                        BgColor.0 = NewColor;
                    }
                    StyleButton::IncreaseWidth => {
                        VuisNodeComponent.WidthPx += 10.0;
                        NodeComponent.width = Val::Px(VuisNodeComponent.WidthPx);
                    }
                    StyleButton::DecreaseWidth => {
                        VuisNodeComponent.WidthPx = f32::max(10.0, VuisNodeComponent.WidthPx - 10.0);
                        NodeComponent.width = Val::Px(VuisNodeComponent.WidthPx);
                    }
                    StyleButton::IncreaseHeight => {
                        VuisNodeComponent.HeightPx += 10.0;
                        NodeComponent.height = Val::Px(VuisNodeComponent.HeightPx);
                    }
                    StyleButton::DecreaseHeight => {
                        VuisNodeComponent.HeightPx = f32::max(10.0, VuisNodeComponent.HeightPx - 10.0);
                        NodeComponent.height = Val::Px(VuisNodeComponent.HeightPx);
                    }
                }
            }
        }
    }
}

pub fn PropertiesUpdateSystem(
    QuerySelected: Query<&VuisNode, With<SelectedNode>>,
    mut QueryText: Query<&mut Text, With<SelectedNodeInfoText>>,
) {
    let mut InfoString = "No Selection".to_string();
    if let Ok(Node) = QuerySelected.single() {
        InfoString = format!("ID: {}\nWidth: {}px\nHeight: {}px\nIs Image: {}", Node.Id, Node.WidthPx, Node.HeightPx, Node.IsImage);
    }
    for mut TextComponent in QueryText.iter_mut() {
        TextComponent.0 = InfoString.clone();
    }
}