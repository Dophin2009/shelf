use std::io;
use std::path::PathBuf;

use super::{Finish, Rollback, ShouldFinish};

#[derive(Debug, Clone)]
pub struct WriteOp {
    path: PathBuf,
    contents: String,
}

impl Finish for WriteOp {
    type Output = ();
    type Error = io::Error;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        todo!()
    }
}

impl ShouldFinish for WriteOp {
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        todo!()
    }
}

impl Rollback for WriteOp {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
