## unreleased

 * Move from Unions to pointer casts and transmute to allow niche optimisation
 * Split code into modules.
 * Implement Borrow, Hash, PartialOrd and Ord traits to allow use in sets/maps.
 * Support rustc 1.63 (the version currently in Debian stable).
 * Add functions to create MAString/MAStringBuilder from a slice of chars.
 * Implement the From trait and add conviniance macros for creating the types
   in this package.
 * Add with_capacity methods to allow construction with pre-allocated capacity
 * Implement FromIterator<u8> for MAByteString and MAByteStringBuilder
 * Implement FromIterator<char> for MAString and MAStringBuilder
 * Implement fmt::write for MAString and MAStringBuilder
 * Relax parameter type for from_utf8* functions.

## [0.2.0] - 2023-05-25

 * First usable release
 * Fix many bugs
 * Add many features
 * Add tests

## [0.1.0] - 2023-03-30

 * Initial proof of concept release.
