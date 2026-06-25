use super::{
    SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner, SurfaceLifecycle,
    SurfaceLifecycleAction,
};

/// Applies planned surface lifecycle transitions to a concrete surface backend.
#[derive(Debug, Clone)]
pub struct SurfaceConfigurationController {
    lifecycle: SurfaceLifecycle,
}

impl SurfaceConfigurationController {
    /// Create a surface configuration controller.
    pub fn new(planner: SurfaceConfigPlanner) -> Self {
        Self {
            lifecycle: SurfaceLifecycle::new(planner),
        }
    }

    /// Access the underlying lifecycle state.
    pub fn lifecycle(&self) -> &SurfaceLifecycle {
        &self.lifecycle
    }

    /// Configure an initial surface size and apply the resulting config to `backend`.
    pub fn configure<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.configure(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    /// Resize a configured surface and apply reconfiguration to `backend` when needed.
    pub fn resize<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.on_resized(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    fn apply_action<B>(&self, backend: &mut B, action: SurfaceLifecycleAction)
    where
        B: SurfaceBackend,
    {
        if matches!(
            action,
            SurfaceLifecycleAction::Configure | SurfaceLifecycleAction::Reconfigure
        ) && let Some(config) = self.lifecycle.current_config()
        {
            backend.configure(config);
        }
    }
}
