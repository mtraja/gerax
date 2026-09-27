use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Error, Fields};

pub(crate) fn expand_aggregate(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Aggregate derive can only be applied to a struct",
            ));
        }
    };

    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Aggregate derive requires a struct with named fields",
            ));
        }
    };

    let require_field = |want: &str, expect: &str| -> Result<(), Error> {
        for field in fields.iter() {
            if field.ident.as_ref().is_some_and(|i| i == want) {
                return Ok(());
            }
        }
        Err(Error::new_spanned(
            input,
            format!("missing `{want}` field (expected {expect})"),
        ))
    };

    require_field("id", "the aggregate identifier")?;
    require_field("version", "a `Version` field")?;
    require_field("pending", "a `Vec<Event>` field")?;

    let id_ty = fields
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "id"))
        .map(|f| &f.ty)
        .expect("id field checked above");

    let mut aggregate_type: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("aggregate") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("aggregate_type") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    aggregate_type = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("unknown `aggregate` attribute"))
                }
            })?;
        }
    }

    let aggregate_type_lit = match aggregate_type {
        Some(value) => syn::LitStr::new(&value, name.span()),
        None => syn::LitStr::new(&name.to_string(), name.span()),
    };

    let fields_init: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let Some(ident) = field.ident.as_ref() else {
                unreachable!("named fields always have identifiers")
            };
            if ident == "id" {
                quote! { id }
            } else if ident == "version" {
                quote! { version: ::gerax_es::Version::initial() }
            } else if ident == "pending" {
                quote! { pending: Vec::new() }
            } else {
                quote! { #ident: Default::default() }
            }
        })
        .collect();

    Ok(quote! {
        impl #impl_generics ::gerax_es::Aggregate for #name #ty_generics #where_clause {
            type Id = #id_ty;

            fn aggregate_type() -> &'static str {
                #aggregate_type_lit
            }

            fn empty(id: Self::Id) -> Self {
                Self {
                    #(#fields_init),*
                }
            }

            fn id(&self) -> &Self::Id {
                &self.id
            }

            fn version(&self) -> ::gerax_es::Version {
                self.version
            }

            fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error> {
                self.apply(&event)?;
                self.pending.push(event);
                Ok(())
            }

            fn take_events(&mut self) -> Vec<Self::Event> {
                std::mem::take(&mut self.pending)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn expand_aggregate_generates_full_impl() {
        let input: DeriveInput = parse_quote! {
            struct Student {
                id: String,
                name: String,
                version: Version,
                pending: Vec<StudentEvent>,
            }
        };
        let tokens = expand_aggregate(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("type Id = String"));
        assert!(output.contains(":: gerax_es :: Aggregate for Student"));
        assert!(output.contains("fn empty"));
        assert!(output.contains("Self :: Id"));
        assert!(output.contains(":: gerax_es :: Version :: initial"));
        assert!(output.contains("fn raise"));
        assert!(output.contains("fn take_events"));
        assert!(output.contains("std :: mem :: take"));
        assert!(output.contains("Default :: default"));
    }

    #[test]
    fn expand_aggregate_defaults_aggregate_type_to_struct_name() {
        let input: DeriveInput = parse_quote! {
            struct Student {
                id: String,
                version: Version,
                pending: Vec<StudentEvent>,
            }
        };
        let tokens = expand_aggregate(&input).unwrap();
        assert!(tokens.to_string().contains("\"Student\""));
    }

    #[test]
    fn expand_aggregate_honors_custom_aggregate_type() {
        let input: DeriveInput = parse_quote! {
            #[aggregate(aggregate_type = "Aluno")]
            struct Student {
                id: String,
                version: Version,
                pending: Vec<StudentEvent>,
            }
        };
        let tokens = expand_aggregate(&input).unwrap();
        assert!(tokens.to_string().contains("\"Aluno\""));
    }

    #[test]
    fn expand_aggregate_fails_on_enum() {
        let input: DeriveInput = parse_quote! {
            enum Student {
                A,
                B,
            }
        };
        let err = expand_aggregate(&input).unwrap_err();
        assert!(err.to_string().contains("only be applied to a struct"));
    }

    #[test]
    fn expand_aggregate_fails_on_named_fieldless_struct() {
        let input: DeriveInput = parse_quote! {
            struct Student;
        };
        let err = expand_aggregate(&input).unwrap_err();
        assert!(err.to_string().contains("named fields"));
    }

    #[test]
    fn expand_aggregate_fails_without_id() {
        let input: DeriveInput = parse_quote! {
            struct Student {
                name: String,
                version: Version,
                pending: Vec<StudentEvent>,
            }
        };
        let err = expand_aggregate(&input).unwrap_err();
        assert!(err.to_string().contains("missing `id` field"));
    }

    #[test]
    fn expand_aggregate_fails_without_version() {
        let input: DeriveInput = parse_quote! {
            struct Student {
                id: String,
                pending: Vec<StudentEvent>,
            }
        };
        let err = expand_aggregate(&input).unwrap_err();
        assert!(err.to_string().contains("missing `version` field"));
    }

    #[test]
    fn expand_aggregate_fails_without_pending() {
        let input: DeriveInput = parse_quote! {
            struct Student {
                id: String,
                version: Version,
            }
        };
        let err = expand_aggregate(&input).unwrap_err();
        assert!(err.to_string().contains("missing `pending` field"));
    }
}
