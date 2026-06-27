use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use bevy::ecs::relationship::Relationship;
use crate::Components::VuisElement::{EditorCanvas, PlaceholderTextComponent, VuisNode};
use crate::Editor::EditorPlugin::{EditorSelection, EditorConfig, CanvasSettings};
use crate::Serialization::VuisFormat::VuisDataNode;
use serde_json;

#[derive(Resource, Default)]
pub struct EditorDragState {
    pub DraggedEntity: Option<Entity>,
    pub Action: Option<DragAction>,
    pub LastCursorPosition: Option<Vec2>,
    pub InitialPosition: Option<Vec2>,
    pub InitialSize: Option<Vec2>,
    pub InitialRotation: Option<f32>,
    pub StartCursorPhysical: Option<Vec2>,
}

#[derive(Resource, Default)]
pub struct CopyPasteBuffer {
    pub CopiedJson: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragAction {
    Move,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    Rotate,
}

fn PhysicalToNodeRelative(
    phys_pt: Vec2,
    node_global: &UiGlobalTransform,
    node_computed: &ComputedNode,
) -> Vec2 {
    if let Some(inv) = node_global.try_inverse() {
        let center_rel_phys = inv.transform_point2(phys_pt);
        let center_rel_logical = center_rel_phys * node_computed.inverse_scale_factor();
        let logical_size = node_computed.size() * node_computed.inverse_scale_factor();
        center_rel_logical + logical_size / 2.0
    } else {
        Vec2::ZERO
    }
}

fn CanvasLogicalToPhysical(
    logical_pt: Vec2,
    canvas_global: &UiGlobalTransform,
    canvas_computed: &ComputedNode,
) -> Vec2 {
    let logical_size = canvas_computed.size() * canvas_computed.inverse_scale_factor();
    let center_rel_logical = logical_pt - logical_size / 2.0;
    let center_rel_phys = center_rel_logical / canvas_computed.inverse_scale_factor();
    canvas_global.transform_point2(center_rel_phys)
}

fn dist_to_segment(p: Vec2, v: Vec2, w: Vec2) -> f32 {
    let l2 = v.distance_squared(w);
    if l2 == 0.0 {
        return p.distance(v);
    }
    let t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2;
    let t = t.clamp(0.0, 1.0);
    let projection = v + t * (w - v);
    p.distance(projection)
}

#[allow(deprecated)]
pub fn SelectionAndDragSystem(
    mut Commands: Commands,
    WindowQuery: Query<&Window, With<PrimaryWindow>>,
    MouseInput: Res<ButtonInput<MouseButton>>,
    mut ParamSet: ParamSet<(
        Query<(Entity, &mut VuisNode, &mut Node, &mut UiTransform)>,
        Query<(Entity, Option<&VuisNode>, Option<&ChildOf>, Option<&InheritedVisibility>, &UiGlobalTransform, &ComputedNode)>,
    )>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    mut SelectedEntity: ResMut<EditorSelection>,
    mut DragState: ResMut<EditorDragState>,
    mut Gizmos: Gizmos,
    mut EguiCtxs: EguiContexts,
    Config: Res<EditorConfig>,
    mut RecordEvents: MessageWriter<crate::Editor::History::RecordHistoryEvent>,
    CanvasSettings: Res<CanvasSettings>,
) {
    let Ok(Window) = WindowQuery.single() else { return; };
    let Ok(CanvasEnt) = QueryCanvas.single() else { return; };

    let W = Window.width();
    let H = Window.height();

    let (canvas_global, canvas_computed) = {
        let query_read = ParamSet.p1();
        let Ok((_, _, _, _, cg, cc)) = query_read.get(CanvasEnt) else { return; };
        (*cg, cc.clone())
    };

    let Some(CursorPosPhysical) = Window.physical_cursor_position() else { return; };

    if MouseInput.just_pressed(MouseButton::Left) {
        let mut Handled = SelectedEntity.IsPointerOverUi;
        if let Ok(Ctx) = EguiCtxs.ctx_mut() {
            if Ctx.wants_pointer_input() {
                Handled = true;
            }
        }

        let QueryReadOnly = ParamSet.p1();

        if !Handled {
            if let Some(SelEnt) = SelectedEntity.SelectedNode {
                if let Ok((_, Some(VNode), _, _, global_trans, computed)) = QueryReadOnly.get(SelEnt) {
                    let phys_size = computed.size();
                    
                    let local_tl = Vec2::new(-phys_size.x / 2.0, -phys_size.y / 2.0);
                    let local_tr = Vec2::new(phys_size.x / 2.0, -phys_size.y / 2.0);
                    let local_bl = Vec2::new(-phys_size.x / 2.0, phys_size.y / 2.0);
                    let local_br = Vec2::new(phys_size.x / 2.0, phys_size.y / 2.0);
                    let local_rot = Vec2::new(0.0, -phys_size.y / 2.0 - 40.0 * Window.scale_factor());

                    let phys_tl = global_trans.transform_point2(local_tl);
                    let phys_tr = global_trans.transform_point2(local_tr);
                    let phys_bl = global_trans.transform_point2(local_bl);
                    let phys_br = global_trans.transform_point2(local_br);
                    let phys_rot = global_trans.transform_point2(local_rot);

                    let hit_dist_phys = 16.0 * Window.scale_factor();
                    let hit_dist_edge = 8.0 * Window.scale_factor();

                    if CursorPosPhysical.distance(phys_tl) <= hit_dist_phys {
                        DragState.DraggedEntity = Some(SelEnt);
                        DragState.Action = Some(DragAction::ResizeTopLeft);
                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                        DragState.StartCursorPhysical = Some(CursorPosPhysical);
                        DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                        DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                        DragState.InitialRotation = Some(VNode.Rotation);
                        Handled = true;
                    } else if CursorPosPhysical.distance(phys_tr) <= hit_dist_phys {
                        DragState.DraggedEntity = Some(SelEnt);
                        DragState.Action = Some(DragAction::ResizeTopRight);
                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                        DragState.StartCursorPhysical = Some(CursorPosPhysical);
                        DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                        DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                        DragState.InitialRotation = Some(VNode.Rotation);
                        Handled = true;
                    } else if CursorPosPhysical.distance(phys_bl) <= hit_dist_phys {
                        DragState.DraggedEntity = Some(SelEnt);
                        DragState.Action = Some(DragAction::ResizeBottomLeft);
                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                        DragState.StartCursorPhysical = Some(CursorPosPhysical);
                        DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                        DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                        DragState.InitialRotation = Some(VNode.Rotation);
                        Handled = true;
                    } else if CursorPosPhysical.distance(phys_br) <= hit_dist_phys {
                        DragState.DraggedEntity = Some(SelEnt);
                        DragState.Action = Some(DragAction::ResizeBottomRight);
                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                        DragState.StartCursorPhysical = Some(CursorPosPhysical);
                        DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                        DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                        DragState.InitialRotation = Some(VNode.Rotation);
                        Handled = true;
                    } else if CursorPosPhysical.distance(phys_rot) <= hit_dist_phys * 2.0 {
                        DragState.DraggedEntity = Some(SelEnt);
                        DragState.Action = Some(DragAction::Rotate);
                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                        DragState.StartCursorPhysical = Some(CursorPosPhysical);
                        DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                        DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                        DragState.InitialRotation = Some(VNode.Rotation);
                        Handled = true;
                    }

                    if !Handled {
                        let d_top = dist_to_segment(CursorPosPhysical, phys_tl, phys_tr);
                        let d_right = dist_to_segment(CursorPosPhysical, phys_tr, phys_br);
                        let d_bottom = dist_to_segment(CursorPosPhysical, phys_bl, phys_br);
                        let d_left = dist_to_segment(CursorPosPhysical, phys_tl, phys_bl);

                        let mut closest_edge = None;
                        let mut min_dist = hit_dist_edge;

                        if d_top < min_dist {
                            min_dist = d_top;
                            closest_edge = Some(DragAction::ResizeTop);
                        }
                        if d_right < min_dist {
                            min_dist = d_right;
                            closest_edge = Some(DragAction::ResizeRight);
                        }
                        if d_bottom < min_dist {
                            min_dist = d_bottom;
                            closest_edge = Some(DragAction::ResizeBottom);
                        }
                        if d_left < min_dist {
                            closest_edge = Some(DragAction::ResizeLeft);
                        }

                        if let Some(action) = closest_edge {
                            DragState.DraggedEntity = Some(SelEnt);
                            DragState.Action = Some(action);
                            DragState.LastCursorPosition = Some(CursorPosPhysical);
                            DragState.StartCursorPhysical = Some(CursorPosPhysical);
                            DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                            DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                            DragState.InitialRotation = Some(VNode.Rotation);
                            Handled = true;
                        }
                    }

                    if !Handled {
                        if computed.contains_point(*global_trans, CursorPosPhysical) {
                            DragState.DraggedEntity = Some(SelEnt);
                            DragState.Action = Some(DragAction::Move);
                            DragState.LastCursorPosition = Some(CursorPosPhysical);
                            DragState.StartCursorPhysical = Some(CursorPosPhysical);
                            DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                            DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                            DragState.InitialRotation = Some(VNode.Rotation);
                            Handled = true;
                        }
                    }
                }
            }
        }

        if !Handled {
            let mut ClickedNode = None;
            for (Entity, VNodeOpt, _, VisOpt, global_trans, computed) in QueryReadOnly.iter() {
                if Entity == CanvasEnt { continue; }
                if let Some(Vis) = VisOpt {
                    if !Vis.get() { continue; }
                }
                if VNodeOpt.is_none() { continue; }

                if computed.contains_point(*global_trans, CursorPosPhysical) {
                    ClickedNode = Some(Entity);
                }
            }

            if let Some(Ent) = ClickedNode {
                SelectedEntity.SelectedNode = Some(Ent);
                DragState.DraggedEntity = Some(Ent);
                DragState.Action = Some(DragAction::Move);
                DragState.LastCursorPosition = Some(CursorPosPhysical);
                DragState.StartCursorPhysical = Some(CursorPosPhysical);
                if let Ok((_, Some(VNode), _, _, _, _)) = QueryReadOnly.get(Ent) {
                    DragState.InitialPosition = Some(Vec2::new(VNode.PositionX, VNode.PositionY));
                    DragState.InitialSize = Some(Vec2::new(VNode.WidthPx, VNode.HeightPx));
                    DragState.InitialRotation = Some(VNode.Rotation);
                }
            } else {
                let canvas_rect_logical = canvas_computed.size() * canvas_computed.inverse_scale_factor();
                let cursor_canvas_logical = PhysicalToNodeRelative(CursorPosPhysical, &canvas_global, &canvas_computed);
                let IsOnCanvas = cursor_canvas_logical.x >= 0.0 && cursor_canvas_logical.x <= canvas_rect_logical.x &&
                                 cursor_canvas_logical.y >= 0.0 && cursor_canvas_logical.y <= canvas_rect_logical.y;

                if IsOnCanvas {
                    SelectedEntity.SelectedNode = None;
                    DragState.DraggedEntity = None;
                    DragState.Action = None;
                    DragState.LastCursorPosition = None;
                }
            }
        }
    }

    if MouseInput.pressed(MouseButton::Left) {
        if let Some(DragEnt) = DragState.DraggedEntity {
            if let Some(Act) = DragState.Action {
                if let Some(StartCursorPhys) = DragState.StartCursorPhysical {
                    let DeltaPos = CursorPosPhysical - StartCursorPhys;
                    if DeltaPos.length_squared() > 0.0001 {
                        let ParentEnt = {
                            let QueryReadOnly = ParamSet.p1();
                            QueryReadOnly.get(DragEnt).ok().and_then(|(_, _, p, _, _, _)| p.map(|p| p.get())).unwrap_or(CanvasEnt)
                        };

                        let (parent_global, parent_computed) = {
                            let QueryReadOnly = ParamSet.p1();
                            if let Ok((_, _, _, _, pg, pc)) = QueryReadOnly.get(ParentEnt) {
                                (*pg, pc.clone())
                            } else {
                                (canvas_global, canvas_computed.clone())
                            }
                        };

                        let mut SnappedX = None;
                        let mut SnappedY = None;

                        let mut next_pos_x = None;
                        let mut next_pos_y = None;
                        let mut next_width = None;
                        let mut next_height = None;
                        let mut next_rotation = None;

                        match Act {
                            DragAction::Move => {
                                let start_local = PhysicalToNodeRelative(StartCursorPhys, &parent_global, &parent_computed);
                                let current_local = PhysicalToNodeRelative(CursorPosPhysical, &parent_global, &parent_computed);
                                let delta_local = current_local - start_local;

                                let initial_pos = DragState.InitialPosition.unwrap();
                                let unsnapped_x = initial_pos.x + delta_local.x;
                                let unsnapped_y = initial_pos.y + delta_local.y;

                                let mut final_x = unsnapped_x;
                                let mut final_y = unsnapped_y;

                                if Config.SnappingEnabled {
                                    let (my_w, my_h) = {
                                        let QueryReadOnly = ParamSet.p1();
                                        if let Ok((_, Some(vnode), _, _, _, _)) = QueryReadOnly.get(DragEnt) {
                                            (vnode.WidthPx, vnode.HeightPx)
                                        } else {
                                            (100.0, 100.0)
                                        }
                                    };
                                    let my_hw = my_w / 2.0;
                                    let my_hh = my_h / 2.0;

                                    let mut SnapLinesX = vec![CanvasSettings.Width / 2.0, 0.0, CanvasSettings.Width];
                                    let mut SnapLinesY = vec![CanvasSettings.Height / 2.0, 0.0, CanvasSettings.Height];

                                    {
                                        let QueryReadOnly = ParamSet.p1();
                                        for (OtherEnt, VNodeOpt, _, VisOpt, other_global, other_computed) in QueryReadOnly.iter() {
                                            if OtherEnt == CanvasEnt || OtherEnt == DragEnt { continue; }
                                            if let Some(Vis) = VisOpt { if !Vis.get() { continue; } }
                                            if VNodeOpt.is_none() { continue; }

                                            let other_center_phys = other_global.transform_point2(Vec2::ZERO);
                                            let other_center = PhysicalToNodeRelative(other_center_phys, &canvas_global, &canvas_computed);
                                            let other_size = other_computed.size() * other_computed.inverse_scale_factor();
                                            
                                            SnapLinesX.push(other_center.x);
                                            SnapLinesX.push(other_center.x - other_size.x / 2.0);
                                            SnapLinesX.push(other_center.x + other_size.x / 2.0);
                                            SnapLinesY.push(other_center.y);
                                            SnapLinesY.push(other_center.y - other_size.y / 2.0);
                                            SnapLinesY.push(other_center.y + other_size.y / 2.0);
                                        }
                                    }

                                    let logical_center_relative = Vec2::new(unsnapped_x + my_hw, unsnapped_y + my_hh);
                                    let logical_size = parent_computed.size() * parent_computed.inverse_scale_factor();
                                    let center_rel_logical = logical_center_relative - logical_size / 2.0;
                                    let center_rel_phys = center_rel_logical / parent_computed.inverse_scale_factor();
                                    let phys_center = parent_global.transform_point2(center_rel_phys);

                                    let my_center = PhysicalToNodeRelative(phys_center, &canvas_global, &canvas_computed);

                                    let MyLinesX = [my_center.x, my_center.x - my_hw, my_center.x + my_hw];
                                    let MyLinesY = [my_center.y, my_center.y - my_hh, my_center.y + my_hh];

                                    let mut BestDistX = 10.0;
                                    let mut BestOffsetX = 0.0;

                                    for &mx in &MyLinesX {
                                        for &tx in &SnapLinesX {
                                            let dist = (mx - tx).abs();
                                            if dist < BestDistX {
                                                BestDistX = dist;
                                                BestOffsetX = tx - mx;
                                                SnappedX = Some(tx);
                                            }
                                        }
                                    }

                                    let mut BestDistY = 10.0;
                                    let mut BestOffsetY = 0.0;

                                    for &my in &MyLinesY {
                                        for &ty in &SnapLinesY {
                                            let dist = (my - ty).abs();
                                            if dist < BestDistY {
                                                BestDistY = dist;
                                                BestOffsetY = ty - my;
                                                SnappedY = Some(ty);
                                            }
                                        }
                                    }

                                    if BestOffsetX != 0.0 || BestOffsetY != 0.0 {
                                        let snapped_center_logical = my_center + Vec2::new(BestOffsetX, BestOffsetY);
                                        let snapped_phys_center = CanvasLogicalToPhysical(snapped_center_logical, &canvas_global, &canvas_computed);
                                        let snapped_center_relative = PhysicalToNodeRelative(snapped_phys_center, &parent_global, &parent_computed);
                                        final_x = snapped_center_relative.x - my_hw;
                                        final_y = snapped_center_relative.y - my_hh;
                                    }
                                }

                                next_pos_x = Some(final_x);
                                next_pos_y = Some(final_y);
                            }
                            DragAction::Rotate => {
                                let phys_center = {
                                    let QueryReadOnly = ParamSet.p1();
                                    if let Ok((_, _, _, _, global_trans, _)) = QueryReadOnly.get(DragEnt) {
                                        global_trans.transform_point2(Vec2::ZERO)
                                    } else {
                                        Vec2::ZERO
                                    }
                                };

                                if let Some(inv) = parent_global.try_inverse() {
                                    let cursor_parent_local = inv.transform_point2(CursorPosPhysical);
                                    let node_center_parent_local = inv.transform_point2(phys_center);
                                    let parent_local_vec = cursor_parent_local - node_center_parent_local;
                                    let target_angle = f32::atan2(parent_local_vec.y, parent_local_vec.x);
                                    next_rotation = Some(target_angle + std::f32::consts::FRAC_PI_2);
                                }
                            }
                            _ => {
                                let initial_size = DragState.InitialSize.unwrap();
                                let initial_pos = DragState.InitialPosition.unwrap();
                                let initial_rotation = DragState.InitialRotation.unwrap();

                                let HW = initial_size.x / 2.0;
                                let HH = initial_size.y / 2.0;

                                let (local_fixed, local_dragged_start, keep_x, keep_y) = match Act {
                                    DragAction::ResizeBottomRight => (Vec2::new(-HW, -HH), Vec2::new(HW, HH), true, true),
                                    DragAction::ResizeTopLeft => (Vec2::new(HW, HH), Vec2::new(-HW, -HH), true, true),
                                    DragAction::ResizeTopRight => (Vec2::new(-HW, HH), Vec2::new(HW, -HH), true, true),
                                    DragAction::ResizeBottomLeft => (Vec2::new(HW, -HH), Vec2::new(-HW, HH), true, true),
                                    DragAction::ResizeRight => (Vec2::new(-HW, 0.0), Vec2::new(HW, 0.0), true, false),
                                    DragAction::ResizeLeft => (Vec2::new(HW, 0.0), Vec2::new(-HW, 0.0), true, false),
                                    DragAction::ResizeBottom => (Vec2::new(0.0, -HH), Vec2::new(0.0, HH), false, true),
                                    DragAction::ResizeTop => (Vec2::new(0.0, HH), Vec2::new(0.0, -HH), false, true),
                                    _ => (Vec2::ZERO, Vec2::ZERO, true, true),
                                };

                                let start_local = PhysicalToNodeRelative(StartCursorPhys, &parent_global, &parent_computed);
                                let current_local = PhysicalToNodeRelative(CursorPosPhysical, &parent_global, &parent_computed);
                                let delta_parent = current_local - start_local;

                                let cos_rot = (-initial_rotation).cos();
                                let sin_rot = (-initial_rotation).sin();
                                let mut delta_local = Vec2::new(
                                    delta_parent.x * cos_rot - delta_parent.y * sin_rot,
                                    delta_parent.x * sin_rot + delta_parent.y * cos_rot,
                                );

                                if !keep_x { delta_local.x = 0.0; }
                                if !keep_y { delta_local.y = 0.0; }

                                let mut local_dragged_new = local_dragged_start + delta_local;
                                if keep_x && (local_dragged_new.x - local_fixed.x).abs() < 10.0 {
                                    let sign = (local_dragged_start.x - local_fixed.x).signum();
                                    local_dragged_new.x = local_fixed.x + sign * 10.0;
                                }
                                if keep_y && (local_dragged_new.y - local_fixed.y).abs() < 10.0 {
                                    let sign = (local_dragged_start.y - local_fixed.y).signum();
                                    local_dragged_new.y = local_fixed.y + sign * 10.0;
                                }

                                let local_new_center = (local_fixed + local_dragged_new) / 2.0;
                                let final_w = if keep_x { (local_dragged_new.x - local_fixed.x).abs() } else { initial_size.x };
                                let final_h = if keep_y { (local_dragged_new.y - local_fixed.y).abs() } else { initial_size.y };

                                let cos_p = initial_rotation.cos();
                                let sin_p = initial_rotation.sin();
                                let parent_new_center_offset = Vec2::new(
                                    local_new_center.x * cos_p - local_new_center.y * sin_p,
                                    local_new_center.x * sin_p + local_new_center.y * cos_p,
                                );

                                let original_center_parent = initial_pos + Vec2::new(HW, HH);
                                let new_center_parent = original_center_parent + parent_new_center_offset;

                                next_pos_x = Some(new_center_parent.x - final_w / 2.0);
                                next_pos_y = Some(new_center_parent.y - final_h / 2.0);
                                next_width = Some(final_w);
                                next_height = Some(final_h);
                            }
                        }

                        let mut QueryNodesMut = ParamSet.p0();
                        if let Ok((_, mut VNode, mut NodeComp, mut TransComp)) = QueryNodesMut.get_mut(DragEnt) {
                            if let Some(x) = next_pos_x { VNode.PositionX = x; }
                            if let Some(y) = next_pos_y { VNode.PositionY = y; }
                            if let Some(w) = next_width { VNode.WidthPx = w; }
                            if let Some(h) = next_height { VNode.HeightPx = h; }
                            if let Some(r) = next_rotation { VNode.Rotation = r; }

                            NodeComp.left = Val::Px(VNode.PositionX);
                            NodeComp.top = Val::Px(VNode.PositionY);
                            NodeComp.width = Val::Px(VNode.WidthPx);
                            NodeComp.height = Val::Px(VNode.HeightPx);
                            TransComp.rotation = Rot2::radians(-VNode.Rotation);
                        }

                        let PhysToWorld = |phys: Vec2| -> Vec2 {
                            let logical = phys / Window.scale_factor();
                            Vec2::new(logical.x - W / 2.0, H / 2.0 - logical.y)
                        };

                        if let Some(x_logical) = SnappedX {
                            let phys_p1 = CanvasLogicalToPhysical(Vec2::new(x_logical, -5000.0), &canvas_global, &canvas_computed);
                            let phys_p2 = CanvasLogicalToPhysical(Vec2::new(x_logical, 5000.0), &canvas_global, &canvas_computed);
                            let world_p1 = PhysToWorld(phys_p1);
                            let world_p2 = PhysToWorld(phys_p2);
                            Gizmos.line_2d(world_p1, world_p2, Color::srgb(0.0, 1.0, 1.0));
                        }
                        if let Some(y_logical) = SnappedY {
                            let phys_p1 = CanvasLogicalToPhysical(Vec2::new(-5000.0, y_logical), &canvas_global, &canvas_computed);
                            let phys_p2 = CanvasLogicalToPhysical(Vec2::new(5000.0, y_logical), &canvas_global, &canvas_computed);
                            let world_p1 = PhysToWorld(phys_p1);
                            let world_p2 = PhysToWorld(phys_p2);
                            Gizmos.line_2d(world_p1, world_p2, Color::srgb(0.0, 1.0, 1.0));
                        }

                        DragState.LastCursorPosition = Some(CursorPosPhysical);
                    }
                }
            }
        }
    }

    if MouseInput.just_released(MouseButton::Left) {
        if let Some(DragEnt) = DragState.DraggedEntity {
            if DragState.Action == Some(DragAction::Move) {
                let mut IsOutside = false;
                let mut CenterCanvas = Vec2::ZERO;

                {
                    let QueryReadOnly = ParamSet.p1();
                    let ParentEnt = QueryReadOnly.get(DragEnt).ok().and_then(|(_, _, p, _, _, _)| p.map(|p| p.get())).unwrap_or(CanvasEnt);
                    if ParentEnt != CanvasEnt {
                        if let Ok((_, _, _, _, global_trans, _)) = QueryReadOnly.get(DragEnt) {
                            let phys_center = global_trans.transform_point2(Vec2::ZERO);
                            let center_logical = PhysicalToNodeRelative(phys_center, &canvas_global, &canvas_computed);
                            if center_logical.x < 0.0 || center_logical.x > CanvasSettings.Width || center_logical.y < 0.0 || center_logical.y > CanvasSettings.Height {
                                IsOutside = true;
                                CenterCanvas = center_logical;
                            }
                        }
                    }
                }

                if IsOutside {
                    Commands.entity(CanvasEnt).add_child(DragEnt);
                    
                    let mut QueryNodes = ParamSet.p0();
                    if let Ok((_, mut VNode, mut NodeComp, mut TransComp)) = QueryNodes.get_mut(DragEnt) {
                        VNode.PositionX = CenterCanvas.x - VNode.WidthPx / 2.0;
                        VNode.PositionY = CenterCanvas.y - VNode.HeightPx / 2.0;
                        
                        NodeComp.left = Val::Px(VNode.PositionX);
                        NodeComp.top = Val::Px(VNode.PositionY);
                        TransComp.rotation = Rot2::radians(-VNode.Rotation);
                    }
                }
            }
        }
        DragState.DraggedEntity = None;
        DragState.Action = None;
        DragState.LastCursorPosition = None;
        DragState.InitialPosition = None;
        DragState.InitialSize = None;
        DragState.InitialRotation = None;
        DragState.StartCursorPhysical = None;

        RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
    }

    if let Some(SelEnt) = SelectedEntity.SelectedNode {
        let QueryReadOnly = ParamSet.p1();
        if let Ok((_, Some(_VNode), _, _, global_trans, computed)) = QueryReadOnly.get(SelEnt) {
            let phys_size = computed.size();
            
            let local_tl = Vec2::new(-phys_size.x / 2.0, -phys_size.y / 2.0);
            let local_tr = Vec2::new(phys_size.x / 2.0, -phys_size.y / 2.0);
            let local_bl = Vec2::new(-phys_size.x / 2.0, phys_size.y / 2.0);
            let local_br = Vec2::new(phys_size.x / 2.0, phys_size.y / 2.0);
            let local_rot = Vec2::new(0.0, -phys_size.y / 2.0 - 40.0 * Window.scale_factor());

            let phys_tl = global_trans.transform_point2(local_tl);
            let phys_tr = global_trans.transform_point2(local_tr);
            let phys_bl = global_trans.transform_point2(local_bl);
            let phys_br = global_trans.transform_point2(local_br);
            let phys_rot = global_trans.transform_point2(local_rot);

            let PhysToWorld = |phys: Vec2| -> Vec2 {
                let logical = phys / Window.scale_factor();
                Vec2::new(logical.x - W / 2.0, H / 2.0 - logical.y)
            };

            let world_tl = PhysToWorld(phys_tl);
            let world_tr = PhysToWorld(phys_tr);
            let world_bl = PhysToWorld(phys_bl);
            let world_br = PhysToWorld(phys_br);
            let world_rot = PhysToWorld(phys_rot);

            Gizmos.line_2d(world_tl, world_tr, Color::srgb(0.0, 0.8, 1.0));
            Gizmos.line_2d(world_tr, world_br, Color::srgb(0.0, 0.8, 1.0));
            Gizmos.line_2d(world_br, world_bl, Color::srgb(0.0, 0.8, 1.0));
            Gizmos.line_2d(world_bl, world_tl, Color::srgb(0.0, 0.8, 1.0));

            let handle_size = Vec2::splat(10.0);
            Gizmos.rect_2d(Isometry2d::from_translation(world_tl), handle_size, Color::WHITE);
            Gizmos.rect_2d(Isometry2d::from_translation(world_tr), handle_size, Color::WHITE);
            Gizmos.rect_2d(Isometry2d::from_translation(world_bl), handle_size, Color::WHITE);
            Gizmos.rect_2d(Isometry2d::from_translation(world_br), handle_size, Color::WHITE);

            let top_center = (world_tl + world_tr) / 2.0;
            Gizmos.line_2d(top_center, world_rot, Color::WHITE);
            Gizmos.circle_2d(Isometry2d::from_translation(world_rot), 6.0, Color::srgb(1.0, 0.5, 0.0));
        }
    }
}

pub fn KeyboardMoveSystem(
    KeyboardInput: Res<ButtonInput<KeyCode>>,
    mut SelectedEntity: ResMut<EditorSelection>,
    mut QueryNodes: Query<(&mut VuisNode, &mut Node)>,
    mut EguiCtxs: EguiContexts,
    mut Commands: Commands,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    mut RecordEvents: MessageWriter<crate::Editor::History::RecordHistoryEvent>,
) {
    if let Ok(Ctx) = EguiCtxs.ctx_mut() {
        if Ctx.egui_wants_keyboard_input() {
            return;
        }
    }

    if KeyboardInput.just_pressed(KeyCode::Delete) {
        if let Some(SelEnt) = SelectedEntity.SelectedNode {
            if QueryCanvas.get(SelEnt).is_err() {
                if let Ok(mut EntCmds) = Commands.get_entity(SelEnt) {
                    EntCmds.despawn();
                }
                SelectedEntity.SelectedNode = None;
                RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
            }
        }
    }

    if let Some(SelEnt) = SelectedEntity.SelectedNode {
        if let Ok((mut VNode, mut NodeComp)) = QueryNodes.get_mut(SelEnt) {
            let mut Step = 1.0;
            if KeyboardInput.pressed(KeyCode::ShiftLeft) || KeyboardInput.pressed(KeyCode::ShiftRight) {
                Step = 10.0;
            }

            let mut Moved = false;

            if KeyboardInput.just_pressed(KeyCode::ArrowLeft) {
                VNode.PositionX -= Step;
                Moved = true;
            }
            if KeyboardInput.just_pressed(KeyCode::ArrowRight) {
                VNode.PositionX += Step;
                Moved = true;
            }
            if KeyboardInput.just_pressed(KeyCode::ArrowUp) {
                VNode.PositionY -= Step;
                Moved = true;
            }
            if KeyboardInput.just_pressed(KeyCode::ArrowDown) {
                VNode.PositionY += Step;
                Moved = true;
            }

            if Moved {
                NodeComp.left = Val::Px(VNode.PositionX);
                NodeComp.top = Val::Px(VNode.PositionY);
                RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
            }
        }
    }
}

pub fn KeyboardCopyPasteSystem(
    KeyboardInput: Res<ButtonInput<KeyCode>>,
    SelectedEntity: Res<EditorSelection>,
    mut CopyBuffer: ResMut<CopyPasteBuffer>,
    QueryNodes: Query<(&VuisNode, Option<&Children>)>,
    QueryText: Query<&Text, Without<PlaceholderTextComponent>>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    mut Commands: Commands,
    mut Images: ResMut<Assets<Image>>,
    mut Fonts: ResMut<Assets<Font>>,
    mut RecordEvents: MessageWriter<crate::Editor::History::RecordHistoryEvent>,
    mut EguiCtxs: EguiContexts,
) {
    if let Ok(Ctx) = EguiCtxs.ctx_mut() {
        if Ctx.egui_wants_keyboard_input() {
            return;
        }
    }

    let Ctrl = KeyboardInput.pressed(KeyCode::ControlLeft) || KeyboardInput.pressed(KeyCode::ControlRight);
    if !Ctrl {
        return;
    }

    if KeyboardInput.just_pressed(KeyCode::KeyC) {
        if let Some(selected) = SelectedEntity.SelectedNode {
            if let Some(node_data) = crate::Serialization::VuisSerializer::BuildDataTree(selected, &QueryNodes, &QueryText) {
                if let Ok(json) = serde_json::to_string(&node_data) {
                    CopyBuffer.CopiedJson = Some(json);
                }
            }
        }
    }

    if KeyboardInput.just_pressed(KeyCode::KeyV) {
        if let Some(json) = &CopyBuffer.CopiedJson {
            if let Ok(mut node_data) = serde_json::from_str::<VuisDataNode>(json) {
                let canvas_ent = if let Ok(c) = QueryCanvas.single() { c } else { return; };
                let parent = SelectedEntity.SelectedNode.unwrap_or(canvas_ent);

                node_data.PositionX += 20.0;
                node_data.PositionY += 20.0;
                node_data.Id = format!("{} Copy", node_data.Id);

                crate::Serialization::VuisSerializer::SpawnDataTree(&mut Commands, &mut Images, &mut Fonts, parent, &node_data);

                RecordEvents.write(crate::Editor::History::RecordHistoryEvent);
            }
        }
    }
}