use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EditorState {
    #[default]
    Editing,
    Preview,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, AppBuilder: &mut App) {
        AppBuilder.init_state::<EditorState>();
    }
}