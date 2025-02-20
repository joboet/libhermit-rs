use crate::errno::*;
use crate::synch::recmutex::RecursiveMutex;
use alloc::boxed::Box;

extern "C" fn __sys_recmutex_init(recmutex: *mut *mut RecursiveMutex) -> i32 {
	if recmutex.is_null() {
		return -EINVAL;
	}

	// Create a new boxed recursive mutex and return a pointer to the raw memory.
	let boxed_mutex = Box::new(RecursiveMutex::new());
	unsafe {
		*recmutex = Box::into_raw(boxed_mutex);
	}

	0
}

#[no_mangle]
pub extern "C" fn sys_recmutex_init(recmutex: *mut *mut RecursiveMutex) -> i32 {
	kernel_function!(__sys_recmutex_init(recmutex))
}

extern "C" fn __sys_recmutex_destroy(recmutex: *mut RecursiveMutex) -> i32 {
	if recmutex.is_null() {
		return -EINVAL;
	}

	// Consume the pointer to the raw memory into a Box again
	// and drop the Box to free the associated memory.
	unsafe {
		Box::from_raw(recmutex);
	}

	0
}

#[no_mangle]
pub extern "C" fn sys_recmutex_destroy(recmutex: *mut RecursiveMutex) -> i32 {
	kernel_function!(__sys_recmutex_destroy(recmutex))
}

extern "C" fn __sys_recmutex_lock(recmutex: *mut RecursiveMutex) -> i32 {
	if recmutex.is_null() {
		return -EINVAL;
	}

	let mutex = unsafe { &*recmutex };
	mutex.acquire();

	0
}

#[no_mangle]
pub extern "C" fn sys_recmutex_lock(recmutex: *mut RecursiveMutex) -> i32 {
	kernel_function!(__sys_recmutex_lock(recmutex))
}

extern "C" fn __sys_recmutex_unlock(recmutex: *mut RecursiveMutex) -> i32 {
	if recmutex.is_null() {
		return -EINVAL;
	}

	let mutex = unsafe { &*recmutex };
	mutex.release();

	0
}

#[no_mangle]
pub extern "C" fn sys_recmutex_unlock(recmutex: *mut RecursiveMutex) -> i32 {
	kernel_function!(__sys_recmutex_unlock(recmutex))
}
