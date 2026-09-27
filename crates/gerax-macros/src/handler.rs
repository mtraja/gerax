use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Error, ItemFn, Pat};

use crate::util::to_upper_camel_case;

pub(crate) fn expand_handler(input: &ItemFn) -> Result<TokenStream2, Error> {
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
