use minirtos_abi::{SysError, UserMutPtr, UserPtr};

pub(crate) fn read_user<T: Copy>(ptr: UserPtr<T>) -> Result<T, SysError> {
    if ptr.is_null() {
        return Err(SysError::InvalidArgument);
    }

    // ToDo:
    // Validate that the entire object is inside readable user memory.
    let value = unsafe { (ptr.raw() as *const T).read() };

    Ok(value)
}

pub(crate) fn write_user<T: Copy>(ptr: UserMutPtr<T>, value: T) -> Result<(), SysError> {
    if ptr.is_null() {
        return Err(SysError::InvalidArgument);
    }

    // ToDo:
    // Validate that the entire object is inside writable user memory.
    unsafe {
        (ptr.raw() as *mut T).write(value);
    }

    Ok(())
}
