use super::{Journal, Record};

impl<T> Journal<T> {
    /// Start a transaction.
    #[inline]
    pub fn lock(&mut self) -> Transaction<'_, T> {
        Transaction { journal: self }
    }
}

/// A handle to a [`Journal`] that facilitate transactions.
///
/// # Drop
///
/// When dropped, the transaction is committed.
#[derive(Debug)]
pub struct Transaction<'j, T> {
    journal: &'j mut Journal<T>,
}

impl<'j, T> Transaction<'j, T> {
    /// Returns a reference to the journal on which this transaction is operating.
    #[inline]
    pub fn journal(&self) -> &Journal<T> {
        self.journal
    }

    /// Append a new [`Record::Atom`] record to the journal.
    #[inline]
    pub fn append(&mut self, datum: T) {
        self.journal.append(Record::Atom(datum))
    }
}

impl<'j, T> Drop for Transaction<'j, T> {
    /// Commit the transaction on drop.
    #[inline]
    fn drop(&mut self) {
        self.journal.append(Record::Commit);
    }
}

#[cfg(test)]
mod test {
    use super::super::test::{Datum::*, COMMIT, FORWARD};
    use super::Journal;

    #[test]
    fn test_append() {
        let mut journal = Journal::new();
        let mut t = journal.lock();

        t.append(Forward);
        assert_eq!(&[FORWARD], t.journal().records());

        t.append(Forward);
        assert_eq!(&[FORWARD, FORWARD], t.journal().records());
    }

    #[test]
    fn test_drop() {
        let mut journal = Journal::new();

        {
            let mut t = journal.lock();

            t.append(Forward);
            assert_eq!(&[FORWARD], t.journal().records());

            t.append(Forward);
            assert_eq!(&[FORWARD, FORWARD], t.journal().records());
        }

        assert_eq!(&[FORWARD, FORWARD, COMMIT], journal.records());
    }
}
