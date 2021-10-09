mod copy;
mod link;
mod mkdir;
mod rm;

pub use self::copy::*;
pub use self::link::*;
pub use self::mkdir::*;
pub use self::rm::*;

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use crate::cache::FileMetaTyp;
use crate::journal::Journal;
use crate::journal::JournalError;
use crate::journal::Record;

trait Finish {
    type Output;
    type Error;

    fn finish(&self) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone)]
pub enum Op {
    Link(LinkOp),
    Copy(CopyOp),
    Mkdir(MkdirOp),
    Rm(RmOp),
}

#[derive(Debug, Clone)]
pub enum OpOutput {
    None,
}

impl From<()> for OpOutput {
    #[inline]
    fn from(_: ()) -> Self {
        Self::None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("i/o error")]
    Io(io::Error),
}

impl Finish for Op {
    type Output = OpOutput;
    type Error = OpError;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = match self {
            Op::Link(op) => op.finish(),
            Op::Copy(op) => op.finish(),
            Op::Mkdir(op) => op.finish(),
            Op::Rm(op) => op.finish(),
        };

        res?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct OpJournal<W>
where
    W: Write,
{
    inner: Journal<Op, W>,
    state: HashMap<PathBuf, FileMeta>,
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

    #[inline]
    fn update_state(&mut self, record: &Record<Op>) {
        match record {
            Record::Action(action) => match action {
                Op::Link(LinkOp { src, dest }) => {
                    let data = FileMeta::new_link(src.clone());
                    self.insert_state(dest, data);
                }
                Op::Copy(CopyOp { src: _, dest }) => {
                    let data = FileMeta::new_file();
                    self.insert_state(dest, data);
                }
                Op::Mkdir(MkdirOp { path, parents: _ }) => {
                    self.insert_state(path, FileMeta::new_dir())
                }
                Op::Rm(RmOp { path, dir: _ }) => self.remove_state(&path),
            },
            Record::Commit => {}
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
    fn new_parts(inner: Journal<Op, W>, state: HashMap<PathBuf, FileMeta>) -> Self {
        Self { inner, state }
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
