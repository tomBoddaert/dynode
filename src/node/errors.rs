#[cfg(feature = "alloc")]
use crate::alloc;
use core::{
    alloc::{AllocError, Layout, LayoutError},
    error::Error,
    fmt,
};

#[derive(Clone, PartialEq, Eq)]
enum Internal {
    Layout { error: LayoutError },
    Alloc { error: AllocError, layout: Layout },
}

impl Internal {
    fn handle(self) -> ! {
        #[cfg(feature = "alloc")]
        alloc::handle_alloc_error(match self {
            Self::Layout { .. } => Layout::new::<()>(),
            Self::Alloc { layout, .. } => layout,
        });

        #[cfg(not(feature = "alloc"))]
        panic!("{self}")
    }

    fn source(&self) -> &(dyn Error + 'static) {
        match self {
            Self::Layout { error } => error,
            Self::Alloc { error, .. } => error,
        }
    }
}

impl fmt::Display for Internal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout { error } => write!(f, "{error}"),
            Self::Alloc { error, layout } => write!(
                f,
                "{error} (size: {}, align: {})",
                layout.size(),
                layout.align()
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
/// The error type returned when allocation fails from [`DynList`](crate::DynList) operations.
///
/// This can either be from an arithmetic error when calculating the layout or from an allocator when allocating.
pub struct AllocateError<Value = ()> {
    internal: Internal,
    value: Value,
}

impl<Value> AllocateError<Value> {
    #[inline]
    /// Handles the error by calling [`handle_alloc_error`](alloc::handle_alloc_error) if the `alloc` feature is enabled, or panicking otherwise.
    pub fn handle(self) -> ! {
        self.internal.handle()
    }

    #[inline]
    /// Gets the value held in the error.
    ///
    /// This is usually from attempting to insert the value into a list.
    pub fn into_value(self) -> Value {
        self.value
    }

    #[inline]
    /// Seperates the value from the error.
    pub fn into_parts(self) -> (Value, AllocateError) {
        (
            self.value,
            AllocateError {
                internal: self.internal,
                value: (),
            },
        )
    }

    #[inline]
    /// Unwraps the result using [`Self::handle`] when it is an error.
    pub fn unwrap_result<T>(result: Result<T, Self>) -> T {
        match result {
            Ok(value) => value,
            Err(err) => err.handle(),
        }
    }

    /// Gets the layout for the allocation.
    ///
    /// If this returns [`None`], the layout calculation failed.
    pub const fn layout(&self) -> Option<Layout> {
        match self.internal {
            Internal::Layout { .. } => None,
            Internal::Alloc { layout, .. } => Some(layout),
        }
    }
}

impl AllocateError {
    #[inline]
    /// Places a value into the error.
    pub const fn with_value<Value>(self, value: Value) -> AllocateError<Value> {
        AllocateError {
            internal: self.internal,
            value,
        }
    }

    #[inline]
    pub(crate) const fn new_layout(error: LayoutError) -> Self {
        Self {
            internal: Internal::Layout { error },
            value: (),
        }
    }

    #[inline]
    pub(crate) const fn new_alloc(error: AllocError, layout: Layout) -> Self {
        Self {
            internal: Internal::Alloc { error, layout },
            value: (),
        }
    }
}

impl<Value> fmt::Debug for AllocateError<Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple;
        match &self.internal {
            Internal::Layout { .. } => {
                tuple = f.debug_tuple("AllocateError::Layout");
            }
            Internal::Alloc { layout, .. } => {
                tuple = f.debug_tuple("AllocateError::Alloc");
                tuple.field(&layout);
            }
        }

        tuple.finish()
    }
}

impl<Value> fmt::Display for AllocateError<Value> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.internal.fmt(f)
    }
}

impl<Value> Error for AllocateError<Value> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.internal.source())
    }
}

impl<Value> From<AllocateError<Value>> for AllocError {
    #[inline]
    fn from(_value: AllocateError<Value>) -> Self {
        Self
    }
}
