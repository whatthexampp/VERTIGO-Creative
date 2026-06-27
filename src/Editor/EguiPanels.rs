use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::{BackgroundGradient, LinearGradient, ColorStop, InterpolationColorSpace};
use bevy_egui::{egui, EguiContexts};
use rfd::FileDialog;
use std::fs;
use crate::Components::VuisElement::{VuisNode, VuisAnimationState, EditorCanvas, SelectedNode, load_image_from_bytes};
use crate::Serialization::VuisSerializer::{SaveVuisEvent, LoadVuisEvent};
use crate::Editor::EditorPlugin::{EditorSelection, EditorConfig};
use bevy::text::{FontWeight, FontStyle};
use bevy::ecs::system::SystemParam;

#[derive(Component)]
pub struct PlaceholderTextComponent(pub Entity);

#[derive(SystemParam)]
pub struct EditorUiAssetsAndEvents<'w> {
    pub Images: ResMut<'w, Assets<Image>>,
    pub Fonts: ResMut<'w, Assets<Font>>,
    pub SaveEvents: MessageWriter<'w, SaveVuisEvent>,
    pub LoadEvents: MessageWriter<'w, LoadVuisEvent>,
    pub UndoEvents: MessageWriter<'w, crate::Editor::History::UndoEvent>,
    pub RedoEvents: MessageWriter<'w, crate::Editor::History::RedoEvent>,
    pub RecordEvents: MessageWriter<'w, crate::Editor::History::RecordHistoryEvent>,
}

pub fn PlaceholderUpdateSystem(
    mut Commands: Commands,
    QueryNodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    QueryMainText: Query<&Text, Without<PlaceholderTextComponent>>,
    QueryPlaceholder: Query<&PlaceholderTextComponent>,
    mut QueryPlaceholderMut: Query<(&mut Text, &mut Visibility, &PlaceholderTextComponent)>,
) {
    for (node_entity, vnode, children_opt) in QueryNodes.iter() {
        if !vnode.IsInput { continue; }
        
        let mut has_placeholder = false;
        
        if let Some(children) = children_opt {
            for child in children.iter() {
                if QueryPlaceholder.get(child).is_ok() {
                    has_placeholder = true;
                }
            }
        }
        
        if !has_placeholder {
            let p_ent = Commands.spawn((
                Text::new(vnode.Placeholder.clone()),
                TextFont { font_size: FontSize::Px(vnode.FontSizePx), ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                PlaceholderTextComponent(node_entity),
            )).id();
            Commands.entity(node_entity).add_child(p_ent);
        }
    }
    
    for (mut p_text, mut p_vis, p_comp) in QueryPlaceholderMut.iter_mut() {
        if let Ok((_, vnode, children_opt)) = QueryNodes.get(p_comp.0) {
            p_text.0 = vnode.Placeholder.clone();
            let mut has_main_text = false;
            if let Some(children) = children_opt {
                for child in children.iter() {
                    if QueryMainText.get(child).is_ok() {
                        if let Ok(text) = QueryMainText.get(child) {
                            if !text.0.is_empty() {
                                has_main_text = true;
                            }
                        }
                    }
                }
            }
            if has_main_text || !vnode.IsInput {
                *p_vis = Visibility::Hidden;
            } else {
                *p_vis = Visibility::Inherited;
            }
        }
    }
}

pub fn TextStylingUpdateSystem(
    QueryNodes: Query<(&VuisNode, Option<&Children>)>,
    mut QueryTextFonts: Query<(&Text, &mut TextFont)>,
) {
    for (vnode, children_opt) in QueryNodes.iter() {
        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((text, mut text_font)) = QueryTextFonts.get_mut(child) {
                    let is_bold = vnode.IsBold || text.0.contains("[b]");
                    let is_italic = vnode.IsItalic || text.0.contains("[i]");

                    if is_bold || is_italic {
                        if !vnode.FontFamily.is_empty() {
                            text_font.font = FontSource::Family(vnode.FontFamily.clone().into());
                        } else {
                            text_font.font = FontSource::Family("sans-serif".into());
                        }
                    } else if !vnode.FontFamily.is_empty() {
                        text_font.font = FontSource::Family(vnode.FontFamily.clone().into());
                    } else if vnode.FontData.is_none() {
                        text_font.font = FontSource::Family("sans-serif".into());
                    }

                    text_font.font_size = FontSize::Px(vnode.FontSizePx);

                    text_font.weight = if is_bold {
                        FontWeight::BOLD
                    } else {
                        FontWeight::default()
                    };

                    text_font.style = if is_italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::default()
                    };
                }
            }
        }
    }
}

