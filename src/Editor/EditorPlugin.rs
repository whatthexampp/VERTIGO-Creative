use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;
use crate::Editor::Canvas::{SetupCanvas, ScaleCanvasSystem, GridLayoutUpdateSystem, GridLayoutApplySystem, SyncNodeGridLinesSystem};
use crate::Editor::EguiPanels::{EditorUiSystem, SelectionHighlightSystem, AnimationSystem};
use crate::Editor::Selection::{SelectionAndDragSystem, KeyboardMoveSystem, KeyboardCopyPasteSystem, EditorDragState, CopyPasteBuffer};
use crate::Editor::History::{
    UndoRedoHistory, RecordHistoryEvent, UndoEvent, RedoEvent,
    RecordHistorySystem, UndoRedoSystem, KeyboardHistorySystem, InitializeHistorySystem
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorSet {
    Ui,
    Selection,
}

#[derive(Resource, Default)]
pub struct EditorSelection {
    pub SelectedNode: Option<Entity>,
    pub IsPointerOverUi: bool,
    pub DraggedHierarchyEntity: Option<Entity>,
}

#[derive(Resource)]
pub struct EditorConfig {
    pub SnappingEnabled: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            SnappingEnabled: true,
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, AppBuilder: &mut App) {
        AppBuilder.init_resource::<EditorSelection>();
        AppBuilder.init_resource::<EditorConfig>();
        AppBuilder.init_resource::<EditorDragState>();
        AppBuilder.init_resource::<CopyPasteBuffer>();
        AppBuilder.init_resource::<UndoRedoHistory>();
        AppBuilder.add_message::<RecordHistoryEvent>();
        AppBuilder.add_message::<UndoEvent>();
        AppBuilder.add_message::<RedoEvent>();
        AppBuilder.add_systems(Startup, SetupCanvas);
        AppBuilder.configure_sets(EguiPrimaryContextPass, EditorSet::Selection.after(EditorSet::Ui));
        AppBuilder.add_systems(EguiPrimaryContextPass, (
            EditorUiSystem.in_set(EditorSet::Ui),
            SelectionAndDragSystem.in_set(EditorSet::Selection),
        ));
        AppBuilder.add_systems(Update, (
            ScaleCanvasSystem, 
            SelectionHighlightSystem, 
            AnimationSystem,
            KeyboardMoveSystem,
            KeyboardCopyPasteSystem,
            crate::Editor::EguiPanels::PlaceholderUpdateSystem,
            crate::Editor::EguiPanels::TextStylingUpdateSystem,
            InitializeHistorySystem,
            RecordHistorySystem,
            UndoRedoSystem,
            KeyboardHistorySystem,
            GridLayoutUpdateSystem,
            GridLayoutApplySystem,
            SyncNodeGridLinesSystem,
        ));
    }
}