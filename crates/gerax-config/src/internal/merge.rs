use serde_json::Value;

/// Realiza merge recursivo entre duas árvores de configuração.
///
/// `source` sobrescreve valores existentes em `target`.
///
/// Regras:
///
/// - Objetos são mesclados recursivamente.
/// - Valores simples são substituídos.
/// - Arrays são substituídos completamente.
pub(crate) fn merge(
    target: &mut Value,
    source: Value,
) {
    match (target, source) {

        (
            Value::Object(target_map),
            Value::Object(source_map),
        ) => {

            for (key, value) in source_map {

                match target_map.get_mut(&key) {

                    Some(existing) => {
                        merge(existing, value);
                    }

                    None => {
                        target_map.insert(
                            key,
                            value,
                        );
                    }
                }
            }
        }


        (target, source) => {
            *target = source;
        }
    }
}