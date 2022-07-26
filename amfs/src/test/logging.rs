use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    static ref MUTEX: Mutex<i32> = Mutex::new(0i32);
}

pub fn init_log() {
    let mut lock = MUTEX.lock().unwrap();
    if *lock == 0 {
        use log::LevelFilter;
        use log4rs::{
            append::console::ConsoleAppender,
            config::{Appender, Config, Root},
            encode::pattern::PatternEncoder,
        };

        let encoder = PatternEncoder::new("{h({l:>5})} {t:.<25} - {m}{n}");

        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(encoder))
            .build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Debug))
            .unwrap();

        log4rs::init_config(config).unwrap();
        *lock = 1;
    }
}
