#[cfg(feature = "alloc")]
use crate::alloc;
use core::{
    alloc::{AllocError, Layout, LayoutError},
    error::Error,
    fmt,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllocateError {
    Layout { error: LayoutError },
    Alloc { error: AllocError, layout: Layout },
}

impl AllocateError {
    pub fn handle(self) -> ! {
        #[cfg(feature = "alloc")]
        alloc::handle_alloc_error(match self {
            Self::Layout { error: _error } => Layout::new::<()>(),
            Self::Alloc {
                error: _error,
                layout,
            } => layout,
        });

        #[cfg(not(feature = "alloc"))]
        match self {
            Self::Layout { error } => panic!("{error}"),
            Self::Alloc { error, layout } => panic!("{error} ({layout:?})"),
        }
    }

    pub fn unwrap_alloc<T>(result: Result<T, Self>) -> T {
        match result {
            Ok(value) => value,
            Err(err) => err.handle(),
        }
    }

    pub fn to_std<T>(result: Result<T, Self>) -> Result<T, AllocError> {
        result.map_err(|_| AllocError)
    }
}

impl fmt::Display for AllocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout { error: _error } => {
                write!(f, "failed to calculate layout for allocation")
            }
            Self::Alloc {
                error,
                layout: _layout,
            } => error.fmt(f),
        }
    }
}

impl Error for AllocateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Layout { error } => error,
            Self::Alloc {
                error,
                layout: _layout,
            } => error,
        })
    }
}

impl From<LayoutError> for AllocateError {
    fn from(value: LayoutError) -> Self {
        Self::Layout { error: value }
    }
}

impl From<AllocateError> for AllocError {
    fn from(_value: AllocateError) -> Self {
        Self
    }
}
