//! The pixel-art editor (D-012, D-015). Layered: `domain` (documents, commands, history),
//! `tools` (pointer strategies), `views` (panels drawn with px-ui), `app` (windows and
//! dispatch) and `ports` (what adapters provide). No window library, no file system.
//! Memory profile: allocation-conscious interactive application.
#![forbid(unsafe_code)]

pub mod app;
pub mod assets;
pub mod domain;
pub mod ports;
pub mod tools;
pub mod views;
