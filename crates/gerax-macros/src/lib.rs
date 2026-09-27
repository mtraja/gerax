use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

mod aggregate;
mod cqrs;
mod entity;
mod handler;
mod util;

#[proc_macro_derive(Entity, attributes(entity))]
pub fn entity_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    entity::expand_entity(&input)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    handler::expand_handler(&input)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

#[proc_macro_derive(Aggregate, attributes(aggregate))]
pub fn aggregate_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    aggregate::expand_aggregate(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Command, attributes(command))]
pub fn command_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    cqrs::expand_command(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Query, attributes(query))]
pub fn query_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    cqrs::expand_query(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
