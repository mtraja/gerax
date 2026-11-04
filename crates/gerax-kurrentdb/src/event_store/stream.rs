//! Construção e interpretação de nomes de stream.

/// Separador entre `aggregate_type` e `aggregate_id`.
pub const STREAM_SEPARATOR: char = '-';

/// Nome canônico da stream de um Aggregate.
///
/// ```text
/// {aggregate_type}-{aggregate_id}
/// ```
///
/// Exemplos: `Aluno-123`, `Paciente-456`, `Pedido-789`.
pub fn stream_name(aggregate_type: &str, aggregate_id: &str) -> String {
    format!("{aggregate_type}{STREAM_SEPARATOR}{aggregate_id}")
}

/// Interpreta um nome de stream no formato `{aggregate_type}-{aggregate_id}`.
///
/// O primeiro separador é usado para dividir os campos; o restante
/// (inclusive separadores adicionais) pertence ao `aggregate_id`. Isso
/// permite identificadores como `aluno-2026-0001`.
///
/// Retorna `None` quando o nome não contém o separador.
pub fn parse_stream_name(stream: &str) -> Option<(&str, &str)> {
    stream
        .split_once(STREAM_SEPARATOR)
        .filter(|(aggregate_type, aggregate_id)| {
            !aggregate_type.is_empty() && !aggregate_id.is_empty()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constroi_nome_no_padrao_do_gerax() {
        assert_eq!(stream_name("Aluno", "123"), "Aluno-123");
        assert_eq!(stream_name("Paciente", "456"), "Paciente-456");
        assert_eq!(stream_name("Pedido", "789"), "Pedido-789");
    }

    #[test]
    fn interpreta_nome_com_separadores_no_identificador() {
        let (aggregate_type, aggregate_id) = parse_stream_name("Aluno-2026-0001").unwrap();
        assert_eq!(aggregate_type, "Aluno");
        assert_eq!(aggregate_id, "2026-0001");
    }

    #[test]
    fn rejeita_nomes_sem_separador_ou_com_campos_vazios() {
        assert!(parse_stream_name("Aluno123").is_none());
        assert!(parse_stream_name("-123").is_none());
        assert!(parse_stream_name("Aluno-").is_none());
    }
}
