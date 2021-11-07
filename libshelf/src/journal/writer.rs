use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

use serde::{de::DeserializeOwned, Serialize};

use super::{Journal, Record};

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("i/o error")]
    Io(#[from] io::Error),
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
}

impl<T> Journal<T>
where
    T: Serialize,
{
    #[inline]
    pub fn write<W>(&self, w: W, start: usize) -> Result<(), WriteError>
    where
        W: Write,
    {
        const BATCH_FLUSH_SIZE: usize = 5;

        if start >= self.size() {
            return Ok(());
        }

        let mut w = BufWriter::new(w);

        let mut i = 0;
        for record in self.records().iter().skip(start) {
            write_record(record, &mut w)?;
            i += 1;
            if i % BATCH_FLUSH_SIZE == 0 {
                w.flush()?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("i/o error")]
    Io(#[from] io::Error),
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
}

impl<T> Journal<T>
where
    T: DeserializeOwned,
{
    #[inline]
    pub fn load<R>(r: R) -> Result<Self, ReadError>
    where
        R: Read,
    {
        let r = BufReader::new(r);
        let mut journal = Journal::new();
        for line in r.lines() {
            let line = line?;
            let record = read_record(&line)?;
            journal.append(record);
        }

        Ok(journal)
    }
}

#[inline]
fn write_record<T, W>(record: &Record<T>, w: W) -> Result<(), WriteError>
where
    T: Serialize,
    W: Write,
{
    let _ = serde_json::to_writer(w, record)?;
    w.write_all(b"\n");
    Ok(())
}

#[inline]
fn read_record<T>(line: &str) -> Result<Record<T>, ReadError>
where
    T: DeserializeOwned,
{
    let record = serde_json::from_str(line)?;
    Ok(record)
}
