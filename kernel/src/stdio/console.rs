use crate::{
    interface::{IoAll, IoRead, IoWrite},
    synchronization::{InitStateLock, ReadWriteEx},
};

/// Console placeholder
struct NullConsole;

impl IoWrite for NullConsole {
    fn write_char(&self, _c: char) {}

    fn write_fmt(&self, _args: core::fmt::Arguments) -> core::fmt::Result {
        core::fmt::Result::Ok(())
    }

    fn flush(&self) {}
}

impl IoRead for NullConsole {
    fn clear_input(&self) {}

    fn read_char(&self) -> char {
        ' '
    }

    fn try_read_char(&self) -> Option<char> {
        None
    }
}

static NULL_CONSOLE: NullConsole = NullConsole {};

static CURR_CONSOLE: InitStateLock<&'static (dyn IoAll + Sync)> = InitStateLock::new(&NULL_CONSOLE);

/// Register a new console.
pub fn register_console(new_console: &'static (dyn IoAll + Sync)) {
    CURR_CONSOLE.write(|con| *con = new_console);
}

/// Return a reference to the console.
pub(crate) fn console() -> &'static dyn IoAll {
    CURR_CONSOLE.read(|con| *con)
}