#[allow(deprecated)]
pub fn EditorUiSystem(
    mut Contexts: EguiContexts,
    mut Commands: Commands,
    mut QueryNodes: Query<(Entity, &mut VuisNode, &mut BackgroundColor, &mut Node, Option<&mut BorderColor>, &mut Transform)>,
    mut QueryText: Query<&mut Text>,
    mut QueryTextFont: Query<&mut TextFont>,
    mut QueryTextColor: Query<&mut TextColor>,
    mut QueryAnimState: Query<&mut VuisAnimationState>,
    QueryPlaceholder: Query<&PlaceholderTextComponent>,
    QueryChildren: Query<&Children>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    mut SelectedEntity: ResMut<EditorSelection>,
    mut Config: ResMut<EditorConfig>,
    mut Helper: EditorUiAssetsAndEvents<'_>,
) {
    let Ok(Ctx) = Contexts.ctx_mut() else { return; };

    let _top_resp = egui::Panel::top("top_panel").show(Ctx, |Ui| {
        Ui.horizontal(|Ui| {
            if Ui.button("Save .vuis").clicked() {
                if let Some(Path) = FileDialog::new().add_filter("VUIS", &["vuis"]).save_file() {
                    Helper.SaveEvents.write(SaveVuisEvent { FilePath: Path.to_string_lossy().to_string() });
                }
            }
            if Ui.button("Load .vuis").clicked() {
                if let Some(Path) = FileDialog::new().add_filter("VUIS", &["vuis"]).pick_file() {
                    Helper.LoadEvents.write(LoadVuisEvent { FilePath: Path.to_string_lossy().to_string() });
                }
            }
            Ui.separator();
            if Ui.button("Undo (Ctrl+Z)").clicked() {
                Helper.UndoEvents.write(crate::Editor::History::UndoEvent);
            }
            if Ui.button("Redo (Ctrl+Y)").clicked() {
                Helper.RedoEvents.write(crate::Editor::History::RedoEvent);
            }
            Ui.separator();
            if Ui.button("Add Node").clicked() {
                if let Ok(Canvas) = QueryCanvas.single() {
                    let Target = SelectedEntity.SelectedNode.unwrap_or(Canvas);
                    let Child = Commands.spawn((
                        VuisNode { 
                            Id: "Node".to_string(), 
                            BackgroundColor: Color::srgb(0.5, 0.5, 0.5), 
                            WidthPx: 100.0, 
                            HeightPx: 100.0, 
                            PositionX: 50.0, 
                            PositionY: 50.0, 
                            AnimTargetX: 50.0,
                            AnimTargetY: 50.0,
                            ..default() 
                        },
                        VuisAnimationState::default(),
                        Node { position_type: PositionType::Absolute, left: Val::Px(50.0), top: Val::Px(50.0), width: Val::Px(100.0), height: Val::Px(100.0), ..default() },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                        Transform::IDENTITY,
                    )).id();
                    Commands.entity(Target).add_child(Child);
                    Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                }
            }
            if Ui.button("Add Text").clicked() {
                if let Ok(Canvas) = QueryCanvas.single() {
                    let Target = SelectedEntity.SelectedNode.unwrap_or(Canvas);
                    let Child = Commands.spawn((
                        VuisNode { 
                            Id: "Text".to_string(), 
                            BackgroundColor: Color::srgba(0.0, 0.0, 0.0, 0.0), 
                            TextColor: Color::WHITE,
                            WidthPx: 0.0, 
                            HeightPx: 0.0, 
                            HasText: true, 
                            PositionX: 50.0, 
                            PositionY: 50.0, 
                            AnimTargetX: 50.0,
                            AnimTargetY: 50.0,
                            ..default() 
                        },
                        VuisAnimationState::default(),
                        Node { 
                            position_type: PositionType::Absolute, 
                            left: Val::Px(50.0), 
                            top: Val::Px(50.0), 
                            width: Val::Auto, 
                            height: Val::Auto, 
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default() 
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                        Transform::IDENTITY,
                    )).id();

                    let TextChild = Commands.spawn((
                        Text::new("New Text".to_string()),
                        TextFont { font_size: FontSize::Px(16.0), ..default() },
                        TextColor(Color::WHITE),
                    )).id();

                    Commands.entity(Child).add_child(TextChild);
                    Commands.entity(Target).add_child(Child);
                    Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                }
            }
            if Ui.button("Add Input").clicked() {
                if let Ok(Canvas) = QueryCanvas.single() {
                    let Target = SelectedEntity.SelectedNode.unwrap_or(Canvas);
                    let Child = Commands.spawn((
                        VuisNode { 
                            Id: "Input".to_string(), 
                            BackgroundColor: Color::WHITE, 
                            TextColor: Color::BLACK,
                            WidthPx: 150.0, 
                            HeightPx: 30.0, 
                            HasText: true, 
                            IsInput: true, 
                            PositionX: 50.0, 
                            PositionY: 50.0, 
                            AnimTargetX: 50.0,
                            AnimTargetY: 50.0,
                            ..default() 
                        },
                        VuisAnimationState::default(),
                        Node { 
                            position_type: PositionType::Absolute, 
                            left: Val::Px(50.0), 
                            top: Val::Px(50.0), 
                            width: Val::Px(150.0), 
                            height: Val::Px(30.0), 
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default() 
                        },
                        BackgroundColor(Color::WHITE),
                        Transform::IDENTITY,
                    )).id();

                    let TextChild = Commands.spawn((
                        Text::new("".to_string()),
                        TextFont { font_size: FontSize::Px(16.0), ..default() },
                        TextColor(Color::BLACK),
                        EditableText::default(),
                    )).id();

                    Commands.entity(Child).add_child(TextChild);
                    Commands.entity(Target).add_child(Child);
                    Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                }
            }
            if Ui.button("Add Image").clicked() {
                if let Ok(Canvas) = QueryCanvas.single() {
                    let Target = SelectedEntity.SelectedNode.unwrap_or(Canvas);
                    let Child = Commands.spawn((
                        VuisNode { 
                            Id: "Image".to_string(), 
                            BackgroundColor: Color::srgba(0.0, 0.0, 0.0, 0.0), 
                            WidthPx: 100.0, 
                            HeightPx: 100.0, 
                            IsImage: true, 
                            PositionX: 50.0, 
                            PositionY: 50.0, 
                            AnimTargetX: 50.0,
                            AnimTargetY: 50.0,
                            ..default() 
                        },
                        VuisAnimationState::default(),
                        Node { position_type: PositionType::Absolute, left: Val::Px(50.0), top: Val::Px(50.0), width: Val::Px(100.0), height: Val::Px(100.0), ..default() },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                        Transform::IDENTITY,
                    )).id();
                    Commands.entity(Target).add_child(Child);
                    Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                }
            }
            Ui.separator();
            if Ui.button("Delete Selected").clicked() {
                if let Some(Ent) = SelectedEntity.SelectedNode {
                    if QueryCanvas.get(Ent).is_err() {
                        if let Ok(mut EntCmds) = Commands.get_entity(Ent) {
                            EntCmds.despawn();
                        }
                        SelectedEntity.SelectedNode = None;
                        Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                    }
                }
            }
        });
    });

    let mut ReparentAction = None;

    let left_resp = egui::Panel::left("left_panel").show(Ctx, |Ui| {
        Ui.heading("Hierarchy");
        if let Ok(Canvas) = QueryCanvas.single() {
            if let Ok(Children) = QueryChildren.get(Canvas) {
                for Child in Children.iter() {
                    RenderHierarchy(Ui, Child.clone(), &QueryNodes, &QueryChildren, &mut SelectedEntity, &mut ReparentAction);
                }
            }
        }
    }).response;

    if left_resp.hovered() && Ctx.input(|i| i.pointer.any_released()) {
        if let Some(dragged) = SelectedEntity.DraggedHierarchyEntity {
            if ReparentAction.is_none() {
                if let Ok(Canvas) = QueryCanvas.single() {
                    ReparentAction = Some((dragged, Canvas));
                }
            }
        }
    }

    if Ctx.input(|i| i.pointer.any_released()) {
        SelectedEntity.DraggedHierarchyEntity = None;
    }

    if let Some((ChildEnt, ParentEnt)) = ReparentAction {
        if !IsDescendant(ParentEnt, ChildEnt, &QueryChildren) {
            Commands.entity(ParentEnt).add_child(ChildEnt);
            Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
        }
    }

    let _right_resp = egui::Panel::right("right_panel").show(Ctx, |Ui| {
        Ui.heading("Properties");
        if let Some(Ent) = SelectedEntity.SelectedNode {
            if let Ok((_, mut VNode, mut BgColor, mut UiNode, BorderColorOpt, mut TransComp)) = QueryNodes.get_mut(Ent) {
                Ui.horizontal(|Ui| {
                    Ui.label("ID:");
                    Ui.text_edit_singleline(&mut VNode.Id);
                });

                Ui.horizontal(|Ui| {
                    let mut is_hidden = VNode.IsHidden;
                    if Ui.checkbox(&mut is_hidden, "Hidden").changed() {
                        VNode.IsHidden = is_hidden;
                        if is_hidden {
                            Commands.entity(Ent).insert(Visibility::Hidden);
                        } else {
                            Commands.entity(Ent).insert(Visibility::Inherited);
                        }
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("X Position:");
                    if Ui.add(egui::DragValue::new(&mut VNode.PositionX)).changed() {
                        UiNode.left = Val::Px(VNode.PositionX);
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Y Position:");
                    if Ui.add(egui::DragValue::new(&mut VNode.PositionY)).changed() {
                        UiNode.top = Val::Px(VNode.PositionY);
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Width:");
                    if Ui.add(egui::DragValue::new(&mut VNode.WidthPx)).changed() {
                        UiNode.width = if VNode.WidthPx <= 0.0 { Val::Auto } else { Val::Px(VNode.WidthPx) };
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Height:");
                    if Ui.add(egui::DragValue::new(&mut VNode.HeightPx)).changed() {
                        UiNode.height = if VNode.HeightPx <= 0.0 { Val::Auto } else { Val::Px(VNode.HeightPx) };
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Rotation:");
                    let mut Degrees = VNode.Rotation.to_degrees();
                    if Ui.add(egui::DragValue::new(&mut Degrees)).changed() {
                        VNode.Rotation = Degrees.to_radians();
                        TransComp.rotation = Quat::from_rotation_z(-VNode.Rotation);
                    }
                });

                Ui.horizontal(|Ui| {
                    if VNode.HasText {
                        Ui.label("Text Color:");
                        let srgba = VNode.TextColor.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.TextColor = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            if let Ok(Children) = QueryChildren.get(Ent) {
                                for Child in Children.iter() {
                                    if let Ok(mut text_color) = QueryTextColor.get_mut(Child) {
                                        text_color.0 = VNode.TextColor;
                                    }
                                }
                            }
                        }
                    } else {
                        Ui.label("Color:");
                        let srgba = VNode.BackgroundColor.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.BackgroundColor = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            BgColor.0 = VNode.BackgroundColor;
                        }
                    }
                });

                if VNode.HasText {
                    Ui.horizontal(|Ui| {
                        Ui.label("Bg Color:");
                        let srgba = VNode.BackgroundColor.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.BackgroundColor = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            BgColor.0 = VNode.BackgroundColor;
                        }
                    });
                }

                let mut gradient_changed = false;
                Ui.horizontal(|Ui| {
                    if Ui.checkbox(&mut VNode.IsGradient, "Use Gradient").changed() {
                        gradient_changed = true;
                    }
                });

                if VNode.IsGradient {
                    Ui.horizontal(|Ui| {
                        Ui.label("Gradient Color 1:");
                        let srgba = VNode.GradientColor1.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.GradientColor1 = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            gradient_changed = true;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Gradient Color 2:");
                        let srgba = VNode.GradientColor2.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.GradientColor2 = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            gradient_changed = true;
                        }
                    });
                }

                if gradient_changed {
                    if VNode.IsGradient {
                        Commands.entity(Ent).insert(BackgroundGradient::from(LinearGradient {
                            color_space: InterpolationColorSpace::Srgba,
                            angle: 0.0,
                            stops: vec![
                                ColorStop::percent(VNode.GradientColor1, 0.0),
                                ColorStop::percent(VNode.GradientColor2, 100.0),
                            ],
                        }));
                    } else {
                        Commands.entity(Ent).remove::<BackgroundGradient>();
                    }
                }

                Ui.horizontal(|Ui| {
                    Ui.label("Border Radius:");
                    if Ui.add(egui::DragValue::new(&mut VNode.BorderRadiusPx).range(0.0..=100.0)).changed() {
                        UiNode.border_radius = BorderRadius::all(Val::Px(VNode.BorderRadiusPx));
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Border Width:");
                    if Ui.add(egui::DragValue::new(&mut VNode.BorderWidthPx).range(0.0..=50.0)).changed() {
                        UiNode.border = UiRect::all(Val::Px(VNode.BorderWidthPx));
                    }
                });

                Ui.horizontal(|Ui| {
                    Ui.label("Border Color:");
                    let srgba = VNode.BorderColor.to_srgba();
                    let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                    if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                        VNode.BorderColor = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                        if let Some(mut border_color) = BorderColorOpt {
                            *border_color = BorderColor::all(VNode.BorderColor);
                        } else {
                            Commands.entity(Ent).insert(BorderColor::all(VNode.BorderColor));
                        }
                    }
                });

                let mut shadow_changed = false;
                Ui.horizontal(|Ui| {
                    if Ui.checkbox(&mut VNode.HasShadow, "Use Shadow").changed() {
                        shadow_changed = true;
                    }
                });

                if VNode.HasShadow {
                    Ui.horizontal(|Ui| {
                        Ui.label("Shadow Color:");
                        let srgba = VNode.ShadowColor.to_srgba();
                        let mut ColorArr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if Ui.color_edit_button_rgba_unmultiplied(&mut ColorArr).changed() {
                            VNode.ShadowColor = Color::srgba(ColorArr[0], ColorArr[1], ColorArr[2], ColorArr[3]);
                            shadow_changed = true;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Offset X:");
                        if Ui.add(egui::DragValue::new(&mut VNode.ShadowOffsetX)).changed() {
                            shadow_changed = true;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Offset Y:");
                        if Ui.add(egui::DragValue::new(&mut VNode.ShadowOffsetY)).changed() {
                            shadow_changed = true;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Blur:");
                        if Ui.add(egui::DragValue::new(&mut VNode.ShadowBlur).range(0.0..=100.0)).changed() {
                            shadow_changed = true;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Spread:");
                        if Ui.add(egui::DragValue::new(&mut VNode.ShadowSpread).range(-100.0..=100.0)).changed() {
                            shadow_changed = true;
                        }
                    });
                }

                if shadow_changed {
                    if VNode.HasShadow {
                        Commands.entity(Ent).insert(BoxShadow::new(
                            VNode.ShadowColor,
                            Val::Px(VNode.ShadowOffsetX),
                            Val::Px(VNode.ShadowOffsetY),
                            Val::Px(VNode.ShadowSpread),
                            Val::Px(VNode.ShadowBlur),
                        ));
                    } else {
                        Commands.entity(Ent).remove::<BoxShadow>();
                    }
                }

                if VNode.IsImage {
                    if Ui.button("Select Image").clicked() {
                        if let Some(Path) = FileDialog::new().add_filter("Image", &["png", "jpg", "jpeg"]).pick_file() {
                            if let Ok(Bytes) = fs::read(&Path) {
                                VNode.ImageData = Some(Bytes.clone());
                                if let Some(LoadedImage) = load_image_from_bytes(&Bytes) {
                                    let Handle = Helper.Images.add(LoadedImage);
                                    Commands.entity(Ent).insert(ImageNode::new(Handle));
                                    Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                                }
                            }
                        }
                    }
                }

                if VNode.HasText {
                    Ui.horizontal(|Ui| {
                        let mut is_input = VNode.IsInput;
                        if Ui.checkbox(&mut is_input, "Is Input").changed() {
                            VNode.IsInput = is_input;
                            if let Ok(Children) = QueryChildren.get(Ent) {
                                for Child in Children.iter() {
                                    if QueryText.get(Child).is_ok() {
                                        if is_input {
                                            Commands.entity(Child).insert(EditableText::default());
                                        } else {
                                            Commands.entity(Child).remove::<EditableText>();
                                        }
                                    }
                                }
                            }
                        }
                    });

                    if VNode.IsInput {
                        Ui.horizontal(|Ui| {
                            Ui.label("Placeholder:");
                            Ui.text_edit_singleline(&mut VNode.Placeholder);
                        });
                    }

                    Ui.horizontal(|Ui| {
                        Ui.checkbox(&mut VNode.IsBold, "Bold");
                        Ui.checkbox(&mut VNode.IsItalic, "Italic");
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Font Family:");
                        Ui.text_edit_singleline(&mut VNode.FontFamily);
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Font Size:");
                        Ui.add(egui::DragValue::new(&mut VNode.FontSizePx).range(4.0..=120.0));
                    });

                    let mut main_text_child = None;
                    if let Ok(Children) = QueryChildren.get(Ent) {
                        for Child in Children.iter() {
                            if QueryText.get(Child).is_ok() && QueryPlaceholder.get(Child).is_err() {
                                main_text_child = Some(Child);
                                break;
                            }
                        }
                    }

                    if let Some(Child) = main_text_child {
                        if let Ok(mut TextComp) = QueryText.get_mut(Child) {
                            Ui.horizontal(|Ui| {
                                Ui.label("Text:");
                                Ui.text_edit_singleline(&mut TextComp.0);
                            });
                        }

                        if Ui.button("Change Font").clicked() {
                            if let Some(Path) = FileDialog::new().add_filter("Font", &["ttf", "otf"]).pick_file() {
                                if let Ok(Bytes) = fs::read(&Path) {
                                    if ttf_parser::Face::parse(&Bytes, 0).is_ok() {
                                        VNode.FontData = Some(Bytes.clone());
                                        let LoadedFont = Font::from_bytes(Bytes);
                                        let Handle = Helper.Fonts.add(LoadedFont);
                                        if let Ok(mut TextFontComp) = QueryTextFont.get_mut(Child) {
                                            TextFontComp.font = FontSource::Handle(Handle.clone());
                                        } else {
                                            Commands.entity(Child).insert(TextFont { font: FontSource::Handle(Handle.clone()), font_size: FontSize::Px(VNode.FontSizePx), ..default() });
                                        }
                                        Helper.RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
                                    }
                                }
                            }
                        }
                    }
                }

                Ui.separator();
                Ui.heading("Grid Layout");
                Ui.horizontal(|Ui| {
                    Ui.checkbox(&mut VNode.IsGrid, "Use Grid Layout");
                });

                if VNode.IsGrid {
                    Ui.horizontal(|Ui| {
                        Ui.label("Grid Columns:");
                        let mut cols = VNode.GridColumns;
                        if Ui.add(egui::DragValue::new(&mut cols).range(1..=32)).changed() {
                            VNode.GridColumns = cols;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Grid Rows:");
                        let mut rows = VNode.GridRows;
                        if Ui.add(egui::DragValue::new(&mut rows).range(1..=32)).changed() {
                            VNode.GridRows = rows;
                        }
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Column Gap:");
                        Ui.add(egui::DragValue::new(&mut VNode.GridColumnGap).range(0.0..=100.0));
                    });

                    Ui.horizontal(|Ui| {
                        Ui.label("Row Gap:");
                        Ui.add(egui::DragValue::new(&mut VNode.GridRowGap).range(0.0..=100.0));
                    });
                }

                Ui.separator();
                Ui.heading("Animation");
                Ui.horizontal(|Ui| {
                    Ui.label("Target Width:");
                    Ui.add(egui::DragValue::new(&mut VNode.AnimTargetWidth));
                });
                Ui.horizontal(|Ui| {
                    Ui.label("Target Height:");
                    Ui.add(egui::DragValue::new(&mut VNode.AnimTargetHeight));
                });
                Ui.horizontal(|Ui| {
                    Ui.label("Target X:");
                    Ui.add(egui::DragValue::new(&mut VNode.AnimTargetX));
                });
                Ui.horizontal(|Ui| {
                    Ui.label("Target Y:");
                    Ui.add(egui::DragValue::new(&mut VNode.AnimTargetY));
                });
                Ui.horizontal(|Ui| {
                    Ui.label("Target Rotation:");
                    let mut Degrees = VNode.AnimTargetRotation.to_degrees();
                    if Ui.add(egui::DragValue::new(&mut Degrees)).changed() {
                        VNode.AnimTargetRotation = Degrees.to_radians();
                    }
                });
                Ui.horizontal(|Ui| {
                    Ui.label("Duration:");
                    Ui.add(egui::DragValue::new(&mut VNode.AnimDuration));
                });

                if let Ok(mut AnimState) = QueryAnimState.get_mut(Ent) {
                    let anim_label = if AnimState.IsPlaying { "Pause Animation" } else { "Play Animation" };
                    if Ui.button(anim_label).clicked() {
                        AnimState.IsPlaying = !AnimState.IsPlaying;
                    }
                }
            }
        } else {
            Ui.heading("Canvas Settings");
            Ui.separator();
            Ui.checkbox(&mut Config.SnappingEnabled, "Snap Elements");
        }
    });

    SelectedEntity.IsPointerOverUi = Ctx.wants_pointer_input() || Ctx.is_pointer_over_area() || Ctx.is_using_pointer();
}

fn RenderHierarchy(
    Ui: &mut egui::Ui,
    Entity: Entity,
    QueryNodes: &Query<(Entity, &mut VuisNode, &mut BackgroundColor, &mut Node, Option<&mut BorderColor>, &mut Transform)>,
    QueryChildren: &Query<&Children>,
    SelectedEntity: &mut EditorSelection,
    ReparentAction: &mut Option<(Entity, Entity)>,
) {
    if let Ok((_, VNode, _, _, _, _)) = QueryNodes.get(Entity) {
        let IsSelected = Some(Entity) == SelectedEntity.SelectedNode;
        
        let response = Ui.selectable_label(IsSelected, &VNode.Id);
        let interact_response = Ui.interact(response.rect, egui::Id::new(Entity.to_bits()).with("interact"), egui::Sense::click_and_drag());
        
        if response.clicked() || interact_response.clicked() {
            SelectedEntity.SelectedNode = Some(Entity);
        }

        if interact_response.drag_started() {
            SelectedEntity.DraggedHierarchyEntity = Some(Entity);
        }

        if response.hovered() || interact_response.hovered() {
            if Ui.input(|i| i.pointer.any_released()) {
                if let Some(dragged) = SelectedEntity.DraggedHierarchyEntity {
                    if dragged != Entity {
                        *ReparentAction = Some((dragged, Entity));
                    }
                }
            }
        }

        if let Ok(Children) = QueryChildren.get(Entity) {
            Ui.indent(Entity.to_bits(), |Ui| {
                for Child in Children.iter() {
                    RenderHierarchy(Ui, Child, QueryNodes, QueryChildren, SelectedEntity, ReparentAction);
                }
            });
        }
    }
}

fn IsDescendant(Target: Entity, PotentialAncestor: Entity, QueryChildren: &Query<&Children>) -> bool {
    if Target == PotentialAncestor { return true; }
    if let Ok(Children) = QueryChildren.get(PotentialAncestor) {
        for Child in Children.iter() {
            if IsDescendant(Target, Child, QueryChildren) {
                return true;
            }
        }
    }
    false
}

pub fn SelectionHighlightSystem(
    mut Commands: Commands,
    QueryAll: Query<Entity, With<VuisNode>>,
    SelectedEntity: Res<EditorSelection>,
) {
    if SelectedEntity.is_changed() {
        for Entity in QueryAll.iter() {
            if Some(Entity) == SelectedEntity.SelectedNode {
                Commands.entity(Entity).insert((
                    Outline::new(Val::Px(2.0), Val::Px(2.0), Color::WHITE),
                    SelectedNode,
                ));
            } else {
                if let Ok(mut EntCmds) = Commands.get_entity(Entity) {
                    EntCmds.remove::<Outline>();
                    EntCmds.remove::<SelectedNode>();
                }
            }
        }
    }
}

pub fn AnimationSystem(
    Time: Res<Time>,
    mut QueryNodes: Query<(&VuisNode, &mut Node, &mut Transform, &mut VuisAnimationState)>,
) {
    for (VNode, mut UiNode, mut Trans, mut State) in QueryNodes.iter_mut() {
        if State.IsPlaying && VNode.AnimDuration > 0.0 {
            State.Timer += Time.delta_secs();
            if State.Timer >= VNode.AnimDuration {
                State.Timer = 0.0;
                State.Forward = !State.Forward;
            }
            let Progress = State.Timer / VNode.AnimDuration;
            let Eased = if State.Forward { Progress } else { 1.0 - Progress };
            
            let CurrentWidth = VNode.WidthPx + (VNode.AnimTargetWidth - VNode.WidthPx) * Eased;
            let CurrentHeight = VNode.HeightPx + (VNode.AnimTargetHeight - VNode.HeightPx) * Eased;
            let CurrentX = VNode.PositionX + (VNode.AnimTargetX - VNode.PositionX) * Eased;
            let CurrentY = VNode.PositionY + (VNode.AnimTargetY - VNode.PositionY) * Eased;
            let CurrentRot = VNode.Rotation + (VNode.AnimTargetRotation - VNode.Rotation) * Eased;

            UiNode.width = if CurrentWidth <= 0.0 { Val::Auto } else { Val::Px(CurrentWidth) };
            UiNode.height = if CurrentHeight <= 0.0 { Val::Auto } else { Val::Px(CurrentHeight) };
            UiNode.left = Val::Px(CurrentX);
            UiNode.top = Val::Px(CurrentY);
            Trans.rotation = Quat::from_rotation_z(-CurrentRot);
        } else if !State.IsPlaying {
            UiNode.width = if VNode.WidthPx <= 0.0 { Val::Auto } else { Val::Px(VNode.WidthPx) };
            UiNode.height = if VNode.HeightPx <= 0.0 { Val::Auto } else { Val::Px(VNode.HeightPx) };
            UiNode.left = Val::Px(VNode.PositionX);
            UiNode.top = Val::Px(VNode.PositionY);
            Trans.rotation = Quat::from_rotation_z(-VNode.Rotation);
        }
    }
}