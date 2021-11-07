use std::io::{self, BufWriter, Write};

use serde::Serialize;

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

#[inline]
fn write_record<T, W>(record: &Record<T>, w: W) -> Result<(), WriteError>
where
    T: Serialize,
    W: Write,
{
    let _ = serde_json::to_writer(w, record)?;
    Ok(())
}
