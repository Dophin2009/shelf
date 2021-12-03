#![allow(unused_macros)]

// Utility for creating tl_* macros.
macro_rules! tl_fmt {
    ($level:ident; $color:literal; [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!(["{$", $color, "}==>{/$} ", $($format_str),+] $(, $arg )*)
    };
}

macro_rules! tl_error {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        tl_error!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        tl_fmt!(error; "red"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! tl_info {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        tl_info!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        tl_fmt!(info; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

// Utility for creating sl_* macros.
macro_rules! sl_fmt {
    ($level:ident; $color:literal; [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!(["{:4}", "{$", $color, "}>{/$} ", $($format_str),+], "" $(, $arg )*)
    };
}

macro_rules! sl_error {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_error!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(error; "red"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_warn {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_warn!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(warn; "yellow"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_info {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_info!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(info; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_debug {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_debug!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(debug; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_trace {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_trace!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(trace; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

// Utility for creating sl_* macros.
macro_rules! sl_i_fmt {
    ($level:ident; $color:literal; [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!(["{:4}", "{$", $color, "}>{/$} ", "{:2}", $($format_str),+], "", "" $(, $arg )*)
    };
}

macro_rules! sl_i_error {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_i_error!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_i_fmt!(error; "red"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_i_warn {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_i_warn!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_i_fmt!(warn; "yellow"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_i_info {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_i_info!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_i_fmt!(info; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_i_debug {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_i_debug!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_i_fmt!(debug; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

macro_rules! sl_i_trace {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_i_trace!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_i_fmt!(trace; "dimmed"; [$($format_str),+] $(, $arg )*)
    };
}

// Utility for creating sl_* macros.
macro_rules! idx_fmt {
    ($level:ident; $color:literal; $i:expr, $n:expr, [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!(["{$", $color, "}[{}/{}]{/$} " $(, $format_str)+], $i, $n $(, $arg )*)
    };
}

macro_rules! idx_debug {
    ($i:expr, $n:expr, $format_str:literal $(, $arg:expr)* $(,)?) => {
        idx_debug!($i, $n, [$format_str] $(, $arg )*)
    };
    ($i:expr, $n:expr, [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        idx_fmt!(debug; "dimmed"; $i, $n, [$($format_str),+] $(, $arg )*)
    };
}
