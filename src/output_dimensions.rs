use std::fmt::{Display, Formatter};
#[derive(Debug, Clone)]
pub enum OutputDimensions {
    Original,
    ScaledWithRatio(usize, usize),
    ScaledExact(usize, usize),
}

impl Display for OutputDimensions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        return match self {
            OutputDimensions::Original => {
                write!(f, "original")
            }
            OutputDimensions::ScaledExact(x,y) => {
                write!(f, "{}x{} exact", x, y)
            }
            OutputDimensions::ScaledWithRatio(x,y) => {
                write!(f, "{}x{} keep ratio", x, y)
            }
        }
    }
}

impl From<(&str, &str, bool)> for OutputDimensions {
    fn from(possible_dimensions: (&str, &str, bool)) -> Self {
        if let Result::Ok(x) = possible_dimensions.0.parse::<usize>() {
            if let Result::Ok(y) = possible_dimensions.1.parse::<usize>() {
                if possible_dimensions.2 {
                    return OutputDimensions::ScaledExact(x,y);
                }
                return OutputDimensions::ScaledWithRatio(x,y);
            }
        }
        OutputDimensions::Original
    }
}
