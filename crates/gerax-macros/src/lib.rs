use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, ItemFn, Pat, parse_macro_input};

#[proc_macro_derive(Entity, attributes(entity))]
pub fn entity_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_entity(&input)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    expand_handler(&input)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

fn expand_handler(input: &ItemFn) -> Result<TokenStream2, Error> {
    let func_name = &input.sig.ident;
    let vis = &input.vis;
    let async_token = &input.sig.asyncness;
    let sig = &input.sig;
    let generics = &sig.generics;

    if async_token.is_none() {
        return Err(Error::new_spanned(input, "handler function must be async"));
    }

    let generic_names: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
            _ => None,
        })
        .collect();

    let mut state_type: Option<syn::Type> = None;
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Type::Path(type_path) = pat_type.ty.as_ref() {
                if let Some(seg) = type_path.path.segments.last() {
                    if seg.ident == "State" {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                state_type = Some(inner_ty.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let state_type = state_type
        .ok_or_else(|| Error::new_spanned(input, "handler must have a `State<T>` parameter"))?;

    let is_state_generic = if let syn::Type::Path(type_path) = &state_type {
        if let Some(seg) = type_path.path.segments.last() {
            generic_names.contains(&seg.ident.to_string())
        } else {
            false
        }
    } else {
        false
    };

    let mut extracts = Vec::new();
    let mut arg_names = Vec::new();

    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            let (var_name, ty) = extract_param_name_and_type(&pat_type.pat, pat_type.ty.as_ref())?;
            arg_names.push(var_name.clone());

            let is_infallible = is_infallible_type(pat_type.ty.as_ref());

            if is_infallible {
                extracts.push(quote! {
                    let #var_name = <#ty as ::gerax_http::routing::extractors::FromContext<S>>::from_context(&ctx).unwrap();
                });
            } else {
                extracts.push(quote! {
                    let #var_name = <#ty as ::gerax_http::routing::extractors::FromContext<S>>::from_context(&ctx)
                        .map_err(|e| ::gerax_http::HttpServerError::HandlerError(e.to_string()))?;
                });
            }
        }
    }

    let wrapper_name = syn::Ident::new(
        &format!("{}Handler", to_upper_camel_case(&func_name.to_string())),
        func_name.span(),
    );

    let wrapper_impl = if is_state_generic {
        quote! {
            #vis struct #wrapper_name;

            #[::async_trait::async_trait]
            impl<S: Send + Sync + 'static> ::gerax_http::routing::Handler<S> for #wrapper_name {
                async fn call(
                    &self,
                    ctx: ::gerax_http::routing::Context<S>,
                ) -> ::gerax_http::ServerResult<::gerax_http::routing::Response> {
                    #(#extracts)*
                    #func_name(#(#arg_names),*).await
                }
            }
        }
    } else {
        quote! {
            #vis struct #wrapper_name;

            #[::async_trait::async_trait]
            impl ::gerax_http::routing::Handler<#state_type> for #wrapper_name {
                async fn call(
                    &self,
                    ctx: ::gerax_http::routing::Context<#state_type>,
                ) -> ::gerax_http::ServerResult<::gerax_http::routing::Response> {
                    #(#extracts)*
                    #func_name(#(#arg_names),*).await
                }
            }
        }
    };

    let original_func = quote! {
        #input
    };

    Ok(quote! {
        #original_func
        #wrapper_impl
    })
}

fn extract_param_name_and_type(
    pat: &Pat,
    ty: &syn::Type,
) -> Result<(syn::Ident, syn::Type), Error> {
    match pat {
        Pat::Ident(pat_ident) => Ok((pat_ident.ident.clone(), ty.clone())),
        Pat::TupleStruct(pat_tuple_struct) => {
            if pat_tuple_struct.elems.len() == 1 {
                if let Pat::Ident(inner_ident) = &pat_tuple_struct.elems[0] {
                    return Ok((inner_ident.ident.clone(), ty.clone()));
                }
            }
            Err(Error::new_spanned(
                pat,
                "unsupported pattern in handler parameter, use `name: Type` or `Type(name)`",
            ))
        }
        _ => Err(Error::new_spanned(
            pat,
            "unsupported pattern in handler parameter, use `name: Type` or `Type(name)`",
        )),
    }
}

fn is_infallible_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            let name = seg.ident.to_string();
            return name == "State" || name == "Request";
        }
    }
    false
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
            ));
        }
    };

    let fields = match &data.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Entity derive macro can only be applied to structs with named fields",
            ));
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
        .ok_or_else(|| {
            Error::new_spanned(
                input,
                "struct must have an `id` field of type `Option<String>`",
            )
        })?;

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

#[proc_macro_derive(Command, attributes(command))]
pub fn command_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_command(input: &DeriveInput) -> Result<TokenStream2, Error> {
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

#[proc_macro_derive(Query, attributes(query))]
pub fn query_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_query(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_query(input: &DeriveInput) -> Result<TokenStream2, Error> {
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

fn to_upper_camel_case(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

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
        assert!(
            err.to_string()
                .contains("`id` field must be of type `Option<String>`")
        );
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
