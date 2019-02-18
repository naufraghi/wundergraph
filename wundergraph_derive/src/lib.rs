#![recursion_limit = "1024"]
#![deny(missing_debug_implementations, missing_copy_implementations)]
#![cfg_attr(feature = "cargo-clippy", allow(renamed_and_removed_lints))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy))]
// Clippy lints
#![cfg_attr(feature = "cargo-clippy", allow(type_complexity))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        wrong_pub_self_convention,
        used_underscore_binding,
        use_self,
        unseparated_literal_suffix,
        unnecessary_unwrap,
        unimplemented,
        single_match_else,
        shadow_unrelated,
        option_map_unwrap_or_else,
        option_map_unwrap_or,
        needless_continue,
        mutex_integer,
        needless_borrow,
        items_after_statements,
        filter_map,
        expl_impl_clone_on_copy,
        else_if_without_else,
        doc_markdown,
        default_trait_access,
        option_unwrap_used,
        result_unwrap_used,
        wrong_pub_self_convention,
        mut_mut,
        non_ascii_literal,
        unicode_not_nfc,
        enum_glob_use,
        if_not_else,
        items_after_statements,
        used_underscore_binding
    )
)]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

mod diagnostic_shim;
mod field;
mod meta;
mod model;
mod resolved_at_shim;
mod utils;

mod build_filter;
mod filter;
mod filter_value;
mod from_lookahead;
mod inner_filter;
mod nameable;
mod wundergraph_entity;

use self::diagnostic_shim::Diagnostic;
use proc_macro::TokenStream;

#[proc_macro_derive(Nameable)]
pub fn derive_nameable(input: TokenStream) -> TokenStream {
    expand_derive(input, nameable::derive)
}

#[proc_macro_derive(FilterValue)]
pub fn derive_filter_value(input: TokenStream) -> TokenStream {
    expand_derive(input, filter_value::derive)
}

#[proc_macro_derive(InnerFilter)]
pub fn derive_inner_filter(input: TokenStream) -> TokenStream {
    expand_derive(input, inner_filter::derive)
}

#[proc_macro_derive(BuildFilter, attributes(wundergraph))]
pub fn derive_build_filter(input: TokenStream) -> TokenStream {
    expand_derive(input, build_filter::derive)
}

#[proc_macro_derive(WundergraphEntity, attributes(wundergraph, table_name, primary_key))]
pub fn derive_wundergraph_entity(input: TokenStream) -> TokenStream {
    expand_derive(input, wundergraph_entity::derive)
}

#[proc_macro_derive(WundergraphFilter, attributes(wundergraph, table_name))]
pub fn derive_wundergraph_filter(input: TokenStream) -> TokenStream {
    expand_derive(input, filter::derive)
}

#[proc_macro_derive(FromLookAhead)]
pub fn derive_from_lookahead(input: TokenStream) -> TokenStream {
    expand_derive(input, from_lookahead::derive)
}

fn expand_derive(
    input: TokenStream,
    f: fn(&syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic>,
) -> TokenStream {
    let item = syn::parse(input).expect("Failed to parse item");
    match f(&item) {
        Ok(x) => x.into(),
        Err(e) => {
            e.emit();
            "".parse().expect("Failed to parse item")
        }
    }
}
