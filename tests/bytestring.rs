use mastring::MAByteString;
use mastring::MAByteStringBuilder;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;
#[cfg(miri)]
use core::sync::atomic::AtomicPtr;
use std::collections::HashSet;
use std::collections::BTreeSet;
use mastring::mabs;
use mastring::CustomCow;
use std::borrow::Cow;

#[cfg(test)]
macro_rules! assert_mode {
    ($s:expr, $expectedmode:expr) => {
        for _n in 1..=1 { // loop for break, since block labels are not supported in rust 1.63
            let mode = $s.get_mode();
            let expectedmode = $expectedmode;
            #[cfg(miri)]
            if (($s.as_ptr() as usize) & (mem::align_of::<AtomicPtr<usize>>() - 1)) != 0 {
                //miri sometimes gives us unaligned vecs, this can lead to
                //control blocks not fitting inline. This should't break correctness, but
                //it can result in strings being in a different mode from expected.
                if (mode == "unique") && (expectedmode == "cbinline (unique)") { break }
                if (mode == "cbowned (unique)") && (expectedmode == "cbinline (unique)") { break }
                if (mode == "cbowned (shared)") && (expectedmode == "cbinline (shared)") { break }
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
    assert_mode!(s2,"cbinline (shared)");
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
    assert_mode!(s2,"cbinline (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_mode!(s2,"cbinline (unique)");
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
    assert_mode!(s2,"cbowned (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_mode!(s2,"cbowned (unique)");
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
    assert_mode!(s2,"cbowned (shared)");
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
   assert_mode!(s2,"short");
   drop(s2);
   assert_mode!(s,"short");
   
   let s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
   assert_mode!(s,"static");
   let s2 = s.clone();
   assert_mode!(s,"static");
   assert_mode!(s2,"static");
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
   assert_mode!(s2,"cbowned (shared)");
   drop(s2);
   assert_mode!(s,"cbowned (unique)");
   let s2 = s.clone();
   assert_mode!(s,"cbowned (shared)");
   assert_mode!(s2,"cbowned (shared)");
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

#[test]
fn test_sets() {
    let mut h = HashSet::new();
    h.insert(MAByteString::from_static(b"The quick brown fox jumped over the lazy dog"));
    h.insert(MAByteString::from_slice(b"The quick brown fox jumped over the smart dog"));
    h.insert(MAByteString::from_static(b"foo"));
    h.insert(MAByteString::from_static(b"bar"));
    assert_eq!(h.contains(&MAByteString::from_static(b"The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAByteString::from_static(b"The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains(b"foo" as &[u8]),true);
    assert_eq!(h.contains(b"baz" as &[u8]),false);

    let mut h = BTreeSet::new();
    h.insert(MAByteString::from_static(b"The quick brown fox jumped over the lazy dog"));
    h.insert(MAByteString::from_slice(b"The quick brown fox jumped over the smart dog"));
    h.insert(MAByteString::from_static(b"foo"));
    h.insert(MAByteString::from_static(b"bar"));
    assert_eq!(h.contains(&MAByteString::from_static(b"The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAByteString::from_static(b"The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains(b"foo" as &[u8]),true);
    assert_eq!(h.contains(b"baz" as &[u8]),false);
}

#[test]
fn test_comparision() {
    assert!(MAByteString::from_static(b"A") < MAByteString::from_static(b"B"));
}

#[test]
fn test_macro() {
    let s = mabs!(b"foo");
    assert_mode!(s,"short");
    let s = mabs!(b"The quick brown fox jumped over the smart dog");
    assert_mode!(s,"static");
    let s = mabs!([1,2,3,4]);
    assert_mode!(s,"short");
    let s = mabs!([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]);
    assert_mode!(s,"static");
    let foo = b"foo";
    let s = mabs!(foo);
    assert_mode!(s,"short");
    let foo = b"The slow brown fox jumped over the sleeping dog";
    let s = mabs!(foo);
    assert_mode!(s,"cbinline (unique)");
    let s = mabs!(foo.to_vec());
    assert_mode!(s,"unique");
    let four = 4;
    let s = mabs!(&[1,2,3,four]);
    assert_mode!(s,"short");
    let s = mabs!(&foo.to_vec());
    assert_mode!(s,"cbinline (unique)");
    let s = mabs!(s);
    assert_mode!(s,"cbinline (unique)");
    let s = mabs!(&s);
    assert_mode!(s,"cbinline (shared)");
    let sb = MAByteStringBuilder::from_slice(b"The slow brown fox jumped over the sleeping dog");
    let s = mabs!(&sb);
    assert_mode!(s,"cbinline (unique)");
    let s = mabs!(sb);
    assert_mode!(s,"cbinline (unique)");

}

#[test]
fn test_collect() {
    let s: MAByteString = [b'a',b'b',b'c',b'd'].iter().collect();
    assert_eq!(s,b"abcd");
    let s: MAByteString = [b'a',b'b',b'c',b'd',b'e',b'f',b'g'].into_iter().collect();
    assert_eq!(s,b"abcdefg");

    let a: [&[u8];10] = [b"a",b"b",b"c",b"d",b"e",b"f",b"g",b"h",b"i",b"jay"];
    let s: MAByteString = a.into_iter().collect();
    assert_eq!(s,b"abcdefghijay");

    let s: MAByteString = [b"a".to_vec(),b"b".to_vec(),b"c".to_vec()].into_iter().collect();
    assert_eq!(s,b"abc");

    let s: MAByteString = [Box::from(b"a" as &[u8]),Box::from(b"b" as &[u8]),Box::from(b"c" as &[u8])].into_iter().collect();
    assert_eq!(s,b"abc");

    let s: MAByteString = [Cow::Borrowed(b"a" as &[u8]),Cow::Owned(b"b".to_vec()),Cow::Borrowed(b"c")].into_iter().collect();
    assert_eq!(s,b"abc");

    let s: MAByteString = [CustomCow::Owned(mabs!(b"a")),CustomCow::Owned(mabs!(b"b")),CustomCow::Borrowed(b"c" as &[u8])].into_iter().collect();
    assert_eq!(s,b"abc");

    let s: MAByteString = [mabs!(b"a"),mabs!(b"b"),mabs!(b"c")].into_iter().collect();
    assert_eq!(s,b"abc");


}
