use mastring::MAStringBuilder;
use mastring::MAByteStringBuilder;
use mastring::MAString;
use core::mem;
use core::ops::Deref;
use core::ops::DerefMut;
use std::collections::HashSet;
use std::collections::BTreeSet;
use mastring::masb;

#[test]
fn test_new() {
    let s = MAStringBuilder::new();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"short");
}

#[test]
fn test_from_slice() {
    let s = MAStringBuilder::from_slice("test");
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_from_string() {
    let s = MAStringBuilder::from_string("test".to_string());
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let s = MAStringBuilder::from_string(v);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_from_mas() {
    let s = MAStringBuilder::from_mas(MAString::from_slice("test"));
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");

    let s = MAStringBuilder::from_mas(MAString::from_string("the quick brown fox jumped over the lazy dog".to_string()));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");

    let s = MAStringBuilder::from_mas(MAString::from_slice("the quick brown fox jumped over the lazy dog"));
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_get_mode() {
    let s = MAStringBuilder::from_slice("test");
    assert_eq!(s.get_mode(),"short");

    let s = MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_reserve() {
    let mut s = MAStringBuilder::from_slice("test");
    assert_eq!(s.get_mode(),"short");
    s.reserve(10);
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    s.reserve(100);
    assert_eq!(s,"test");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_eq!(s.get_mode(),"unique");
    s.reserve(10); //should do nothing
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string());
    assert_eq!(s.get_mode(),"unique");
    s.reserve(10); // should do nothing
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    s.reserve("the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_eq!(s.get_mode(),"unique");

    let mut s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    s.reserve(100);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    s.reserve(200);
    assert_eq!(s,"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 200);
    assert!(s.capacity() <= 250);
}

#[test]
fn test_capacity() {
    let s = MAStringBuilder::from_slice("test");  // short string
    assert_eq!(s.capacity(),mem::size_of::<MAStringBuilder>()-1);
    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let s = MAStringBuilder::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);

    let mut v = String::with_capacity(100);
    v+="the quick brown fox jumped over the lazy dog";
    let veccap = v.capacity();
    let s = MAStringBuilder::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    assert!(s.capacity() >= veccap - mem::size_of::<usize>()*2);
}

#[test]
fn test_clear() {
    let mut s = MAStringBuilder::from_slice("test");
    assert_eq!(s.get_mode(),"short");
    s.clear();
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,"");

    let v = "the quick brown fox jumped over the lazy dog".to_string();
    let veccap = v.capacity();
    let mut s = MAStringBuilder::from_string(v);
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);
    s.clear();
    assert_eq!(s,"");
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s.capacity(),veccap);

}

#[test]
fn test_into_string() {
    let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_string();
    assert_eq!(v.as_ptr(),ptr);
    assert_eq!(v.capacity(),capacity);
}

#[test]
fn test_clone_and_drop() {
   let s = MAStringBuilder::from_slice("test");
   assert_eq!(s.get_mode(),"short");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"short");
   assert_eq!(s2.get_mode(),"short");
   drop(s2);
   assert_eq!(s.get_mode(),"short");

   let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
   assert_eq!(s.get_mode(),"unique");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"unique");
   assert_eq!(s2.get_mode(),"unique");
   drop(s2);
   assert_eq!(s.get_mode(),"unique");

   let s = MAStringBuilder::from_string("the quick brown fox jumped over the lazy dog".to_string());
   assert_eq!(s.get_mode(),"unique");
   let s2 = s.clone();
   assert_eq!(s.get_mode(),"unique");
   assert_eq!(s2.get_mode(),"unique");
   drop(s2);
   assert_eq!(s.get_mode(),"unique");
}

#[test]
fn test_deref() {
    let s = MAStringBuilder::from_slice("test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    let r = s.deref();
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");

}

#[test]
fn test_deref_mut() {
    let mut s = MAStringBuilder::from_slice("test");
    let ps = &s as *const _ as usize;
    assert_eq!(s.get_mode(),"short");
    let r = s.deref_mut();
    assert_eq!(r,"test");
    assert_eq!(r.len(),4);
    let pdata = r.as_ptr() as usize;
    assert!(pdata >= ps);
    assert!(pdata+4 < ps+mem::size_of_val(&s));

    let mut s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    let ptr = s.as_ptr() as usize;
    let r = s.deref_mut();
    assert_eq!(r.as_ptr() as usize, ptr);
    assert_eq!(r,"the quick brown fox jumped over the lazy dog");
    let rptr = r.as_ptr();
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(rptr as usize, s.as_ptr() as usize);
    
}

#[test]
fn test_add_and_eq() {
    let mut s  = MAStringBuilder::from_slice("test");
    s = s + "foo";
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s,"testfoo");
    s = s + "the quick brown fox jumped over the lazy dog";
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s,"testfoothe quick brown fox jumped over the lazy dog");
    assert_eq!(s,MAStringBuilder::from_slice(&s));

    let mut s  = MAStringBuilder::from_slice("test");
    s += "foo";
    assert_eq!("short",s.get_mode());
    assert_eq!("testfoo",s);
    s += "the quick brown fox jumped over the lazy dog";
    assert_eq!("unique",s.get_mode());
    assert_eq!("testfoothe quick brown fox jumped over the lazy dog",s);
    assert_eq!(MAStringBuilder::from_slice(&s),s);

}

