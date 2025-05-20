use mastring::MAByteString;
use mastring::MAByteStringBuilder;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;
#[cfg(miri)]
use core::sync::atomic::AtomicPtr;

#[cfg(test)]
macro_rules! assert_mode {
    ($s:expr, $expectedmode:expr) => {
        #[allow(unused_labels)] // the label is only used under miri.
        'skipcheck: {
            let mode = $s.get_mode();
            let expectedmode = $expectedmode;
            #[cfg(miri)]
            if (($s.as_ptr() as usize) & (mem::align_of::<AtomicPtr<usize>>() - 1)) != 0 {
                //miri sometimes gives us unaligned vecs, this can lead to
                //control blocks not fitting inline. This should't break correctness, but
                //it can result in strings being in a different mode from expected.
                if (mode == "unique") && (expectedmode == "cbinline (unique)") { break 'skipcheck }
                if (mode == "cbowned (unique)") && (expectedmode == "cbinline (unique)") { break 'skipcheck }
                if (mode == "cbowned (shared)") && (expectedmode == "cbinline (shared)") { break 'skipcheck }
            }
            assert_eq!(mode,expectedmode);
        }
    }
}

#[test]
fn test_new() {
    let s = MAByteString::new();
    assert_eq!(s,b"");
    assert_mode!(s,"short");
}

#[test]
fn test_from_slice() {
    let s = MAByteString::from_slice(b"test");
    assert_eq!(s,b"test");
    assert_mode!(s,"short");

    let s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_from_vec() {
    let s = MAByteString::from_vec(b"test".to_vec());
    assert_eq!(s,b"test");
    assert_mode!(s,"short");

    let s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");

    let mut v = Vec::with_capacity(100);
    v.extend_from_slice(b"the quick brown fox jumped over the lazy dog");
    let s = MAByteString::from_vec(v);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_from_static() {
    let s = MAByteString::from_static(b"test");
    assert_eq!(s,b"test");
    assert_mode!(s,"short");

    let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
}

#[test]
fn test_from_builder() {
    let s = MAByteString::from_builder(MAByteStringBuilder::from_slice(b"test"));
    assert_eq!(s,b"test");
    assert_mode!(s,"short");

    let s = MAByteString::from_builder(MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec()));
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");

    let s = MAByteString::from_builder(MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog"));
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_get_mode() {
    let s = MAByteString::from_static(b"test");
    assert_mode!(s,"short");

    let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");

    let s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_mode!(s,"unique");
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    drop(s2);
    assert_mode!(s,"cbowned (unique)");

    let s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    assert_eq!(s2.get_mode(),"cbinline (shared)");
}

#[test]
fn test_reserve() {
    let mut s = MAByteString::from_static(b"test");
    assert_mode!(s,"short");
    s.reserve(10);
    assert_eq!(s,b"test");
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    s.reserve(100);
    assert_eq!(s,b"test");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_mode!(s,"cbinline (unique)");
    s.reserve(10); //should do nothing
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(10); // no extra space requested, but string must be copied because
                   // it's currently in static memory.
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(100);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    assert_eq!(s2.get_mode(),"cbinline (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbinline (unique)");
    assert!(s2.capacity() >= 100);
    assert!(s2.capacity() <= 150);

    let mut s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_mode!(s,"unique");
    s.reserve(10); // should do nothing
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    assert_eq!(s2.get_mode(),"cbowned (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbowned (unique)");
    assert!(s2.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s2.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    s.reserve(b"the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_mode!(s,"unique");

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(100);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    s.reserve(200);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 200);
    assert!(s.capacity() <= 250);
}

#[test]
fn test_capacity() {
    let s = MAByteString::from_static(b"test");  // short string
    assert_eq!(s.capacity(),mem::size_of::<MAByteString>()-1);
    let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.capacity(),0);
    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let s = MAByteString::from_vec(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    assert_eq!(s.capacity(),veccap);
    assert_eq!(s2.get_mode(),"cbowned (shared)");
    assert_eq!(s2.capacity(),veccap);

    let mut v = Vec::with_capacity(100);
    v.extend_from_slice(b"the quick brown fox jumped over the lazy dog");
    let veccap = v.capacity();
    let s = MAByteString::from_vec(v);
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() < veccap);
    assert!(s.capacity() >= veccap - mem::size_of::<usize>()*2);
}

#[test]
fn test_clear() {
    let mut s = MAByteString::from_static(b"test");
    assert_mode!(s,"short");
    s.clear();
    assert_mode!(s,"short");
    assert_eq!(s,b"");

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"short");

    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let mut s = MAByteString::from_vec(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);

    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let mut s = MAByteString::from_vec(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    drop(s2);
    assert_mode!(s,"cbowned (unique)");
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"cbowned (unique)");
    assert_eq!(s.capacity(),veccap);

    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let mut s = MAByteString::from_vec(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of::<MAByteString>()-1);
    drop(s2);

    let mut s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    let cap = s.capacity();
    assert_mode!(s,"cbinline (unique)");
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(s.capacity(),cap);

    let mut s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    s.clear();
    assert_eq!(s,b"");
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of::<MAByteString>()-1);
    drop(s2);

}

#[test]
fn test_into_vec() {
    let s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_eq!(v.as_ptr(),ptr);
    #[cfg(miri)]
    // miri sometimes gives us unaligned vecs, which can cause the
    // MAByteString to end up in "unique" mode which in turn causes the
    // capacity of the vec to be equal to that of the MAByteString
    assert!(v.capacity() >= capacity);
    #[cfg(not(miri))]
    assert!(v.capacity() > capacity);


    let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_ne!(v.as_ptr(),ptr);

    let s = MAByteString::from_static(b"test");
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_ne!(v.as_ptr(),ptr);

}

#[test]
fn test_clone_and_drop() {
   let s = MAByteString::from_static(b"test");
   assert_mode!(s,"short");
   let s2 = s.clone();
   assert_mode!(s,"short");
   assert_eq!(s2.get_mode(),"short");
   drop(s2);
   assert_mode!(s,"short");
   
   let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
   assert_mode!(s,"static");
   let s2 = s.clone();
   assert_mode!(s,"static");
   assert_eq!(s2.get_mode(),"static");
   drop(s2);
   assert_mode!(s,"static");
   
   let s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
   assert_mode!(s,"cbinline (unique)");
   let s2 = s.clone();
   assert_mode!(s,"cbinline (shared)");
   assert_mode!(s2,"cbinline (shared)");
   drop(s2);
   assert_mode!(s,"cbinline (unique)");

   let s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
   assert_mode!(s,"unique");
   let s2 = s.clone();
   assert_mode!(s,"cbowned (shared)");
   assert_eq!(s2.get_mode(),"cbowned (shared)");
   drop(s2);
   assert_mode!(s,"cbowned (unique)");
   let s2 = s.clone();
   assert_mode!(s,"cbowned (shared)");
   assert_eq!(s2.get_mode(),"cbowned (shared)");
   drop(s2);
   assert_mode!(s,"cbowned (unique)");

}

#[test]
fn test_deref() {
    let s = MAByteString::from_static(b"test");
    let ps = &s as *const _ as usize;
    assert_mode!(s,"short");
    let r = s.deref();
    assert_eq!(r,b"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let r = s.deref();
    assert_eq!(r,b"the quick brown fox jumped over the lazy dog");

}

#[test]
fn test_deref_mut() {
    let mut s = MAByteString::from_static(b"test");
    let ps = &s as *const _ as usize;
    assert_mode!(s,"short");
    let r = s.deref_mut();
    assert_eq!(r,b"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let ptr = s.as_ptr() as usize;
    let r = s.deref_mut();
    assert_ne!(r.as_ptr() as usize, ptr);
    assert_eq!(r,b"the quick brown fox jumped over the lazy dog");
    let rptr = r.as_ptr();
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(rptr as usize, s.as_ptr() as usize);
    
}

#[test]
fn test_add_and_eq() {
    let mut s  = MAByteString::from_static(b"test");
    s = s + b"foo";
    assert_mode!(s,"short");
    assert_eq!(s,b"testfoo");
    assert_eq!(s,b"testfoo" as &[u8]);
    s = s + b"the quick brown fox jumped over the lazy dog";
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(s,b"testfoothe quick brown fox jumped over the lazy dog");
    assert_eq!(s,b"testfoothe quick brown fox jumped over the lazy dog" as &[u8]);
    assert_eq!(s,MAByteString::from_slice(&s));

    let mut s  = MAByteString::from_static(b"test");
    s += b"foo";
    assert_eq!("short",s.get_mode());
    assert_eq!(b"testfoo",s);
    assert_eq!(b"testfoo" as &[u8],s);
    s += b"the quick brown fox jumped over the lazy dog";
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(b"testfoothe quick brown fox jumped over the lazy dog",s);
    assert_eq!(b"testfoothe quick brown fox jumped over the lazy dog" as &[u8],s);
    assert_eq!(MAByteString::from_slice(&s),s);

}

#[test]
fn test_debug() {
    let s  = MAByteString::from_static(b"test \" \\ \x19\x7f\x80\xff");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#"b"test \" \\ \x19\x7f\x80\xff""#);

    let s  = MAByteString::from_static(b"test \" \\ \x19\x7f\x80\xffhello");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#"b"test \" \\ \x19\x7f\x80\xffhello""#);

}

#[test]
fn test_size() {
    assert_eq!(mem::size_of::<MAByteString>(),mem::size_of::<usize>()*4);
    assert_eq!(mem::size_of::<MAByteString>(),mem::size_of::<usize>()*4);
}

