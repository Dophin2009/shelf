use std::io::{Read, Write};

use crate::journal::{self, Journal, JournalError, Record};

use super::{Finish, Op, OpError, OpOutput};

#[derive(Debug)]
pub struct OpJournal<'lua, W>
where
    W: Write,
{
    inner: Journal<Op<'lua>, W>,
}

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
    #[inline]
    pub fn new(w: W) -> Self {
        Self::new_parts(Journal::new(w))
    }

    #[inline]
    fn new_parts(inner: Journal<Op<'lua>, W>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn load<R>(w: W, r: R) -> Result<Self, JournalError>
    where
        R: Read,
    {
        let inner = Journal::load(w, r)?;
        let journal = Self::new_parts(inner);
        Ok(journal)
    }

    #[inline]
    pub fn journal(&self) -> &Journal<Op<'lua>, W> {
        &self.inner
    }

    /// Execute an op and append a record.
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

    /// Finish an op. This checks the state, and if the operation is unecessary, will not finish it
    /// and return `Ok(None)`.
    #[inline]
    pub fn finish(&self, op: &Op<'lua>) -> Result<OpOutput<'lua>, OpJournalError> {
        let ret = op.finish()?;
        Ok(ret)
    }

    /// Returns a [`RollbackIter`]. Callers should call [`Self::finish`] on outputted items.
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, 'lua, W> {
        let inner = self.inner.rollback();
        RollbackIter::new(inner)
    }

    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_, 'lua, W>> {
        let inner = self.inner.rollback_last()?;
        Some(RollbackIter::new(inner))
    }
}

#[derive(Debug)]
pub struct Iter<'j, 'lua> {
    inner: journal::Iter<'j, Op<'lua>>,
}

impl<'lua, W> OpJournal<'lua, W>
where
    W: Write,
{
    #[inline]
    pub fn iter(&self) -> Iter<'_, 'lua> {
        Iter::new(self)
    }
}

impl<'j, 'lua> Iter<'j, 'lua> {
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
