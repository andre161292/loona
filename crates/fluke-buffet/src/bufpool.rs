use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
    marker::PhantomData,
    ops::{self, Bound, RangeBounds},
};

use memmap2::MmapMut;

pub type BufResult<T, B> = (std::io::Result<T>, B);

pub const BUF_SIZE: u16 = 4096;

#[cfg(not(feature = "miri"))]
pub const NUM_BUF: u32 = 64 * 1024;

#[cfg(feature = "miri")]
pub const NUM_BUF: u32 = 64;

thread_local! {
    pub static BUF_POOL: BufPool = const { BufPool::new_empty(BUF_SIZE, NUM_BUF) };
    static BUF_POOL_DESTRUCTOR: RefCell<Option<MmapMut>> = const { RefCell::new(None) };
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not mmap buffer")]
    Mmap(#[from] std::io::Error),

    #[error("out of memory")]
    OutOfMemory,
}

/// A buffer pool
pub(crate) struct BufPool {
    buf_size: u16,
    num_buf: u32,
    inner: RefCell<Option<BufPoolInner>>,
}

struct BufPoolInner {
    // this is tied to an [MmapMut] that gets deallocated at thread exit
    // thanks to [BUF_POOL_DESTRUCTOR]
    ptr: *mut u8,

    // index of free blocks
    free: VecDeque<u32>,

    // ref counts start as all zeroes, get incremented when a block is borrowed
    ref_counts: Vec<i16>,
}

impl BufPool {
    pub(crate) const fn new_empty(buf_size: u16, num_buf: u32) -> BufPool {
        BufPool {
            buf_size,
            num_buf,
            inner: RefCell::new(None),
        }
    }

    pub(crate) fn alloc(&self) -> Result<BufMut> {
        let mut inner = self.borrow_mut()?;

        if let Some(index) = inner.free.pop_front() {
            inner.ref_counts[index as usize] += 1;
            Ok(BufMut {
                index,
                off: 0,
                len: self.buf_size as _,
                _non_send: PhantomData,
            })
        } else {
            Err(Error::OutOfMemory)
        }
    }

    fn inc(&self, index: u32) {
        let mut inner = self.inner.borrow_mut();
        let inner = inner.as_mut().unwrap();

        inner.ref_counts[index as usize] += 1;
    }

    fn dec(&self, index: u32) {
        let mut inner = self.inner.borrow_mut();
        let inner = inner.as_mut().unwrap();

        inner.ref_counts[index as usize] -= 1;
        if inner.ref_counts[index as usize] == 0 {
            inner.free.push_back(index);
        }
    }

    #[cfg(test)]
    pub(crate) fn num_free(&self) -> Result<usize> {
        Ok(self.borrow_mut()?.free.len())
    }

    fn borrow_mut(&self) -> Result<RefMut<BufPoolInner>> {
        let mut inner = self.inner.borrow_mut();
        if inner.is_none() {
            let len = self.num_buf as usize * self.buf_size as usize;

            let ptr: *mut u8;

            #[cfg(feature = "miri")]
            {
                let mut map = vec![0; len];
                ptr = map.as_mut_ptr();
                std::mem::forget(map);
            }

            #[cfg(not(feature = "miri"))]
            {
                let mut map = memmap2::MmapOptions::new().len(len).map_anon()?;
                ptr = map.as_mut_ptr();
                BUF_POOL_DESTRUCTOR.with(|destructor| {
                    *destructor.borrow_mut() = Some(map);
                });
            }

            let mut free = VecDeque::with_capacity(self.num_buf as usize);
            for i in 0..self.num_buf {
                free.push_back(i);
            }
            let ref_counts = vec![0; self.num_buf as usize];

            *inner = Some(BufPoolInner {
                ptr,
                free,
                ref_counts,
            });
        }

        let r = RefMut::map(inner, |o| o.as_mut().unwrap());
        Ok(r)
    }

    /// Returns the base pointer for a block
    ///
    /// # Safety
    ///
    /// Borrow-checking is on you!
    #[inline(always)]
    unsafe fn base_ptr(&self, index: u32) -> *mut u8 {
        let start = index as usize * self.buf_size as usize;
        self.inner.borrow_mut().as_mut().unwrap().ptr.add(start)
    }
}

/// A mutable buffer. Cannot be cloned, but can be written to
pub struct BufMut {
    pub(crate) index: u32,
    pub(crate) off: u16,
    pub(crate) len: u16,

    // makes this type non-Send, which we do want
    _non_send: PhantomData<*mut ()>,
}

impl BufMut {
    #[inline(always)]
    pub fn alloc() -> Result<BufMut, Error> {
        BUF_POOL.with(|bp| bp.alloc())
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len as _
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Turn this buffer immutable. The reference count doesn't change, but the
    /// immutable view can be cloned.
    #[inline]
    pub fn freeze(self) -> Buf {
        let b = Buf {
            index: self.index,
            off: self.off,
            len: self.len,

            _non_send: PhantomData,
        };

        std::mem::forget(self); // don't decrease ref count

        b
    }

    /// Dangerous: freeze a slice of this. Must only be used if you can
    /// guarantee this portion won't be written to anymore.
    pub(crate) fn freeze_slice(&self, range: impl RangeBounds<usize>) -> Buf {
        let b = Buf {
            index: self.index,
            off: self.off,
            len: self.len,

            _non_send: PhantomData,
        };

        b.slice(range)
    }

    /// Split this buffer in twain. Both parts can be written to.  Panics if
    /// `at` is out of bounds.
    #[inline]
    pub fn split_at(self, at: usize) -> (Self, Self) {
        assert!(at <= self.len as usize);

        let left = BufMut {
            index: self.index,
            off: self.off,
            len: at as _,

            _non_send: PhantomData,
        };

        let right = BufMut {
            index: self.index,
            off: self.off + at as u16,
            len: (self.len - at as u16),

            _non_send: PhantomData,
        };

        std::mem::forget(self); // don't decrease ref count
        BUF_POOL.with(|bp| bp.inc(left.index)); // in fact, increase it by 1

        (left, right)
    }

    /// Skip over the first `n` bytes, panics if out of bound
    pub fn skip(&mut self, n: usize) {
        assert!(n <= self.len as usize);

        let u16_n: u16 = n.try_into().unwrap();
        self.off += u16_n;
        self.len -= u16_n;
    }
}

impl ops::Deref for BufMut {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                BUF_POOL.with(|bp| bp.base_ptr(self.index).add(self.off as _)),
                self.len as _,
            )
        }
    }
}

