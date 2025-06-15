use mastring::MAString;
use mastring::MAStringBuilder;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;
use mastring::MAByteString;
#[cfg(miri)]
use core::sync::atomic::AtomicPtr;
use std::collections::HashSet;
use std::collections::BTreeSet;
use mastring::mas;
use std::borrow::Cow;
use mastring::CustomCow;

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
    let s = MAString::new();
    assert_eq!(s,"");
    assert_mode!(s,"short");
}

#[test]
fn test_from_slice() {
    let s = MAString::from_slice("test");
    assert_eq!(s,"test");
    assert_mode!(s,"short");

    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_from_str() {
    let s = MAString::from_string("test".to_string());
    assert_eq!(s,"test");
    assert_mode!(s,"short");

    let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let s = MAString::from_string(v);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_from_static() {
    let s = MAString::from_static("test");
    assert_eq!(s,"test");
    assert_mode!(s,"short");

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
}

#[test]
fn test_from_builder() {
    let s = MAString::from_builder(MAStringBuilder::from_slice("test"));
    assert_eq!(s,"test");
    assert_mode!(s,"short");

    let s = MAString::from_builder(MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string()));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");

    let s = MAString::from_builder(MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog"));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
}

#[test]
fn test_get_mode() {
    let s = MAString::from_static("test");
    assert_mode!(s,"short");

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");

    let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_mode!(s,"unique");
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    drop(s2);
    assert_mode!(s,"cbowned (unique)");

    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    assert_mode!(s2,"cbinline (shared)");
}

#[test]
fn test_reserve() {
    let mut s = MAString::from_static("test");
    assert_mode!(s,"short");
    s.reserve(10);
    assert_eq!(s,"test");
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    s.reserve(100);
    assert_eq!(s,"test");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_mode!(s,"cbinline (unique)");
    s.reserve(10); //should do nothing
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(10); // no extra space requested, but string must be copied because
                   // it's currently in static memory.
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
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

    let mut s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_mode!(s,"unique");
    s.reserve(10); // should do nothing
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
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

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    s.reserve("the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_mode!(s,"unique");

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    s.reserve(200);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
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
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    assert_eq!(s.capacity(),veccap);
    assert_mode!(s2,"cbowned (shared)");
    assert_eq!(s2.capacity(),veccap);

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let veccap = v.capacity();
    let s = MAString::from_string(v);
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() < veccap);
    assert!(s.capacity() >= veccap - mem::size_of::<usize>()*2);
}

#[test]
fn test_clear() {
    let mut s = MAString::from_static("test");
    assert_mode!(s,"short");
    s.clear();
    assert_mode!(s,"short");
    assert_eq!(s,"");

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"short");

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    drop(s2);
    assert_mode!(s,"cbowned (unique)");
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"cbowned (unique)");
    assert_eq!(s.capacity(),veccap);

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAString::from_string(v);
    assert_mode!(s,"unique");
    assert_eq!(s.capacity(),veccap);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of::<MAString>()-1);
    drop(s2);

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    let cap = s.capacity();
    assert_mode!(s,"cbinline (unique)");
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(s.capacity(),cap);

    let mut s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    s.clear();
    assert_eq!(s,"");
    assert_mode!(s,"short");
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
    #[cfg(miri)]
    // miri sometimes gives us unaligned vecs, which can cause the
    // MAString to end up in "unique" mode which in turn causes the
    // capacity of the vec to be equal to that of the MAString
    assert!(v.capacity() >= capacity);
    #[cfg(not(miri))]
    assert!(v.capacity() > capacity);

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    let ptr = s.as_ptr();
    let v = s.into_string();
    assert_ne!(v.as_ptr(),ptr);
}

