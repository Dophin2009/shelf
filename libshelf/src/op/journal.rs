use crate::journal::{self, Journal, Record};

use super::{Finish, Op, OpError, OpOutput};

/// Write-ahead logging for [`Op`] that permits rollback.
#[derive(Debug)]
pub struct OpJournal<'lua> {
    /// This struct is just a wrapper on [`Journal`].
    inner: Journal<Op<'lua>>,
}

impl<'lua> OpJournal<'lua> {
    /// Create a new, empty journal.
    #[inline]
    pub fn new() -> Self {
        Self::new_parts(Journal::new())
    }

    #[inline]
    fn new_parts(inner: Journal<Op<'lua>>) -> Self {
        Self { inner }
    }

    /// Get the inner journal.
    #[inline]
    pub fn journal(&self) -> &Journal<Op<'lua>> {
        &self.inner
    }

    /// Return the number of records in the journal.
    #[inline]
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    /// Return the latest/last appended record.
    #[inline]
    pub fn latest(&self) -> Option<&Record<Op<'lua>>> {
        self.inner.latest()
    }

    /// Return the oldest/first appended record.
    #[inline]
    pub fn oldest(&self) -> Option<&Record<Op<'lua>>> {
        self.inner.oldest()
    }

    /// Return true if the journal is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Retrieve the record at the given index, where the oldest record has an index of 0.
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&Record<Op<'lua>>> {
        self.inner.get(idx)
    }

    /// Retrieve the record of the given index, where the newest record has an index of 0.
    #[inline]
    pub fn get_back(&self, idx: usize) -> Option<&Record<Op<'lua>>> {
        self.inner.get_back(idx)
    }

    /// Return an immutable slice to the records.
    #[inline]
    pub fn records(&self) -> &[Record<Op<'lua>>] {
        self.inner.records()
    }
}

/// Iterator on a journal.
#[derive(Debug)]
pub struct Iter<'j, 'lua> {
    inner: journal::iter::Iter<'j, Op<'lua>>,
}

impl<'lua> OpJournal<'lua> {
    /// Return an iterator on the journal.
    #[inline]
    pub fn iter(&self) -> Iter<'_, 'lua> {
        Iter::new(self)
    }
}

impl<'j, 'lua> Iter<'j, 'lua> {
    /// Create a new iterator for the given journal.
    #[inline]
    fn new(journal: &'j OpJournal<'lua>) -> Self {
        Self {
            inner: journal.inner.iter(),
        }
    }
}

impl<'j, 'lua> Iterator for Iter<'j, 'lua> {
    type Item = &'j Record<Op<'lua>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j, 'lua> DoubleEndedIterator for Iter<'j, 'lua> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

/// An iterator that performs rollback on a [`OpJournal`]. See [`OpJournal::rollback`] and
/// [`OpJournal::rollback_last`].
#[derive(Debug)]
pub struct RollbackIter<'j, 'lua> {
    inner: journal::RollbackIter<'j, Op<'lua>>,
}

impl<'lua> OpJournal<'lua> {
    /// Return a [`RollbackIter`]. Callers should call [`Self::finish`] on outputted items. See
    /// [`journal::RollbackIter`]
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, 'lua> {
        let inner = self.inner.rollback();
        RollbackIter::new(inner)
    }

    /// Return a [`RollbackIter`] if the latest record is the a commit. See
    /// [`journal::RollbackIter`].
    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_, 'lua>> {
        let inner = self.inner.rollback_last()?;
        Some(RollbackIter::new(inner))
    }
}

impl<'j, 'lua> RollbackIter<'j, 'lua> {
    #[inline]
    fn new(inner: journal::RollbackIter<'j, Op<'lua>>) -> Self {
        Self { inner }
    }
}

impl<'j, 'lua> Iterator for RollbackIter<'j, 'lua> {
    type Item = Op<'lua>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// A handle to a [`Journal`] that facilitate transactions.
#[derive(Debug)]
pub struct Transaction<'j, 'lua> {
    inner: journal::Transaction<'j, Op<'lua>>,
}

/// The completed state of a transaction. This is returned by a successful [`Transaction::commit`]
/// call.
#[derive(Debug)]
pub struct CompletedTransaction<'j, 'lua> {
    inner: journal::CompletedTransaction<'j, Op<'lua>>,
}

impl<'lua> OpJournal<'lua> {
    /// Start a transaction.
    #[inline]
    pub fn lock(&mut self) -> Transaction<'_, 'lua> {
        Transaction {
            inner: self.inner.lock(),
        }
    }
}

impl<'j, 'lua> Transaction<'j, 'lua> {
    /// Append a new action record to the journal.
    #[inline]
    pub fn append_and_finish(&mut self, op: Op<'lua>) -> Result<OpOutput<'lua>, OpError> {
        self.inner.append(op.clone());
        self.finish(&op)
    }

    /// Finish an op.
    #[inline]
    fn finish(&self, op: &Op<'lua>) -> Result<OpOutput<'lua>, OpError> {
        op.finish()
    }

    /// Commit the transaction by appending a commit record of the journal, returning a
    /// [`CompletedTransaction`].
    #[inline]
    pub fn commit(self) -> CompletedTransaction<'j, 'lua> {
        let inner = self.inner.commit();
        CompletedTransaction { inner }
    }
}

impl<'j, 'lua> Transaction<'j, 'lua> {
    /// Return a [`RollbackIter`] to cancel the current transaction by rolling back any uncommitted
    /// records.
    #[inline]
    pub fn cancel(self) -> RollbackIter<'j, 'lua> {
        let inner = self.inner.cancel();
        RollbackIter::new(inner)
    }
}

impl<'j, 'lua> CompletedTransaction<'j, 'lua> {
    /// Return a [`RollbackIter`] to rollback the just-completed transaction.
    #[inline]
    pub fn rollback(self) -> RollbackIter<'j, 'lua> {
        let inner = self.inner.rollback();
        RollbackIter::new(inner)
    }
}
