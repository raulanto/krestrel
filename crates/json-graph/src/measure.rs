/// Trait for measuring text bounds without binding directly to a UI renderer.
pub trait TextMeasurer: Send + Sync {
    /// Returns the (width, height) in pixels for the given text and font size.
    fn measure_text(&self, text: &str, font_size: f32) -> (f32, f32);
}

/// Approximate text measurer using standard monospace proportions.
/// Ideal for headless layout calculations, unit tests, and SVG exports.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApproximateMeasurer {
    /// Width multiplier relative to font size (default ~0.60 for monospace fonts).
    pub char_width_ratio: f32,
    /// Line height multiplier relative to font size (default ~1.35).
    pub line_height_ratio: f32,
}

impl ApproximateMeasurer {
    pub fn new() -> Self {
        Self {
            char_width_ratio: 0.60,
            line_height_ratio: 1.35,
        }
    }
}

impl TextMeasurer for ApproximateMeasurer {
    fn measure_text(&self, text: &str, font_size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, font_size * self.line_height_ratio);
        }

        let max_line_len = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        let num_lines = text.lines().count().max(1);

        let width = max_line_len as f32 * (font_size * self.char_width_ratio);
        let height = num_lines as f32 * (font_size * self.line_height_ratio);

        (width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approximate_measurer() {
        let measurer = ApproximateMeasurer::new();
        let (w, h) = measurer.measure_text("hello", 14.0);
        assert!(w > 0.0);
        assert!(h > 0.0);
        assert_eq!(w, 5.0 * (14.0 * 0.60));
    }
}
