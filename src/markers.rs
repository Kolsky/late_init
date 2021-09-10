use core::marker::PhantomData;
use core::ptr;

pub struct InitSt<T, const IS_INIT: bool>(pub PhantomData<T>);

pub trait Uninit {}

pub trait Init {}

impl<T> Uninit for InitSt<T, false> {}

impl<T> Init for InitSt<T, true> {}

pub trait AutoInit {
    type T;
    unsafe fn init(self, dst: *mut Self::T);
}

impl<T: Default> AutoInit for InitSt<T, false> {
    type T = T;

    unsafe fn init(self, dst: *mut T) {
        ptr::write(dst, T::default())
    }
}

impl<T> AutoInit for InitSt<T, true> {
    type T = T;

    unsafe fn init(self, _: *mut T) {}
}