use gerax_core::Entity;

use crate::proto::ProtoSchema;
use crate::proto::mapper::{NamingConvention, RustType};

/// Campo de uma entidade no schema Protobuf.
#[derive(Debug, Clone)]
pub struct ProtoField {
    /// Nome do campo no Protobuf.
    pub name: String,
    /// Tipo Rust mapeado para Protobuf.
    pub rust_type: RustType,
}

/// Entidade registrada no builder de schema.
#[derive(Debug, Clone)]
pub struct ProtoEntity {
    /// Nome da mensagem Protobuf (PascalCase).
    pub name: String,
    /// Campos da entidade.
    pub fields: Vec<ProtoField>,
}

/// Builder para geração de schema Protobuf a partir de entidades Gerax.
///
/// Registra entidades e produz definições `.proto` ou builders
/// de servidor/cliente tipados.
#[derive(Debug, Clone, Default)]
pub struct ProtoBuilder {
    entities: Vec<ProtoEntity>,
}

impl ProtoBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Registra uma entidade no schema.
    ///
    /// A entidade deve implementar `gerax_core::Entity`.
    /// O nome da coleção será convertido para `PascalCase` e usado como
    /// nome da mensagem Protobuf.
    pub fn entity<T: Entity + Send + Sync + 'static>(mut self) -> Self {
        let name = NamingConvention::entity_to_message_name(T::collection_name());
        self.entities.push(ProtoEntity {
            name,
            fields: Vec::new(),
        });
        self
    }

    /// Adiciona um campo à última entidade registrada.
    ///
    /// Se nenhuma entidade foi registrada, o campo é ignorado.
    pub fn field(mut self, name: impl Into<String>, rust_type: RustType) -> Self {
        if let Some(last) = self.entities.last_mut() {
            last.fields.push(ProtoField {
                name: name.into(),
                rust_type,
            });
        }
        self
    }

    /// Gera o schema Protobuf completo.
    ///
    /// Inclui:
    /// - `syntax = "proto3";`
    /// - `package gerax;`
    /// - Mensagens para cada entidade
    /// - Mensagens de request/response para cada operação CRUD
    /// - Serviço `EntityService` com RPCs CRUD por entidade
    pub fn build(self) -> ProtoSchema {
        let mut content = String::new();

        content.push_str("syntax = \"proto3\";\n\n");
        content.push_str("package gerax;\n\n");

        // Mensagens das entidades
        for entity in &self.entities {
            content.push_str(&format!("message {} {{\n", entity.name));
            for (i, field) in entity.fields.iter().enumerate() {
                let proto_type = field.rust_type.to_proto_type();
                content.push_str(&format!("  {} {} = {};\n", proto_type, field.name, i + 1));
            }
            content.push_str("}\n\n");
        }

        // Mensagens de request/response e serviço
        content.push_str("service EntityService {\n");
        for entity in &self.entities {
            let entity_name = &entity.name;

            // Create
            content.push_str(&format!(
                "  rpc Create{}(Create{}Request) returns (Create{}Response);\n",
                entity_name, entity_name, entity_name
            ));
            content.push_str(&format!("message Create{}Request {{\n", entity_name));
            for (i, field) in entity.fields.iter().enumerate() {
                let proto_type = field.rust_type.to_proto_type();
                content.push_str(&format!("  {} {} = {};\n", proto_type, field.name, i + 1));
            }
            content.push_str("}\n\n");
            content.push_str(&format!("message Create{}Response {{\n", entity_name));
            content.push_str(&format!("  {} entity = 1;\n", entity_name));
            content.push_str("}\n\n");

            // Read
            content.push_str(&format!(
                "  rpc Read{}(Read{}Request) returns (Read{}Response);\n",
                entity_name, entity_name, entity_name
            ));
            content.push_str(&format!("message Read{}Request {{\n", entity_name));
            content.push_str("  string id = 1;\n");
            content.push_str("}\n\n");
            content.push_str(&format!("message Read{}Response {{\n", entity_name));
            content.push_str(&format!("  {} entity = 1;\n", entity_name));
            content.push_str("}\n\n");

            // Update
            content.push_str(&format!(
                "  rpc Update{}(Update{}Request) returns (Update{}Response);\n",
                entity_name, entity_name, entity_name
            ));
            content.push_str(&format!("message Update{}Request {{\n", entity_name));
            content.push_str("  string id = 1;\n");
            for (i, field) in entity.fields.iter().enumerate() {
                let proto_type = field.rust_type.to_proto_type();
                content.push_str(&format!("  {} {} = {};\n", proto_type, field.name, i + 2));
            }
            content.push_str("}\n\n");
            content.push_str(&format!("message Update{}Response {{\n", entity_name));
            content.push_str("  bool success = 1;\n");
            content.push_str("}\n\n");

            // Delete
            content.push_str(&format!(
                "  rpc Delete{}(Delete{}Request) returns (Delete{}Response);\n",
                entity_name, entity_name, entity_name
            ));
            content.push_str(&format!("message Delete{}Request {{\n", entity_name));
            content.push_str("  string id = 1;\n");
            content.push_str("}\n\n");
            content.push_str(&format!("message Delete{}Response {{\n", entity_name));
            content.push_str("  bool success = 1;\n");
            content.push_str("}\n\n");

            // List
            content.push_str(&format!(
                "  rpc List{}(List{}Request) returns (List{}Response);\n",
                entity_name, entity_name, entity_name
            ));
            content.push_str(&format!("message List{}Request {{\n", entity_name));
            content.push_str("}\n\n");
            content.push_str(&format!("message List{}Response {{\n", entity_name));
            content.push_str(&format!("  repeated {} items = 1;\n", entity_name));
            content.push_str("}\n\n");
        }
        content.push_str("}\n");

        ProtoSchema::new("gerax.proto", content)
    }
}
