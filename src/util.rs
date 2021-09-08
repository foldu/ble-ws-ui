pub fn now_local() -> time::OffsetDateTime {
    // FIXME: unsound https://github.com/time-rs/time/issues/293
    #[allow(unused_unsafe)]
    unsafe {
        time::OffsetDateTime::now_local().unwrap()
    }
}
