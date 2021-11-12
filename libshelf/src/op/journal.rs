
use serde::{Deserialize, Serialize};

use crate::journal::{self, Journal, Record, Rollback};

use super::ctx::FinishCtx;
use super::{
    CopyOp, CreateOp, Finish, Finished, FinishedError, LinkOp, MkdirOp, RmOp, Undo, UndoFinished,
    WriteOp,
};

#[derive(Debug, thiserror::Error)]
pub enum JournalOpError {
    #[error("link op error")]
    Link(#[from] FinishedError<LinkOp>),
    #[error("copy op error")]
    Copy(#[from] FinishedError<CopyOp>),
    #[error("create op error")]
    Create(#[from] FinishedError<CreateOp>),
    #[error("write op error")]
    Write(#[from] FinishedError<WriteOp>),
    #[error("mkdir op error")]
    Mkdir(#[from] FinishedError<MkdirOp>),
    #[error("rm op error")]
    Rm(#[from] FinishedError<RmOp>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum JournalOp {
    Link(LinkOp),
    LinkUndo(Undo<LinkOp>),
    Copy(CopyOp),
    CopyUndo(Undo<CopyOp>),
    Create(CreateOp),
    CreateUndo(Undo<CreateOp>),
    Write(WriteOp),
    WriteUndo(Undo<WriteOp>),
    Mkdir(MkdirOp),
    MkdirUndo(Undo<MkdirOp>),
    Rm(RmOp),
    RmUndo(Undo<RmOp>),
}

/// Generate [`From`], [`Finish`] implementations for [`Op`].
macro_rules! Op_impls {
    ($($Variant:ident => $SubOp:ty),*) => {
        $(
            impl From<$SubOp> for JournalOp {
                #[inline]
                fn from(op: $SubOp) -> Self {
                    Self::$Variant(op)
                }
            }

        )*

        impl Finish for JournalOp {
            type Output = JournalOpFinish;
            type Error = JournalOpError;

            #[inline]
            fn finish(&self, ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
                let res = match self {
                    $(
                        Self::$Variant(op) => op.finish(ctx)?.into(),
                    )*
                };

                Ok(res)
            }
        }
    };
}

Op_impls!(
    Link => LinkOp,
    LinkUndo => Undo<LinkOp>,
    Copy => CopyOp,
    CopyUndo => Undo<CopyOp>,
    Create => CreateOp,
    CreateUndo => Undo<CreateOp>,
    Write => WriteOp,
    WriteUndo => Undo<WriteOp>,
    Mkdir => MkdirOp,
    MkdirUndo => Undo<MkdirOp>,
    Rm => RmOp,
    RmUndo => Undo<RmOp>
);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum JournalOpFinish {
    Link(Finished<LinkOp>),
    LinkUndo(UndoFinished<LinkOp>),
    Copy(Finished<CopyOp>),
    CopyUndo(UndoFinished<CopyOp>),
    Create(Finished<CreateOp>),
    CreateUndo(UndoFinished<CreateOp>),
    Write(Finished<WriteOp>),
    WriteUndo(UndoFinished<WriteOp>),
    Mkdir(Finished<MkdirOp>),
    MkdirUndo(UndoFinished<MkdirOp>),
    Rm(Finished<RmOp>),
    RmUndo(UndoFinished<RmOp>),
}

macro_rules! JournalOpFinish_impls {
    ($($Variant:ident => $SubOp:ty),*) => {
        $(
            impl From<$SubOp> for JournalOpFinish {
                #[inline]
                fn from(v: $SubOp) -> Self {
                    Self::$Variant(v)
                }
            }
        )*

        impl Rollback for JournalOpFinish {
            type Output = JournalOp;

            #[inline]
            fn rollback(&self) -> Self::Output {
                match self {
                    $(
                        Self::$Variant(op) => op.rollback().into(),
                    )*
                }
            }
        }
    };
}

JournalOpFinish_impls!(
    Link => Finished<LinkOp>,
    LinkUndo => UndoFinished<LinkOp>,
    Copy => Finished<CopyOp>,
    CopyUndo => UndoFinished<CopyOp>,
    Create => Finished<CreateOp>,
    CreateUndo => UndoFinished<CreateOp>,
    Write => Finished<WriteOp>,
    WriteUndo => UndoFinished<WriteOp>,
    Mkdir => Finished<MkdirOp>,
    MkdirUndo => UndoFinished<MkdirOp>,
    Rm => Finished<RmOp>,
    RmUndo => UndoFinished<RmOp>
);

#[derive(Debug, Deserialize, Serialize)]
struct JournalOpAtom {
    op: JournalOpFinish,
    ctx: FinishCtx,
}

impl Rollback for JournalOpAtom {
    type Output = Result<Self, JournalOpError>;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let undo = self.op.rollback();
        let undof = undo.finish(&self.ctx)?;

        Ok(Self {
            op: undof,
            ctx: self.ctx.clone(),
        })
    }
}

/// Write-ahead logging for [`Op`] that permits rollback.
#[derive(Debug)]
pub struct OpJournal {
    /// This struct is just a wrapper on [`Journal`].
    inner: Journal<JournalOpAtom>,
}

impl OpJournal {
    /// Create a new, empty journal.
    #[inline]
    pub fn new() -> Self {
        Self::new_parts(Journal::new())
    }

    #[inline]
    fn new_parts(inner: Journal<JournalOpAtom>) -> Self {
        Self { inner }
    }

    /// Return the number of records in the journal.
    #[inline]
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    /// Return the latest/last appended record.
    #[inline]
    pub fn latest(&self) -> Option<Record<&JournalOpFinish>> {
        self.inner.latest().map(map_record)
    }

    /// Return the oldest/first appended record.
    #[inline]
    pub fn oldest(&self) -> Option<Record<&JournalOpFinish>> {
        self.inner.oldest().map(map_record)
    }

    /// Return true if the journal is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Retrieve the record at the given index, where the oldest record has an index of 0.
    #[inline]
    pub fn get(&self, idx: usize) -> Option<Record<&JournalOpFinish>> {
        self.inner.get(idx).map(map_record)
    }

    /// Retrieve the record of the given index, where the newest record has an index of 0.
    #[inline]
    pub fn get_back(&self, idx: usize) -> Option<Record<&JournalOpFinish>> {
        self.inner.get_back(idx).map(map_record)
    }
}

/// Iterator on a journal.
#[derive(Debug)]
pub struct Iter<'j> {
    inner: journal::iter::Iter<'j, JournalOpAtom>,
}

impl OpJournal {
    /// Return an iterator on the journal.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
}

impl<'j> Iter<'j> {
    /// Create a new iterator for the given journal.
    #[inline]
    fn new(journal: &'j OpJournal) -> Self {
        Self {
            inner: journal.inner.iter(),
        }
    }
}

impl<'j> Iterator for Iter<'j> {
    type Item = Record<&'j JournalOpFinish>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(map_record)
    }
}

impl<'j> DoubleEndedIterator for Iter<'j> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(map_record)
    }
}

/// An iterator that performs rollback on a [`OpJournal`]. See [`OpJournal::rollback`] and
/// [`OpJournal::rollback_last`].
#[derive(Debug)]
pub struct RollbackIter<'j> {
    inner: journal::RollbackIter<'j, JournalOpAtom>,
}

