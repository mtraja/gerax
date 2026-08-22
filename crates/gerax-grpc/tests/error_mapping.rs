//! Testes de mapeamento de erros (`DbError` -> `GrpcError`).

use gerax_db::DbError;
use gerax_grpc::GrpcError;

#[test]
fn db_not_found_maps_to_grpc_not_found() {
    let err = GrpcError::from(DbError::NotFoundError("missing-id".to_string()));
    assert!(matches!(err, GrpcError::NotFound(id) if id == "missing-id"));
}

#[test]
fn db_serialization_error_maps_to_grpc_serialization_error() {
    let err = GrpcError::from(DbError::SerializationError("bad utf8".to_string()));
    assert!(matches!(err, GrpcError::SerializationError(msg) if msg == "bad utf8"));
}

#[test]
fn db_connection_error_maps_to_grpc_rpc_error() {
    let io_err = std::io::Error::other("connection refused");
    let err = GrpcError::from(DbError::ConnectionError(Box::new(io_err)));
    assert!(matches!(err, GrpcError::RpcError(_)));
}
