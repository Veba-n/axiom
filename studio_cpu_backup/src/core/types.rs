use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Anchor {
    TopLeft, TopCenter, TopRight,
    Center,
    BottomLeft, BottomCenter, BottomRight,
}
impl Default for Anchor { fn default() -> Self { Self::TopLeft } }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum TextAlign { Left, Center, Right }
impl Default for TextAlign { fn default() -> Self { Self::Left } }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum TextValign { Top, Middle, Bottom }
impl Default for TextValign { fn default() -> Self { Self::Top } }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum GradientDirection { Horizontal, Vertical, Diagonal }
impl Default for GradientDirection { fn default() -> Self { Self::Horizontal } }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum AnimationType { None, PulseColor, Blink, Wave, Ripple, Typewriter, MatrixRain, Glitch }
impl Default for AnimationType { fn default() -> Self { Self::None } }
