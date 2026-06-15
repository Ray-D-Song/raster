// crates/ui/src/geometry.rs
use std::fmt::{self, Debug, Display, Formatter};

use gpui::{AbsoluteLength, Axis, Corner, Length, Pixels};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A enum for defining the placement of the element.
///
/// See also: [`Side`] if you need to define the left, right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Placement {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

impl Display for Placement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Placement::Top => write!(f, "Top"),
            Placement::Bottom => write!(f, "Bottom"),
            Placement::Left => write!(f, "Left"),
            Placement::Right => write!(f, "Right"),
        }
    }
}

impl Placement {
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        match self {
            Placement::Left | Placement::Right => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_vertical(&self) -> bool {
        match self {
            Placement::Top | Placement::Bottom => true,
            _ => false,
        }
    }

    #[inline]
    pub fn axis(&self) -> Axis {
        match self {
            Placement::Top | Placement::Bottom => Axis::Vertical,
            Placement::Left | Placement::Right => Axis::Horizontal,
        }
    }
}

/// A corner or edge-center anchor for placing floating elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    LeftCenter,
    RightCenter,
}

impl Anchor {
    pub fn other_side_along(self, axis: Axis) -> Self {
        match axis {
            Axis::Vertical => match self {
                Anchor::TopLeft => Anchor::BottomLeft,
                Anchor::TopCenter => Anchor::BottomCenter,
                Anchor::TopRight => Anchor::BottomRight,
                Anchor::BottomLeft => Anchor::TopLeft,
                Anchor::BottomCenter => Anchor::TopCenter,
                Anchor::BottomRight => Anchor::TopRight,
                Anchor::LeftCenter => Anchor::LeftCenter,
                Anchor::RightCenter => Anchor::RightCenter,
            },
            Axis::Horizontal => match self {
                Anchor::TopLeft => Anchor::TopRight,
                Anchor::TopCenter => Anchor::TopCenter,
                Anchor::TopRight => Anchor::TopLeft,
                Anchor::BottomLeft => Anchor::BottomRight,
                Anchor::BottomCenter => Anchor::BottomCenter,
                Anchor::BottomRight => Anchor::BottomLeft,
                Anchor::LeftCenter => Anchor::RightCenter,
                Anchor::RightCenter => Anchor::LeftCenter,
            },
        }
    }
}

impl From<Anchor> for Corner {
    fn from(anchor: Anchor) -> Self {
        match anchor {
            Anchor::TopLeft | Anchor::TopCenter | Anchor::LeftCenter => Corner::TopLeft,
            Anchor::TopRight | Anchor::RightCenter => Corner::TopRight,
            Anchor::BottomLeft | Anchor::BottomCenter => Corner::BottomLeft,
            Anchor::BottomRight => Corner::BottomRight,
        }
    }
}

/// A enum for defining the side of the element.
///
/// See also: [`Placement`] if you need to define the 4 edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

impl Side {
    /// Returns true if the side is left.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    /// Returns true if the side is right.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right)
    }
}

/// A trait to extend the [`Axis`] enum with utility methods.
pub trait AxisExt {
    fn is_horizontal(self) -> bool;
    fn is_vertical(self) -> bool;
}

impl AxisExt for Axis {
    #[inline]
    fn is_horizontal(self) -> bool {
        self == Axis::Horizontal
    }

    #[inline]
    fn is_vertical(self) -> bool {
        self == Axis::Vertical
    }
}

/// A trait to extend the [`Length`] enum with utility methods.
pub trait LengthExt {
    /// Converts the [`Length`] to [`Pixels`] based on a given `base_size` and `rem_size`.
    ///
    /// If the [`Length`] is [`Length::Auto`], it returns `None`.
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels>;
}

impl LengthExt for Length {
    fn to_pixels(&self, base_size: AbsoluteLength, rem_size: Pixels) -> Option<Pixels> {
        match self {
            Length::Auto => None,
            Length::Definite(len) => Some(len.to_pixels(base_size, rem_size)),
        }
    }
}

/// A struct for defining the edges of an element.
///
/// A extend version of [`gpui::Edges`] to serialize/deserialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, Eq, PartialEq)]
#[repr(C)]
pub struct Edges<T: Clone + Debug + Default + PartialEq> {
    /// The size of the top edge.
    pub top: T,
    /// The size of the right edge.
    pub right: T,
    /// The size of the bottom edge.
    pub bottom: T,
    /// The size of the left edge.
    pub left: T,
}

impl<T> Edges<T>
where
    T: Clone + Debug + Default + PartialEq,
{
    /// Creates a new `Edges` instance with all edges set to the same value.
    pub fn all(value: T) -> Self {
        Self {
            top: value.clone(),
            right: value.clone(),
            bottom: value.clone(),
            left: value,
        }
    }
}
