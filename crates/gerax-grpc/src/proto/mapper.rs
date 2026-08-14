//! Mapeamento de tipos Rust para Protobuf e convenções de nomenclatura.

/// Mapeamento de tipos Rust para tipos Protobuf.
///
/// Utilizado pelo `ProtoBuilder` para gerar schemas `.proto` a partir de
/// entidades `gerax-core::Entity`.
///
/// Tipos suportados:
/// - `String` → `string`
/// - `i32` / `i64` → `int32` / `int64`
/// - `f32` / `f64` → `float` / `double`
/// - `bool` → `bool`
/// - `Vec<T>` → `repeated <tipo>`
#[derive(Debug, Clone)]
pub enum RustType {
    String,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Vec(Box<RustType>),
}

impl RustType {
    /// Retorna a representação Protobuf do tipo.
    pub fn to_proto_type(&self) -> String {
        match self {
            RustType::String => "string".to_string(),
            RustType::I32 => "int32".to_string(),
            RustType::I64 => "int64".to_string(),
            RustType::F32 => "float".to_string(),
            RustType::F64 => "double".to_string(),
            RustType::Bool => "bool".to_string(),
            RustType::Vec(inner) => format!("repeated {}", inner.to_proto_type()),
        }
    }
}

/// Aplica a convenção de nomenclatura do Gerax para Protobuf.
///
/// - Nome de entidade: `Entity::collection_name()` em `snake_case` é convertido
///   para `PascalCase` para nomes de mensagem Protobuf.
/// - Nome de arquivo: `<collection_name>.proto`.
pub struct NamingConvention;

impl NamingConvention {
    /// Converte um nome de coleção `snake_case` para `PascalCase`.
    ///
    /// Exemplos:
    /// - `"user"` → `"User"`
    /// - `"blog_post"` → `"BlogPost"`
    /// - `"order_item"` → `"OrderItem"`
    pub fn entity_to_message_name(collection_name: &str) -> String {
        collection_name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }

    /// Gera o nome de arquivo `.proto` para uma entidade.
    pub fn proto_file_name(collection_name: &str) -> String {
        format!("{}.proto", collection_name)
    }

    /// Converte nome de campo Rust para snake_case (garantia de convenção).
    ///
    /// Protobuf convenciona `snake_case` para campos; como Rust também usa
    /// `snake_case` para campos, esta função é identidade, mas permite
    /// normalizações futuras.
    pub fn field_name(field_name: &str) -> &str {
        field_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_conversion() {
        assert_eq!(NamingConvention::entity_to_message_name("user"), "User");
        assert_eq!(
            NamingConvention::entity_to_message_name("blog_post"),
            "BlogPost"
        );
        assert_eq!(
            NamingConvention::entity_to_message_name("order_item"),
            "OrderItem"
        );
        assert_eq!(NamingConvention::entity_to_message_name("a"), "A");
    }

    #[test]
    fn proto_file_name() {
        assert_eq!(NamingConvention::proto_file_name("user"), "user.proto");
        assert_eq!(
            NamingConvention::proto_file_name("blog_post"),
            "blog_post.proto"
        );
    }

    #[test]
    fn field_name_is_identity() {
        assert_eq!(NamingConvention::field_name("user_id"), "user_id");
        assert_eq!(NamingConvention::field_name("createdAt"), "createdAt");
    }
}
