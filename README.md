A string library that aims to be easy to use and to reduce memory allocations.

This crate was inspired by the discussion 'Cow: Is it *actually* a
"copy-on-write smart pointer"?'on reddit, and particularly the following
comment by render787.

> When business logic needs to store string data, they don't want to run
> through a decision tree in their mind of &'static str, String, Cow, Arc,
> and all the pros and cons, or moreover, estimate in their mind how much
> time it would take them to plumb explicit lifetimes through the whole thing
> and how likely it is to work or that they will have to back out of it
> eventually, which is always complicated. If there was an `EasyString` that
> was never much worse than any of these options and didn't require explicit
> lifetimes, it's a good thing."

I distilled this down to a set of design goals.

1. It should be constructible from a string literal without any memory
   allocation. It should further be possible to do this in a const context
   so that global variables can be initialised.
2. It should require the same or less memory allocations than Arc<str> in a
   clone-heavy context.
3. It should require the same or less memory allocations than String in a
   maniupulation dominated context.
4. It shouldn't be too large, it probablly will end up bigger than the standard
   library types, but hopefully not by too much.
5. It should be cheap to convert from std::string, since many of the libaries
   your program calls are likely to use std::string.

The challenge was how to resolve goals 2 and 5. Reference countring requires
a "control block" which be located on the heap, but the existing memory
allocation for a std::string will often no room for a control block.

The soloution was a string type MAString four pointers in size with four modes.

 * Short string, strings up to 31 bytes (on 64-bit architectures can be stored
   directly in the string type without needing any memory allocation.
 * Static string, a pointer to a string in static memory.
 * Uniquely owned string.
 * String in shared ownership with co-located control block.
 * String in shared ownership with non co-located control block.

The crate also offers additional types.

 * MAByteString, similar to MAString but can store arbitrary bytes.
 * MAStringBuilder and MAByteStringBuilder, simplified types without the shared
   ownership logic to optimise string maniupulation.
 * CustomCow, while MAString replaces Cow<&'static,str> it cannot replace all
   uses of Cow. Unfortunately the standard library's Cow type does not allow
   for custom string types, so this library includes it's own, which is generic
   on the owned type rather than the borrowed type.

To facilitate concise construction of the types, macros are provided. These are
named as initialisms of their corresponding types, so mas! creates a MAString,
mabs! creates a MAByteString, masb creates a MAStringBuilder and MAByteString.
MAStrings can also be created through the From/Into traits and through specific
factory functions.

Usage example.

```rust
use mastring::{MAString, mas};
use std::{env, sync::Mutex};
static globalstring : Mutex<MAString> = Mutex::new(mas!("hi there"));

fn main() {
    println!("old global {}",globalstring.lock().unwrap());
    let env: MAString = env::var("global").map_or(mas!("undef"),From::from);
    *globalstring.lock().unwrap() = env;
    println!("env global {}",globalstring.lock().unwrap());

    let concatenated = mas!("foo") + "bar";
    println!("{}",concatenated);

    let collected: MAString = ["a","beta","c","d"].into_iter().collect();
    println!("{}",collected);

}
```

