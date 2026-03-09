#[cfg(debug_assertions)]
use parking_lot::Mutex;

#[cfg(debug_assertions)]
lazy_static! {
    pub static ref FRAME_DEBUG_INFO: Mutex<String> = Mutex::new(String::new());
}

#[allow(unused_macros)]
macro_rules! printlnd {
    () => {
        #[cfg(debug_assertions)]
        crate::debug::FRAME_DEBUG_INFO.lock().push('\n');
    };
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        let s = format!($($arg)+);
        #[cfg(debug_assertions)]
        crate::debug::FRAME_DEBUG_INFO.lock().push_str(&s);
        printlnd!();
    };
}

#[allow(unused_macros)]
macro_rules! d {
    () => {
        printlnd!("[{}:{}:{}]", ::std::file!(), ::std::line!(), ::std::column!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                printlnd!("[{}:{}:{}] {} = {:#?}",
                    ::std::file!(),
                    ::std::line!(),
                    ::std::column!(),
                    ::std::stringify!($val),
                    // The `&T: Debug` check happens here (not in the format literal desugaring)
                    // to avoid format literal related messages and suggestions.
                    &&tmp as &dyn ::std::fmt::Debug,
                );
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(d!($val)),+,)
    };
}
