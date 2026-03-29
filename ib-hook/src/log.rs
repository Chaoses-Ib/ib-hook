#[cfg(feature = "tracing")]
pub use tracing::*;

#[cfg(not(feature = "tracing"))]
mod dummy {
    #[macro_export]
    macro_rules! debug {
        ($($args:tt)*) => {{}};
    }

    #[macro_export]
    macro_rules! error {
        ($($args:tt)*) => {{}};
    }

    #[macro_export]
    macro_rules! info {
        ($($args:tt)*) => {{}};
    }

    #[macro_export]
    macro_rules! trace {
        ($($args:tt)*) => {{}};
    }

    #[macro_export]
    macro_rules! warn {
        ($($args:tt)*) => {{}};
    }
}
#[cfg(not(feature = "tracing"))]
#[allow(unused_imports)]
pub use crate::{debug, error, info, trace, warn};
