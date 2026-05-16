#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background,
    Nebula,
    Lanes,
    SensorOverlay,
    Bodies,
    Fleets,
    Selection,
    Labels,
    Cursor,
    Tooltip,
}

impl RenderLayer {
    pub const fn z_base(self) -> u16 {
        match self {
            RenderLayer::Background => 0,
            RenderLayer::Nebula => 20,
            RenderLayer::Lanes => 40,
            RenderLayer::SensorOverlay => 60,
            RenderLayer::Bodies => 80,
            RenderLayer::Fleets => 100,
            RenderLayer::Selection => 120,
            RenderLayer::Labels => 140,
            RenderLayer::Cursor => 160,
            RenderLayer::Tooltip => 180,
        }
    }
}
