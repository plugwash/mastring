use mastring::MAByteStringBuilder;
use mastring::MAByteString;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;
use std::collections::HashSet;
use std::collections::BTreeSet;
use mastring::mabsb;

#[test]
fn test_new() {
    let s = MAByteStringBuilder::new();
    assert_eq!(s,b"");
    assert_eq!(s.get_mode(),"short");
}

#[test]
fn test_from_slice() {
    let s = MAByteStringBuilder::from_slice(b"test");
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_from_vec() {
    let s = MAByteStringBuilder::from_vec(b"test".to_vec());
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let mut v = Vec::with_capacity(100);
    v.extend_from_slice(b"the quick brown fox jumped over the lazy dog");
    let s = MAByteStringBuilder::from_vec(v);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_from_mabs() {
    let s = MAByteStringBuilder::from_mabs(MAByteString::from_slice(b"test"));
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAByteStringBuilder::from_mabs(MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec()));
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let s = MAByteStringBuilder::from_mabs(MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog"));
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    let s2 = s.clone();
    drop(s2);
    let s =MAByteStringBuilder::from_mabs(s);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

}

#[test]
fn test_get_mode() {
    let s = MAByteStringBuilder::from_slice(b"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_reserve() {
    let mut s = MAByteStringBuilder::from_slice(b"test");
    assert_eq!(s.get_mode(),"short");
    s.reserve(10);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    s.reserve(100);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_eq!(s.get_mode(),"unique");
    s.reserve(10); //should do nothing
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    s.reserve(100);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s.get_mode(),"unique");
    s.reserve(10); // should do nothing
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    s.reserve(b"the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_eq!(s.get_mode(),"unique");

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    s.reserve(100);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    s.reserve(200);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 200);
    assert!(s.capacity() <= 250);
}

#[test]
fn test_capacity() {
    let s = MAByteStringBuilder::from_slice(b"test");  // short string
    assert_eq!(s.capacity(),mem::size_of::<MAByteStringBuilder>()-1);
    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let s = MAByteStringBuilder::from_vec(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);

    let mut v = Vec::with_capacity(100);
    v.extend_from_slice(b"the quick brown fox jumped over the lazy dog");
    let veccap = v.capacity();
    let s = MAByteStringBuilder::from_vec(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    assert!(s.capacity() >= veccap - mem::size_of::<usize>()*2);
}

#[test]
fn test_clear() {
    let mut s = MAByteStringBuilder::from_slice(b"test");
    assert_eq!(s.get_mode(),"short");
    s.clear();
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,b"");

    let v = b"the quick brown fox jumped over the lazy dog".to_vec();
    let veccap = v.capacity();
    let mut s = MAByteStringBuilder::from_vec(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    s.clear();
    assert_eq!(s,b"");
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);

}

#[test]
fn test_into_vec() {
    let s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_eq!(v.as_ptr(),ptr);
    assert_eq!(v.capacity(),capacity);

    let s = MAByteStringBuilder::from_slice(b"test");
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_ne!(v.as_ptr(),ptr);
}

#[test]
fn test_clone_and_drop() {
   let s = MAByteStringBuilder::from_slice(b"test");
   assert_eq!(s.get_mode(),"short");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"short");
   assert_eq!(s2.get_mode(),"short");
   drop(s2);
   assert_eq!(s.get_mode(),"short");

   let s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
   assert_eq!(s.get_mode(),"unique");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"unique");
   assert_eq!(s2.get_mode(),"unique");
   drop(s2);
   assert_eq!(s.get_mode(),"unique");

   let s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
   assert_eq!(s.get_mode(),"unique");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"unique");
   assert_eq!(s2.get_mode(),"unique");
   drop(s2);
   assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_deref() {
    let s = MAByteStringBuilder::from_slice(b"test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref();
    assert_eq!(r,b"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    let r = s.deref();
    assert_eq!(r,b"the quick brown fox jumped over the lazy dog");

}

#[test]
fn test_deref_mut() {
    let mut s = MAByteStringBuilder::from_slice(b"test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref_mut();
    assert_eq!(r,b"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    let ptr = s.as_ptr() as usize;
    let r = s.deref_mut();
    assert_eq!(r.as_ptr() as usize, ptr);
    assert_eq!(r,b"the quick brown fox jumped over the lazy dog");
    let rptr = r.as_ptr();
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(rptr as usize, s.as_ptr() as usize);
    
}

#[test]
fn test_add_and_eq() {
    let mut s  = MAByteStringBuilder::from_slice(b"test");
    s = s + b"foo";
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,b"testfoo");
    assert_eq!(s,b"testfoo" as &[u8]);
    s = s + b"the quick brown fox jumped over the lazy dog";
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s,b"testfoothe quick brown fox jumped over the lazy dog");
    assert_eq!(s,b"testfoothe quick brown fox jumped over the lazy dog" as &[u8]);
    assert_eq!(s,MAByteStringBuilder::from_slice(&s));

    let mut s  = MAByteStringBuilder::from_slice(b"test");
    s += b"foo";
    assert_eq!("short",s.get_mode());
    assert_eq!(b"testfoo",s);
    assert_eq!(b"testfoo" as &[u8],s);
    s += b"the quick brown fox jumped over the lazy dog";
    assert_eq!("unique",s.get_mode());
    assert_eq!(b"testfoothe quick brown fox jumped over the lazy dog",s);
    assert_eq!(b"testfoothe quick brown fox jumped over the lazy dog" as &[u8],s);
    assert_eq!(MAByteStringBuilder::from_slice(&s),s);

}

#[test]
fn test_debug() {
    let s  = MAByteStringBuilder::from_slice(b"test \" \\ \x19\x7f\x80\xff");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#"b"test \" \\ \x19\x7f\x80\xff""#)
}

#[test]
fn test_size() {
    assert_eq!(mem::size_of::<MAByteStringBuilder>(),mem::size_of::<usize>()*4);
    assert_eq!(mem::size_of::<MAByteStringBuilder>(),mem::size_of::<usize>()*4);
}

#[test]
fn test_sets() {
    let mut h = HashSet::new();
    h.insert(MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the lazy dog"));
    h.insert(MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the smart dog"));
    h.insert(MAByteStringBuilder::from_slice(b"foo"));
    h.insert(MAByteStringBuilder::from_slice(b"bar"));
    assert_eq!(h.contains(&MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains(b"foo" as &[u8]),true);
    assert_eq!(h.contains(b"baz" as &[u8]),false);

    let mut h = BTreeSet::new();
    h.insert(MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the lazy dog"));
    h.insert(MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the smart dog"));
    h.insert(MAByteStringBuilder::from_slice(b"foo"));
    h.insert(MAByteStringBuilder::from_slice(b"bar"));
    assert_eq!(h.contains(&MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAByteStringBuilder::from_slice(b"The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains(b"foo" as &[u8]),true);
    assert_eq!(h.contains(b"baz" as &[u8]),false);
}

#[test]
fn test_comparision() {
    assert!(MAByteStringBuilder::from_slice(b"A") < MAByteStringBuilder::from_slice(b"B"));
}

#[test]
fn test_macro() {
    let s = mabsb!(b"foo");
    assert_eq!(s.get_mode(),"short");
    let s = mabsb!(b"The quick brown fox jumped over the smart dog");
    assert_eq!(s.get_mode(),"unique");
    let s = mabsb!([1,2,3,4]);
    assert_eq!(s.get_mode(),"short");
    let s = mabsb!([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]);
    assert_eq!(s.get_mode(),"unique");
    let foo = b"foo";
    let s = mabsb!(foo);
    assert_eq!(s.get_mode(),"short");
    let foo = b"The slow brown fox jumped over the sleeping dog";
    let s = mabsb!(foo);
    assert_eq!(s.get_mode(),"unique");
    let s = mabsb!(foo.to_vec());
    assert_eq!(s.get_mode(),"unique");
    let four = 4;
    let s = mabsb!(&[1,2,3,four]);
    assert_eq!(s.get_mode(),"short");
    let s = mabsb!(&foo.to_vec());
    assert_eq!(s.get_mode(),"unique");
    let s = mabsb!(s);
    assert_eq!(s.get_mode(),"unique");
    let s = mabsb!(&s);
    assert_eq!(s.get_mode(),"unique");
    let sb = MAByteString::from_slice(b"The slow brown fox jumped over the sleeping dog");
    let s = mabsb!(&sb);
    assert_eq!(s.get_mode(),"unique");
    let s = mabsb!(sb);
    assert_eq!(s.get_mode(),"unique");

}
