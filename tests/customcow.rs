use mastring::CustomCow;
use mastring::mas;
use mastring::MAString;

#[test]
fn test_into_owned() {
    let c : CustomCow<MAString> = CustomCow::Borrowed("hi");
    assert_eq!("hi",c.into_owned());
    let c = CustomCow::Owned(mas!("there"));
    assert_eq!("there",c.into_owned());
}