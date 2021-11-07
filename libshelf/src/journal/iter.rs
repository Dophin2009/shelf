use std::slice;

use super::{Journal, Record};

/// Iterator on a [`Journal`] that emits records from oldest to newest.
#[derive(Debug)]
pub struct Iter<'j, T> {
    inner: slice::Iter<'j, Record<T>>,
}

impl<T> Journal<T> {
    /// Create an iterator with immutable access on this journal. See [`Iter`].
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }
}

impl<'j, T> Iter<'j, T> {
    /// Initialize a new iterator for the given journal.
    #[inline]
    pub(super) fn new(journal: &'j Journal<T>) -> Self {
        Self {
            inner: journal.records.iter(),
        }
    }
}

impl<'j, T> Iterator for Iter<'j, T> {
    type Item = &'j Record<T>;

    /// Retrieve the next record from the journal, returning None if there are no more new records.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j, T> DoubleEndedIterator for Iter<'j, T> {
    /// Retrieve the next (previous) record from the journal, returning None if the oldest record
    /// has already been returned.
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

#[cfg(test)]
mod test {
    use super::super::test::{Datum, BACKWARD, COMMIT, FORWARD};
    use super::Journal;

    #[test]
    fn test_iter_empty() -> Result<(), Box<dyn std::error::Error>> {
        let journal: Journal<Datum> = Journal::new();
        assert_eq!(None, journal.iter().next());

        Ok(())
    }

    #[test]
    fn test_iter_forward() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();

        journal.append(FORWARD);
        journal.append(BACKWARD);
        journal.append(COMMIT);

        let mut iter = journal.iter();
        assert_eq!(Some(&FORWARD), iter.next());
        assert_eq!(Some(&BACKWARD), iter.next());
        assert_eq!(Some(&COMMIT), iter.next());
        assert_eq!(None, iter.next());

        Ok(())
    }

    #[test]
    fn test_iter_backward() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();

        journal.append(FORWARD);
        journal.append(BACKWARD);
        journal.append(COMMIT);

        let mut iter = journal.iter();
        assert_eq!(Some(&COMMIT), iter.next_back());
        assert_eq!(Some(&BACKWARD), iter.next_back());
        assert_eq!(Some(&FORWARD), iter.next_back());
        assert_eq!(None, iter.next_back());

        Ok(())
    }
}