#[test]
fn test_clone_and_drop() {
   let s = MAString::from_static("test");
   assert_mode!(s,"short");
   let s2 = s.clone();
   assert_mode!(s,"short");
   assert_mode!(s2,"short");
   drop(s2);
   assert_mode!(s,"short");
   
   let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
   assert_mode!(s,"static");
   let s2 = s.clone();
   assert_mode!(s,"static");
   assert_mode!(s2,"static");
   drop(s2);
   assert_mode!(s,"static");
   
   let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
   assert_mode!(s,"cbinline (unique)");
   let s2 = s.clone();
   assert_mode!(s,"cbinline (shared)");
   assert_mode!(s2,"cbinline (shared)");
   drop(s2);
   assert_mode!(s,"cbinline (unique)");

   let s = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
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
    let s = MAString::from_static("test");
    let ps = &s as *const _ as usize;
    assert_mode!(s,"short");
    let r = s.deref();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let r = s.deref();
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");

}

#[test]
fn test_deref_mut() {
    let mut s = MAString::from_static("test");
    let ps = &s as *const _ as usize;
    assert_mode!(s,"short");
    let r = s.deref_mut();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let mut s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let ptr = s.as_ptr() as usize;
    let r = s.deref_mut();
    assert_ne!(r.as_ptr() as usize, ptr);
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");
    let rptr = r.as_ptr();
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(rptr as usize, s.as_ptr() as usize);
    
}

#[test]
fn test_add_and_eq() {
    let mut s  = MAString::from_static("test");
    s = s + "foo";
    assert_mode!(s,"short");
    assert_eq!(s,"testfoo");
    s = s + "the quick brown fox jumped over the lazy dog";
    assert_mode!(s,"cbinline (unique)");
    assert_eq!(s,"testfoothe quick brown fox jumped over the lazy dog");
    assert_eq!(s,MAString::from_slice(&s));

    let mut s  = MAString::from_static("test");
    s += "foo";
    assert_eq!("short",s.get_mode());
    assert_eq!("testfoo",s);
    s += "the quick brown fox jumped over the lazy dog";
    assert_mode!(s,"cbinline (unique)");
    assert_eq!("testfoothe quick brown fox jumped over the lazy dog",s);
    assert_eq!(MAString::from_slice(&s),s);

}

