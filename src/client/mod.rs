// client/mod.rs — Qt + OpenGL 3D client entry point.
//
// All game logic lives in Rust (app, state, net).
// The C++ layer (cpp/vqwidget.cpp) does all rendering:
//   • OpenGL 3D for the game world (tiles, entities)
//   • QPainter overlay for all HUD / menu screens

pub mod bridge;
pub mod net;
pub mod state;
pub mod app;

/// Start the Qt application.  Blocks until the window is closed.
pub fn run() {
    let game = Box::new(app::GameApp::new());
    bridge::ffi::run_app(game);
}
