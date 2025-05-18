use mastring::MAString;
use mastring::MAStringBuilder;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;

#[test]
fn test_new() {
    let s = MAString::new();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"short");
}

#[test]
fn test_from_slice() {
    let s = MAString::from_slice("test");
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
}

#[test]
fn test_from_str() {
    let s = MAString::from_string("test".to_string());
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let s = MAString::from_string(v);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
}

#[test]
fn test_from_static() {
    let s = MAString::from_static("test");
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
}

#[test]
fn test_from_builder() {
    let s = MAString::from_builder(MAStringBuilder::from_slice("test"));
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAString::from_builder(MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string()));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let s = MAString::from_builder(MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog"));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
}

#[test]
fn test_get_mode() {
    let s = MAString::from_static("test");
    assert_eq!(s.get_mode(),"short");

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");

    let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s.get_mode(),"unique");
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbowned (shared)");
    drop(s2);
    assert_eq!(s.get_mode(),"cbowned (unique)");

    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbinline (shared)");
    assert_eq!(s2.get_mode(),"cbinline (shared)");
}

#[test]
fn test_reserve() {
    let mut s = MAString::from_static("test");
    assert_eq!(s.get_mode(),"short");
    s.reserve(10);
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    s.reserve(100);
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_eq!(s.get_mode(),"cbinline (unique)");
    s.reserve(10); //should do nothing
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    s.reserve(10); // no extra space requested, but string must be copied because
                   // it's currently in static memory.
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbinline (shared)");
    assert_eq!(s2.get_mode(),"cbinline (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbinline (unique)");
    assert!(s2.capacity() >= 100);
    assert!(s2.capacity() <= 150);

    let mut s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s.get_mode(),"unique");
    s.reserve(10); // should do nothing
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbowned (shared)");
    assert_eq!(s2.get_mode(),"cbowned (shared)");
    s.reserve(10); // no extra space requested, but string must be copied because it's currently in static memory.
    //s now has a new buffer
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbowned (unique)");
    assert!(s2.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s2.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    s.reserve("the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_eq!(s.get_mode(),"unique");

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    s.reserve(200);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 200);
    assert!(s.capacity() <= 250);
}

#[test]
fn test_capacity() {
    let s = MAString::from_static("test");  // short string
    assert_eq!(s.capacity(),mem::size_of::<MAString>()-1);
    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.capacity(),0);
    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let s = MAString::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbowned (shared)");
    assert_eq!(s.capacity(),veccap);
    assert_eq!(s2.get_mode(),"cbowned (shared)");
    assert_eq!(s2.capacity(),veccap);

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let veccap = v.capacity();
    let s = MAString::from_string(v);
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() < veccap);
    assert!(s.capacity() >= veccap - mem::size_of::<usize>()*2);
}

#[test]
fn test_clear() {
    let mut s = MAString::from_static("test");
    assert_eq!(s.get_mode(),"short");
    s.clear();
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,"");

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"short");

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    drop(s2);
    assert_eq!(s.get_mode(),"cbowned (unique)");
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"cbowned (unique)");
    assert_eq!(s.capacity(),veccap);

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbowned (shared)");
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of::<MAString>()-1);
    drop(s2);

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    let cap = s.capacity();
    assert_eq!(s.get_mode(),"cbinline (unique)");
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert_eq!(s.capacity(),cap);

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbinline (shared)");
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of::<MAString>()-1);
    drop(s2);

}

#[test]
fn test_into_string() {
    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_string();
    assert_eq!(v.as_ptr(),ptr);
    assert!(v.capacity() > capacity);

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    let ptr = s.as_ptr();
    let v = s.into_string();
    assert_ne!(v.as_ptr(),ptr);
}

#[test]
fn test_clone_and_drop() {
   let s = MAString::from_static("test");
   assert_eq!(s.get_mode(),"short");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"short");
   assert_eq!(s2.get_mode(),"short");
   drop(s2);
   assert_eq!(s.get_mode(),"short");
   
   let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
   assert_eq!(s.get_mode(),"static");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"static");
   assert_eq!(s2.get_mode(),"static");
   drop(s2);
   assert_eq!(s.get_mode(),"static");
   
   let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
   assert_eq!(s.get_mode(),"cbinline (unique)");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"cbinline (shared)");
   assert_eq!(s2.get_mode(),"cbinline (shared)");
   drop(s2);
   assert_eq!(s.get_mode(),"cbinline (unique)");

   let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
   assert_eq!(s.get_mode(),"unique");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"cbowned (shared)");
   assert_eq!(s2.get_mode(),"cbowned (shared)");
   drop(s2);
   assert_eq!(s.get_mode(),"cbowned (unique)");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"cbowned (shared)");
   assert_eq!(s2.get_mode(),"cbowned (shared)");
   drop(s2);
   assert_eq!(s.get_mode(),"cbowned (unique)");

}

#[test]
fn test_deref() {
    let s = MAString::from_static("test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    let r = s.deref();
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");

}

#[test]
fn test_deref_mut() {
    let mut s = MAString::from_static("test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref_mut();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    let ptr = s.as_ptr() as usize;
    let r = s.deref_mut();
    assert_ne!(r.as_ptr() as usize, ptr);
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");
    let rptr = r.as_ptr();
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert_eq!(rptr as usize, s.as_ptr() as usize);
    
}

#[test]
fn test_add_and_eq() {
    let mut s  = MAString::from_static("test");
    s = s + "foo";
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,"testfoo");
    s = s + "the quick brown fox jumped over the lazy dog";
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert_eq!(s,"testfoothe quick brown fox jumped over the lazy dog");
    assert_eq!(s,MAString::from_slice(&s));

    let mut s  = MAString::from_static("test");
    s += "foo";
    assert_eq!("short",s.get_mode());
    assert_eq!("testfoo",s);
    s += "the quick brown fox jumped over the lazy dog";
    assert_eq!("cbinline (unique)",s.get_mode());
    assert_eq!("testfoothe quick brown fox jumped over the lazy dog",s);
    assert_eq!(MAString::from_slice(&s),s);

}

#[test]
fn test_debug() {
    let s  = MAString::from_static("test \" \\ ");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#""test \" \\ ""#)
}


