use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use crate::journal::{self, Journal, JournalError};

use super::{CopyOp, Finish, LinkOp, MkdirOp, Op, OpError, OpOutput, RmOp};

#[derive(Debug)]
pub struct OpJournal<W>
where
    W: Write,
{
    inner: Journal<Op, W>,
}

#[derive(Debug, thiserror::Error)]
pub enum OpJournalError {
    #[error("journal error")]
    Journal(#[from] JournalError),
    #[error("op error")]
    Op(#[from] OpError),
}

impl<W> OpJournal<W>
where
    W: Write,
{
    #[inline]
    pub fn new(w: W) -> Self {
        Self::new_parts(Journal::new(w))
    }

    #[inline]
    fn new_parts(inner: Journal<Op, W>) -> Self {
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
    pub fn journal(&self) -> &Journal<Op, W> {
        &self.inner
    }

    /// Execute an op and append a record.
    #[inline]
    pub fn append_and_finish(&mut self, op: Op) -> Result<Option<OpOutput>, OpJournalError> {
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
    pub fn finish(&self, op: &Op) -> Result<Option<OpOutput>, OpJournalError> {
        // Finish the op if necessary.
        if !self.should_finish(&op)? {
            return Ok(None);
        }

        op.finish().map(|out| Some(out))
    }

    /// Checks if an op should finish, given the current state.
    #[inline]
    fn should_finish(&self, op: &Op) -> Result<bool, OpJournalError> {
        op.should_finish()
    }

    /// Returns a [`RollbackIter`]. Callers should call [`Self::finish`] on outputted items.
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, W> {
        RollbackIter::new(self)
    }
}

#[derive(Debug)]
pub struct Iter<'j> {
    inner: journal::Iter<'j, Op>,
}

impl<W> OpJournal<W>
where
    W: Write,
{
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }
}

impl<'j> Iter<'j> {
    #[inline]
    fn new<W>(journal: &'j OpJournal<W>) -> Self
    where
        W: Write,
    {
        Self {
            inner: journal.inner.iter(),
        }
    }
}

impl<'j> Iterator for Iter<'j> {
    type Item = &'j Record<Op>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j> DoubleEndedIterator for Iter<'j> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

#[derive(Debug)]
pub struct RollbackIter<'j, W>
where
    W: Write,
{
    inner: journal::RollbackIter<'j, Op, W>,
}

impl<'j, W> RollbackIter<'j, W>
where
    W: Write,
{
    #[inline]
    pub fn new(journal: &'j OpJournal<W>) -> Self {
        Self {
            inner: journal.inner.rollback(),
        }
    }
}

impl<'j, W> Iterator for RollbackIter<'j, W>
where
    W: Write,
{
    type Item = Result<Op, JournalError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
