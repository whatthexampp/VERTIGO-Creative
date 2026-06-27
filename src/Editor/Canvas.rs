use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::Components::VuisElement::{EditorCanvas, VuisNode};

#[derive(Component)]
pub struct NodeGridLine;

pub fn SetupCanvas(mut Commands: Commands) {
    Commands.spawn(Camera2d);

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
        ));
    });
}

pub fn ScaleCanvasSystem(
    WindowQuery: Query<&Window, With<PrimaryWindow>>,
    mut QueryCanvas: Query<&mut Transform, With<EditorCanvas>>,
) {
    let Ok(Window) = WindowQuery.single() else { return; };
    let ScaleX = Window.width() / 1920.0;
    let ScaleY = Window.height() / 1080.0;
    let ScaleFactor = f32::min(ScaleX, ScaleY).max(0.1);

    for mut transform in QueryCanvas.iter_mut() {
        transform.scale = Vec3::new(ScaleFactor, ScaleFactor, 1.0);
    }
}

pub fn GridLayoutUpdateSystem(
    QueryNodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    mut QueryNodeStyles: Query<(Option<&VuisNode>, &mut Node)>,
) {
    for (_, parent_vnode, children_opt) in QueryNodes.iter() {
        let is_grid = parent_vnode.IsGrid;
        if let Some(children) = children_opt {
            for child_ent in children.iter() {
                if let Ok((child_vnode_opt, mut child_node)) = QueryNodeStyles.get_mut(child_ent) {
                    if is_grid {
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
        if vnode.IsGrid {
            ui_node.display = Display::Grid;
            ui_node.grid_template_columns = vec![RepeatedGridTrack::flex(vnode.GridColumns as u16, 1.0)];
            ui_node.grid_template_rows = vec![RepeatedGridTrack::flex(vnode.GridRows as u16, 1.0)];
            ui_node.column_gap = Val::Px(vnode.GridColumnGap);
            ui_node.row_gap = Val::Px(vnode.GridRowGap);
        } else {
            ui_node.display = Display::Flex;
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

        if !vnode.IsGrid {
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