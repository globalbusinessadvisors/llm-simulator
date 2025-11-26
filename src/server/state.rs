//! Server state management

use std::sync::Arc;
use crate::config::SimulatorConfig;
use crate::engine::SimulationEngine;
use crate::telemetry::SimulatorMetrics;
use super::shutdown::ShutdownState;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<SimulationEngine>,
    pub metrics: Arc<SimulatorMetrics>,
    pub config: Arc<SimulatorConfig>,
    pub shutdown: Arc<ShutdownState>,
}

impl AppState {
    pub fn new(config: SimulatorConfig) -> Self {
        Self {
            engine: Arc::new(SimulationEngine::new(config.clone())),
            metrics: Arc::new(SimulatorMetrics::new()),
            config: Arc::new(config.clone()),
            shutdown: Arc::new(ShutdownState::new(config.server.request_timeout)),
        }
    }
}
