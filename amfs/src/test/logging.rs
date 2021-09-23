use std::sync::Mutex;

lazy_static! {
    static ref MUTEX: Mutex<i32> = Mutex::new(0i32);
}

pub fn init_log() {
    let mut lock = MUTEX.lock().unwrap();
    if *lock == 0 {
        use log::LevelFilter;
        use log4rs::append::console::ConsoleAppender;
        use log4rs::config::{Appender, Config, Root};

        let stdout = ConsoleAppender::builder().build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Debug))
            .unwrap();

        log4rs::init_config(config).unwrap();
        *lock = 1;
    }
}
