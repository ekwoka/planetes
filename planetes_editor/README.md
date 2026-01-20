# Planetes Editor

An experimental editor for Bevy game development.

The Planetes Editor is designed as a plugin to add to your own Bevy application, allowing you to author Bevy Scenes. It allows for adding, modifying, and removing entities, and components in a bespoke manner, with UI similar to visual editors like Blender.

## Usage

It's recommended to use a feature in your application to build with the editor vs as a normal game, or make a separate `bin` entry point that adds the editor to your application.

```rs
#[cfg(feature = "editor")]
app.add_plugins(planetes_editor::plugin);
```

You may also want to not load some systems or plugins when the editor is enabled, such as physics or audio systems, or ensure that they are disabled when the editor is active.

Currently, `planetes_editor` has a feature for `avian` support which will disable the physics tick when the editor is active.

The idea is that the editor enables you to have a specific built editor for your game, that you could even distribute as a part of modding SDK. In this way it easily has access to all of the components and behaviors that exist in your real game.

## Running Sample

To test the editor, you can run the following command in the root:

```sh
cargo run --features editor
```