#[test]
fn test_display() {
    let s  = MAString::from_static("test \" \\ ");
    let sd = format!("{}",s);
    assert_eq!(sd,r#"test " \ "#)
}

#[test]
fn test_debug() {
    let s  = MAString::from_static("test \" \\ ");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#""test \" \\ ""#)
}

#[test]
fn test_as_mut_slice() {
    let mut s = MAString::from_static("test");
    s.as_mut_str().make_ascii_uppercase();
    assert_eq!(s,"TEST");
}

#[test]
fn test_into_vec() {
    let s = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_eq!(v.as_ptr(),ptr);
    #[cfg(miri)]
    // miri sometimes gives us unaligned vecs, which can cause the
    // MAString to end up in "unique" mode which in turn causes the
    // capacity of the vec to be equal to that of the MAString
    assert!(v.capacity() >= capacity);
    #[cfg(not(miri))]
    assert!(v.capacity() > capacity);

    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_ne!(v.as_ptr(),ptr);
}

#[test]
fn test_from_utf8_unchecked() {
    unsafe {
        let s = MAString::from_utf8_unchecked(MAByteString::from_static(b"test"));
        assert_eq!(s,"test");
    }
}

#[test]
fn test_from_utf8() {
    let s = MAString::from_utf8(MAByteString::from_static(b"test"));
    assert_eq!(s,Ok(MAString::from_static("test")));
    let s = MAString::from_utf8(MAByteString::from_static(b"\xFF"));
    assert_eq!(s.unwrap_err().as_bytes(),b"\xFF");
}

#[test]
fn test_from_utf8_lossy() {
    let s = MAString::from_utf8_lossy(MAByteString::from_static(b"test"));
    assert_eq!(s,MAString::from_static("test"));
    let s = MAString::from_utf8_lossy(MAByteString::from_static(b"\xFF"));
    assert_eq!(s,MAString::from_static("\u{FFFD}"));
}

#[test]
fn test_into_bytes() {
    let s = MAString::from_static("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.into_bytes(),b"the quick brown fox jumped over the lazy dog");
}

#[test]
fn test_sets() {
    let mut h = HashSet::new();
    h.insert(MAString::from_static("The quick brown fox jumped over the lazy dog"));
    h.insert(MAString::from_slice("The quick brown fox jumped over the smart dog"));
    h.insert(MAString::from_static("foo"));
    h.insert(MAString::from_static("bar"));
    assert_eq!(h.contains(&MAString::from_static("The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAString::from_static("The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains("foo"),true);
    assert_eq!(h.contains("baz"),false);

    let mut h = BTreeSet::new();
    h.insert(MAString::from_static("The quick brown fox jumped over the lazy dog"));
    h.insert(MAString::from_slice("The quick brown fox jumped over the smart dog"));
    h.insert(MAString::from_static("foo"));
    h.insert(MAString::from_static("bar"));
    assert_eq!(h.contains(&MAString::from_static("The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAString::from_static("The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains("foo"),true);
    assert_eq!(h.contains("baz"),false);
}

#[test]
fn test_comparision() {
    assert!(MAString::from_static("A") < MAString::from_static("B"));
}

#[test]
fn test_macro() {
    let s = mas!("foo");
    assert_mode!(s,"short");
    let s = mas!("The quick brown fox jumped over the smart dog");
    assert_mode!(s,"static");
    let s = mas!(['1','2','3','4']);
    assert_mode!(s,"short");
    let s = mas!(['1','2','3','4','5','6','7','8','9','0','1','2','3','4','5','6','7','8','9','0','1','2','3','4','5','6','7','8','9','0','1','2','\u{03A9}','\u{2261}','\u{1f980}']);
    assert_eq!(s,"12345678901234567890123456789012\u{03A9}\u{2261}\u{1f980}");
    assert_mode!(s,"static");
    let foo = "foo";
    let s = mas!(foo);
    assert_mode!(s,"short");
    let foo = "The slow brown fox jumped over the sleeping dog";
    let s = mas!(foo);
    assert_mode!(s,"cbinline (unique)");
    let s = mas!(foo.to_string());
    assert_mode!(s,"unique");
    let four = '4';
    let s = mas!(&['1','2','3',four]);
    assert_eq!(s,"1234");
    assert_mode!(s,"short");
    let s = mas!(&foo.to_string());
    assert_mode!(s,"cbinline (unique)");
    let s = mas!(s);
    assert_mode!(s,"cbinline (unique)");
    let s = mas!(&s);
    assert_mode!(s,"cbinline (shared)");
    let sb = MAStringBuilder::from_slice("The slow brown fox jumped over the sleeping dog");
    let s = mas!(&sb);
    assert_mode!(s,"cbinline (unique)");
    let s = mas!(sb);
    assert_mode!(s,"cbinline (unique)");

}

#[test]
fn test_collect() {
    let s: MAString = ['a','b','c','d'].iter().collect();
    assert_eq!(s,"abcd");
    let s: MAString = ['a','b','c','d','e','f','g'].into_iter().collect();
    assert_eq!(s,"abcdefg");

    let s: MAString = ["a","b","c","d","e","f","g","h","i","jay"].into_iter().collect();
    assert_eq!(s,"abcdefghijay");

    let s: MAString = ["a".to_string(),"b".to_string(),"c".to_string()].into_iter().collect();
    assert_eq!(s,"abc");

    let s: MAString = [Box::from("a"),Box::from("b"),Box::from("c")].into_iter().collect();
    assert_eq!(s,"abc");

    let s: MAString = [Cow::Borrowed("a"),Cow::Owned("b".to_string()),Cow::Borrowed("c")].into_iter().collect();
    assert_eq!(s,"abc");

    let s: MAString = [CustomCow::Borrowed("a"),CustomCow::Owned(mas!("b")),CustomCow::Borrowed("c")].into_iter().collect();
    assert_eq!(s,"abc");

    let s: MAString = [mas!("a"),mas!("b"),mas!("c")].into_iter().collect();
    assert_eq!(s,"abc");


}
