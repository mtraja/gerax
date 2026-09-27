use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Error};

pub(crate) fn expand_command(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let output_ty = parse_output_attr(input, "command")?;
    let name_lit = syn::LitStr::new(&name.to_string(), name.span());

    Ok(quote! {
        impl #impl_generics ::gerax_cqrs::Message for #name #ty_generics #where_clause {
            type Output = #output_ty;
        }

        impl #impl_generics ::gerax_cqrs::Command for #name #ty_generics #where_clause {}

        impl #impl_generics ::gerax_cqrs::CommandMetadata for #name #ty_generics #where_clause {
            const NAME: &'static str = #name_lit;
        }
    })
}

pub(crate) fn expand_query(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let output_ty = parse_output_attr(input, "query")?;
    let name_lit = syn::LitStr::new(&name.to_string(), name.span());

    Ok(quote! {
        impl #impl_generics ::gerax_cqrs::Message for #name #ty_generics #where_clause {
            type Output = #output_ty;
        }

        impl #impl_generics ::gerax_cqrs::Query for #name #ty_generics #where_clause {}

        impl #impl_generics ::gerax_cqrs::QueryMetadata for #name #ty_generics #where_clause {
            const NAME: &'static str = #name_lit;
        }
    })
}

fn parse_output_attr(input: &DeriveInput, attr_name: &str) -> Result<syn::Type, Error> {
    let mut output_ty: Option<syn::Type> = None;
    for attr in &input.attrs {
        if attr.path().is_ident(attr_name) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("output") {
                    let value = meta.value()?;
                    let ty: syn::Type = value.parse()?;
                    output_ty = Some(ty);
                    Ok(())
                } else {
                    Err(meta.error(format!("unknown attribute `{attr_name}`")))
                }
            })?;
        }
    }
    output_ty.ok_or_else(|| {
        Error::new_spanned(
            input,
            format!("missing `#[{attr_name}(output = ...)]` attribute"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn derive_command_generates_message_command_and_metadata() {
        let input: DeriveInput = parse_quote! {
            #[command(output = Aluno)]
            struct CreateAluno {
                pub nome: String,
            }
        };
        let tokens = expand_command(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("type Output = Aluno"));
        assert!(output.contains(":: gerax_cqrs :: Command"));
        assert!(output.contains(":: gerax_cqrs :: CommandMetadata"));
        assert!(output.contains("CreateAluno"));
    }

    #[test]
    fn derive_command_missing_output_attr_fails() {
        let input: DeriveInput = parse_quote! {
            struct CreateAluno {
                pub nome: String,
            }
        };
        let err = expand_command(&input).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn derive_command_complex_output_type() {
        let input: DeriveInput = parse_quote! {
            #[command(output = Vec<Aluno>)]
            struct ListAlunos;
        };
        let tokens = expand_command(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("Vec < Aluno >"));
    }

    #[test]
    fn derive_query_generates_message_query_and_metadata() {
        let input: DeriveInput = parse_quote! {
            #[query(output = Option<Aluno>)]
            struct GetAluno {
                pub id: u64,
            }
        };
        let tokens = expand_query(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("type Output = Option < Aluno >"));
        assert!(output.contains(":: gerax_cqrs :: Query"));
        assert!(output.contains(":: gerax_cqrs :: QueryMetadata"));
        assert!(output.contains("GetAluno"));
    }

    #[test]
    fn derive_query_complex_output_type() {
        let input: DeriveInput = parse_quote! {
            #[query(output = Result<Aluno, String>)]
            struct GetAlunoResult {
                pub id: u64,
            }
        };
        let tokens = expand_query(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("Result < Aluno , String >"));
    }

    #[test]
    fn derive_query_nested_output_type() {
        let input: DeriveInput = parse_quote! {
            #[query(output = Option<Vec<Aluno>>)]
            struct ListAlunosOption;
        };
        let tokens = expand_query(&input).unwrap();
        let output = tokens.to_string();
        assert!(output.contains("Option < Vec < Aluno > >"));
    }
}
