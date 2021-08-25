fn main() {
    #[cfg(not(any(
        feature = "lua54",
        feature = "lua53",
        feature = "lua52",
        feature = "lua51",
        feature = "luajit"
    )))]
    compile_error!("You must enable one of the features: lua54, lua53, lua52, lua51, luajit");
}
