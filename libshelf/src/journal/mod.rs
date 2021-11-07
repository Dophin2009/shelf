pub mod iter;
pub mod rollback;

pub use self::rollback::{Rollback, RollbackIter};

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use super::{Journal, Record};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Datum {
        Forward,
        Backward,
    }

    pub const FORWARD: Record<Datum> = Record::Action(Datum::Forward);
    pub const BACKWARD: Record<Datum> = Record::Action(Datum::Backward);
    pub const COMMIT: Record<Datum> = Record::Commit;

    pub(super) fn mk_expected(
        records: &[Record<Datum>],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let lines: Vec<_> = records
            .iter()
            .map(|record| -> Result<String, serde_json::Error> {
                Ok(format!("{}\n", serde_json::to_string(record)?))
            })
            .collect::<Result<_, _>>()?;
        let expected = lines.join("");
        Ok(expected)
    }

    pub(super) fn assert_writer(expected: &str, journal: &Journal<Datum, &mut Vec<u8>>) {
        assert_eq!(expected.as_bytes(), journal.writer.get_ref().as_slice());
    }

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
}
