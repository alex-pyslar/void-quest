#pragma once
// C++ entry-point called from Rust main().
// All game logic lives in Rust; this file is Qt/OpenGL plumbing only.
#include "rust/cxx.h"

namespace vq {
struct GameApp;
void run_app(rust::Box<GameApp> game);
} // namespace vq
