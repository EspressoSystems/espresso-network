pub fn output_success(msg: impl AsRef<str>) {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::info!("{}", msg.as_ref());
    } else {
        println!("{}", msg.as_ref());
    }
}

pub fn output_error(msg: impl AsRef<str>) -> ! {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::error!("{}", msg.as_ref());
    } else {
        eprintln!("{}", msg.as_ref());
    }
    std::process::exit(1);
}
