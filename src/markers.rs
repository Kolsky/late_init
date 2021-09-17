pub struct InitSt<T, const IS_INIT: bool>(pub *mut T);

pub trait Uninit {}

pub trait Init {}

impl<T> Uninit for InitSt<T, false> {}

impl<T> Init for InitSt<T, true> {}

pub trait AutoInit: Sized {
    unsafe fn init(self) {}
}

impl<T: Default> AutoInit for InitSt<T, false> {
    unsafe fn init(self) {
        core::ptr::write(self.0, T::default())
    }
}

impl<T> AutoInit for InitSt<T, true> {}