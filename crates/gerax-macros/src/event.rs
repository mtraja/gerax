use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitInt, LitStr};

#[derive(Default)]
struct EventAttr {
    name: Option<LitStr>,
    version: Option<LitInt>,
    integration: bool,
}

pub(crate) fn expand_event(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let type_attr = parse_event_attr(input)?;

    if matches!(input.data, Data::Union(_)) {
        return Err(Error::new_spanned(
            input,
            "Event derive can only be applied to a struct or enum",
        ));
    }

    let event_type_body = match &input.data {
        Data::Struct(_) => {
            let event_name = type_event_name(&type_attr, name)?;
            quote! { #event_name }
        }
        Data::Enum(data) => {
            if type_attr.name.is_some() {
                return Err(Error::new_spanned(
                    name,
                    "`name` is not supported at enum level; annotate each variant with \
                     `#[event(name = ...)]`",
                ));
            }

            let mut arms = Vec::with_capacity(data.variants.len());
            for variant in &data.variants {
                let variant_attr = parse_variant_event_attr(variant)?;
                let variant_name = variant_event_name(&type_attr, &variant_attr, &variant.ident)?;

                let ident = &variant.ident;
                let pattern = match &variant.fields {
                    Fields::Named(_) => quote! { Self::#ident { .. } },
                    Fields::Unnamed(_) => quote! { Self::#ident(..) },
                    Fields::Unit => quote! { Self::#ident },
                };

                arms.push(quote! { #pattern => #variant_name });
            }

            quote! {
                match self {
                    #(#arms),*
                }
            }
        }
        Data::Union(_) => unreachable!("union checked above"),
    };

    // The `Event` trait requires `Send + Sync + 'static`. Declaring the
    // bound on the impl keeps generic event types compiling and lets the
    // compiler reject only the offending instantiations.
    let mut generics = input.generics.clone();
    generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote!(
            Self: ::core::marker::Send + ::core::marker::Sync + 'static
        ));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let integration_impl = type_attr.integration.then(|| {
        quote! {
            impl #impl_generics ::gerax_eda::IntegrationEvent
                for #name #ty_generics #where_clause {}
        }
    });

    Ok(quote! {
        impl #impl_generics ::gerax_eda::Event for #name #ty_generics #where_clause {
            fn event_type(&self) -> &'static str {
                #event_type_body
            }
        }

        #integration_impl
    })
}

fn type_event_name(attr: &EventAttr, ident: &syn::Ident) -> Result<LitStr, Error> {
    let name = match &attr.name {
        Some(name) => name.clone(),
        None => LitStr::new(&ident.to_string(), ident.span()),
    };
    apply_version(name, attr.version.as_ref())
}

fn variant_event_name(
    type_attr: &EventAttr,
    variant_attr: &EventAttr,
    ident: &syn::Ident,
) -> Result<LitStr, Error> {
    let name = match &variant_attr.name {
        Some(name) => name.clone(),
        None => LitStr::new(&ident.to_string(), ident.span()),
    };
    let version = variant_attr.version.as_ref().or(type_attr.version.as_ref());
    apply_version(name, version)
}

fn apply_version(name: LitStr, version: Option<&LitInt>) -> Result<LitStr, Error> {
    let value = name.value();
    if value.trim().is_empty() {
        return Err(Error::new_spanned(&name, "event name cannot be empty"));
    }

    let Some(version) = version else {
        return Ok(name);
    };

    let number: u64 = version.base10_parse()?;
    if number == 0 {
        return Err(Error::new_spanned(
            version,
            "event version must be greater than zero",
        ));
    }

    Ok(LitStr::new(&format!("{value}.v{number}"), name.span()))
}

fn parse_event_attr(input: &DeriveInput) -> Result<EventAttr, Error> {
    let mut attr = EventAttr::default();

    for attribute in &input.attrs {
        if !attribute.path().is_ident("event") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if attr.name.is_some() {
                    return Err(meta.error("duplicate `name` in `event` attribute"));
                }
                attr.name = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("version") {
                if attr.version.is_some() {
                    return Err(meta.error("duplicate `version` in `event` attribute"));
                }
                attr.version = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("integration") {
                if meta.input.peek(syn::Token![=]) {
                    return Err(meta.error("`integration` does not take a value"));
                }
                attr.integration = true;
                Ok(())
            } else {
                Err(meta.error(
                    "unknown `event` attribute; expected `name`, `version` or `integration`",
                ))
            }
        })?;
    }

    Ok(attr)
}

fn parse_variant_event_attr(variant: &syn::Variant) -> Result<EventAttr, Error> {
    let mut attr = EventAttr::default();

    for attribute in &variant.attrs {
        if !attribute.path().is_ident("event") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if attr.name.is_some() {
                    return Err(meta.error("duplicate `name` in `event` attribute"));
                }
                attr.name = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("version") {
                if attr.version.is_some() {
                    return Err(meta.error("duplicate `version` in `event` attribute"));
                }
                attr.version = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("integration") {
                Err(meta.error("`integration` is only allowed at type level"))
            } else {
                Err(meta.error("unknown `event` variant attribute; expected `name` or `version`"))
            }
        })?;
    }

    Ok(attr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn struct_defaults_to_rust_name() {
        let input: DeriveInput = parse_quote! {
            struct AlunoMatriculado {
                aluno_id: String,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains(":: gerax_eda :: Event for AlunoMatriculado"));
        assert!(output.contains("\"AlunoMatriculado\""));
        assert!(output.contains("Send"));
        assert!(output.contains("Sync"));
        assert!(!output.contains("IntegrationEvent"));
    }

    #[test]
    fn struct_honors_custom_name() {
        let input: DeriveInput = parse_quote! {
            #[event(name = "aluno.matriculado")]
            struct AlunoMatriculado {
                aluno_id: String,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains("\"aluno.matriculado\""));
    }

    #[test]
    fn struct_appends_version_suffix() {
        let input: DeriveInput = parse_quote! {
            #[event(name = "aluno.matriculado", version = 2)]
            struct AlunoMatriculadoV2 {
                aluno_id: String,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains("\"aluno.matriculado.v2\""));
    }

    #[test]
    fn struct_can_implement_integration_event() {
        let input: DeriveInput = parse_quote! {
            #[event(name = "aluno.matriculado", version = 1, integration)]
            struct AlunoMatriculadoV1 {
                aluno_id: String,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains(":: gerax_eda :: IntegrationEvent"));
    }

    #[test]
    fn generic_struct_adds_self_bounds() {
        let input: DeriveInput = parse_quote! {
            struct Evento<T> {
                payload: T,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains("impl < T >"));
        assert!(output.contains("where Self : :: core :: marker :: Send"));
        assert!(output.contains("Send + :: core :: marker :: Sync"));
    }

    #[test]
    fn enum_defaults_to_variant_names() {
        let input: DeriveInput = parse_quote! {
            enum StudentEvent {
                StudentCreated { id: String },
                StudentRenamed(String),
                StudentDeleted,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains("Self :: StudentCreated { .. } => \"StudentCreated\""));
        assert!(output.contains("Self :: StudentRenamed (..) => \"StudentRenamed\""));
        assert!(output.contains("Self :: StudentDeleted => \"StudentDeleted\""));
    }

    #[test]
    fn enum_honors_variant_names_and_versions() {
        let input: DeriveInput = parse_quote! {
            #[event(version = 1)]
            enum StudentEvent {
                #[event(name = "student.created")]
                StudentCreated { id: String },
                #[event(name = "student.renamed", version = 3)]
                StudentRenamed { id: String },
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains("\"student.created.v1\""));
        assert!(output.contains("\"student.renamed.v3\""));
    }

    #[test]
    fn enum_can_implement_integration_event() {
        let input: DeriveInput = parse_quote! {
            #[event(integration)]
            enum StudentEvent {
                StudentCreated,
            }
        };
        let output = expand_event(&input).unwrap().to_string();
        assert!(output.contains(":: gerax_eda :: IntegrationEvent"));
    }

    #[test]
    fn enum_level_name_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[event(name = "student")]
            enum StudentEvent {
                StudentCreated,
            }
        };
        let error = expand_event(&input).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("`name` is not supported at enum level")
        );
    }

    #[test]
    fn union_is_rejected() {
        let input: DeriveInput = parse_quote! {
            union StudentEvent {
                id: u64,
            }
        };
        let error = expand_event(&input).unwrap_err();
        assert!(error.to_string().contains("struct or enum"));
    }

    #[test]
    fn unknown_attribute_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[event(topic = "aluno")]
            struct AlunoMatriculado;
        };
        let error = expand_event(&input).unwrap_err();
        assert!(error.to_string().contains("unknown `event` attribute"));
    }

    #[test]
    fn integration_with_value_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[event(integration = true)]
            struct AlunoMatriculado;
        };
        let error = expand_event(&input).unwrap_err();
        assert!(error.to_string().contains("does not take a value"));
    }

    #[test]
    fn zero_version_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[event(version = 0)]
            struct AlunoMatriculado;
        };
        let error = expand_event(&input).unwrap_err();
        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn empty_name_is_rejected() {
        let input: DeriveInput = parse_quote! {
            #[event(name = "")]
            struct AlunoMatriculado;
        };
        let error = expand_event(&input).unwrap_err();
        assert!(error.to_string().contains("cannot be empty"));
    }
}
