use crate::font_data::ContourPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Knife,
    Hand,
    Ruler,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "選択",
            Tool::Pen => "ペン",
            Tool::Knife => "ナイフ",
            Tool::Hand => "ハンド",
            Tool::Ruler => "定規",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tool::Select => "◇",
            Tool::Pen => "✒",
            Tool::Knife => "✂",
            Tool::Hand => "✋",
            Tool::Ruler => "📏",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Pen => "P",
            Tool::Knife => "K",
            Tool::Hand => "H",
            Tool::Ruler => "R",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PenState {
    pub preview_points: Vec<ContourPoint>,
    pub is_drawing: bool,
    pub drag_preview: Option<((f64, f64), (f64, f64))>,
}

impl PenState {
    pub fn new() -> Self {
        Self {
            preview_points: Vec::new(),
            is_drawing: false,
            drag_preview: None,
        }
    }

    pub fn start_path(&mut self, x: f64, y: f64) {
        self.preview_points.clear();
        self.preview_points.push(ContourPoint::on_curve(x, y));
        self.is_drawing = true;
        self.drag_preview = None;
    }

    pub fn add_point(&mut self, x: f64, y: f64, is_off_curve: bool) {
        self.drag_preview = None;
        if is_off_curve {
            self.preview_points.push(ContourPoint::off_curve(x, y));
        } else {
            self.preview_points.push(ContourPoint::on_curve(x, y));
        }
    }

    /// Adds an anchor created by a click-drag. The drag endpoint becomes the
    /// outgoing handle; for subsequent anchors its reflected point is the
    /// incoming handle, matching the usual pen-tool cubic workflow.
    pub fn add_dragged_anchor(&mut self, x: f64, y: f64, handle_x: f64, handle_y: f64) {
        self.drag_preview = None;
        if !self.is_drawing {
            self.start_path(x, y);
        } else {
            self.preview_points.push(ContourPoint::off_curve(
                2.0 * x - handle_x,
                2.0 * y - handle_y,
            ));
            self.preview_points.push(ContourPoint::on_curve(x, y));
        }
        if (handle_x - x).abs() > f64::EPSILON || (handle_y - y).abs() > f64::EPSILON {
            self.preview_points
                .push(ContourPoint::off_curve(handle_x, handle_y));
        }
        self.is_drawing = true;
    }

    pub fn begin_drag(&mut self, x: f64, y: f64) {
        self.drag_preview = Some(((x, y), (x, y)));
    }

    pub fn update_drag(&mut self, x: f64, y: f64) {
        if let Some((anchor, _)) = self.drag_preview {
            self.drag_preview = Some((anchor, (x, y)));
        }
    }

    pub fn finish_path(&mut self) -> Option<Vec<ContourPoint>> {
        if self.preview_points.len() > 1 {
            let points = self.preview_points.clone();
            self.preview_points.clear();
            self.is_drawing = false;
            self.drag_preview = None;
            Some(points)
        } else {
            self.preview_points.clear();
            self.is_drawing = false;
            self.drag_preview = None;
            None
        }
    }

    pub fn cancel(&mut self) {
        self.preview_points.clear();
        self.is_drawing = false;
        self.drag_preview = None;
    }
}

impl Default for PenState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragged_pen_anchors_create_reflected_incoming_handles() {
        let mut pen = PenState::new();
        pen.add_dragged_anchor(0.0, 0.0, 30.0, 0.0);
        pen.add_dragged_anchor(100.0, 0.0, 130.0, 0.0);
        assert_eq!(pen.preview_points.len(), 5);
        assert_eq!(pen.preview_points[0], ContourPoint::on_curve(0.0, 0.0));
        assert_eq!(pen.preview_points[1], ContourPoint::off_curve(30.0, 0.0));
        assert_eq!(pen.preview_points[2], ContourPoint::off_curve(70.0, 0.0));
        assert_eq!(pen.preview_points[3], ContourPoint::on_curve(100.0, 0.0));
        assert_eq!(pen.preview_points[4], ContourPoint::off_curve(130.0, 0.0));
    }

    #[test]
    fn drag_preview_tracks_anchor_and_handle_until_commit() {
        let mut pen = PenState::new();
        pen.begin_drag(10.0, 20.0);
        pen.update_drag(40.0, 60.0);
        assert_eq!(pen.drag_preview, Some(((10.0, 20.0), (40.0, 60.0))));
        pen.add_dragged_anchor(10.0, 20.0, 40.0, 60.0);
        assert!(pen.drag_preview.is_none());
    }
}
