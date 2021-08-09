use std::fmt;

#[macro_export]
macro_rules! fail {
    ($res:expr) => {
        fail!($res, _err => {})
    };
    ($res:expr, $err:ident => $block:block) => {
        match $res {
            Ok(v) => v,
            Err($err) => {
                $block;
                return Err(crate::error::EmptyError);
            }
        }
    };
}

#[derive(Debug)]
pub struct EmptyError;

impl fmt::Display for EmptyError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl std::error::Error for EmptyError {}
