use std::io::{Read, Write};

use crate::journal::{self, Journal, JournalError, Record};

use super::{Finish, Op, OpError, OpOutput};

/// Write-ahead logging for [`Op`] that permits rollback.
#[derive(Debug)]
pub struct OpJournal<'lua, W>
where
    W: Write,
{
    /// This struct is just a wrapper on [`Journal`].
    inner: Journal<Op<'lua>, W>,
}

/// Error type for [`OpJournal`] operations that combines [`JournalError`] and [`OpError`].
#[derive(Debug, thiserror::Error)]
pub enum OpJournalError {
    #[error("journal error")]
    Journal(#[from] JournalError),
    #[error("op error")]
    Op(#[from] OpError),
}

impl<'lua, W> OpJournal<'lua, W>
where
    W: Write,
{
    /// Create a new, empty journal.
    #[inline]
    pub fn new(w: W) -> Self {
        Self::new_parts(Journal::new(w))
    }

    #[inline]
    fn new_parts(inner: Journal<Op<'lua>, W>) -> Self {
        Self { inner }
    }

    /// Load a journal from an existing reader.
    #[inline]
    pub fn load<R>(w: W, r: R) -> Result<Self, JournalError>
    where
        R: Read,
    {
        let inner = Journal::load(w, r)?;
        let journal = Self::new_parts(inner);
        Ok(journal)
    }

    /// Get the inner journal.
    #[inline]
    pub fn journal(&self) -> &Journal<Op<'lua>, W> {
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

    /// Return true if the journal is currently in a pending transaction state (there are more
    /// than zero records, and the latest record is a non-commit record).
    #[inline]
    pub fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
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

    /// Append a record (see [`Journal::append`]) and finish the op (see [`Journal::finish`]).
    #[inline]
    pub fn append_and_finish(&mut self, op: Op<'lua>) -> Result<OpOutput<'lua>, OpJournalError> {
        // Append the record.
        let record = Record::Action(op.clone());
        self.inner.append(record)?;

        // Finish the op.
        self.finish(&op)
    }

    /// Append a commit record.
    #[inline]
    pub fn commit(&mut self) -> Result<(), JournalError> {
        self.inner.append(Record::Commit)?;
        Ok(())
    }

    /// Finish an op.
    #[inline]
    pub fn finish(&self, op: &Op<'lua>) -> Result<OpOutput<'lua>, OpJournalError> {
        let ret = op.finish()?;
        Ok(ret)
    }

    /// Return a [`RollbackIter`]. Callers should call [`Self::finish`] on outputted items. See
    /// [`journal::RollbackIter`]
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, 'lua, W> {
        let inner = self.inner.rollback();
        RollbackIter::new(inner)
    }

    /// Return a [`RollbackIter`] if the latest record is the a commit. See
    /// [`journal::RollbackIter`].
    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_, 'lua, W>> {
        let inner = self.inner.rollback_last()?;
        Some(RollbackIter::new(inner))
    }
}

/// Iterator on a journal.
#[derive(Debug)]
pub struct Iter<'j, 'lua> {
    inner: journal::Iter<'j, Op<'lua>>,
}

impl<'lua, W> OpJournal<'lua, W>
where
    W: Write,
{
    /// Return an iterator on the journal.
    #[inline]
    pub fn iter(&self) -> Iter<'_, 'lua> {
        Iter::new(self)
    }
}

impl<'j, 'lua> Iter<'j, 'lua> {
    /// Create a new iterator for the given journal.
    #[inline]
    fn new<W>(journal: &'j OpJournal<'lua, W>) -> Self
    where
        W: Write,
    {
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
pub struct RollbackIter<'j, 'lua, W>
where
    W: Write,
{
    inner: journal::RollbackIter<'j, Op<'lua>, W>,
}

impl<'j, 'lua, W> RollbackIter<'j, 'lua, W>
where
    W: Write,
{
    #[inline]
    fn new(inner: journal::RollbackIter<'j, Op<'lua>, W>) -> Self {
        Self { inner }
    }
}

impl<'j, 'lua, W> Iterator for RollbackIter<'j, 'lua, W>
where
    W: Write,
{
    type Item = Result<Op<'lua>, JournalError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
