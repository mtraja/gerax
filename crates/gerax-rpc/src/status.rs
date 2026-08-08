//! Status de resposta RPC.

/// Status de resposta RPC.
///
/// Segue a semântica do gRPC status codes, mas é independente de protocolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RpcStatus {
    #[default]
    /// Operação concluída com sucesso.
    Ok,

    /// Requisição cancelada pelo cliente.
    Cancelled,

    /// Valor desconhecido/indefinido.
    Unknown,

    /// Argumento inválido (ex: validação falhou).
    InvalidArgument,

    /// Tempo limite excedido.
    DeadlineExceeded,

    /// Recurso não encontrado.
    NotFound,

    /// Recurso já existe (ex: create duplicado).
    AlreadyExists,

    /// Permissão negada.
    PermissionDenied,

    /// Recurso esgotado (ex: quota, rate limit).
    ResourceExhausted,

    /// Pré-condição falhou.
    FailedPrecondition,

    /// Operação abortada.
    Aborted,

    /// Tentativa fora do intervalo válido.
    OutOfRange,

    /// Operação não implementada.
    Unimplemented,

    /// Erro interno do servidor.
    Internal,

    /// Serviço indisponível.
    Unavailable,

    /// Erro de autenticação.
    Unauthenticated,
}

impl RpcStatus {
    /// Retorna o código numérico do status.
    pub fn code(&self) -> i32 {
        match self {
            RpcStatus::Ok => 0,
            RpcStatus::Cancelled => 1,
            RpcStatus::Unknown => 2,
            RpcStatus::InvalidArgument => 3,
            RpcStatus::DeadlineExceeded => 4,
            RpcStatus::NotFound => 5,
            RpcStatus::AlreadyExists => 6,
            RpcStatus::PermissionDenied => 7,
            RpcStatus::ResourceExhausted => 8,
            RpcStatus::FailedPrecondition => 9,
            RpcStatus::Aborted => 10,
            RpcStatus::OutOfRange => 11,
            RpcStatus::Unimplemented => 12,
            RpcStatus::Internal => 13,
            RpcStatus::Unavailable => 14,
            RpcStatus::Unauthenticated => 16,
        }
    }

    /// Retorna a descrição do status.
    pub fn description(&self) -> &'static str {
        match self {
            RpcStatus::Ok => "OK",
            RpcStatus::Cancelled => "Cancelled",
            RpcStatus::Unknown => "Unknown",
            RpcStatus::InvalidArgument => "Invalid Argument",
            RpcStatus::DeadlineExceeded => "Deadline Exceeded",
            RpcStatus::NotFound => "Not Found",
            RpcStatus::AlreadyExists => "Already Exists",
            RpcStatus::PermissionDenied => "Permission Denied",
            RpcStatus::ResourceExhausted => "Resource Exhausted",
            RpcStatus::FailedPrecondition => "Failed Precondition",
            RpcStatus::Aborted => "Aborted",
            RpcStatus::OutOfRange => "Out of Range",
            RpcStatus::Unimplemented => "Unimplemented",
            RpcStatus::Internal => "Internal",
            RpcStatus::Unavailable => "Unavailable",
            RpcStatus::Unauthenticated => "Unauthenticated",
        }
    }
}

impl From<i32> for RpcStatus {
    fn from(code: i32) -> Self {
        match code {
            0 => RpcStatus::Ok,
            1 => RpcStatus::Cancelled,
            2 => RpcStatus::Unknown,
            3 => RpcStatus::InvalidArgument,
            4 => RpcStatus::DeadlineExceeded,
            5 => RpcStatus::NotFound,
            6 => RpcStatus::AlreadyExists,
            7 => RpcStatus::PermissionDenied,
            8 => RpcStatus::ResourceExhausted,
            9 => RpcStatus::FailedPrecondition,
            10 => RpcStatus::Aborted,
            11 => RpcStatus::OutOfRange,
            12 => RpcStatus::Unimplemented,
            13 => RpcStatus::Internal,
            14 => RpcStatus::Unavailable,
            16 => RpcStatus::Unauthenticated,
            _ => RpcStatus::Unknown,
        }
    }
}
