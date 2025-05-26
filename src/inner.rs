use core::sync::atomic::AtomicPtr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::sync::atomic::fence;
use core::mem::size_of;
use core::mem;
use core::mem::align_of;
use core::ptr;
use core::slice;
use core::cmp::max;
use core::cmp::min;
use crate::limitedusize::LimitedU8;
use crate::limitedusize::LimitedUSize;

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;

//contol block values
//stores reference count * 2
//even numbers for owned
//odd numbers for inline

#[repr(C)]
pub (super) struct InnerLong {
    #[cfg(target_endian="big")]
    pub (super) len: usize,
    pub (super) cap: usize,
    pub (super) ptr: * mut u8,
    pub (super) cbptr: AtomicPtr<AtomicUsize>,
    #[cfg(target_endian="little")]
    pub (super) len: usize,
}

impl InnerLong {
    #[inline]
    pub (super) fn from_vec(mut v: Vec<u8>, allowcb: bool, mincap: usize) -> Self {
        // it would be nice to use into_raw_parts here, but it's unstable.
        let len = v.len();
        let cap = v.capacity();
        assert!(mincap <= cap);
        let mincap = max(len,mincap);
        let ptr = v.as_mut_ptr();
        mem::forget(v);
        let mut cbptr : * mut AtomicUsize = ptr::null_mut();
        if allowcb {
            //println!("checking if we have room for a control block");
            unsafe {
                //check if we have room for a control block.
                //math wont overflow because a vec is limited to isize,
                //which has half the range of usize.
                let end = ptr.add(mincap);
                let mut cbstart = mincap + end.align_offset(align_of::<AtomicUsize>());
                let cbrequired = cbstart + size_of::<AtomicUsize>();
                if cbrequired <= cap {
                    let cbextraspace = (cap - cbrequired) & !(align_of::<AtomicUsize>()-1);
                    cbstart += cbextraspace;
                    //println!("allocating control block");
                    cbptr = ptr.add(cbstart) as * mut AtomicUsize;
                    *cbptr = AtomicUsize::new(3);
                } else {
                    //println!("no room for control block!");
                }
            }
        }
        InnerLong { len : len, cap: cap, ptr: ptr, cbptr: AtomicPtr::new(cbptr) }
    }

    #[inline]
    pub (super) fn from_slice(s: &[u8], allowcb: bool, mincap: usize) -> Self {
        let len = max(s.len(),mincap);
        let mask = align_of::<AtomicUsize>() - 1;
        let veccap = ((len + mask) & !mask) + size_of::<AtomicUsize>();
        //println!("len:{len} allowcb:{allowcb} veccap:{veccap}");
        let mut v = Vec::with_capacity(veccap);
        v.extend_from_slice(s);
        Self::from_vec(v,allowcb,mincap)
    }

    // ensure the pointer is unique
    // if the string is copied, then mincap sets the minimum capacity of the new
    // string, excluding control block space. However if the string is not copied
    // then it's capacity is left unchanged.
    #[inline]
    pub (super) fn make_unique(&mut self, mincap: usize, allowcb: bool) {
        //println!("in make_unique mincap={mincap} allowcb={allowcb}");
        if self.cap == 0 { // static string, we need to copy
            unsafe {
                *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),allowcb,mincap);
            }
        } else {
            let cbptr = self.cbptr.load(Ordering::Relaxed);
            if cbptr.is_null() {
                // we already have unique ownership of the string.
            } else {
                unsafe {
                    let refcount = (*cbptr).load(Ordering::Relaxed) >> 1;
                    if refcount == 1 {
                        // we are the only owner of the String
                        if !allowcb {
                            // switch it from shared ownership mode to unique ownership mode.
                            if ((*cbptr).load(Ordering::Relaxed) & 1) == 0 {
                                //free control block pointer
                                let _ = Box::from_raw(cbptr);
                            }
                            self.cbptr.store(ptr::null_mut(),Ordering::Relaxed);
                        }
                    } else {
                        // there are other owners, we need to copy
                        *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),allowcb,mincap) ;
                    }
                }
            }
        }
    }

    pub (super) fn usablecap(&self) -> usize {
        // check for an inline control block.
        let ptr = self.ptr as usize;
        // use relaxed as the only time the value critically matters is
        // when we already have unique ownership.
        let cbptr = self.cbptr.load(Ordering::Relaxed) as usize;
        if cbptr < ptr {
            // no control block, or outline control block located before data
            self.cap
        } else {
            // inline control block, or outline control block located after data.
            min(cbptr - ptr,self.cap)
        }
    }

    // reallocate the buffer if it's capacity is less than requested.
    // implement an exponential reallocation
    // SAFETY: callers must ensure that the innerlong has unique ownership
    // before calling, use make_unique if needed.
    pub (super) unsafe fn reserve(&mut self, mut mincap: usize, allowcb: bool) {
        let cap = self.cap;
        if mincap <= cap { 
            if mincap <= self.usablecap() { return }
            // an inline control block is reducing our usable capacity, get rid of it
            self.cbptr = AtomicPtr::new(ptr::null_mut());
            return
        };
        mincap = max(mincap, cap * 2);
        unsafe {
            *self = Self::from_slice(slice::from_raw_parts(self.ptr, self.len), allowcb,  mincap);
        }
    }

}

impl Drop for InnerLong {
    fn drop(&mut self) {
        let len = self.len;
        let cap = self.cap;
        if cap == 0 { return } //static string
        let cbptr = self.cbptr.load(Ordering::Relaxed);
        unsafe {
            if !cbptr.is_null() {
                let oldcb = (*cbptr).fetch_sub(2, Ordering::Release); //decrease the refcount
                if oldcb > 3 { return } // there are still other references
                fence(Ordering::Acquire);
                if (oldcb & 1) == 0 { //owned control block
                    let _ = Box::from_raw(cbptr);
                }
            }
            // we hold the only reference, turn it back into a vec so rust will free it.
            let _ = Vec::from_raw_parts(self.ptr, len, cap);
        }
    }

}

pub (super) const SHORTLEN : usize = size_of::<InnerLong>()-1;

#[repr(C)]
#[derive(Clone,Copy)]
pub (super) struct InnerShort {
    #[cfg(target_endian="big")]
    pub (super) len: u8,
    pub (super) data: [u8;SHORTLEN],
    #[cfg(target_endian="little")]
    pub (super) len: u8,
}

// This defines a layout with the niche we want and the interior mutability
// we need.
#[repr(C)]
pub (super) struct InnerNiche {

    //these fields are not meant to be used directly, merely to define
    //the data type layout. 
    #[cfg(target_endian="big")]
    _len: usize,
    _cap: usize,
    _ptr: * mut u8,
    _cbptr: AtomicPtr<AtomicUsize>,
    #[cfg(target_endian="little")]
    _len: LimitedUSize,
}

const _ : () = {
   assert!((SHORTLEN + 0x80) <= LimitedU8::MAX);
};


