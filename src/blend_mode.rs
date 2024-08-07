/// Blend modes used in a Photoshop document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlendMode {
    /// Pass-through blend mode.
    PassThrough,
    /// Normal blend mode.
    Normal,
    /// Dissolve blend mode.
    Dissolve,
    /// Darken blend mode.
    Darken,
    /// Multiply blend mode.
    Multiply,
    /// Colour burn blend mode.
    ColorBurn,
    /// Linear burn blend mode.
    LinearBurn,
    /// Dark colour blend mode.
    DarkerColor,
    /// Lighten blend mode.
    Lighten,
    /// Screen blend mode.
    Screen,
    /// Colour dodge blend mode.
    ColorDodge,
    /// Linear dodge blend mode.
    LinearDodge,
    /// Lighter colour blend mode.
    LighterColor,
    /// Overlay blend mode.
    Overlay,
    /// Soft light blend mode.
    SoftLight,
    /// Hard light blend mode.
    HardLight,
    /// Vivid light blend mode.
    VividLight,
    /// Linear light blend mode.
    LinearLight,
    /// Pin light blend mode.
    PinLight,
    /// Hard mix blend mode.
    HardMix,
    /// Difference blend mode.
    Difference,
    /// Exclusion blend mode.
    Exclusion,
    /// Subtract blend mode.
    Subtract,
    /// Divide blend mode.
    Divide,
    /// Hue blend mode.
    Hue,
    /// Saturation blend mode.
    Saturation,
    /// Colour blend mode.
    Color,
    /// Luminosity blend mode.
    Luminosity,
}

impl BlendMode {
    /// Returns the blend mode as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PassThrough => "pass",
            Self::Normal => "norm",
            Self::Dissolve => "diss",
            Self::Darken => "dark",
            Self::Multiply => "mul ",
            Self::ColorBurn => "idiv",
            Self::LinearBurn => "lbrn",
            Self::DarkerColor => "dkCl",
            Self::Lighten => "lite",
            Self::Screen => "scrn",
            Self::ColorDodge => "div ",
            Self::LinearDodge => "lddg",
            Self::LighterColor => "lgCl",
            Self::Overlay => "over",
            Self::SoftLight => "sLit",
            Self::HardLight => "hLit",
            Self::VividLight => "vLit",
            Self::LinearLight => "lLit",
            Self::PinLight => "pLit",
            Self::HardMix => "hMix",
            Self::Difference => "diff",
            Self::Exclusion => "smud",
            Self::Subtract => "fsub",
            Self::Divide => "fdiv",
            Self::Hue => "hue ",
            Self::Saturation => "sat ",
            Self::Color => "colr",
            Self::Luminosity => "lum ",
        }
    }
}

impl From<&str> for BlendMode {
    fn from(value: &str) -> Self {
        match value {
            "pass" => Self::PassThrough,
            "norm" => Self::Normal,
            "diss" => Self::Dissolve,
            "dark" => Self::Darken,
            "mul " => Self::Multiply,
            "idiv" => Self::ColorBurn,
            "lbrn" => Self::LinearBurn,
            "dkCl" => Self::DarkerColor,
            "lite" => Self::Lighten,
            "scrn" => Self::Screen,
            "div " => Self::ColorDodge,
            "lddg" => Self::LinearDodge,
            "lgCl" => Self::LighterColor,
            "over" => Self::Overlay,
            "sLit" => Self::SoftLight,
            "hLit" => Self::HardLight,
            "vLit" => Self::VividLight,
            "lLit" => Self::LinearLight,
            "pLit" => Self::PinLight,
            "hMix" => Self::HardMix,
            "diff" => Self::Difference,
            "smud" => Self::Exclusion,
            "fsub" => Self::Subtract,
            "fdiv" => Self::Divide,
            "hue " => Self::Hue,
            "sat " => Self::Saturation,
            "colr" => Self::Color,
            "lum " => Self::Luminosity,
            _ => Self::Normal,
        }
    }
}

