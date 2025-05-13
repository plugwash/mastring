use mastring::MAString;
use std::sync::Mutex;

static GLOBALSTRING : Mutex<MAString> = Mutex::new(MAString::from_static("global string"));

fn main() {
    let foo = MAString::from_static("hello");
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let foo = MAString::from_static("the quick brown fox jumped over the lazy dog");
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let foo = MAString::from_slice("hello");
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let foo = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let foo = MAString::from_string("hello".to_string());
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let foo = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    println!("{} {}",foo,foo.get_mode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.get_mode());
    println!("{} {}",bar,bar.get_mode());

    let globalstring = GLOBALSTRING.lock().unwrap();
    println!("{} {}",globalstring,globalstring.get_mode());

}