impl ops::DerefMut for BufMut {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(
                BUF_POOL.with(|bp| bp.base_ptr(self.index).add(self.off as _)),
                self.len as _,
            )
        }
    }
}

mod iobufmut {
    use crate::{ReadInto, RollMut};

    use super::BufMut;
    pub trait Sealed {}
    impl Sealed for BufMut {}
    impl Sealed for RollMut {}
    impl Sealed for ReadInto {}
    impl Sealed for Vec<u8> {}
}

/// The IoBufMut trait is implemented by buffer types that can be passed to
/// io-uring operations.
///
/// # Safety
///
/// If the address returned by `io_buf_mut_stable_mut_ptr` is not actually stable
/// and moves while an io_uring operation is in-flight, the kernel might write
/// to the wrong memory location.
pub unsafe trait IoBufMut: iobufmut::Sealed {
    /// Gets a pointer to the start of the buffer
    fn io_buf_mut_stable_mut_ptr(&mut self) -> *mut u8;

    /// Gets the capacity of the buffer
    fn io_buf_mut_capacity(&self) -> usize;

    /// Gets a mutable slice of the buffer
    ///
    /// # Safety
    ///
    /// An arbitrary implementor may return invalid pointers or lengths.
    unsafe fn slice_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.io_buf_mut_stable_mut_ptr(), self.io_buf_mut_capacity())
    }
}

unsafe impl IoBufMut for BufMut {
    fn io_buf_mut_stable_mut_ptr(&mut self) -> *mut u8 {
        unsafe { BUF_POOL.with(|bp| bp.base_ptr(self.index).add(self.off as _)) }
    }

    fn io_buf_mut_capacity(&self) -> usize {
        self.len as usize
    }
}

unsafe impl IoBufMut for Vec<u8> {
    fn io_buf_mut_stable_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    fn io_buf_mut_capacity(&self) -> usize {
        self.capacity()
    }
}

impl Drop for BufMut {
    fn drop(&mut self) {
        BUF_POOL.with(|bp| bp.dec(self.index));
    }
}

/// A read-only buffer. Can be cloned, but cannot be written to.
pub struct Buf {
    pub(crate) index: u32,
    pub(crate) off: u16,
    pub(crate) len: u16,

    // makes this type non-Send, which we do want
    _non_send: PhantomData<*mut ()>,
}

