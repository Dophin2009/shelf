use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::slice;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

fn main() {}

pub trait Rollback {
    fn rollback(&self) -> Self;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Record<T> {
    Action(T),
    Commit,
}

/// Write-ahead logging.
#[derive(Debug)]
pub struct Journal<T, W>
where
    T: Serialize,
    W: Write,
{
    records: Vec<Record<T>>,
    writer: BufWriter<W>,
}

/// Error type for errors that may occur when working with [`Journal`].
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("i/o error")]
    Io(#[from] io::Error),
    #[error("serialization/deserialization error")]
    Serde(#[from] serde_json::Error),
}

impl<T, W> Journal<T, W>
where
    T: Serialize,
    W: Write,
{
    /// Create a new, empty journal.
    #[inline]
    pub fn new(w: W) -> Self {
        let writer = BufWriter::new(w);
        Self {
            records: Vec::new(),
            writer,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.records.len()
    }

    #[inline]
    pub fn get(&self, idx: usize) -> Option<&Record<T>> {
        self.records.get(idx)
    }

    #[inline]
    pub fn get_back(&self, idx: usize) -> Option<&Record<T>> {
        if self.records.is_empty() || idx >= self.records.len() {
            return None;
        }

        let idx = self.records.len() - idx - 1;
        self.records.get(idx)
    }

    /// Append a new record to the journal.
    #[inline]
    pub fn append(&mut self, record: Record<T>) -> Result<(), JournalError> {
        // Write the record to the output buffer.
        self.write_record(&record)?;
        // Push the record.
        self.records.push(record);

        Ok(())
    }

    /// Flush the journal writer.
    #[inline]
    pub fn flush(&mut self) -> Result<(), JournalError> {
        self.writer.flush()?;
        Ok(())
    }

    #[inline]
    fn write_record(&mut self, record: &Record<T>) -> Result<(), JournalError> {
        let s = self.serialize_record(record)?;
        self.writer.write_all(s.as_bytes())?;

        Ok(())
    }

    #[inline]
    fn serialize_record(&self, record: &Record<T>) -> Result<String, serde_json::Error> {
        serde_json::to_string(record)
    }
}

impl<T, W> Journal<T, W>
where
    T: Serialize + DeserializeOwned,
    W: Write,
{
    /// Populate a journal from an existing one.
    #[inline]
    pub fn load<R>(w: W, r: R) -> Result<Self, JournalError>
    where
        R: Read,
    {
        let mut journal = Self::new(w);

        for line in BufReader::new(r).lines() {
            let line = line?;

            let r = journal.deserialize_record(&line)?;
            journal.append(r)?;
        }

        Ok(journal)
    }

    #[inline]
    fn deserialize_record(&self, s: &str) -> Result<Record<T>, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[derive(Debug)]
pub struct Iter<'j, T> {
    inner: slice::Iter<'j, Record<T>>,
}

impl<T, W> Journal<T, W>
where
    T: Serialize,
    W: Write,
{
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }
}

impl<'j, T> Iter<'j, T> {
    #[inline]
    fn new<W>(journal: &'j Journal<T, W>) -> Self
    where
        T: Serialize,
        W: Write,
    {
        Self {
            inner: journal.records.iter(),
        }
    }
}

impl<'j, T> Iterator for Iter<'j, T>
where
    T: Serialize,
{
    type Item = &'j Record<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j, T> DoubleEndedIterator for Iter<'j, T>
where
    T: Serialize,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<T, W> Journal<T, W>
where
    T: Serialize,
    W: Write,
{
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }
}

#[derive(Debug)]
pub struct IterMut<'j, T> {
    inner: slice::IterMut<'j, Record<T>>,
}

impl<'j, T> IterMut<'j, T> {
    #[inline]
    fn new<W>(journal: &'j mut Journal<T, W>) -> Self
    where
        T: Serialize,
        W: Write,
    {
        Self {
            inner: journal.records.iter_mut(),
        }
    }
}

impl<'j, T> Iterator for IterMut<'j, T>
where
    T: Serialize,
{
    type Item = &'j mut Record<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j, T> DoubleEndedIterator for IterMut<'j, T>
where
    T: Serialize,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

#[derive(Debug)]
pub struct RollbackIter<'j, T, W>
where
    T: Rollback + Clone + Serialize,
    W: Write,
{
    journal: &'j mut Journal<T, W>,
    idx: usize,
}

impl<T, W> Journal<T, W>
where
    T: Rollback + Clone + Serialize,
    W: Write,
{
    /// Rollback until the last commit.
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, T, W> {
        RollbackIter::new(self)
    }
}

impl<'j, T, W> RollbackIter<'j, T, W>
where
    T: Rollback + Clone + Serialize,
    W: Write,
{
    #[inline]
    pub fn new(journal: &'j mut Journal<T, W>) -> Self {
        let idx = journal.size();
        Self { journal, idx }
    }
}

impl<'j, T, W> Iterator for RollbackIter<'j, T, W>
where
    T: Rollback + Clone + Serialize,
    W: Write,
{
    type Item = Result<T, JournalError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx != 0 {
            self.idx -= 1;
        } else {
            return None;
        }

        let action = self
            .journal
            .records
            .get(self.idx)
            .and_then(|data| match data {
                Record::Action(data) => Some(data),
                Record::Commit => None,
            })
            .map(|data| data.rollback())?;

        // Append the rollback record to the journal.
        match self.journal.append(Record::Action(action.clone())) {
            Ok(_) => {}
            Err(err) => return Some(Err(err)),
        }

        Some(Ok(action))
    }
}
