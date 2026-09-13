// SPDX-License-Identifier: GPL-3.0-or-later

pub fn read_print(input: &str) -> String {
   todo!()
}


// pragma: only compile/run? when testing
#[cfg(test)]
mod tests {
    // we are creating a namespace here called "test"
    // and we want to reference to a function? in the root namespace
    // child namespaces are branches of the root i.e. main namespace I guess
    // but symbol name resolution doesn't fall back through parents
    // so without this statement we have to keep referencing our parent, which
    // is annoying
    // So we pull that symbol into our namespaces
    use super::read_print;

    // These are round-trip tests to make sure we can read in a value and get
    // back the same values.

    #[test]
    fn fixnum() {
        // This equality assertion is probably a debug level thing, not a unit
	// test thing. But it's part of Rust's "prelude", a set of functions
	// whose module don't need to be explicitly imported.
        assert_eq!(read_print("42"), "42");
    }
    #[test]
    fn negative_fixnum() {
        assert_eq!(read_print("-7"), "-7");
    }
    #[test]
    fn plus_prefixed_fixnum() {
        assert_eq!(read_print("+5"), "5");
    }
    #[test]
    fn nil_atom() {
        assert_eq!(read_print("nil"), "nil");
    }
    #[test]
    fn symbol() {
        assert_eq!(read_print("foo"), "foo");
    }
    #[test]
    fn lone_dash_is_symbol() {
        assert_eq!(read_print("-"), "-");
    }
    #[test]
    fn lone_plus_is_symbol() {
        assert_eq!(read_print("+"), "+");
    }
    #[test]
    fn one_plus_is_symbol() {
        assert_eq!(read_print("1+"), "1+");
    }
    #[test]
    fn one_minus_is_symbol() {
        assert_eq!(read_print("1-"), "1-");
    }
    #[test]
    fn string_atom() {
        assert_eq!(read_print("\"hi\""), "\"hi\"");
    }
    #[test]
    fn string_escapes() {
        assert_eq!(read_print(r#""a\"b\\c\nd""#), r#""a\"b\\c\nd""#);
    }
    #[test]
    fn multiple_atoms_one_per_line() {
        assert_eq!(read_print("1 2 3"), "1\n2\n3");
    }
    #[test]
    fn empty_input() {
        assert_eq!(read_print(""), "");
    }
}