impl OpJournal {
    /// Return a [`RollbackIter`]. See [`journal::RollbackIter`]
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_> {
        let inner = self.inner.rollback();
        RollbackIter::new(inner)
    }

    /// Return a [`RollbackIter`] if the latest record is the a commit. See
    /// [`journal::RollbackIter`].
    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_>> {
        let inner = self.inner.rollback_last()?;
        Some(RollbackIter::new(inner))
    }
}

impl<'j> RollbackIter<'j> {
    #[inline]
    fn new(inner: journal::RollbackIter<'j, JournalOpAtom>) -> Self {
        Self { inner }
    }
}

impl<'j> RollbackIter<'j> {
    #[inline]
    pub fn next(&mut self) -> Option<Result<&'_ JournalOpFinish, JournalOpError>> {
        match self.inner.next_get()? {
            Ok(datum) => self.inner.next_append(datum).map(|datum| &datum.op).map(Ok),
            Err(err) => Some(Err(err)),
        }
    }
}

/// A handle to a [`Journal`] that facilitate transactions.
#[derive(Debug)]
pub struct Transaction<'j> {
    inner: journal::Transaction<'j, JournalOpAtom>,
}

impl OpJournal {
    /// Start a transaction.
    #[inline]
    pub fn lock(&mut self) -> Transaction<'_> {
        Transaction {
            inner: self.inner.lock(),
        }
    }
}

impl<'j> Transaction<'j> {
    /// Append a new action record to the journal by finishing the op, and return the result of
    /// finishing.
    #[inline]
    pub fn append_finish<O>(
        &mut self,
        op: O,
        ctx: &FinishCtx,
    ) -> Result<&JournalOpFinish, JournalOpError>
    where
        O: Into<JournalOp>,
    {
        let op = op.into();
        let datum = self.finish(&op, ctx)?;

        self.inner.append(datum);

        match self.inner.journal().latest().unwrap() {
            Record::Atom(ref atom) => Ok(&atom.op),
            Record::Commit => unreachable!(),
        }
    }

    /// Finish an op and return an [`OpAtom`] to append.
    #[inline]
    fn finish(&self, op: &JournalOp, ctx: &FinishCtx) -> Result<JournalOpAtom, JournalOpError> {
        let opf = op.finish(ctx)?;
        Ok(JournalOpAtom {
            op: opf.into(),
            ctx: ctx.clone(),
        })
    }
}

#[inline]
fn map_record<'r>(record: &'r Record<JournalOpAtom>) -> Record<&'r JournalOpFinish> {
    match record {
        Record::Atom(datum) => Record::Atom(&datum.op),
        Record::Commit => todo!(),
    }
}
