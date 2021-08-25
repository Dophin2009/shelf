use std::error::Error as StdError;
use std::fmt;

// Convenient [`std::error::Error`]-compatible vehicle for transporting multiple errors.
#[derive(Debug, Clone)]
pub struct MultipleError<E, D>(pub Vec<(E, D)>)
where
    E: StdError,
    D: fmt::Debug;

impl<E, D> MultipleError<E, D>
where
    E: StdError,
    D: fmt::Debug,
{
    #[inline]
    pub fn new(inner: Vec<(E, D)>) -> Self {
        Self(inner)
    }
}

impl<E> MultipleError<E, ()>
where
    E: StdError,
{
    #[inline]
    pub fn of(inner: Vec<E>) -> Self {
        Self::new(inner.into_iter().map(|err| (err, ())).collect())
    }
}

impl<E, D> fmt::Display for MultipleError<E, D>
where
    E: StdError,
    D: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errs: Vec<_> = self.0.iter().map(|(err, _)| err).collect();
        write!(f, "{:?}", errs)
    }
}

impl<E, D> StdError for MultipleError<E, D>
where
    E: StdError,
    D: fmt::Debug,
{
}

impl<E, D> From<Vec<(E, D)>> for MultipleError<E, D>
where
    E: StdError,
    D: fmt::Debug,
{
    #[inline]
    fn from(inner: Vec<(E, D)>) -> Self {
        Self::new(inner)
    }
}

impl<E, D> Into<Vec<(E, D)>> for MultipleError<E, D>
where
    E: StdError,
    D: fmt::Debug,
{
    #[inline]
    fn into(self) -> Vec<(E, D)> {
        self.0
    }
}
