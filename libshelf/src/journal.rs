use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::slice;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub trait Rollback<R> {
    fn rollback(&self) -> R;
}

/// Record type to be recorded in a journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

    /// Return the number of records in the journal.
    #[inline]
    pub fn size(&self) -> usize {
        self.records.len()
    }

    /// Return the latest/last appended record.
    #[inline]
    pub fn latest(&self) -> Option<&Record<T>> {
        self.records.last()
    }

    /// Return the oldest/first appended record.
    #[inline]
    pub fn oldest(&self) -> Option<&Record<T>> {
        self.records.first()
    }

    /// Return true if the journal is currently in a pending transaction state (there are more
    /// than zero records, and the latest record is a non-commit record).
    #[inline]
    pub fn in_transaction(&self) -> bool {
        match self.latest().unwrap_or(&Record::Commit) {
            Record::Action(_) => true,
            Record::Commit => false,
        }
    }

    /// Return true if the journal is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Retrieve the record at the given index, where the oldest record has an index of 0.
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&Record<T>> {
        self.records.get(idx)
    }

    /// Retrieve the record of the given index, where the newest record has an index of 0.
    #[inline]
    pub fn get_back(&self, idx: usize) -> Option<&Record<T>> {
        if self.records.is_empty() || idx >= self.records.len() {
            None
        } else {
            let idx = self.records.len() - idx - 1;
            self.records.get(idx)
        }
    }

    /// Return an immutable slice to the records.
    #[inline]
    pub fn records(&self) -> &[Record<T>] {
        &self.records
    }

    /// Append a new record to the journal and write. This immediately flushes the writer, if the
    /// write succeeded (see [`Write`]).
    #[inline]
    pub fn append(&mut self, record: Record<T>) -> Result<(), JournalError> {
        // Write the record to the output buffer.
        self.write_record(&record)?;
        self.writer.flush()?;
        // Push the record.
        self.records.push(record);

        Ok(())
    }

    #[inline]
    fn write_record(&mut self, record: &Record<T>) -> Result<(), JournalError> {
        self.serialize_record(record)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    #[inline]
    fn serialize_record(&mut self, record: &Record<T>) -> Result<(), serde_json::Error> {
        serde_json::to_writer(&mut self.writer, record)
    }
}

impl<T, W> Journal<T, W>
where
    T: Serialize + DeserializeOwned,
    W: Write,
{
    /// Populate a journal from an existing reader.
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

/// Iterator on a [`Journal`] that emits records from oldest to newest.
#[derive(Debug)]
pub struct Iter<'j, T> {
    inner: slice::Iter<'j, Record<T>>,
}

impl<T, W> Journal<T, W>
where
    T: Serialize,
    W: Write,
{
    /// Create an iterator with immutable access on this journal. See [`Iter`].
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }
}

impl<'j, T> Iter<'j, T> {
    /// Initialize a new iterator for the given journal.
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

