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
