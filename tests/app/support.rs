use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gromaq::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use gromaq::dirty::DirtyRegion;
use gromaq::pty::{PtyConfig, PtyError};
use gromaq::renderer::{
    GpuRenderer, SurfaceBackend, SurfaceFrameBackend, SurfaceFrameError, SurfaceGlyphFrame,
};
use gromaq::{CursorSnapshot, GridSnapshot, GromaqError};

#[derive(Debug, Default)]
pub(crate) struct MockPtySession {
    pub(crate) output: RefCell<VecDeque<Vec<u8>>>,
    pub(crate) input: RefCell<Vec<Vec<u8>>>,
    pub(crate) resizes: RefCell<Vec<NativePtyResize>>,
}

impl NativePtySessionIo for MockPtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.borrow_mut().pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.input.borrow_mut().push(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError> {
        self.resizes.borrow_mut().push(size);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct MockPtySpawner {
    pub(crate) configs: RefCell<Vec<PtyConfig>>,
}

impl NativePtySpawner for MockPtySpawner {
    type Session = MockPtySession;

    fn spawn(&self, config: PtyConfig) -> Result<Self::Session, PtyError> {
        self.configs.borrow_mut().push(config);
        let session = MockPtySession::default();
        session.output.borrow_mut().push_back(b"hello\r\n".to_vec());
        Ok(session)
    }
}

#[derive(Debug, Default)]
pub(crate) struct MockFrameRenderer {
    pub(crate) frames: Vec<RenderedFrame>,
    pub(crate) render_delay: Duration,
    pub(crate) render_error: Option<GromaqError>,
}

#[derive(Debug)]
pub(crate) struct RenderedFrame {
    pub(crate) first_line: String,
    pub(crate) cursor: CursorSnapshot,
    pub(crate) dirty_regions: Vec<DirtyRegion>,
}

impl GpuRenderer for MockFrameRenderer {
    fn render_frame(
        &mut self,
        grid: &GridSnapshot,
        cursor: CursorSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> gromaq::Result<()> {
        if let Some(error) = self.render_error.take() {
            return Err(error);
        }
        if !self.render_delay.is_zero() {
            std::thread::sleep(self.render_delay);
        }
        self.frames.push(RenderedFrame {
            first_line: grid.line_text(0),
            cursor,
            dirty_regions: dirty_regions.to_vec(),
        });
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct MockSurfaceBackend {
    pub(crate) configured_sizes: RefCell<Vec<(u32, u32)>>,
    pub(crate) presented_clear_colors: RefCell<Vec<[f64; 4]>>,
    pub(crate) presented_glyph_frames: RefCell<Vec<PresentedGlyphFrame>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentedGlyphFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) quads: usize,
    pub(crate) atlas_pixels: usize,
}

impl SurfaceBackend for MockSurfaceBackend {
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.configured_sizes
            .borrow_mut()
            .push((config.width, config.height));
    }
}

impl SurfaceFrameBackend for MockSurfaceBackend {
    fn clear_and_present(&mut self, clear_color: [f64; 4]) -> Result<(), SurfaceFrameError> {
        self.presented_clear_colors.borrow_mut().push(clear_color);
        Ok(())
    }

    fn present_glyph_frame(
        &mut self,
        frame: SurfaceGlyphFrame<'_>,
    ) -> Result<(), SurfaceFrameError> {
        self.presented_glyph_frames
            .borrow_mut()
            .push(PresentedGlyphFrame {
                width: frame.width,
                height: frame.height,
                quads: frame.batch.quads.len(),
                atlas_pixels: frame.atlas.rgba.len() / 4,
            });
        Ok(())
    }
}

pub(crate) fn test_app_config_path(name: &str) -> PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-app-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

pub(crate) fn supported_surface_capabilities() -> wgpu::SurfaceCapabilities {
    wgpu::SurfaceCapabilities {
        formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
        present_modes: vec![wgpu::PresentMode::Fifo],
        alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
        usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
    }
}

pub(crate) fn system_mono_font() -> Vec<u8> {
    let candidates = [
        PathBuf::from("/System/Library/Fonts/SFNSMono.ttf"),
        PathBuf::from("/System/Library/Fonts/Menlo.ttc"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Courier New.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"),
        PathBuf::from("/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf"),
        PathBuf::from("/usr/share/fonts/liberation/LiberationMono-Regular.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf"),
    ];
    let path = candidates
        .into_iter()
        .find(|path| path.exists())
        .expect("system monospace test font is available");
    std::fs::read(path).expect("test font can be read")
}