    /// Retrieve the next record from the journal, returning None if there are no more new records.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'j, T> DoubleEndedIterator for Iter<'j, T>
where
    T: Serialize,
{
    /// Retrieve the next (previous) record from the journal, returning None if the oldest record
    /// has already been returned.
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

/// An iterator that performs rollback on a [`Journal`]. See [`Journal::rollback`] and
/// [`Journal::rollback_last`].
#[derive(Debug)]
pub struct RollbackIter<'j, T, W>
where
    T: Rollback<T> + Clone + Serialize,
    W: Write,
{
    journal: &'j mut Journal<T, W>,

    /// The current record index, where the oldest record has an index of 0.
    idx: usize,

    /// Flag that indicates whether or not any rollback records were appended.
    /// See [`RollbackIter::next`].
    appended: bool,
}

impl<T, W> Journal<T, W>
where
    T: Rollback<T> + Clone + Serialize,
    W: Write,
{
    /// Return a [`RollbackIter`] that rolls-back until the last commit.
    ///
    /// If the latest record is a commit or there are no records, the iterator will do nothing.
    ///
    /// See [`RollbackIter`].
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, T, W> {
        RollbackIter::new(self)
    }

    /// Return a [`RollbackIter`] that rolls-back the last transaction.
    ///
    /// If the latest record is a commit, the iterator will begin rolling back from the
    /// second-to-last commit; otherwise, this returns nothing.
    ///
    /// See [`RollbackIter`].
    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_, T, W>> {
        match self.latest()? {
            Record::Commit => Some(RollbackIter::new_idx(self, 1)),
            _ => None,
        }
    }
}

impl<'j, T, W> RollbackIter<'j, T, W>
where
    T: Rollback<T> + Clone + Serialize,
    W: Write,
{
    /// Create a new rollback iterator at the latest reverse position.
    #[inline]
    fn new(journal: &'j mut Journal<T, W>) -> Self {
        Self::new_idx(journal, 0)
    }

    /// Create a new rollback iterator at the given reverse position.
    #[inline]
    fn new_idx(journal: &'j mut Journal<T, W>, idx: usize) -> Self {
        Self {
            journal,
            idx,
            appended: false,
        }
    }
}

impl<'j, T, W> Iterator for RollbackIter<'j, T, W>
where
    T: Rollback<T> + Clone + Serialize,
    W: Write,
{
    type Item = Result<T, JournalError>;

    /// Look at the next record and perform the following operations depending on the record type:
    /// -   Action: append the record's rollback to the journal and return `Some` with the rollback
    ///             data.
    /// -   Commit or no record: if no rollback records have been appended yet, do nothing and
    ///     return `None`; otherwise, append a commit record to the journal and return `None`.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (data, record) = match self.journal.get_back(self.idx) {
            Some(Record::Action(data)) => {
                let rdata = data.rollback();
                (Some(rdata.clone()), Record::Action(rdata))
            }
            Some(Record::Commit) | None => {
                if !self.appended {
                    return None;
                } else {
                    (None, Record::Commit)
                }
            }
        };

        self.idx += 1;

        // Append the record to the journal.
        match self.journal.append(record) {
            Ok(_) => {
                self.idx += 1;
                match data {
                    Some(data) => {
                        self.appended = true;
                        Some(Ok(data))
                    }
                    None => None,
                }
            }
            Err(err) => Some(Err(err)),
        }
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use super::{Journal, Record, Rollback};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Datum {
        Forward,
        Backward,
    }

    impl Rollback<Datum> for Datum {
        #[inline]
        fn rollback(&self) -> Self {
            match self {
                Datum::Forward => Self::Backward,
                Datum::Backward => Self::Forward,
            }
        }
    }

    const FORWARD: Record<Datum> = Record::Action(Datum::Forward);
    const BACKWARD: Record<Datum> = Record::Action(Datum::Backward);
    const COMMIT: Record<Datum> = Record::Commit;

    #[test]
    fn test_size() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert_eq!(0, journal.size());

        journal.append(FORWARD)?;
        assert_eq!(1, journal.size());

        journal.append(BACKWARD)?;
        assert_eq!(2, journal.size());

        journal.append(COMMIT)?;
        assert_eq!(3, journal.size());

        Ok(())
    }

    #[test]
    fn test_latest() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert_eq!(None, journal.latest());

        journal.append(FORWARD)?;
        assert_eq!(Some(&FORWARD), journal.latest());

        journal.append(BACKWARD)?;
        assert_eq!(Some(&BACKWARD), journal.latest());

        journal.append(COMMIT)?;
        assert_eq!(Some(&COMMIT), journal.latest());

