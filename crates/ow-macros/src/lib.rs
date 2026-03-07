extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn api_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(ProtoConvert, attributes(proto))]
pub fn proto_convert(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    TokenStream::new()
}
