#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextAnimationDefaults {
    pub(crate) glyph_delay_ms: i32,
    pub(crate) reveal_steps: i32,
    pub(crate) reveal_step_delay_ms: i32,
    pub(crate) settle_steps: i32,
    pub(crate) settle_step_delay_ms: i32,
    pub(crate) auto_advance_enabled: bool,
    pub(crate) auto_advance_delay_ms: i32,
}

impl Default for TextAnimationDefaults {
    fn default() -> Self {
        Self {
            glyph_delay_ms: 16,
            reveal_steps: 8,
            reveal_step_delay_ms: 50,
            settle_steps: 1,
            settle_step_delay_ms: 0,
            auto_advance_enabled: false,
            auto_advance_delay_ms: 0,
        }
    }
}

impl TextAnimationDefaults {
    pub(crate) fn set_glyph_delay(&mut self, delay_ms: i32) {
        self.glyph_delay_ms = delay_ms.max(0);
    }

    pub(crate) fn set_reveal(&mut self, steps: i32, step_delay_ms: i32) {
        if steps > 0 {
            self.reveal_steps = steps;
            self.reveal_step_delay_ms = step_delay_ms;
        }
    }

    pub(crate) fn set_settle(&mut self, steps: i32, step_delay_ms: i32) {
        if steps > 0 {
            self.settle_steps = steps;
            self.settle_step_delay_ms = step_delay_ms;
        }
    }

    pub(crate) fn set_auto_advance(&mut self, enabled: i32, delay_ms: i32) {
        self.auto_advance_enabled = enabled != 0;
        self.auto_advance_delay_ms = delay_ms;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextLayoutDefaults {
    /// Target dword_507638. Used as the native horizontal/layout advance.
    pub(crate) layout_advance: i32,
    /// Target dword_50763C. A zero script value is normalized to one.
    pub(crate) scale_denominator: i32,
    /// Target dword_565BB0, added by sub_436FF0.
    pub(crate) character_spacing: i32,
    /// Target dword_507640, validated to 25..=100.
    pub(crate) font_percent: i32,
    /// Target dword_565BDC, used by 91:8E's exact cursor boundary test.
    pub(crate) boundary_offset: i32,
    /// Target dword_565CF0.
    pub(crate) mode_flag: i32,
    /// Target dword_5076B0 configured by 91:99.
    pub(crate) scale_reciprocal_16_16: i32,
}

impl Default for TextLayoutDefaults {
    fn default() -> Self {
        Self {
            layout_advance: 0,
            scale_denominator: 1,
            character_spacing: 0,
            font_percent: 100,
            boundary_offset: 0,
            mode_flag: 0,
            scale_reciprocal_16_16: 0,
        }
    }
}

impl TextLayoutDefaults {
    pub(crate) fn configure(&mut self, args: [i32; 6]) -> Result<(), TextLayoutError> {
        let [
            layout_advance,
            scale_denominator,
            character_spacing,
            font_percent,
            boundary_offset,
            mode_flag,
        ] = args;
        if !(25..=100).contains(&font_percent) {
            return Err(TextLayoutError::FontPercent(font_percent));
        }
        if boundary_offset < 0 {
            return Err(TextLayoutError::BoundaryOffset(boundary_offset));
        }
        self.layout_advance = layout_advance;
        self.scale_denominator = scale_denominator.max(1);
        self.character_spacing = character_spacing;
        self.font_percent = font_percent;
        self.boundary_offset = boundary_offset;
        self.mode_flag = mode_flag;
        Ok(())
    }

    pub(crate) fn set_scale_divisor(&mut self, divisor: i32) -> bool {
        if divisor == 0 {
            return false;
        }
        // sub_407B60 divides unsigned.
        self.scale_reciprocal_16_16 =
            ((divisor as u32).wrapping_add(0xffff) / divisor as u32) as i32;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextLayoutError {
    FontPercent(i32),
    BoundaryOffset(i32),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TextStyleDefaults {
    pub(crate) font_name: Option<String>,
    /// Target dword_565CE0. Nonpositive values select the 91:98 percentage.
    pub(crate) ruby_height: i32,
    /// Target dword_565CE4. Its remaining consumers are not yet named.
    pub(crate) ruby_font_field: i32,
    /// Target dword_565CE8.
    pub(crate) ruby_x_offset: i32,
    /// Target dword_565CEC.
    pub(crate) ruby_y_offset: i32,
    /// Target dword_507644.
    pub(crate) override_0: i32,
    /// Target dword_507648.
    pub(crate) override_1: i32,
}

impl TextStyleDefaults {
    pub(crate) fn set_fields(&mut self, values: [i32; 6]) {
        self.ruby_height = values[0];
        self.ruby_font_field = values[1];
        self.ruby_x_offset = values[2];
        self.ruby_y_offset = values[3];
        self.override_0 = values[4];
        self.override_1 = values[5];
    }
}
