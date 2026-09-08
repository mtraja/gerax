//! Entidades e mensagens do domínio.

use std::sync::atomic::{AtomicU64, Ordering};

use gerax_cqrs::{Command, CommandMetadata, Message, Query, QueryMetadata};
use gerax_macros::{Command, Query};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq)]
pub struct Aluno {
    pub id: u64,
    pub nome: String,
    pub email: String,
}

#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}

#[derive(Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}

impl CreateAluno {
    pub fn novo_id() -> u64 {
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

#[allow(dead_code)]
pub fn assert_names() {
    assert_eq!(CreateAluno::NAME, "CreateAluno");
    assert_eq!(GetAluno::NAME, "GetAluno");
}

#[allow(dead_code)]
fn assert_message_bounds() {
    fn is_message<T: Message + Send + Sync + 'static>() {}
    is_message::<CreateAluno>();
    is_message::<GetAluno>();
}

#[allow(dead_code)]
fn assert_semantics() {
    fn is_command<T: Command>() {}
    fn is_query<Q: Query>() {}
    is_command::<CreateAluno>();
    is_query::<GetAluno>();
}

#[allow(dead_code)]
fn assert_command_metadata<T: CommandMetadata>() {}
#[allow(dead_code)]
fn assert_query_metadata<Q: QueryMetadata>() {}

#[allow(dead_code)]
fn assert_metadata() {
    assert_command_metadata::<CreateAluno>();
    assert_query_metadata::<GetAluno>();
}