impl From<graphics::BlendMode> for BlendMode {
    fn from(value: graphics::BlendMode) -> Self {
        match value {
            graphics::BlendMode::Addition => BlendMode::LinearDodge,
            graphics::BlendMode::Color => BlendMode::Color,
            graphics::BlendMode::ColorBurn => BlendMode::ColorBurn,
            graphics::BlendMode::ColorDodge => BlendMode::ColorDodge,
            graphics::BlendMode::Darken => BlendMode::Darken,
            graphics::BlendMode::Difference => BlendMode::Difference,
            graphics::BlendMode::Divide => BlendMode::Divide,
            graphics::BlendMode::Exclusion => BlendMode::Exclusion,
            graphics::BlendMode::HardLight => BlendMode::HardLight,
            graphics::BlendMode::Hue => BlendMode::Hue,
            graphics::BlendMode::Lighten => BlendMode::Lighten,
            graphics::BlendMode::Luminosity => BlendMode::Luminosity,
            graphics::BlendMode::Multiply => BlendMode::Multiply,
            graphics::BlendMode::Normal => BlendMode::Normal,
            graphics::BlendMode::Overlay => BlendMode::Overlay,
            graphics::BlendMode::PassThrough => BlendMode::PassThrough,
            graphics::BlendMode::Saturation => BlendMode::Saturation,
            graphics::BlendMode::Screen => BlendMode::Screen,
            graphics::BlendMode::SoftLight => BlendMode::SoftLight,
            graphics::BlendMode::Subtract => BlendMode::Subtract,
            graphics::BlendMode::DestinationIn => BlendMode::Normal,
            graphics::BlendMode::DestinationOut => BlendMode::Normal,
        }
    }
}

impl From<BlendMode> for graphics::BlendMode {
    fn from(value: BlendMode) -> Self {
        match value {
            BlendMode::PassThrough => graphics::BlendMode::PassThrough,
            BlendMode::Normal => graphics::BlendMode::Normal,
            BlendMode::Darken => graphics::BlendMode::Darken,
            BlendMode::Multiply => graphics::BlendMode::Multiply,
            BlendMode::ColorBurn => graphics::BlendMode::ColorBurn,
            BlendMode::Lighten => graphics::BlendMode::Lighten,
            BlendMode::Screen => graphics::BlendMode::Screen,
            BlendMode::ColorDodge => graphics::BlendMode::ColorDodge,
            BlendMode::Overlay => graphics::BlendMode::Overlay,
            BlendMode::SoftLight => graphics::BlendMode::SoftLight,
            BlendMode::HardLight => graphics::BlendMode::HardLight,
            BlendMode::Difference => graphics::BlendMode::Difference,
            BlendMode::Exclusion => graphics::BlendMode::Exclusion,
            BlendMode::Subtract => graphics::BlendMode::Subtract,
            BlendMode::Divide => graphics::BlendMode::Divide,
            BlendMode::Hue => graphics::BlendMode::Hue,
            BlendMode::Saturation => graphics::BlendMode::Saturation,
            BlendMode::Color => graphics::BlendMode::Color,
            BlendMode::Luminosity => graphics::BlendMode::Luminosity,
            // These blend modes aren’t compatible —
            // what’s the best way to handle that?
            BlendMode::Dissolve => graphics::BlendMode::Normal,
            BlendMode::LinearBurn => graphics::BlendMode::Normal,
            BlendMode::DarkerColor => graphics::BlendMode::Normal,
            BlendMode::LinearDodge => graphics::BlendMode::Normal,
            BlendMode::LighterColor => graphics::BlendMode::Normal,
            BlendMode::VividLight => graphics::BlendMode::Normal,
            BlendMode::LinearLight => graphics::BlendMode::Normal,
            BlendMode::PinLight => graphics::BlendMode::Normal,
            BlendMode::HardMix => graphics::BlendMode::Normal,
        }
    }
}
