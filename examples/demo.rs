use mastring::MAString;
fn main() {
    let foo = MAString::from_static("hello");
    println!("{} {}",foo,foo.getMode());
}