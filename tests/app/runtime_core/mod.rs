use std::path::PathBuf;
use std::time::Duration;

use gromaq::app::{
    NativeAppAction, NativeAppConfig, NativeAppLifecycle, NativePtyResize,
    NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use gromaq::pty::ShellCommand;
use gromaq::{MemoryClipboard, SelectionRange};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use crate::support::{MockFrameRenderer, MockPtySession, MockPtySpawner};

mod clipboard;
mod keyboard;
mod metrics;
mod native_keyboard;
mod responses;
mod shell;
