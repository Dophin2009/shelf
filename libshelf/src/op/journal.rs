use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use crate::journal::{self, Journal, JournalError};

use super::{CopyOp, Finish, LinkOp, MkdirOp, Op, OpError, OpOutput, RmOp};

trait ShouldFinish: Finish {
    fn should_finish(&self) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub struct OpJournal<W>
where
    W: Write,
{
    inner: Journal<Op, W>,
    state: HashMap<PathBuf, FileMeta>,
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
        Self::new_parts(Journal::new(w), HashMap::new())
    }

    #[inline]
    fn new_parts(inner: Journal<Op, W>, state: HashMap<PathBuf, FileMeta>) -> Self {
        Self { inner, state }
    }

    #[inline]
    pub fn load<R>(w: W, r: R) -> Result<Self, JournalError>
    where
        R: Read,
    {
        let inner = Journal::load(w, r)?;
        let journal = Self::new_parts(inner, HashMap::new());

        let state = inner
            .iter()
            .filter_map(|record| journal.update_state(record))
            .collect();

        Ok(journal)
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

        let output = op.finish()?;

        // Update to the new state.
        self.update_state(op);

        Ok(Some(output))
    }

    /// Checks if an op should finish, given the current state.
    #[inline]
    fn should_finish(&self, op: &Op) -> Result<bool, OpJournalError> {
        op.should_finish()
    }

    /// Update the state for an op (that was presumably just finished).
    #[inline]
    fn update_state(&mut self, op: &Op) {
        match op {
            Op::Link(LinkOp { src, dest }) => {
                self.insert_state(dest, FileMeta::new_link(src.clone()))
            }
            Op::Copy(CopyOp { src: _, dest }) => {
                self.insert_state(dest, FileMeta::new_file());
            }
            Op::Mkdir(MkdirOp { path, parents: _ }) => {
                self.insert_state(path, FileMeta::new_dir());
            }
            Op::Rm(RmOp { path, dir: _ }) => {
                self.remove_state(&path);
            }
        };
    }

    #[inline]
    fn insert_state(&mut self, path: PathBuf, data: FileMeta) {
        self.state.insert(path, data);
    }

    #[inline]
    fn remove_state(&mut self, path: &PathBuf) {
        self.state.remove(&path);
    }

    #[inline]
    fn get_state(&self, path: &PathBuf) {
        self.state.get(path)
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FileMeta {
    pub typ: FileMetaTyp,
}

impl FileMeta {
    #[inline]
    pub fn new(typ: FileMetaTyp) -> Self {
        Self { typ }
    }

    #[inline]
    pub fn new_file() -> Self {
        Self {
            typ: FileMetaTyp::File,
        }
    }

    #[inline]
    pub fn new_dir() -> Self {
        Self {
            typ: FileMetaTyp::Dir,
        }
    }

    #[inline]
    pub fn new_link(target: PathBuf) -> Self {
        Self {
            typ: FileMetaTyp::Link { target },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FileMetaTyp {
    File,
    Dir,
    Link { target: PathBuf },
}