        Ok(())
    }

    #[test]
    fn test_oldest() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert_eq!(None, journal.oldest());

        journal.append(FORWARD)?;
        assert_eq!(Some(&FORWARD), journal.oldest());

        journal.append(BACKWARD)?;
        assert_eq!(Some(&FORWARD), journal.oldest());

        journal.append(COMMIT)?;
        assert_eq!(Some(&FORWARD), journal.oldest());

        Ok(())
    }

    #[test]
    fn test_in_transaction() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert!(!journal.in_transaction());

        journal.append(FORWARD)?;
        assert!(journal.in_transaction());

        journal.append(BACKWARD)?;
        assert!(journal.in_transaction());

        journal.append(COMMIT)?;
        assert!(!journal.in_transaction());

        journal.append(BACKWARD)?;
        assert!(journal.in_transaction());

        Ok(())
    }

    #[test]
    fn test_get() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert_eq!(None, journal.get(0));

        journal.append(FORWARD)?;
        assert_eq!(Some(&FORWARD), journal.get(0));

        journal.append(BACKWARD)?;
        assert_eq!(Some(&BACKWARD), journal.get(1));

        journal.append(COMMIT)?;
        assert_eq!(Some(&COMMIT), journal.get(2));

        Ok(())
    }

    #[test]
    fn test_get_back() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        assert_eq!(None, journal.get_back(0));

        journal.append(FORWARD)?;
        assert_eq!(Some(&FORWARD), journal.get_back(0));

        journal.append(BACKWARD)?;
        assert_eq!(Some(&BACKWARD), journal.get_back(0));
        assert_eq!(Some(&FORWARD), journal.get_back(1));

        journal.append(COMMIT)?;
        assert_eq!(Some(&COMMIT), journal.get_back(0));
        assert_eq!(Some(&BACKWARD), journal.get_back(1));
        assert_eq!(Some(&FORWARD), journal.get_back(2));
        assert_eq!(None, journal.get_back(3));

        Ok(())
    }

    #[test]
    fn test_push() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        let mut records = Vec::new();

        records.push(FORWARD);
        journal.append(FORWARD)?;
        assert_eq!(&records, journal.records());
        assert_writer(&mk_expected(&records)?, &journal);

        records.push(FORWARD);
        journal.append(FORWARD)?;
        assert_eq!(&records, journal.records());
        assert_writer(&mk_expected(&records)?, &journal);

        records.push(BACKWARD);
        journal.append(BACKWARD)?;
        assert_eq!(&records, journal.records());
        assert_writer(&mk_expected(&records)?, &journal);

        records.push(COMMIT);
        journal.append(COMMIT)?;
        assert_eq!(&records, journal.records());
        assert_writer(&mk_expected(&records)?, &journal);

        Ok(())
    }

    #[test]
    fn test_load() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);

        journal.append(FORWARD)?;
        journal.append(FORWARD)?;
        journal.append(BACKWARD)?;
        journal.append(BACKWARD)?;
        journal.append(COMMIT)?;
        journal.append(FORWARD)?;
        journal.append(COMMIT)?;
        journal.append(BACKWARD)?;
        journal.append(COMMIT)?;

        drop(journal);

        let records = &[
            FORWARD, FORWARD, BACKWARD, BACKWARD, COMMIT, FORWARD, COMMIT, BACKWARD, COMMIT,
        ];

        let mut loaded_writer = Vec::new();
        let loaded = Journal::load(&mut loaded_writer, &*writer)?;
        assert_eq!(records, loaded.records());
        assert_writer(&mk_expected(records)?, &loaded);

        Ok(())
    }

    #[test]
    fn test_iter_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let journal: Journal<Datum, _> = Journal::new(&mut writer);

        assert_eq!(None, journal.iter().next());

        Ok(())
    }

    #[test]
    fn test_iter_forward() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal: Journal<Datum, _> = Journal::new(&mut writer);

        journal.append(FORWARD)?;
        journal.append(BACKWARD)?;
        journal.append(COMMIT)?;

        let mut iter = journal.iter();
        assert_eq!(Some(&FORWARD), iter.next());
        assert_eq!(Some(&BACKWARD), iter.next());
        assert_eq!(Some(&COMMIT), iter.next());
        assert_eq!(None, iter.next());

        Ok(())
    }

    #[test]
    fn test_iter_backward() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal: Journal<Datum, _> = Journal::new(&mut writer);

        journal.append(FORWARD)?;
        journal.append(BACKWARD)?;
        journal.append(COMMIT)?;

        let mut iter = journal.iter();
        assert_eq!(Some(&COMMIT), iter.next_back());
        assert_eq!(Some(&BACKWARD), iter.next_back());
        assert_eq!(Some(&FORWARD), iter.next_back());
        assert_eq!(None, iter.next_back());

        Ok(())
    }

    #[test]
    fn test_rollback_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal: Journal<Datum, _> = Journal::new(&mut writer);

        // No records; rollback does nothing.
        let mut rollback = journal.rollback();
        assert!(rollback.next().is_none());
        assert!(journal.is_empty());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_only() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);

        journal.append(COMMIT)?;

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());
        assert_eq!(&[COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_double() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);

        journal.append(COMMIT)?;
        journal.append(COMMIT)?;

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());
        assert_eq!(&[COMMIT, COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_no_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        let mut records = Vec::new();

        records.push(FORWARD);
        journal.append(FORWARD)?;

        let mut rollback = journal.rollback();

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Datum::Backward, rollback.next().unwrap()?);
        records.push(BACKWARD);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        // No more rollback to be done; None.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        // Same as last
        assert!(rollback.next().is_none());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_last() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);

        journal.append(FORWARD)?;
        journal.append(COMMIT)?;

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());
        assert_eq!(&[FORWARD, COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_after_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        let mut records = vec![FORWARD, COMMIT, FORWARD, BACKWARD];

        journal.append(FORWARD)?;
        journal.append(COMMIT)?;
        journal.append(FORWARD)?;
        journal.append(BACKWARD)?;

        let mut rollback = journal.rollback();

        // Rollback should push a FOWARD record to the journal on next.
        assert_eq!(Datum::Forward, rollback.next().unwrap()?);
        records.push(FORWARD);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Datum::Backward, rollback.next().unwrap()?);
        records.push(BACKWARD);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        // No more rollback to be done; None.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        Ok(())
    }

    #[test]
    fn test_rollback_last_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal: Journal<Datum, _> = Journal::new(&mut writer);

        // No records; no rollback iter.
        let rollback = journal.rollback_last();
        assert!(rollback.is_none());

        Ok(())
    }

    #[test]
    fn test_rollback_last_non_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);
        journal.append(FORWARD)?;

        // Latest is not commit; no rollback iter.
        let rollback = journal.rollback_last();
        assert!(rollback.is_none());

        Ok(())
    }

    #[test]
    fn test_rollback_last_normal() -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Vec::new();
        let mut journal = Journal::new(&mut writer);

        let mut records = vec![FORWARD, COMMIT];
        journal.append(FORWARD)?;
        journal.append(COMMIT)?;

        // Latest is commit; rollback iter.
        let mut rollback = journal.rollback_last().unwrap();

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Datum::Backward, rollback.next().unwrap()?);
        records.push(BACKWARD);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        // End of transaction; rollback should return none.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);
        assert_writer(&mk_expected(&records)?, &rollback.journal);

        Ok(())
    }

    fn mk_expected(records: &[Record<Datum>]) -> Result<String, Box<dyn std::error::Error>> {
        let lines: Vec<_> = records
            .iter()
            .map(|record| -> Result<String, serde_json::Error> {
                Ok(format!("{}\n", serde_json::to_string(record)?))
            })
            .collect::<Result<_, _>>()?;
        let expected = lines.join("");
        Ok(expected)
    }

    fn assert_writer(expected: &str, journal: &Journal<Datum, &mut Vec<u8>>) {
        assert_eq!(expected.as_bytes(), journal.writer.get_ref().as_slice());
    }
}
