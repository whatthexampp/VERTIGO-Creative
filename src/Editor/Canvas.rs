use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::input::mouse::{MouseWheel, MouseMotion};
use crate::Components::VuisElement::{EditorCanvas, VuisNode};
use crate::Editor::EditorPlugin::{CanvasSettings, EditorSelection};

#[derive(Component)]
pub struct NodeGridLine;

#[derive(Component)]
pub struct CustomScrollbarTrack {
    pub target: Entity,
}

#[derive(Component)]
pub struct CustomScrollbarThumb {
    pub target: Entity,
}

pub fn SetupCanvas(mut Commands: Commands) {
    Commands.spawn((
        Camera2d,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
    ));

    Commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
    )).with_children(|Parent| {
        Parent.spawn((
            EditorCanvas,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-960.0),
                    top: Val::Px(-540.0),
                    ..default()
                },
                width: Val::Px(1920.0),
                height: Val::Px(1080.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.1, 0.1, 0.1)),
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            UiTransform::IDENTITY,
        ));
    });
}

pub fn ScaleCanvasSystem(
    WindowQuery: Query<&Window, With<PrimaryWindow>>,
    CanvasSettings: Res<CanvasSettings>,
    mut QueryCanvas: Query<(&mut UiTransform, &mut Node), With<EditorCanvas>>,
) {
    let Ok(Window) = WindowQuery.single() else { return; };
    let BaseScaleX = Window.width() / CanvasSettings.Width;
    let BaseScaleY = Window.height() / CanvasSettings.Height;
    let BaseScaleFactor = f32::min(BaseScaleX, BaseScaleY).max(0.1);
    let FinalScale = BaseScaleFactor * CanvasSettings.Zoom;

    for (mut transform, mut node) in QueryCanvas.iter_mut() {
        node.width = Val::Px(CanvasSettings.Width);
        node.height = Val::Px(CanvasSettings.Height);
        node.margin = UiRect {
            left: Val::Px(-CanvasSettings.Width / 2.0),
            top: Val::Px(-CanvasSettings.Height / 2.0),
            ..default()
        };
        transform.scale = Vec2::new(FinalScale, FinalScale);
        transform.translation = Val2 {
            x: Val::Px(CanvasSettings.PanX),
            y: Val::Px(CanvasSettings.PanY),
        };
    }
}

pub fn ZoomAndPanSystem(
    mut MouseWheelEvents: MessageReader<MouseWheel>,
    mut MouseMotionEvents: MessageReader<MouseMotion>,
    MouseInput: Res<ButtonInput<MouseButton>>,
    mut CanvasSettings: ResMut<CanvasSettings>,
    SelectedEntity: Res<EditorSelection>,
) {
    if !SelectedEntity.IsPointerOverUi {
        for event in MouseWheelEvents.read() {
            let ZoomDelta = event.y * 0.05;
            CanvasSettings.Zoom = (CanvasSettings.Zoom + ZoomDelta).clamp(0.1, 5.0);
        }
    }

    for event in MouseMotionEvents.read() {
        if MouseInput.pressed(MouseButton::Middle) {
            CanvasSettings.PanX += event.delta.x;
            CanvasSettings.PanY += event.delta.y;
        }
    }
}

pub fn ScrollInputSystem(
    mut MouseWheelEvents: MessageReader<MouseWheel>,
    mut QueryNodes: Query<(Entity, &VuisNode, &ComputedNode, &UiGlobalTransform, &mut ScrollPosition)>,
    WindowQuery: Query<&Window, With<PrimaryWindow>>,
    SelectedEntity: Res<EditorSelection>,
) {
    let Ok(Window) = WindowQuery.single() else { return; };
    let Some(CursorPosPhysical) = Window.physical_cursor_position() else { return; };

    if SelectedEntity.IsPointerOverUi {
        return;
    }

    for event in MouseWheelEvents.read() {
        for (_entity, vnode, computed, global_trans, mut scroll_pos) in QueryNodes.iter_mut() {
            if !vnode.IsScrollable {
                continue;
            }
            if computed.contains_point(*global_trans, CursorPosPhysical) {
                let scroll_delta = event.y * 30.0;
                let visible_height = computed.size().y * computed.inverse_scale_factor();
                let content_height = computed.content_size().y * computed.inverse_scale_factor();
                let range = (content_height - visible_height).max(0.0);
                scroll_pos.y = (scroll_pos.y - scroll_delta).clamp(0.0, range);
            }
        }
    }
}

