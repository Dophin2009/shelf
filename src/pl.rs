#![allow(unused_macros)]
// Utility for creating tl_* macros.
macro_rules! tl_fmt {
    ($level:ident; [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!([$($format_str),+] $(, $arg )*)
    };
}

macro_rules! tl_error {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        tl_error!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        tl_fmt!(error; ["{$red}==>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

macro_rules! tl_info {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        tl_info!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        tl_fmt!(info; ["{$dimmed}==>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

// Utility for creating sl_* macros.
macro_rules! sl_fmt {
    ($level:ident; [$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        bunt_logger::$level!(["{:4}" $(, $format_str)+], "" $(, $arg )*)
    };
}

macro_rules! sl_error {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_error!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(error; ["{$red}>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

macro_rules! sl_warn {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_warn!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(warn; ["{$yellow}>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

macro_rules! sl_info {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_info!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(info; ["{$dimmed}>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

macro_rules! sl_debug {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_debug!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(debug; ["{$dimmed}>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}

macro_rules! sl_trace {
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        sl_trace!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_fmt!(trace; ["{$dimmed}>{/$} " $(, $format_str)+] $(, $arg )*)
    };
}
