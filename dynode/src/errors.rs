use core::{
    alloc::{AllocError, Layout, LayoutError},
    error::Error,
    fmt,
};

#[derive(Clone, PartialEq, Eq)]
enum AllocateErrorInternal {
    Layout { error: LayoutError },
    Alloc { error: AllocError, layout: Layout },
}

#[derive(Clone, PartialEq, Eq)]
/// The error type returned when an allocation fails.
///
/// This can either be from an arithmetic error when calculating the layout or from an allocator when allocating.
pub struct AllocateError<Value = ()> {
    internal: AllocateErrorInternal,
    value: Value,
}

impl AllocateErrorInternal {
    fn handle(self) -> ! {
        #[cfg(feature = "alloc")]
        crate::alloc::handle_alloc_error(match self {
            Self::Layout { .. } => Layout::new::<()>(),
            Self::Alloc { layout, .. } => layout,
        });

        #[cfg(not(feature = "alloc"))]
        panic!("{self}")
    }
}

impl<Value> AllocateError<Value> {
    #[inline]
    /// Handles the error by calling [`handle_alloc_error`](std::alloc::handle_alloc_error) if the `alloc` feature is enabled, or panicking otherwise.
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

    #[inline]
    /// Gets the layout for the allocation.
    ///
    /// If this returns [`None`], the layout calculation failed.
    pub const fn layout(&self) -> Option<Layout> {
        match self.internal {
            AllocateErrorInternal::Layout { .. } => None,
            AllocateErrorInternal::Alloc { layout, .. } => Some(layout),
        }
    }

    #[inline]
    /// Applies a function `f` to the value.
    ///
    /// This maps from an [`AllocateError<Value>`] to an [`AllocateError<U>`].
    pub fn map<U, F>(self, f: F) -> AllocateError<U>
    where
        F: FnOnce(Value) -> U,
    {
        let (value, empty) = self.into_parts();
        empty.with_value(f(value))
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

    #[must_use]
    #[inline]
    /// Create a new error from a [`LayoutError`].
    pub const fn new_layout(source: LayoutError) -> Self {
        Self {
            internal: AllocateErrorInternal::Layout { error: source },
            value: (),
        }
    }

    #[must_use]
    #[inline]
    /// Create a new error from a [`AllocError`] and the [`Layout`] that could not be allocated.
    pub const fn new_alloc(source: AllocError, layout: Layout) -> Self {
        Self {
            internal: AllocateErrorInternal::Alloc {
                error: source,
                layout,
            },
            value: (),
        }
    }
}

impl fmt::Debug for AllocateErrorInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple;
        match self {
            Self::Layout { .. } => {
                tuple = f.debug_tuple("AllocateError::Layout");
            }
            Self::Alloc { layout, .. } => {
                tuple = f.debug_tuple("AllocateError::Alloc");
                tuple.field(&layout);
            }
        }

        tuple.finish()
    }
}

impl fmt::Display for AllocateErrorInternal {
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

impl fmt::Debug for AllocateError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.internal, f)
    }
}

impl fmt::Display for AllocateError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.internal, f)
    }
}

impl Error for AllocateErrorInternal {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Layout { error } => error,
            Self::Alloc { error, .. } => error,
        })
    }
}

impl Error for AllocateError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.internal.source()
    }
}

impl From<LayoutError> for AllocateError {
    #[inline]
    fn from(value: LayoutError) -> Self {
        Self::new_layout(value)
    }
}

impl<Value> From<AllocateError<Value>> for AllocError {
    #[inline]
    fn from(_value: AllocateError<Value>) -> Self {
        Self
    }
}