pub fn SpawnScrollbarsSystem(
    mut Commands: Commands,
    QueryScrollNodes: Query<(Entity, &VuisNode, Option<&Children>), Changed<VuisNode>>,
    QueryTracks: Query<(Entity, &CustomScrollbarTrack)>,
    QueryThumbs: Query<(Entity, &CustomScrollbarThumb)>,
) {
    for (node_ent, vnode, children_opt) in QueryScrollNodes.iter() {
        let mut has_scrollbar = false;
        if let Some(children) = children_opt {
            for child in children.iter() {
                if QueryTracks.get(child).is_ok() {
                    has_scrollbar = true;
                }
            }
        }

        if vnode.IsScrollable && !has_scrollbar {
            let track_ent = Commands.spawn((
                CustomScrollbarTrack { target: node_ent },
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    top: Val::Px(2.0),
                    width: Val::Px(vnode.ScrollbarWidth),
                    border_radius: BorderRadius::all(Val::Px(vnode.ScrollbarBorderRadius)),
                    ..default()
                },
                BackgroundColor(vnode.ScrollbarTrackColor),
                IgnoreScroll(BVec2::TRUE),
                ZIndex(100),
                UiTransform::default(),
            )).id();

            let thumb_ent = Commands.spawn((
                CustomScrollbarThumb { target: node_ent },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    height: Val::Px(20.0),
                    border_radius: BorderRadius::all(Val::Px(vnode.ScrollbarBorderRadius)),
                    ..default()
                },
                BackgroundColor(vnode.ScrollbarColor),
                IgnoreScroll(BVec2::TRUE),
                ZIndex(101),
                UiTransform::default(),
            )).id();

            Commands.entity(track_ent).add_child(thumb_ent);
            Commands.entity(node_ent).add_child(track_ent);
        } else if !vnode.IsScrollable && has_scrollbar {
            if let Some(children) = children_opt {
                for child in children.iter() {
                    if let Ok((track_ent, _)) = QueryTracks.get(child) {
                        Commands.entity(track_ent).despawn();
                    }
                }
            }
        }
    }

    for (track_ent, track) in QueryTracks.iter() {
        if let Ok((_, vnode, _)) = QueryScrollNodes.get(track.target) {
            Commands.entity(track_ent).insert((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    top: Val::Px(2.0),
                    width: Val::Px(vnode.ScrollbarWidth),
                    border_radius: BorderRadius::all(Val::Px(vnode.ScrollbarBorderRadius)),
                    ..default()
                },
                BackgroundColor(vnode.ScrollbarTrackColor),
            ));
        }
    }

    for (thumb_ent, thumb) in QueryThumbs.iter() {
        if let Ok((_, vnode, _)) = QueryScrollNodes.get(thumb.target) {
            Commands.entity(thumb_ent).insert((
                Node {
                    border_radius: BorderRadius::all(Val::Px(vnode.ScrollbarBorderRadius)),
                    ..default()
                },
                BackgroundColor(vnode.ScrollbarColor),
            ));
        }
    }
}

pub fn UpdateScrollbarsSystem(
    QueryScrollNodes: Query<(Entity, &VuisNode, &ComputedNode, &ScrollPosition)>,
    mut QueryTracks: Query<(&CustomScrollbarTrack, &mut Node), Without<CustomScrollbarThumb>>,
    mut QueryThumbs: Query<(&CustomScrollbarThumb, &mut Node), Without<CustomScrollbarTrack>>,
) {
    for (track, mut track_node) in QueryTracks.iter_mut() {
        if let Ok((_, _, computed, _)) = QueryScrollNodes.get(track.target) {
            let visible_height = computed.size().y * computed.inverse_scale_factor();
            track_node.height = Val::Px(visible_height - 4.0);
        }
    }

    for (thumb, mut thumb_node) in QueryThumbs.iter_mut() {
        if let Ok((_, _vnode, computed, scroll_pos)) = QueryScrollNodes.get(thumb.target) {
            let visible_height = computed.size().y * computed.inverse_scale_factor();
            let content_height = computed.content_size().y * computed.inverse_scale_factor();
            
            if content_height > visible_height && visible_height > 0.0 {
                let ratio = visible_height / content_height;
                let track_height = visible_height - 4.0;
                let thumb_height = (track_height * ratio).max(12.0);
                
                let scrollable_height = content_height - visible_height;
                let scroll_ratio = if scrollable_height > 0.0 { scroll_pos.y / scrollable_height } else { 0.0 };
                let max_thumb_top = track_height - thumb_height;
                let thumb_top = scroll_ratio * max_thumb_top;

                thumb_node.height = Val::Px(thumb_height);
                thumb_node.top = Val::Px(thumb_top);
                thumb_node.display = Display::Flex;
            } else {
                thumb_node.display = Display::None;
            }
        }
    }
}

