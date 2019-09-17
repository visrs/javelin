macro_rules! session_log {
    ($session:ident, $level:expr, $fmt:expr) => {
        session_log!($session, $level, $fmt,)
    };
    ($session:ident, $level:expr, $fmt:expr, $($arg:tt)*) => {
        log!($level, concat!("RTMP Session {}: ", $fmt), $session.id, $($arg)*)
    };
}

macro_rules! session_info {
    ($session:ident, $($arg:tt)+) => { session_log!($session, log::Level::Info, $($arg)+) }
}

macro_rules! session_debug {
    ($session:ident, $($arg:tt)+) => { session_log!($session, log::Level::Debug, $($arg)*) };
}