#[test]
fn test_display() {
    let s  = MAStringBuilder::from_slice("test \" \\ ");
    let sd = format!("{}",s);
    assert_eq!(sd,r#"test " \ "#)
}

#[test]
fn test_debug() {
    let s  = MAStringBuilder::from_slice("test \" \\ ");
    let sd = format!("{:?}",s);
    assert_eq!(sd,r#""test \" \\ ""#)
}

#[test]
fn test_as_mut_slice() {
    let mut s = MAStringBuilder::from_slice("test");
    s.as_mut_str().make_ascii_uppercase();
    assert_eq!(s,"TEST");
}

#[test]
fn test_into_vec() {
    let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    let capacity = s.capacity();
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_eq!(v.as_ptr(),ptr);
    assert_eq!(v.capacity(),capacity);

    let s = MAStringBuilder::from_slice("test");
    let ptr = s.as_ptr();
    let v = s.into_vec();
    assert_ne!(v.as_ptr(),ptr);
}

#[test]
fn test_from_utf8_unchecked() {
    unsafe {
        let s = MAStringBuilder::from_utf8_unchecked(MAByteStringBuilder::from_slice(b"test"));
        assert_eq!(s,"test");
    }
}

#[test]
fn test_from_utf8() {
    let s = MAStringBuilder::from_utf8(MAByteStringBuilder::from_slice(b"test"));
    assert_eq!(s,Ok(MAStringBuilder::from_slice("test")));
    let s = MAStringBuilder::from_utf8(MAByteStringBuilder::from_slice(b"\xFF"));
    assert_eq!(s.unwrap_err().as_bytes(),b"\xFF");
}

#[test]
fn test_from_utf8_lossy() {
    let s = MAStringBuilder::from_utf8_lossy(MAByteStringBuilder::from_slice(b"test"));
    assert_eq!(s,MAStringBuilder::from_slice("test"));
    let s = MAStringBuilder::from_utf8_lossy(MAByteStringBuilder::from_slice(b"\xFF"));
    assert_eq!(s,MAStringBuilder::from_slice("\u{FFFD}"));
}

#[test]
fn test_into_bytes() {
    let s = MAStringBuilder::from_slice("the quick brown fox jumped over the lazy dog");
    assert_eq!(s.into_bytes(),b"the quick brown fox jumped over the lazy dog");
}

#[test]
fn test_sets() {
    let mut h = HashSet::new();
    h.insert(MAStringBuilder::from_slice("The quick brown fox jumped over the lazy dog"));
    h.insert(MAStringBuilder::from_slice("The quick brown fox jumped over the smart dog"));
    h.insert(MAStringBuilder::from_slice("foo"));
    h.insert(MAStringBuilder::from_slice("bar"));
    assert_eq!(h.contains(&MAStringBuilder::from_slice("The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAStringBuilder::from_slice("The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains("foo"),true);
    assert_eq!(h.contains("baz"),false);

    let mut h = BTreeSet::new();
    h.insert(MAStringBuilder::from_slice("The quick brown fox jumped over the lazy dog"));
    h.insert(MAStringBuilder::from_slice("The quick brown fox jumped over the smart dog"));
    h.insert(MAStringBuilder::from_slice("foo"));
    h.insert(MAStringBuilder::from_slice("bar"));
    assert_eq!(h.contains(&MAStringBuilder::from_slice("The quick brown fox jumped over the lazy dog")),true);
    assert_eq!(h.contains(&MAStringBuilder::from_slice("The quick brown fox jumped over the stupid dog")),false);
    assert_eq!(h.contains("foo"),true);
    assert_eq!(h.contains("baz"),false);
}

#[test]
fn test_comparision() {
    assert!(MAStringBuilder::from_slice("A") < MAStringBuilder::from_slice("B"));
}

#[test]
fn test_macro() {
    let s = masb!("foo");
    assert_eq!(s.get_mode(),"short");
    let s = masb!("The quick brown fox jumped over the smart dog");
    assert_eq!(s.get_mode(),"unique");
    let s = masb!(['1','2','3','4']);
    assert_eq!(s.get_mode(),"short");
    let s = masb!(['1','2','3','4','5','6','7','8','9','0','1','2','3','4','5','6','7','8','9','0','1','2','3','4','5','6','7','8','9','0','1','2','\u{03A9}','\u{2261}','\u{1f980}']);
    assert_eq!(s,"12345678901234567890123456789012\u{03A9}\u{2261}\u{1f980}");
    assert_eq!(s.get_mode(),"unique");
    let foo = "foo";
    let s = masb!(foo);
    assert_eq!(s.get_mode(),"short");
    let foo = "The slow brown fox jumped over the sleeping dog";
    let s = masb!(foo);
    assert_eq!(s.get_mode(),"unique");
    let s = masb!(foo.to_string());
    assert_eq!(s.get_mode(),"unique");
    let four = '4';
    let s = masb!(&['1','2','3',four]);
    assert_eq!(s,"1234");
    assert_eq!(s.get_mode(),"short");
    let s = masb!(&foo.to_string());
    assert_eq!(s.get_mode(),"unique");
    let s = masb!(s);
    assert_eq!(s.get_mode(),"unique");
    let s = masb!(&s);
    assert_eq!(s.get_mode(),"unique");
    let sb = MAString::from_slice("The slow brown fox jumped over the sleeping dog");
    let s = masb!(&sb);
    assert_eq!(s.get_mode(),"unique");
    let s = masb!(sb);
    assert_eq!(s.get_mode(),"unique");

}

#[test]
fn test_join() {
    let s = masb!(",").join(["1","2","3","4","5","6","7","8","9","0"]);
    assert_eq!(s,"1,2,3,4,5,6,7,8,9,0");
}
