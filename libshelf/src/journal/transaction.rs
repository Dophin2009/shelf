use super::rollback::Rollback;
use super::{Journal, Record, RollbackIter};

impl<T> Journal<T> {
    /// Start a transaction.
    #[inline]
    pub fn lock(&mut self) -> Transaction<'_, T> {
        Transaction { journal: self }
    }
}

/// A handle to a [`Journal`] that facilitate transactions.
#[derive(Debug)]
pub struct Transaction<'j, T> {
    journal: &'j mut Journal<T>,
}

/// The completed state of a transaction. This is returned by a successful [`Transaction::commit`]
/// call.
#[derive(Debug)]
pub struct CompletedTransaction<'j, T> {
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

    /// Commit the transaction by appending a commit record of the journal, returning a
    /// [`CompletedTransaction`].
    #[inline]
    pub fn commit(self) -> CompletedTransaction<'j, T> {
        self.journal.append(Record::Commit);
        CompletedTransaction {
            journal: self.journal,
        }
    }
}

impl<'j, T> Transaction<'j, T>
where
    T: Rollback<Output = T> + Clone,
{
    /// Return a [`RollbackIter`] to cancel the current transaction by rolling back any uncommitted
    /// records.
    #[inline]
    pub fn cancel(self) -> RollbackIter<'j, T> {
        self.journal.rollback()
    }
}

impl<'j, T> CompletedTransaction<'j, T>
where
    T: Rollback<Output = T> + Clone,
{
    /// Returns a reference to the journal on which this transaction is operating.
    #[inline]
    pub fn journal(&self) -> &Journal<T> {
        self.journal
    }

    /// Return a [`RollbackIter`] to rollback the just-completed transaction.
    #[inline]
    pub fn rollback(self) -> RollbackIter<'j, T> {
        // SAFETY: The latest commit must be a commit, because a `CompletedTransaction` can only be
        // created by a sucessful `Transaction::commit` call.
        self.journal
            .rollback_last()
            .unwrap_or_else(|| unimplemented!())
    }
}

#[cfg(test)]
mod test {
    use super::super::test::{
        Datum::{self, *},
        BACKWARD, COMMIT, FORWARD,
    };
    use super::Journal;

    #[test]
    fn test_append() {
        let mut journal = Journal::new();
        let mut t = journal.lock();

        t.append(Forward);
        assert_eq!(&[FORWARD], t.journal().records());

        t.append(Forward);
        assert_eq!(&[FORWARD, FORWARD], t.journal().records());

        t.commit();
        assert_eq!(&[FORWARD, FORWARD, COMMIT], journal.records());
    }

    #[test]
    fn test_cancel() {
        let mut journal = Journal::new();
        let mut t = journal.lock();

        t.append(Forward);
        assert_eq!(&[FORWARD], t.journal().records());

        t.append(Forward);
        assert_eq!(&[FORWARD, FORWARD], t.journal().records());

        let rb = t.cancel();
        assert_eq!(vec![Backward, Backward], rb.consume());

        assert_eq!(
            &[FORWARD, FORWARD, BACKWARD, BACKWARD, COMMIT],
            journal.records()
        );
    }

    #[test]
    fn test_cancel_empty() {
        let mut journal: Journal<Datum> = Journal::new();
        let t = journal.lock();
        let rb = t.cancel();

        assert!(rb.consume().is_empty());
        assert!(journal.is_empty());
    }

    #[test]
    fn test_completed() {
        let mut journal = Journal::new();
        let mut t = journal.lock();

        t.append(Forward);
        t.append(Forward);

        let t = t.commit();
        let rb = t.rollback();

        assert_eq!(vec![Backward, Backward], rb.consume());
        assert_eq!(
            &[FORWARD, FORWARD, COMMIT, BACKWARD, BACKWARD, COMMIT],
            journal.records()
        );
    }
}