pub fn GridLayoutUpdateSystem(
    QueryNodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    mut QueryNodeStyles: Query<(Option<&VuisNode>, &mut Node)>,
) {
    for (_, parent_vnode, children_opt) in QueryNodes.iter() {
        let is_flow = parent_vnode.LayoutFlow != "None";
        if let Some(children) = children_opt {
            for child_ent in children.iter() {
                if let Ok((child_vnode_opt, mut child_node)) = QueryNodeStyles.get_mut(child_ent) {
                    if is_flow {
                        if child_node.position_type != PositionType::Relative {
                            child_node.position_type = PositionType::Relative;
                            child_node.left = Val::Auto;
                            child_node.top = Val::Auto;
                        }
                    } else {
                        if child_node.position_type != PositionType::Absolute {
                            child_node.position_type = PositionType::Absolute;
                            if let Some(child_vnode) = child_vnode_opt {
                                child_node.left = Val::Px(child_vnode.PositionX);
                                child_node.top = Val::Px(child_vnode.PositionY);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn GridLayoutApplySystem(
    mut QueryNodes: Query<(&VuisNode, &mut Node), Changed<VuisNode>>,
) {
    for (vnode, mut ui_node) in QueryNodes.iter_mut() {
        if vnode.IsScrollable {
            ui_node.overflow = Overflow::scroll_y();
        } else {
            ui_node.overflow = Overflow::visible();
        }

        let flow = if vnode.LayoutFlow == "None" && vnode.IsGrid {
            "Grid"
        } else {
            vnode.LayoutFlow.as_str()
        };

        if flow == "Grid" {
            ui_node.display = Display::Grid;
            ui_node.grid_template_columns = vec![RepeatedGridTrack::flex(vnode.GridColumns as u16, 1.0)];
            ui_node.grid_template_rows = vec![RepeatedGridTrack::flex(vnode.GridRows as u16, 1.0)];
            ui_node.column_gap = Val::Px(vnode.GridColumnGap);
            ui_node.row_gap = Val::Px(vnode.GridRowGap);
        } else if flow == "Vertical" {
            ui_node.display = Display::Flex;
            ui_node.flex_direction = FlexDirection::Column;
            ui_node.row_gap = Val::Px(vnode.GridRowGap);
            ui_node.column_gap = Val::Auto;
            ui_node.grid_template_columns = Vec::new();
            ui_node.grid_template_rows = Vec::new();
        } else if flow == "Horizontal" {
            ui_node.display = Display::Flex;
            ui_node.flex_direction = FlexDirection::Row;
            ui_node.column_gap = Val::Px(vnode.GridColumnGap);
            ui_node.row_gap = Val::Auto;
            ui_node.grid_template_columns = Vec::new();
            ui_node.grid_template_rows = Vec::new();
        } else {
            ui_node.display = Display::Flex;
            ui_node.flex_direction = FlexDirection::Row;
            ui_node.grid_template_columns = Vec::new();
            ui_node.grid_template_rows = Vec::new();
            ui_node.column_gap = Val::Auto;
            ui_node.row_gap = Val::Auto;
        }
    }
}

pub fn SyncNodeGridLinesSystem(
    mut Commands: Commands,
    QueryGridNodes: Query<(Entity, &VuisNode, Option<&Children>), Changed<VuisNode>>,
    QueryLines: Query<Entity, With<NodeGridLine>>,
) {
    for (node_ent, vnode, children_opt) in QueryGridNodes.iter() {
        if let Some(children) = children_opt {
            for child_ent in children.iter() {
                if QueryLines.get(child_ent).is_ok() {
                    Commands.entity(child_ent).despawn();
                }
            }
        }

        if vnode.LayoutFlow != "Grid" && (!vnode.IsGrid || vnode.LayoutFlow != "None") {
            continue;
        }

        let cols = vnode.GridColumns.max(1);
        let rows = vnode.GridRows.max(1);
        let col_gap = vnode.GridColumnGap;
        let row_gap = vnode.GridRowGap;

        let w = vnode.WidthPx;
        let h = vnode.HeightPx;

        if w <= 0.0 || h <= 0.0 {
            continue;
        }

        let col_w = (w - (cols - 1) as f32 * col_gap).max(0.0) / cols as f32;
        let row_h = (h - (rows - 1) as f32 * row_gap).max(0.0) / rows as f32;

        let grid_color = BackgroundColor(Color::srgba(0.9, 0.3, 0.6, 0.35));

        Commands.entity(node_ent).with_children(|parent| {
            for i in 1..cols {
                let x_left = i as f32 * col_w + (i - 1) as f32 * col_gap;
                parent.spawn((
                    NodeGridLine,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x_left),
                        top: Val::Px(0.0),
                        width: Val::Px(1.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    grid_color.clone(),
                    ZIndex(-1),
                ));

                if col_gap > 0.0 {
                    let x_right = x_left + col_gap;
                    parent.spawn((
                        NodeGridLine,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x_right),
                            top: Val::Px(0.0),
                            width: Val::Px(1.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        grid_color.clone(),
                        ZIndex(-1),
                ));
                }
            }

            for j in 1..rows {
                let y_top = j as f32 * row_h + (j - 1) as f32 * row_gap;
                parent.spawn((
                    NodeGridLine,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(y_top),
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        ..default()
                    },
                    grid_color.clone(),
                    ZIndex(-1),
                ));

                if row_gap > 0.0 {
                    let y_bottom = y_top + row_gap;
                    parent.spawn((
                        NodeGridLine,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(y_bottom),
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            ..default()
                        },
                        grid_color.clone(),
                        ZIndex(-1),
                    ));
                }
            }
        });
    }
}