use bevy::prelude::*;
use crate::Components::VuisElement::{EditorCanvas, PlaceholderTextComponent};
use crate::Serialization::VuisFormat::{VuisFile, VuisDataNode};
use crate::Serialization::VuisSerializer::{BuildDataTree, SpawnDataTree};
use crate::Editor::EditorPlugin::EditorSelection;

#[derive(Resource, Default)]
pub struct UndoRedoHistory {
    pub UndoStack: Vec<String>,
    pub RedoStack: Vec<String>,
}

#[derive(Message)]
pub struct RecordHistoryEvent;

#[derive(Message)]
pub struct UndoEvent;

#[derive(Message)]
pub struct RedoEvent;

pub fn RecordHistorySystem(
    mut Events: MessageReader<RecordHistoryEvent>,
    mut History: ResMut<UndoRedoHistory>,
    QueryNodes: Query<(&crate::Components::VuisElement::VuisNode, Option<&Children>)>,
    QueryText: Query<&Text, Without<PlaceholderTextComponent>>,
    QueryCanvas: Query<&Children, With<EditorCanvas>>,
) {
    for _ in Events.read() {
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
                    LayoutFlow: Some("None".to_string()),
                    IsScrollable: Some(false),
                    ScrollbarWidth: Some(8.0),
                    ScrollbarColorRgba: Some([0.5, 0.5, 0.5, 0.8]),
                    ScrollbarTrackColorRgba: Some([0.0, 0.0, 0.0, 0.2]),
                    ScrollbarBorderRadius: Some(4.0),
                    Children: RootChildren,
                },
            };
            if let Ok(JsonString) = serde_json::to_string(&FileData) {
                if History.UndoStack.last() != Some(&JsonString) {
                    History.UndoStack.push(JsonString);
                    History.RedoStack.clear();
                }
            }
        }
    }
}

pub fn UndoRedoSystem(
    mut UndoEvents: MessageReader<UndoEvent>,
    mut RedoEvents: MessageReader<RedoEvent>,
    mut History: ResMut<UndoRedoHistory>,
    mut Commands: Commands,
    mut Images: ResMut<Assets<Image>>,
    mut Fonts: ResMut<Assets<Font>>,
    QueryCanvas: Query<Entity, With<EditorCanvas>>,
    QueryCanvasChildren: Query<&Children, With<EditorCanvas>>,
    mut SelectedEntity: ResMut<EditorSelection>,
) {
    let mut StateToRestore = None;

    for _ in UndoEvents.read() {
        if History.UndoStack.len() > 1 {
            let Current = History.UndoStack.pop().unwrap();
            History.RedoStack.push(Current);
            StateToRestore = Some(History.UndoStack.last().unwrap().clone());
        }
    }

    for _ in RedoEvents.read() {
        if let Some(RedoState) = History.RedoStack.pop() {
            History.UndoStack.push(RedoState.clone());
            StateToRestore = Some(RedoState);
        }
    }

    if let Some(JsonString) = StateToRestore {
        if let Ok(FileData) = serde_json::from_str::<VuisFile>(&JsonString) {
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
            }
        }
    }
}

pub fn InitializeHistorySystem(
    History: Res<UndoRedoHistory>,
    mut RecordEvents: MessageWriter<RecordHistoryEvent>,
) {
    if History.UndoStack.is_empty() {
        RecordEvents.write(RecordHistoryEvent);
    }
}

pub fn KeyboardHistorySystem(
    KeyboardInput: Res<ButtonInput<KeyCode>>,
    mut UndoEvents: MessageWriter<UndoEvent>,
    mut RedoEvents: MessageWriter<RedoEvent>,
    mut EguiCtxs: bevy_egui::EguiContexts,
) {
    if let Ok(Ctx) = EguiCtxs.ctx_mut() {
        if Ctx.egui_wants_keyboard_input() {
            return;
        }
    }

    let Ctrl = KeyboardInput.pressed(KeyCode::ControlLeft) || KeyboardInput.pressed(KeyCode::ControlRight);
    if Ctrl {
        if KeyboardInput.just_pressed(KeyCode::KeyZ) {
            UndoEvents.write(UndoEvent);
        } else if KeyboardInput.just_pressed(KeyCode::KeyY) {
            RedoEvents.write(RedoEvent);
        }
    }
}