extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use quote::quote;

/*

/// Example of [function-like procedural macro][1].
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#function-like-procedural-macros
#[proc_macro]
pub fn test_fixture(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let tokens = quote! {
        struct Hello;
    };

    tokens.into()
}

/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_derive(MyDerive)]
pub fn my_derive(_input: TokenStream) -> TokenStream {
    let tokens = quote! {
        struct Hello;
    };

    tokens.into()
}
*/

/*
    given

    #[test_data(foo)]
    #[test]
    fn test_fixture_created() {}

    macro args: "foo"
    target: "#[test] fn test_fixture_created() {}"
 */
#[proc_macro_attribute]
pub fn test_data(macro_args: TokenStream, target: TokenStream) -> TokenStream {
/*    // eprintln!("target: \"{target}\"");
    eprintln!("macro args: \"{macro_args}\"");

    let target_input = parse_macro_input!(target as DeriveInput);

    // does the target already have #[test]?
    let has_test_macro = !target_input.attrs.iter()
        .any(|attr| attr.path().is_ident("test"));

    let test = if has_test_macro { "" } else { "#[test]" };

*/
    let tokens = quote! {
        struct __Tests {}
        
        impl __Tests {
            // #test
            fn a_test() {
                assert_eq!(0, 0);
            }
        }
    };

    tokens.into()
}