impl Buf {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len as _
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Take an owned slice of this
    pub fn slice(mut self, range: impl RangeBounds<usize>) -> Self {
        let mut new_start = 0;
        let mut new_end = self.len();

        match range.start_bound() {
            Bound::Included(&n) => new_start = n,
            Bound::Excluded(&n) => new_start = n + 1,
            Bound::Unbounded => {}
        }

        match range.end_bound() {
            Bound::Included(&n) => new_end = n + 1,
            Bound::Excluded(&n) => new_end = n,
            Bound::Unbounded => {}
        }

        assert!(new_start <= new_end);
        assert!(new_end <= self.len());

        self.off += new_start as u16;
        self.len = (new_end - new_start) as u16;
        self
    }

    /// Split this buffer in twain.
    /// Panics if `at` is out of bounds.
    #[inline]
    pub fn split_at(self, at: usize) -> (Self, Self) {
        assert!(at <= self.len as usize);

        let left = Buf {
            index: self.index,
            off: self.off,
            len: at as _,

            _non_send: PhantomData,
        };

        let right = Buf {
            index: self.index,
            off: self.off + at as u16,
            len: (self.len - at as u16),

            _non_send: PhantomData,
        };

        std::mem::forget(self); // don't decrease ref count
        BUF_POOL.with(|bp| bp.inc(left.index)); // in fact, increase it by 1

        (left, right)
    }
}

impl ops::Deref for Buf {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                BUF_POOL.with(|bp| bp.base_ptr(self.index).add(self.off as _)),
                self.len as _,
            )
        }
    }
}

impl Clone for Buf {
    fn clone(&self) -> Self {
        BUF_POOL.with(|bp| bp.inc(self.index));
        Self {
            index: self.index,
            off: self.off,
            len: self.len,
            _non_send: PhantomData,
        }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        BUF_POOL.with(|bp| bp.dec(self.index));
    }
}

#[cfg(test)]
mod tests {
    use crate::{Buf, BufMut, BUF_POOL};
    use std::rc::Rc;

    #[test]
    fn size_test() {
        assert_eq!(8, std::mem::size_of::<BufMut>());
        assert_eq!(8, std::mem::size_of::<Buf>());
        assert_eq!(16, std::mem::size_of::<Box<[u8]>>());

        assert_eq!(16, std::mem::size_of::<&[u8]>());

        #[allow(dead_code)]
        enum BufOrBox {
            Buf(Buf),
            Box((Rc<Box<[u8]>>, u32, u32)),
        }
        assert_eq!(16, std::mem::size_of::<BufOrBox>());

        #[allow(dead_code)]
        enum Chunk {
            Buf(Buf),
            Box(Box<[u8]>),
            Static(&'static [u8]),
        }
        assert_eq!(24, std::mem::size_of::<Chunk>());
    }

    #[test]
    fn freeze_test() -> eyre::Result<()> {
        let total_bufs = BUF_POOL.with(|bp| bp.num_free())?;
        let mut bm = BufMut::alloc().unwrap();

        assert_eq!(total_bufs - 1, BUF_POOL.with(|bp| bp.num_free())?);
        assert_eq!(bm.len(), 4096);

        bm[..11].copy_from_slice(b"hello world");
        assert_eq!(&bm[..11], b"hello world");

        let b = bm.freeze();
        assert_eq!(&b[..11], b"hello world");
        assert_eq!(total_bufs - 1, BUF_POOL.with(|bp| bp.num_free())?);

        let b2 = b.clone();
        assert_eq!(&b[..11], b"hello world");
        assert_eq!(total_bufs - 1, BUF_POOL.with(|bp| bp.num_free())?);

        drop(b);
        assert_eq!(total_bufs - 1, BUF_POOL.with(|bp| bp.num_free())?);

        drop(b2);
        assert_eq!(total_bufs, BUF_POOL.with(|bp| bp.num_free())?);

        Ok(())
    }

    #[test]
    fn split_test() -> eyre::Result<()> {
        let total_bufs = BUF_POOL.with(|bp| bp.num_free())?;
        let mut bm = BufMut::alloc().unwrap();

        bm[..12].copy_from_slice(b"yellowjacket");
        let (a, b) = bm.split_at(6);

        assert_eq!(total_bufs - 1, BUF_POOL.with(|bp| bp.num_free())?);
        assert_eq!(&a[..], b"yellow");
        assert_eq!(&b[..6], b"jacket");

        drop((a, b));

        Ok(())
    }
}
