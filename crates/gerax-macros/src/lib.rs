use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error};
use syn::spanned::Spanned;

#[proc_macro_derive(Entity, attributes(entity))]
pub fn entity_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_entity(&input).unwrap_or_else(|e| e.to_compile_error().into()).into()
}

fn expand_entity(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Entity derive macro can only be applied to structs",
            ))
        }
    };

    let fields = match &data.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Entity derive macro can only be applied to structs with named fields",
            ))
        }
    };

    let mut custom_collection_name: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("entity") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("collection_name") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    custom_collection_name = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("unknown entity attribute"))
                }
            })?;
        }
    }

    let id_field = fields
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "id").unwrap_or(false))
        .ok_or_else(|| Error::new_spanned(input, "struct must have an `id` field of type `Option<String>`"))?;

    let id_ty = &id_field.ty;
    let is_option_string = match id_ty {
        syn::Type::Path(type_path) => {
            let last = type_path.path.segments.last().map(|s| &s.ident);
            last == Some(&syn::Ident::new("Option", id_ty.span()))
                && type_path.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(syn::Type::Path(inner)) => {
                                inner.path.is_ident("String")
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
        }
        _ => false,
    };

    if !is_option_string {
        return Err(Error::new_spanned(
            id_ty,
            "`id` field must be of type `Option<String>`",
        ));
    }

    let collection_name = custom_collection_name.unwrap_or_else(|| {
        let snake = to_snake_case(&name.to_string());
        format!("{snake}s")
    });

    let collection_name_lit = syn::LitStr::new(&collection_name, name.span());

    let expanded = quote! {
        impl #impl_generics gerax_core::Entity for #name #ty_generics #where_clause {
            fn collection_name() -> &'static str {
                #collection_name_lit
            }

            fn id(&self) -> Option<String> {
                self.id.clone()
            }

            fn set_id(&mut self, id: String) {
                self.id = Some(id);
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, DeriveInput};

    #[test]
    fn derive_entity_on_non_struct_fails() {
        let input: DeriveInput = parse_quote! {
            enum Foo {
                A,
                B,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(err.to_string().contains("can only be applied to structs"));
    }

    #[test]
    fn derive_entity_without_id_field_fails() {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                name: String,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(err.to_string().contains("`id` field"));
    }

    #[test]
    fn derive_entity_with_wrong_id_type_fails() {
        let input: DeriveInput = parse_quote! {
            struct Foo {
                id: i32,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(err.to_string().contains("`id` field must be of type `Option<String>`"));
    }

    #[test]
    fn derive_entity_with_custom_collection_name() {
        let input: DeriveInput = parse_quote! {
            #[entity(collection_name = "items")]
            struct Item {
                id: Option<String>,
                value: String,
            }
        };
        let tokens = expand_entity(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("items"));
        assert!(output.contains("fn collection_name"));
        assert!(output.contains("fn id"));
        assert!(output.contains("fn set_id"));
    }

    #[test]
    fn derive_entity_generates_snake_case_collection_name() {
        let input: DeriveInput = parse_quote! {
            struct UserProfile {
                id: Option<String>,
                name: String,
            }
        };
        let tokens = expand_entity(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("user_profiles"));
        assert!(output.contains("fn collection_name"));
    }
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
