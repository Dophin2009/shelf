use super::rollback::Rollback;
use super::{Journal, Record, RollbackIter};

impl<T> Journal<T> {
    /// Start a transaction.
    #[inline]
    pub fn lock(&mut self) -> Transaction<'_, T> {
        TraTransaction { journal: self }
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
    /// Start a new transaction on the given [`Journal`].
    #[inline]
    pub(super) fn new(journal: &'j mut Journal<T>) -> Self {
        Self { journal }
    }

    /// Returns a reference to the journal on which this transaction is operating.
    #[inline]
    pub fn journal(&self) -> &'j Journal<T> {
        self.journal
    }

    /// Append a new action record to the journal.
    #[inline]
    pub fn append(&mut self, action: T) {
        self.journal.append(Record::Action(action))
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
