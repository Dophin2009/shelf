use super::{Journal, Record};

pub trait Rollback {
    type Output;

    fn rollback(&self) -> Self::Output;
}

/// An iterator that performs rollback on a [`Journal`]. See [`Journal::rollback`] and
/// [`Journal::rollback_last`].
#[derive(Debug)]
pub struct RollbackIter<'j, T>
where
    T: Rollback<Output = T> + Clone,
{
    journal: &'j mut Journal<T>,

    /// The current record index, where the oldest record has an index of 0.
    idx: usize,

    /// Flag that indicates whether or not any rollback records were appended.
    /// See [`RollbackIter::next`].
    appended: bool,

    /// Flag that indicates rollback is finished.
    done: bool,
}

impl<T> Journal<T>
where
    T: Rollback<Output = T> + Clone,
{
    /// Return a [`RollbackIter`] that rolls-back until the last commit.
    ///
    /// If the latest record is a commit or there are no records, the iterator will do nothing.
    ///
    /// See [`RollbackIter`].
    #[inline]
    pub fn rollback(&mut self) -> RollbackIter<'_, T> {
        RollbackIter::new(self)
    }

    /// Return a [`RollbackIter`] that rolls-back the last transaction.
    ///
    /// If the latest record is a commit, the iterator will begin rolling back from the
    /// second-to-last commit; otherwise, this returns nothing.
    ///
    /// See [`RollbackIter`].
    #[inline]
    pub fn rollback_last(&mut self) -> Option<RollbackIter<'_, T>> {
        match self.latest()? {
            Record::Commit => Some(RollbackIter::new_idx(self, 1)),
            _ => None,
        }
    }
}

impl<'j, T> RollbackIter<'j, T>
where
    T: Rollback<Output = T> + Clone,
{
    /// Create a new rollback iterator at the latest reverse position.
    #[inline]
    fn new(journal: &'j mut Journal<T>) -> Self {
        Self::new_idx(journal, 0)
    }

    /// Create a new rollback iterator at the given reverse position.
    #[inline]
    fn new_idx(journal: &'j mut Journal<T>, idx: usize) -> Self {
        Self {
            journal,
            idx,
            appended: false,
            done: false,
        }
    }

    /// Perform all record rollbacks and return the action values in a `Vec`.
    #[inline]
    pub fn consume(self) -> Vec<T> {
        self.collect()
    }
}

impl<'j, T> Iterator for RollbackIter<'j, T>
where
    T: Rollback<Output = T> + Clone,
{
    type Item = T;

    /// Look at the next record and perform the following operations depending on the record type:
    /// -   Action: append the record's rollback to the journal and return `Some` with the rollback
    ///             data.
    /// -   Commit or no record: if no rollback records have been appended yet, do nothing and
    ///     return `None`; otherwise, append a commit record to the journal and return `None`.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        };

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
        self.journal.append(record);
        self.idx += 1;
        match data {
            Some(data) => {
                self.appended = true;
                Some(data)
            }
            None => {
                self.done = true;
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::test::{Datum, BACKWARD, COMMIT, FORWARD};
    use super::{Journal, Rollback};

    impl Rollback for Datum {
        type Output = Datum;

        #[inline]
        fn rollback(&self) -> Self::Output {
            match self {
                Datum::Forward => Self::Backward,
                Datum::Backward => Self::Forward,
            }
        }
    }

    #[test]
    fn test_rollback_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal: Journal<Datum> = Journal::new();

        // No records; rollback does nothing.
        let mut rollback = journal.rollback();
        assert!(rollback.next().is_none());
        assert!(journal.is_empty());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_only() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();
        journal.append(COMMIT);

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());
        assert_eq!(&[COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_double() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();

        journal.append(COMMIT);
        journal.append(COMMIT);

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());
        assert_eq!(&[COMMIT, COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_no_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();
        let mut records = Vec::new();

        records.push(FORWARD);
        journal.append(FORWARD);

        let mut rollback = journal.rollback();

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Some(Datum::Backward), rollback.next());
        records.push(BACKWARD);

        // No more rollback to be done; None.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);

        // Same as last
        assert!(rollback.next().is_none());

        // Assert that records are correct.
        assert_eq!(&records, journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_commit_last() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();

        journal.append(FORWARD);
        journal.append(COMMIT);

        let mut rollback = journal.rollback();

        // Rollback should do nothing.
        assert!(rollback.next().is_none());

        // Assert that records are correct.
        assert_eq!(&[FORWARD, COMMIT], journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_after_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();
        let mut records = vec![FORWARD, COMMIT, FORWARD, BACKWARD];

        journal.append(FORWARD);
        journal.append(COMMIT);
        journal.append(FORWARD);
        journal.append(BACKWARD);

        let mut rollback = journal.rollback();

        // Rollback should push a FOWARD record to the journal on next.
        assert_eq!(Some(Datum::Forward), rollback.next());
        records.push(FORWARD);

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Some(Datum::Backward), rollback.next());
        records.push(BACKWARD);

        // No more rollback to be done; None.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);

        // Assert that records are correct.
        assert_eq!(&records, journal.records());

        Ok(())
    }

    #[test]
    fn test_rollback_last_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal: Journal<Datum> = Journal::new();

        // No records; no rollback iter.
        let rollback = journal.rollback_last();
        assert!(rollback.is_none());

        Ok(())
    }

    #[test]
    fn test_rollback_last_non_commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();
        journal.append(FORWARD);

        // Latest is not commit; no rollback iter.
        let rollback = journal.rollback_last();
        assert!(rollback.is_none());

        Ok(())
    }

    #[test]
    fn test_rollback_last_normal() -> Result<(), Box<dyn std::error::Error>> {
        let mut journal = Journal::new();

        let mut records = vec![FORWARD, COMMIT];
        journal.append(FORWARD);
        journal.append(COMMIT);

        // Latest is commit; rollback iter.
        let mut rollback = journal.rollback_last().unwrap();

        // Rollback should push a BACKWARD record to the journal on next.
        assert_eq!(Some(Datum::Backward), rollback.next());
        records.push(BACKWARD);

        // End of transaction; rollback should return none.
        assert!(rollback.next().is_none());
        // COMMIT record should have been pushed.
        records.push(COMMIT);

        // Assert that records are correct.
        assert_eq!(&records, journal.records());

        Ok(())
    }
}
