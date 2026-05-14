use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

impl WorldPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPointF {
    pub x: f64,
    pub y: f64,
}

impl ScreenPointF {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: u16,
    pub y: u16,
}

impl ScreenPoint {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub center: WorldPoint,
    pub zoom: f64,
    pub viewport_width: u16,
    pub viewport_height: u16,
}

impl Camera {
    pub fn new(center: WorldPoint, zoom: f64, viewport_width: u16, viewport_height: u16) -> Self {
        Self {
            center,
            zoom: sanitize_zoom(zoom),
            viewport_width,
            viewport_height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportBounds {
    pub min: WorldPoint,
    pub max: WorldPoint,
}

impl ViewportBounds {
    pub fn from_corners(min: WorldPoint, max: WorldPoint) -> Self {
        let min_x = min.x.min(max.x);
        let max_x = min.x.max(max.x);
        let min_y = min.y.min(max.y);
        let max_y = min.y.max(max.y);

        let mut bounds = Self {
            min: WorldPoint::new(min_x, min_y),
            max: WorldPoint::new(max_x, max_y),
        };
        bounds.expand_degenerate_axes();
        bounds
    }

    pub fn from_min_max(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self::from_corners(WorldPoint::new(min_x, min_y), WorldPoint::new(max_x, max_y))
    }

    pub fn from_points(points: &[WorldPoint], padding: f64) -> Option<Self> {
        let first = points.first().copied()?;
        let mut min_x = first.x;
        let mut max_x = first.x;
        let mut min_y = first.y;
        let mut max_y = first.y;

        for point in points.iter().copied().skip(1) {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        Some(Self::from_min_max(
            min_x - padding,
            min_y - padding,
            max_x + padding,
            max_y + padding,
        ))
    }

    pub fn center(&self) -> WorldPoint {
        WorldPoint::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    fn expand_degenerate_axes(&mut self) {
        if (self.max.x - self.min.x).abs() < f64::EPSILON {
            self.min.x -= 1.0;
            self.max.x += 1.0;
        }
        if (self.max.y - self.min.y).abs() < f64::EPSILON {
            self.min.y -= 1.0;
            self.max.y += 1.0;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapViewport {
    pub bounds: ViewportBounds,
    pub camera: Camera,
}

impl MapViewport {
    pub fn new(bounds: ViewportBounds, camera: Camera) -> Self {
        Self { bounds, camera }
    }

    pub fn fit_bounds(bounds: ViewportBounds, viewport_width: u16, viewport_height: u16) -> Self {
        Self::new(
            bounds,
            Camera::new(bounds.center(), 1.0, viewport_width, viewport_height),
        )
    }

    pub fn fit_points(
        points: &[WorldPoint],
        viewport_width: u16,
        viewport_height: u16,
        padding: f64,
    ) -> Option<Self> {
        let bounds = ViewportBounds::from_points(points, padding)?;
        Some(Self::fit_bounds(bounds, viewport_width, viewport_height))
    }

    pub fn viewport_rect(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.camera.viewport_width,
            self.camera.viewport_height,
        )
    }

    pub fn visible_bounds(&self) -> ViewportBounds {
        let width = self.bounds.width() / self.camera.zoom;
        let height = self.bounds.height() / self.camera.zoom;
        ViewportBounds::from_min_max(
            self.camera.center.x - width * 0.5,
            self.camera.center.y - height * 0.5,
            self.camera.center.x + width * 0.5,
            self.camera.center.y + height * 0.5,
        )
    }

    pub fn world_to_screen_f(&self, point: WorldPoint) -> Option<ScreenPointF> {
        self.clip_screen_point_f(self.world_to_screen_unclipped(point)?)
    }

    pub fn world_to_screen_cell(&self, point: WorldPoint) -> Option<ScreenPoint> {
        let point = self.world_to_screen_f(point)?;
        Some(self.screen_point_f_to_cell(point))
    }

    pub fn screen_to_world(&self, point: ScreenPointF) -> WorldPoint {
        let bounds = self.visible_bounds();
        let rel_x = if self.camera.viewport_width <= 1 {
            0.5
        } else {
            point.x / f64::from(self.camera.viewport_width - 1)
        };
        let rel_y = if self.camera.viewport_height <= 1 {
            0.5
        } else {
            point.y / f64::from(self.camera.viewport_height - 1)
        };
        WorldPoint::new(
            bounds.min.x + bounds.width() * rel_x,
            bounds.min.y + bounds.height() * rel_y,
        )
    }

    pub fn clip_screen_point_f(&self, point: ScreenPointF) -> Option<ScreenPointF> {
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;
        if width == 0 || height == 0 {
            return None;
        }

        let max_x = f64::from(width.saturating_sub(1));
        let max_y = f64::from(height.saturating_sub(1));
        if point.x < 0.0 || point.y < 0.0 || point.x > max_x || point.y > max_y {
            None
        } else {
            Some(point)
        }
    }

    pub fn clip_screen_line(
        &self,
        start: ScreenPointF,
        end: ScreenPointF,
    ) -> Option<(ScreenPointF, ScreenPointF)> {
        if self.camera.viewport_width == 0 || self.camera.viewport_height == 0 {
            return None;
        }

        let min_x = 0.0;
        let min_y = 0.0;
        let max_x = f64::from(self.camera.viewport_width.saturating_sub(1));
        let max_y = f64::from(self.camera.viewport_height.saturating_sub(1));

        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let mut t0 = 0.0;
        let mut t1 = 1.0;

        for (p, q) in [
            (-dx, start.x - min_x),
            (dx, max_x - start.x),
            (-dy, start.y - min_y),
            (dy, max_y - start.y),
        ] {
            if p.abs() < f64::EPSILON {
                if q < 0.0 {
                    return None;
                }
                continue;
            }

            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }

            if t0 > t1 {
                return None;
            }
        }

        Some((
            ScreenPointF::new(start.x + dx * t0, start.y + dy * t0),
            ScreenPointF::new(start.x + dx * t1, start.y + dy * t1),
        ))
    }

    pub fn rasterize_screen_line(
        &self,
        start: ScreenPointF,
        end: ScreenPointF,
    ) -> Vec<ScreenPoint> {
        let Some((start, end)) = self.clip_screen_line(start, end) else {
            return Vec::new();
        };

        let steps = ((end.x - start.x).abs().max((end.y - start.y).abs()))
            .ceil()
            .max(1.0) as usize;
        let mut cells = Vec::with_capacity(steps + 1);

        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let point = ScreenPointF::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            );
            let cell = self.screen_point_f_to_cell(point);
            if cells.last().copied() != Some(cell) {
                cells.push(cell);
            }
        }

        cells
    }

    pub fn world_line_to_cells(&self, start: WorldPoint, end: WorldPoint) -> Vec<ScreenPoint> {
        let Some(start) = self.world_to_screen_unclipped(start) else {
            return Vec::new();
        };
        let Some(end) = self.world_to_screen_unclipped(end) else {
            return Vec::new();
        };
        self.rasterize_screen_line(start, end)
    }

    fn world_to_screen_unclipped(&self, point: WorldPoint) -> Option<ScreenPointF> {
        if self.camera.viewport_width == 0 || self.camera.viewport_height == 0 {
            return None;
        }

        let visible = self.visible_bounds();
        let rel_x = (point.x - visible.min.x) / visible.width();
        let rel_y = (point.y - visible.min.y) / visible.height();
        Some(ScreenPointF::new(
            rel_x * f64::from(self.camera.viewport_width.saturating_sub(1)),
            rel_y * f64::from(self.camera.viewport_height.saturating_sub(1)),
        ))
    }

    fn screen_point_f_to_cell(&self, point: ScreenPointF) -> ScreenPoint {
        ScreenPoint::new(
            point
                .x
                .round()
                .clamp(0.0, f64::from(self.camera.viewport_width.saturating_sub(1)))
                as u16,
            point.y.round().clamp(
                0.0,
                f64::from(self.camera.viewport_height.saturating_sub(1)),
            ) as u16,
        )
    }
}

fn sanitize_zoom(zoom: f64) -> f64 {
    if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_project(
        bounds: ViewportBounds,
        width: u16,
        height: u16,
        point: WorldPoint,
    ) -> Option<ScreenPoint> {
        if width == 0 || height == 0 {
            return None;
        }
        let rel_x = (point.x - bounds.min.x) / bounds.width();
        let rel_y = (point.y - bounds.min.y) / bounds.height();
        if !(0.0..=1.0).contains(&rel_x) || !(0.0..=1.0).contains(&rel_y) {
            return None;
        }
        Some(ScreenPoint::new(
            (rel_x * f64::from(width.saturating_sub(1))).round() as u16,
            (rel_y * f64::from(height.saturating_sub(1))).round() as u16,
        ))
    }

    #[test]
    fn projection_preserves_known_overview_mapping() {
        let bounds = ViewportBounds::from_min_max(-560.0, -560.0, 560.0, 560.0);
        let viewport = MapViewport::fit_bounds(bounds, 20, 10);
        let point = WorldPoint::new(0.0, 0.0);
        assert_eq!(
            viewport.world_to_screen_cell(point),
            legacy_project(bounds, 20, 10, point)
        );
    }

    #[test]
    fn projection_preserves_known_sector_mapping() {
        let points = [
            WorldPoint::new(10.0, 10.0),
            WorldPoint::new(110.0, 90.0),
            WorldPoint::new(70.0, 50.0),
        ];
        let viewport = MapViewport::fit_points(&points, 40, 12, 40.0).unwrap();
        let point = points[2];
        assert_eq!(
            viewport.world_to_screen_cell(point),
            legacy_project(viewport.bounds, 40, 12, point)
        );
    }

    #[test]
    fn pan_changes_projected_position() {
        let bounds = ViewportBounds::from_min_max(-100.0, -100.0, 100.0, 100.0);
        let base = MapViewport::fit_bounds(bounds, 21, 11);
        let shifted =
            MapViewport::new(bounds, Camera::new(WorldPoint::new(25.0, 0.0), 1.0, 21, 11));
        let point = WorldPoint::new(0.0, 0.0);
        assert!(
            shifted.world_to_screen_f(point).unwrap().x < base.world_to_screen_f(point).unwrap().x
        );
    }

    #[test]
    fn zoom_changes_projected_position() {
        let bounds = ViewportBounds::from_min_max(-100.0, -100.0, 100.0, 100.0);
        let base = MapViewport::fit_bounds(bounds, 21, 11);
        let zoomed = MapViewport::new(bounds, Camera::new(bounds.center(), 2.0, 21, 11));
        let point = WorldPoint::new(50.0, 0.0);
        assert!(
            zoomed.world_to_screen_f(point).unwrap().x > base.world_to_screen_f(point).unwrap().x
        );
    }

    #[test]
    fn screen_to_world_approximately_reverses_projection() {
        let bounds = ViewportBounds::from_min_max(-100.0, -60.0, 100.0, 60.0);
        let viewport = MapViewport::fit_bounds(bounds, 41, 21);
        let world = WorldPoint::new(25.0, -15.0);
        let screen = viewport.world_to_screen_f(world).unwrap();
        let round_trip = viewport.screen_to_world(screen);
        assert!((round_trip.x - world.x).abs() < 1e-6);
        assert!((round_trip.y - world.y).abs() < 1e-6);
    }

    #[test]
    fn out_of_bounds_points_return_none() {
        let bounds = ViewportBounds::from_min_max(-10.0, -10.0, 10.0, 10.0);
        let viewport = MapViewport::fit_bounds(bounds, 20, 10);
        assert_eq!(viewport.world_to_screen_f(WorldPoint::new(20.0, 0.0)), None);
    }

    #[test]
    fn small_viewports_do_not_panic() {
        let bounds = ViewportBounds::from_min_max(-10.0, -10.0, 10.0, 10.0);
        let viewport = MapViewport::fit_bounds(bounds, 1, 1);
        assert_eq!(
            viewport.world_to_screen_cell(WorldPoint::new(0.0, 0.0)),
            Some(ScreenPoint::new(0, 0))
        );
    }

    #[test]
    fn line_drawing_uses_projected_sub_cell_positions() {
        let bounds = ViewportBounds::from_min_max(0.0, 0.0, 10.0, 10.0);
        let viewport = MapViewport::fit_bounds(bounds, 11, 11);
        let cells =
            viewport.world_line_to_cells(WorldPoint::new(0.2, 0.2), WorldPoint::new(9.8, 0.2));
        assert!(cells.len() >= 10);
        assert_eq!(cells.first().copied(), Some(ScreenPoint::new(0, 0)));
        assert_eq!(cells.last().copied(), Some(ScreenPoint::new(10, 0)));
    }

    #[test]
    fn clipped_line_returns_visible_segment() {
        let bounds = ViewportBounds::from_min_max(-10.0, -10.0, 10.0, 10.0);
        let viewport = MapViewport::fit_bounds(bounds, 11, 11);
        let cells =
            viewport.world_line_to_cells(WorldPoint::new(-20.0, 0.0), WorldPoint::new(5.0, 0.0));
        assert!(!cells.is_empty());
        assert_eq!(cells.first().copied(), Some(ScreenPoint::new(0, 5)));
    }
}
