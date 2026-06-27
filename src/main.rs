mod Core;
mod Components;
mod Serialization;
mod Editor;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiUserTextures};
use Core::State::StatePlugin;
use Editor::EditorPlugin::EditorPlugin;
use Serialization::VuisSerializer::SerializationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .init_resource::<EguiUserTextures>()
        .add_plugins(StatePlugin)
        .add_plugins(EditorPlugin)
        .add_plugins(SerializationPlugin)
        .run();
}