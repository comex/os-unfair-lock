#![cfg(target_os = "macos")]
#![no_std]
#![cfg_attr(feature = "nightly", feature(coerce_unsized, unsize))]

use core::cell::UnsafeCell;
use core::default::Default;
use core::fmt::{self, Debug, Display, Formatter};
use core::ops::{Deref, DerefMut, Drop};

#[allow(non_camel_case_types)]
type os_unfair_lock = u32;

const OS_UNFAIR_LOCK_INIT: os_unfair_lock = 0;

extern "C" {
    // part of libSystem, no link needed
    fn os_unfair_lock_lock(lock: &os_unfair_lock);
    fn os_unfair_lock_unlock(lock: &os_unfair_lock);
    fn os_unfair_lock_trylock(lock: &os_unfair_lock) -> u8;
    fn os_unfair_lock_assert_not_owner(lock: &os_unfair_lock);
}

pub struct Mutex<T: ?Sized> {
    pub lock: os_unfair_lock,
    pub cell: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: ?Sized> {
    pub mutex: &'a Mutex<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T: ?Sized> Mutex<T> {
    #[inline]
    pub const fn new(value: T) -> Self
    where
        T: Sized,
    {
        Mutex {
            lock: OS_UNFAIR_LOCK_INIT,
            cell: UnsafeCell::new(value),
        }
    }
    #[inline]
    pub fn lock<'a>(&'a self) -> MutexGuard<'a, T> {
        unsafe {
            os_unfair_lock_lock(&self.lock);
        }
        MutexGuard { mutex: self }
    }
    #[inline]
    pub fn try_lock<'a>(&'a self) -> Option<MutexGuard<'a, T>> {
        let ok = unsafe { os_unfair_lock_trylock(&self.lock) };
        if ok != 0 {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }
    #[inline]
    pub fn assert_not_owner(&self) {
        unsafe {
            os_unfair_lock_assert_not_owner(&self.lock);
        }
    }
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.cell.into_inner()
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.cell.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.cell.get() }
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            os_unfair_lock_unlock(&self.mutex.lock);
        }
    }
}

// extra impls: Mutex

impl<T: ?Sized + Default> Default for Mutex<T> {
    #[inline]
    fn default() -> Self {
        Mutex::new(T::default())
    }
}

impl<T: ?Sized + Debug> Debug for Mutex<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.lock().fmt(f)
    }
}

impl<T: ?Sized + Display> Display for Mutex<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.lock().fmt(f)
    }
}

impl<T> From<T> for Mutex<T> {
    #[inline]
    fn from(t: T) -> Mutex<T> {
        Mutex::new(t)
    }
}

#[cfg(feature = "nightly")]
impl<T, U> core::ops::CoerceUnsized<Mutex<U>> for Mutex<T> where T: core::ops::CoerceUnsized<U> {}

// extra impls: MutexGuard

impl<'a, T: ?Sized + Debug> Debug for MutexGuard<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: ?Sized + Display> Display for MutexGuard<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

#[cfg(feature = "nightly")]
impl<'a, T: ?Sized, U: ?Sized> core::ops::CoerceUnsized<MutexGuard<'a, U>> for MutexGuard<'a, T> where
    T: core::marker::Unsize<U>
{
}

#[cfg(test)]
mod tests {
    use super::Mutex;
    const TEST_CONST: Mutex<u32> = Mutex::new(42);
    #[test]
    fn basics() {
        let m = TEST_CONST;
        *m.lock() += 1;
        *m.try_lock().unwrap() += 1;
        m.assert_not_owner();
        assert_eq!(*m.lock(), 44);
        assert_eq!(m.into_inner(), 44);
    }
    #[test]
    #[cfg(feature = "nightly")]
    fn unsize() {
        use super::MutexGuard;
        let m: Mutex<[u8; 1]> = Mutex::new([100]);
        (&m as &Mutex<[u8]>).lock()[0] += 1;
        (m.lock() as MutexGuard<'_, [u8]>)[0] += 1;
        let n: Mutex<&'static [u8; 1]> = Mutex::new(&[200]);
        let _: Mutex<&'static [u8]> = n;
    }
}
