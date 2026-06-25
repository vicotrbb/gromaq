use std::cell::RefCell;
use std::fs;

use gromaq::GromaqConfig;
use gromaq::app::{NativeAppConfig, NativeTerminalRuntimeConfig};
use gromaq::cli::{CliExit, NativeAppLaunchConfig};
use gromaq::renderer::RendererConfig;

use super::{
    MockAppLauncher, MockBackend, run_with_backend, run_with_backend_and_app,
    system_mono_font_path, test_cli_config_path,
};

mod check;
mod launch;
mod template;
