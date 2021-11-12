pub mod iter;
pub mod rollback;
pub mod transaction;
pub mod writer;

pub use self::rollback::{Rollback, RollbackIter};
pub use self::transaction::Transaction;

use serde::{Deserialize, Serialize};

/// Record type to be recorded in a journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Record<T> {
    Atom(T),
    Commit,
}

/// Write-ahead logging.
#[derive(Debug)]
pub struct Journal<T> {
    records: Vec<Record<T>>,
}

impl<T> Journal<T> {
    /// Create a new, empty journal.
    #[inline]
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
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

    /// Append a new record to the journal.
    #[inline]
    pub(self) fn append(&mut self, record: Record<T>) {
        // Push the record.
        self.records.push(record);
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

    pub const FORWARD: Record<Datum> = Record::Atom(Datum::Forward);
    pub const BACKWARD: Record<Datum> = Record::Atom(Datum::Backward);
    pub const COMMIT: Record<Datum> = Record::Commit;

    #[test]
    fn test_size() {
        let mut journal = Journal::new();
        assert_eq!(0, journal.size());

        journal.append(FORWARD);
        assert_eq!(1, journal.size());

        journal.append(BACKWARD);
        assert_eq!(2, journal.size());

        journal.append(COMMIT);
        assert_eq!(3, journal.size());
    }

    #[test]
    fn test_latest() {
        let mut journal = Journal::new();
        assert_eq!(None, journal.latest());

        journal.append(FORWARD);
        assert_eq!(Some(&FORWARD), journal.latest());

        journal.append(BACKWARD);
        assert_eq!(Some(&BACKWARD), journal.latest());

        journal.append(COMMIT);
        assert_eq!(Some(&COMMIT), journal.latest());
    }

    #[test]
    fn test_oldest() {
        let mut journal = Journal::new();
        assert_eq!(None, journal.oldest());

        journal.append(FORWARD);
        assert_eq!(Some(&FORWARD), journal.oldest());

        journal.append(BACKWARD);
        assert_eq!(Some(&FORWARD), journal.oldest());

        journal.append(COMMIT);
        assert_eq!(Some(&FORWARD), journal.oldest());
    }

    #[test]
    fn test_get() {
        let mut journal = Journal::new();
        assert_eq!(None, journal.get(0));

        journal.append(FORWARD);
        assert_eq!(Some(&FORWARD), journal.get(0));

        journal.append(BACKWARD);
        assert_eq!(Some(&BACKWARD), journal.get(1));

        journal.append(COMMIT);
        assert_eq!(Some(&COMMIT), journal.get(2));
    }

    #[test]
    fn test_get_back() {
        let mut journal = Journal::new();
        assert_eq!(None, journal.get_back(0));

        journal.append(FORWARD);
        assert_eq!(Some(&FORWARD), journal.get_back(0));

        journal.append(BACKWARD);
        assert_eq!(Some(&BACKWARD), journal.get_back(0));
        assert_eq!(Some(&FORWARD), journal.get_back(1));

        journal.append(COMMIT);
        assert_eq!(Some(&COMMIT), journal.get_back(0));
        assert_eq!(Some(&BACKWARD), journal.get_back(1));
        assert_eq!(Some(&FORWARD), journal.get_back(2));
        assert_eq!(None, journal.get_back(3));
    }

    #[test]
    fn test_push() {
        let mut journal = Journal::new();
        let mut records = Vec::new();

        records.push(FORWARD);
        journal.append(FORWARD);
        assert_eq!(&records, journal.records());

        records.push(FORWARD);
        journal.append(FORWARD);
        assert_eq!(&records, journal.records());

        records.push(BACKWARD);
        journal.append(BACKWARD);
        assert_eq!(&records, journal.records());

        records.push(COMMIT);
        journal.append(COMMIT);
        assert_eq!(&records, journal.records());
    }
}
