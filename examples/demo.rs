use mastring::MAString;
fn main() {
    let foo = MAString::from_static("hello");
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());

    let foo = MAString::from_static("the quick brown fox jumped over the lazy dog");
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());

    let foo = MAString::from_slice("hello");
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());

    let foo = MAString::from_slice("the quick brown fox jumped over the lazy dog");
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());

    let foo = MAString::from_string("hello".to_string());
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());

    let foo = MAString::from_string("the quick brown fox jumped over the lazy dog".to_string());
    println!("{} {}",foo,foo.getMode());
    let bar = foo.clone();
    println!("{} {}",foo,foo.getMode());
    println!("{} {}",bar,bar.getMode());


}